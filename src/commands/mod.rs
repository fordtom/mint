pub mod stats;

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
use crate::output::DataRange;
use crate::output::errors::OutputError;
use h3xy::{HexFile, Segment};
use rayon::prelude::*;
use stats::{BlockStat, BuildStats};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

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

    let mut resolved = Vec::with_capacity(block_args.len());
    for arg in block_args {
        if arg.name.is_empty() {
            return Err(LayoutError::InvalidBlockArgument(format!(
                "Expected BLOCK@FILE, got '@{}'",
                arg.file
            )));
        }
        resolved.push(ResolvedBlock {
            name: arg.name.clone(),
            file: arg.file.clone(),
        });
    }

    Ok((resolved, layouts))
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
        let block = layout.blocks.get(&resolved.name).ok_or_else(|| {
            LayoutError::BlockNotFound(format!("{}@{}", resolved.name, resolved.file))
        })?;
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
    let mut blocks: HashMap<String, HexFile> = HashMap::with_capacity(results.len());
    for (idx, result) in results.into_iter().enumerate() {
        stats.add_block(result.stat);
        let mut hexfile = HexFile::new();
        hexfile.append_segment(Segment::new(
            result.data_range.start_address,
            result.data_range.bytestream,
        ));
        if !result.data_range.crc_bytestream.is_empty() {
            hexfile.append_segment(Segment::new(
                result.data_range.crc_address,
                result.data_range.crc_bytestream,
            ));
        }
        blocks.insert(format!("@{}", idx + 1), hexfile);
    }

    h3xy::cli::execute_in_memory(&args.output.hexview, &blocks)
        .map_err(|e| OutputError::HexOutputError(format!("h3xy: {e}")))?;
    Ok(stats)
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
        let file_entry = report
            .entry(result.block_names.file.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        let serde_json::Value::Object(blocks) = file_entry else {
            return Err(OutputError::FileError(
                "JSON export contains unexpected non-object entry.".to_string(),
            )
            .into());
        };
        if blocks.contains_key(&result.block_names.name) {
            return Err(OutputError::FileError(format!(
                "Duplicate block '{}' in JSON export for file '{}'.",
                result.block_names.name, result.block_names.file
            ))
            .into());
        }
        blocks.insert(result.block_names.name.clone(), value);
    }
    Ok(serde_json::Value::Object(report))
}
