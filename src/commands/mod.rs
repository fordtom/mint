pub mod stats;
mod writer;

use crate::args::Args;
use crate::data::DataSource;
use crate::error::NvmError;
use crate::layout;
use crate::layout::args::BlockNames;
use crate::layout::block::Config;
use crate::layout::errors::LayoutError;
use crate::layout::settings::Endianness;
use crate::output;
use crate::output::DataRange;
use crate::output::errors::OutputError;
use rayon::prelude::*;
use stats::{BlockStat, BuildStats};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use writer::write_output;

#[derive(Debug, Clone)]
struct ResolvedBlock {
    name: String,
    file: String,
}

struct BlockBuildResult {
    block_names: BlockNames,
    data_range: DataRange,
    stat: BlockStat,
}

fn resolve_blocks(
    block_args: &[BlockNames],
) -> Result<(Vec<ResolvedBlock>, HashMap<String, Config>), LayoutError> {
    let unique_files: HashSet<String> = block_args.iter().map(|b| b.file.clone()).collect();

    let layouts: Result<HashMap<String, Config>, LayoutError> = unique_files
        .par_iter()
        .map(|file| layout::load_layout(file).map(|cfg| (file.clone(), cfg)))
        .collect();

    let layouts = layouts?;

    let mut resolved = Vec::new();
    for arg in block_args {
        if arg.name.is_empty() {
            let layout = &layouts[&arg.file];
            for block_name in layout.blocks.keys() {
                resolved.push(ResolvedBlock {
                    name: block_name.clone(),
                    file: arg.file.clone(),
                });
            }
        } else {
            resolved.push(ResolvedBlock {
                name: arg.name.clone(),
                file: arg.file.clone(),
            });
        }
    }

    let mut seen = HashSet::new();
    let deduplicated: Vec<ResolvedBlock> = resolved
        .into_iter()
        .filter(|b| seen.insert((b.file.clone(), b.name.clone())))
        .collect();

    Ok((deduplicated, layouts))
}

fn build_bytestreams(
    blocks: &[ResolvedBlock],
    layouts: &HashMap<String, Config>,
    data_source: Option<&dyn DataSource>,
    strict: bool,
) -> Result<Vec<BlockBuildResult>, NvmError> {
    blocks
        .par_iter()
        .map(|resolved| build_single_bytestream(resolved, layouts, data_source, strict))
        .collect()
}

fn build_single_bytestream(
    resolved: &ResolvedBlock,
    layouts: &HashMap<String, Config>,
    data_source: Option<&dyn DataSource>,
    strict: bool,
) -> Result<BlockBuildResult, NvmError> {
    let result = (|| {
        let layout = &layouts[&resolved.file];
        let block = &layout.blocks[&resolved.name];

        let (bytestream, padding_bytes) =
            block.build_bytestream(data_source, &layout.settings, strict)?;

        let data_range = output::bytestream_to_datarange(
            bytestream,
            &block.header,
            &layout.settings,
            padding_bytes,
        )?;

        let crc_value = extract_crc_value(&data_range.crc_bytestream, &layout.settings.endianness);

        let stat = BlockStat {
            name: resolved.name.clone(),
            start_address: data_range.start_address,
            allocated_size: data_range.allocated_size,
            used_size: data_range.used_size,
            crc_value,
        };

        Ok(BlockBuildResult {
            block_names: BlockNames {
                name: resolved.name.clone(),
                file: resolved.file.clone(),
            },
            data_range,
            stat,
        })
    })();

    result.map_err(|e| NvmError::InBlock {
        block_name: resolved.name.clone(),
        layout_file: resolved.file.clone(),
        source: Box::new(e),
    })
}

fn extract_crc_value(crc_bytestream: &[u8], endianness: &Endianness) -> Option<u32> {
    if crc_bytestream.len() < 4 {
        return None;
    }
    let bytes: [u8; 4] = crc_bytestream[..4].try_into().ok()?;
    Some(match endianness {
        Endianness::Big => u32::from_be_bytes(bytes),
        Endianness::Little => u32::from_le_bytes(bytes),
    })
}

fn output_separate_blocks(
    results: Vec<BlockBuildResult>,
    args: &Args,
) -> Result<BuildStats, NvmError> {
    let block_stats: Result<Vec<BlockStat>, NvmError> = results
        .par_iter()
        .map(|result| {
            let hex_string = output::emit_hex(
                std::slice::from_ref(&result.data_range),
                args.output.record_width as usize,
                args.output.format,
            )?;

            write_output(&args.output, &result.block_names.name, &hex_string)?;
            Ok(result.stat.clone())
        })
        .collect();

    let block_stats = block_stats?;

    let mut stats = BuildStats::new();
    for stat in block_stats {
        stats.add_block(stat);
    }

    Ok(stats)
}

fn output_combined_file(
    results: Vec<BlockBuildResult>,
    layouts: &HashMap<String, Config>,
    args: &Args,
) -> Result<BuildStats, NvmError> {
    let mut stats = BuildStats::new();
    let mut ranges = Vec::new();
    let mut block_ranges = Vec::new();

    for result in results {
        let layout = &layouts[&result.block_names.file];
        let block = &layout.blocks[&result.block_names.name];

        let start = block
            .header
            .start_address
            .checked_add(layout.settings.virtual_offset)
            .ok_or(LayoutError::InvalidBlockArgument(
                "start_address + virtual_offset overflow".into(),
            ))?;

        let end =
            start
                .checked_add(block.header.length)
                .ok_or(LayoutError::InvalidBlockArgument(
                    "start + length overflow".into(),
                ))?;

        stats.add_block(result.stat);
        ranges.push(result.data_range);
        block_ranges.push((result.block_names.name.clone(), start, end));
    }

    check_overlaps(&block_ranges)?;

    let hex_string = output::emit_hex(
        &ranges,
        args.output.record_width as usize,
        args.output.format,
    )?;

    write_output(&args.output, "combined", &hex_string)?;

    Ok(stats)
}

fn check_overlaps(block_ranges: &[(String, u32, u32)]) -> Result<(), NvmError> {
    for i in 0..block_ranges.len() {
        for j in (i + 1)..block_ranges.len() {
            let (ref name_a, a_start, a_end) = block_ranges[i];
            let (ref name_b, b_start, b_end) = block_ranges[j];

            let overlap_start = a_start.max(b_start);
            let overlap_end = a_end.min(b_end);

            if overlap_start < overlap_end {
                let overlap_size = overlap_end - overlap_start;
                let msg = format!(
                    "Block '{}' (0x{:08X}-0x{:08X}) overlaps with block '{}' (0x{:08X}-0x{:08X}). Overlap: 0x{:08X}-0x{:08X} ({} bytes)",
                    name_a,
                    a_start,
                    a_end - 1,
                    name_b,
                    b_start,
                    b_end - 1,
                    overlap_start,
                    overlap_end - 1,
                    overlap_size
                );
                return Err(OutputError::BlockOverlapError(msg).into());
            }
        }
    }
    Ok(())
}

pub fn build(args: &Args, data_source: Option<&dyn DataSource>) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    let (resolved_blocks, layouts) = resolve_blocks(&args.layout.blocks)?;
    let results = build_bytestreams(&resolved_blocks, &layouts, data_source, args.layout.strict)?;

    let mut stats = if args.output.combined {
        output_combined_file(results, &layouts, args)?
    } else {
        output_separate_blocks(results, args)?
    };

    stats.total_duration = start_time.elapsed();
    Ok(stats)
}
