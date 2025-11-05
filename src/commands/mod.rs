pub mod stats;

use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::layout::args::BlockNames;
use crate::layout::block::Config;
use crate::layout::errors::LayoutError;
use crate::layout::settings::Endianness;
use crate::output;
use crate::output::DataRange;
use crate::output::errors::OutputError;
use crate::variant::DataSheet;
use crate::writer::write_output;
use rayon::prelude::*;
use stats::{BlockStat, BuildStats};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Represents a resolved block ready for building
#[derive(Debug, Clone)]
struct ResolvedBlock {
    name: String,
    file: String,
}

/// Result of building a single block's bytestream and metadata
struct BlockBuildResult {
    block_names: BlockNames,
    data_range: DataRange,
    stat: BlockStat,
}

/// Phase 1: Resolve all blocks we need to build
/// - Load all unique layout files in parallel
/// - Expand any "all blocks" specifications (where name is empty)
/// - Deduplicate to avoid building the same block twice
fn resolve_blocks(
    block_args: &[BlockNames],
) -> Result<(Vec<ResolvedBlock>, HashMap<String, Config>), LayoutError> {
    // Collect all unique filenames we need to load
    let unique_files: HashSet<String> = block_args.iter().map(|b| b.file.clone()).collect();

    // Load all layouts in parallel
    let layouts: Result<HashMap<String, Config>, LayoutError> = unique_files
        .par_iter()
        .map(|file| layout::load_layout(file).map(|cfg| (file.clone(), cfg)))
        .collect();

    let layouts = layouts?;

    // Resolve each block argument to concrete blocks
    let mut resolved = Vec::new();
    for arg in block_args {
        if arg.name.is_empty() {
            // Expand to all blocks in this file
            let layout = layouts.get(&arg.file).ok_or_else(|| {
                LayoutError::FileError(format!("Layout not loaded: {}", arg.file))
            })?;

            for block_name in layout.blocks.keys() {
                resolved.push(ResolvedBlock {
                    name: block_name.clone(),
                    file: arg.file.clone(),
                });
            }
        } else {
            // Specific block
            resolved.push(ResolvedBlock {
                name: arg.name.clone(),
                file: arg.file.clone(),
            });
        }
    }

    // Deduplicate: use (file, name) pairs
    let mut seen = HashSet::new();
    let deduplicated: Vec<ResolvedBlock> = resolved
        .into_iter()
        .filter(|b| seen.insert((b.file.clone(), b.name.clone())))
        .collect();

    Ok((deduplicated, layouts))
}

/// Phase 2: Build bytestreams in parallel for all resolved blocks
fn build_bytestreams(
    blocks: &[ResolvedBlock],
    layouts: &HashMap<String, Config>,
    data_sheet: Option<&DataSheet>,
    strict: bool,
) -> Result<Vec<BlockBuildResult>, NvmError> {
    blocks
        .par_iter()
        .map(|resolved| build_single_bytestream(resolved, layouts, data_sheet, strict))
        .collect()
}

fn build_single_bytestream(
    resolved: &ResolvedBlock,
    layouts: &HashMap<String, Config>,
    data_sheet: Option<&DataSheet>,
    strict: bool,
) -> Result<BlockBuildResult, NvmError> {
    let layout = layouts
        .get(&resolved.file)
        .ok_or_else(|| LayoutError::FileError(format!("Layout not found: {}", resolved.file)))?;

    let block = layout
        .blocks
        .get(&resolved.name)
        .ok_or_else(|| LayoutError::BlockNotFound(resolved.name.clone()))?;

    let (bytestream, padding_bytes) =
        block.build_bytestream(data_sheet, &layout.settings, strict)?;

    let data_range = output::bytestream_to_datarange(
        bytestream,
        &block.header,
        &layout.settings,
        layout.settings.byte_swap,
        layout.settings.pad_to_end,
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
}

fn extract_crc_value(crc_bytestream: &[u8], endianness: &Endianness) -> u32 {
    match endianness {
        Endianness::Big => u32::from_be_bytes([
            crc_bytestream[0],
            crc_bytestream[1],
            crc_bytestream[2],
            crc_bytestream[3],
        ]),
        Endianness::Little => u32::from_le_bytes([
            crc_bytestream[0],
            crc_bytestream[1],
            crc_bytestream[2],
            crc_bytestream[3],
        ]),
    }
}

/// Phase 3a: Output separate hex files for each block
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

/// Phase 3b: Combine all blocks into a single hex file
fn output_combined_file(
    results: Vec<BlockBuildResult>,
    layouts: &HashMap<String, Config>,
    args: &Args,
) -> Result<BuildStats, NvmError> {
    let mut stats = BuildStats::new();
    let mut ranges = Vec::new();
    let mut block_ranges = Vec::new();

    for result in results {
        let layout = layouts.get(&result.block_names.file).ok_or_else(|| {
            LayoutError::FileError(format!("Layout not found: {}", result.block_names.file))
        })?;

        let block = layout
            .blocks
            .get(&result.block_names.name)
            .ok_or_else(|| LayoutError::BlockNotFound(result.block_names.name.clone()))?;

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

    // Detect overlaps between declared block memory ranges
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

/// Main unified build function
pub fn build(args: &Args, data_sheet: Option<&DataSheet>) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    // Phase 1: Resolve all blocks and load all layouts
    let (resolved_blocks, layouts) = resolve_blocks(&args.layout.blocks)?;

    // Phase 2: Build all bytestreams in parallel
    let results = build_bytestreams(&resolved_blocks, &layouts, data_sheet, args.layout.strict)?;

    // Phase 3: Output based on combined flag
    let mut stats = if args.output.combined {
        output_combined_file(results, &layouts, args)?
    } else {
        output_separate_blocks(results, args)?
    };

    stats.total_duration = start_time.elapsed();
    Ok(stats)
}
