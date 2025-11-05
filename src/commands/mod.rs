pub mod generate;
pub mod stats;

use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::layout::args::BlockNames;
use crate::layout::errors::LayoutError;
use crate::output;
use crate::output::errors::OutputError;
use crate::variant::DataSheet;
use crate::writer::write_output;
use rayon::prelude::*;
use stats::{BlockStat, BuildStats};
use std::time::Instant;

pub fn build_separate_blocks(
    args: &Args,
    blocks: &[BlockNames],
    data_sheet: Option<&DataSheet>,
) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    let block_stats: Result<Vec<BlockStat>, NvmError> = blocks
        .par_iter()
        .map(|input| generate::build_block_single(input, data_sheet, args))
        .collect();

    let block_stats = block_stats?;

    let mut stats = BuildStats::new();
    for stat in block_stats {
        stats.add_block(stat);
    }
    stats.total_duration = start_time.elapsed();

    Ok(stats)
}

pub fn build_single_file(
    args: &Args,
    blocks: &[BlockNames],
    data_sheet: Option<&DataSheet>,
) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    let mut ranges = Vec::new();
    let mut block_ranges: Vec<(String, u32, u32)> = Vec::new();
    let mut stats = BuildStats::new();

    for input in blocks {
        let result =
            (|| {
                let layout = layout::load_layout(&input.file)?;

                let block = layout
                    .blocks
                    .get(&input.name)
                    .ok_or(LayoutError::BlockNotFound(input.name.clone()))?;

                let (bytestream, padding_bytes) =
                    block.build_bytestream(data_sheet, &layout.settings, args.layout.strict)?;

                let dr = output::bytestream_to_datarange(
                    bytestream,
                    &block.header,
                    &layout.settings,
                    layout.settings.byte_swap,
                    layout.settings.pad_to_end,
                    padding_bytes,
                )?;

                let mut crc_bytes = [
                    dr.crc_bytestream[0],
                    dr.crc_bytestream[1],
                    dr.crc_bytestream[2],
                    dr.crc_bytestream[3],
                ];
                if layout.settings.byte_swap {
                    crc_bytes.swap(0, 1);
                    crc_bytes.swap(2, 3);
                }
                let crc_value = match layout.settings.endianness {
                    layout::settings::Endianness::Big => u32::from_be_bytes(crc_bytes),
                    layout::settings::Endianness::Little => u32::from_le_bytes(crc_bytes),
                };

                let stat = BlockStat {
                    name: input.name.clone(),
                    start_address: dr.start_address,
                    allocated_size: dr.allocated_size,
                    used_size: dr.used_size,
                    crc_value,
                };

                let start = block
                    .header
                    .start_address
                    .checked_add(layout.settings.virtual_offset)
                    .ok_or(LayoutError::InvalidBlockArgument(
                        "start_address + virtual_offset overflow".into(),
                    ))?;
                let end = start.checked_add(block.header.length).ok_or(
                    LayoutError::InvalidBlockArgument("start + length overflow".into()),
                )?;

                Ok((dr, stat, start, end))
            })()
            .map_err(|e| NvmError::InBlock {
                block_name: input.name.clone(),
                layout_file: input.file.clone(),
                source: Box::new(e),
            })?;

        let (dr, stat, start, end) = result;
        stats.add_block(stat);
        ranges.push(dr);
        block_ranges.push((input.name.clone(), start, end));
    }

    // Detect overlaps between declared block memory ranges (inclusive start, exclusive end)
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

    let hex_string = output::emit_hex(
        &ranges,
        args.output.record_width as usize,
        args.output.format,
    )?;

    write_output(&args.output, "combined", &hex_string)?;

    stats.total_duration = start_time.elapsed();

    Ok(stats)
}
