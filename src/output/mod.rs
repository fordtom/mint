pub mod args;
pub mod errors;
pub mod report;

use crate::layout::header::Header;
use errors::OutputError;

#[derive(Debug, Clone)]
pub struct DataRange {
    pub start_address: u32,
    pub bytestream: Vec<u8>,
    pub used_size: u32,
    pub allocated_size: u32,
}

pub fn bytestream_to_datarange(
    bytestream: Vec<u8>,
    header: &Header,
    padding_bytes: u32,
) -> Result<DataRange, OutputError> {
    let block_len_bytes = header.length;

    if bytestream.len() > block_len_bytes as usize {
        return Err(OutputError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    let used_size = (bytestream.len() as u32).saturating_sub(padding_bytes);
    let start_address = header.start_address;

    Ok(DataRange {
        start_address,
        bytestream,
        used_size,
        allocated_size: block_len_bytes,
    })
}
