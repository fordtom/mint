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
use crate::layout::used_values::{NoopValueSink, ValueCollector};
use crate::output;
use crate::output::errors::OutputError;
use crate::output::{DataRange, OutputFile};
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
    used_values: Option<serde_json::Value>,
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
    capture_values: bool,
) -> Result<Vec<BlockBuildResult>, NvmError> {
    blocks
        .par_iter()
        .map(|resolved| {
            build_single_bytestream(resolved, layouts, data_source, strict, capture_values)
        })
        .collect()
}

fn build_single_bytestream(
    resolved: &ResolvedBlock,
    layouts: &HashMap<String, Config>,
    data_source: Option<&dyn DataSource>,
    strict: bool,
    capture_values: bool,
) -> Result<BlockBuildResult, NvmError> {
    let result = (|| {
        let layout = &layouts[&resolved.file];
        let block = &layout.blocks[&resolved.name];
        let mut collector = ValueCollector::new();
        let mut noop = NoopValueSink;
        let value_sink = if capture_values {
            &mut collector as &mut dyn crate::layout::used_values::ValueSink
        } else {
            &mut noop as &mut dyn crate::layout::used_values::ValueSink
        };

        let (bytestream, padding_bytes) =
            block.build_bytestream(data_source, &layout.settings, strict, value_sink)?;

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
            used_values: capture_values.then(|| collector.into_value()),
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

fn output_results(results: Vec<BlockBuildResult>, args: &Args) -> Result<BuildStats, NvmError> {
    let mut stats = BuildStats::new();
    let named_ranges: Vec<(String, DataRange)> = results
        .into_iter()
        .map(|r| {
            stats.add_block(r.stat);
            (r.block_names.name, r.data_range)
        })
        .collect();

    check_overlaps(&named_ranges)?;
    let ranges: Vec<DataRange> = named_ranges.into_iter().map(|(_, r)| r).collect();
    let output_file = OutputFile {
        ranges,
        format: args.output.format,
        record_width: args.output.record_width as usize,
    };

    write_output(&output_file, &args.output)?;
    Ok(stats)
}

fn check_overlaps(named_ranges: &[(String, DataRange)]) -> Result<(), NvmError> {
    for i in 0..named_ranges.len() {
        for j in (i + 1)..named_ranges.len() {
            let (ref name_a, ref range_a) = named_ranges[i];
            let (ref name_b, ref range_b) = named_ranges[j];
            let a_start = range_a.start_address;
            let a_end = a_start + range_a.allocated_size;
            let b_start = range_b.start_address;
            let b_end = b_start + range_b.allocated_size;

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
    let capture_values = args.output.export_json.is_some();
    let mut results = build_bytestreams(
        &resolved_blocks,
        &layouts,
        data_source,
        args.layout.strict,
        capture_values,
    )?;

    if let Some(path) = args.output.export_json.as_ref() {
        let report = take_used_values_report(&mut results)?;
        output::report::write_used_values_json(path, &report)?;
    }

    let mut stats = output_results(results, args)?;

    stats.total_duration = start_time.elapsed();
    Ok(stats)
}

fn take_used_values_report(
    results: &mut [BlockBuildResult],
) -> Result<serde_json::Value, NvmError> {
    let mut report = serde_json::Map::new();
    for result in results {
        let value = result.used_values.take().ok_or_else(|| {
            OutputError::FileError(
                "JSON export requested but values were not captured.".to_string(),
            )
        })?;
        report.insert(result.block_names.name.clone(), value);
    }
    Ok(serde_json::Value::Object(report))
}
