pub mod args;
pub mod errors;
pub mod report;

use crate::layout::header::Header;
use crate::layout::settings::Settings;
use errors::OutputError;

/// Swaps bytes pairwise for word-addressing mode.
fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
}

#[derive(Debug, Clone)]
pub struct DataRange {
    pub start_address: u32,
    pub bytestream: Vec<u8>,
    pub used_size: u32,
    pub allocated_size: u32,
}

pub fn bytestream_to_datarange(
    mut bytestream: Vec<u8>,
    header: &Header,
    settings: &Settings,
    padding_bytes: u32,
) -> Result<DataRange, OutputError> {
    let addr_mult: u32 = if settings.word_addressing { 2 } else { 1 };
    let block_len_bytes = header.length.checked_mul(addr_mult).ok_or_else(|| {
        OutputError::HexOutputError("Block length overflows address space.".to_string())
    })?;

    if bytestream.len() > block_len_bytes as usize {
        return Err(OutputError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    // Apply byte swap for word-addressing mode.
    if settings.word_addressing {
        if !bytestream.len().is_multiple_of(2) {
            bytestream.push(header.padding);
        }
        byte_swap_inplace(&mut bytestream);
    }

    let used_size = (bytestream.len() as u32).saturating_sub(padding_bytes);
    let start_address = header.start_address * addr_mult + settings.virtual_offset;

    Ok(DataRange {
        start_address,
        bytestream,
        used_size,
        allocated_size: block_len_bytes,
    })
}
