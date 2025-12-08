pub mod args;
pub mod checksum;
pub mod errors;

use crate::layout::header::{CrcLocation, Header};
use crate::layout::settings::{CrcArea, Endianness, Settings};
use crate::output::args::OutputFormat;
use errors::OutputError;

use bin_file::{BinFile, IHexFormat};

/// Output data range with optional CRC.
#[derive(Debug, Clone)]
pub struct DataRange {
    pub start_address: u32,
    pub bytestream: Vec<u8>,
    /// CRC address and bytes. None if CRC is disabled.
    pub crc: Option<(u32, Vec<u8>)>,
    pub used_size: u32,
    pub allocated_size: u32,
}

fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
}

fn validate_crc_location(
    length: usize,
    header: &Header,
    crc_location: &CrcLocation,
) -> Result<u32, OutputError> {
    let crc_offset = match crc_location {
        CrcLocation::Address(address) => {
            let crc_offset = address.checked_sub(header.start_address).ok_or_else(|| {
                OutputError::HexOutputError("CRC address before block start.".to_string())
            })?;

            if crc_offset < length as u32 {
                return Err(OutputError::HexOutputError(
                    "CRC overlaps with payload.".to_string(),
                ));
            }

            crc_offset
        }
        CrcLocation::Keyword(option) => match option.as_str() {
            "end" => (length as u32 + 3) & !3,
            _ => {
                return Err(OutputError::HexOutputError(format!(
                    "Invalid CRC location: {}",
                    option
                )));
            }
        },
    };

    if header.length < crc_offset + 4 {
        return Err(OutputError::HexOutputError(
            "CRC location would overrun block.".to_string(),
        ));
    }

    Ok(crc_offset)
}

pub fn bytestream_to_datarange(
    mut bytestream: Vec<u8>,
    header: &Header,
    settings: &Settings,
    byte_swap: bool,
    pad_to_end: bool,
    padding_bytes: u32,
) -> Result<DataRange, OutputError> {
    if bytestream.len() > header.length as usize {
        return Err(OutputError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    // Apply optional byte swap across the entire stream before CRC
    if byte_swap {
        if bytestream.len() % 2 != 0 {
            bytestream.push(header.padding);
        }
        byte_swap_inplace(bytestream.as_mut_slice());
    }

    let allocated_size = header.length;

    // CRC calculation only if both settings.crc and header.crc_location are present
    let crc = match (&settings.crc, &header.crc_location) {
        (Some(crc_settings), Some(crc_loc)) => {
            let crc_offset = validate_crc_location(bytestream.len(), header, crc_loc)?;

            // Padding for CRC alignment
            if matches!(crc_loc, CrcLocation::Keyword(_)) {
                bytestream.resize(crc_offset as usize, header.padding);
            }

            // Fill whole block if the CRC area is block
            if crc_settings.area == CrcArea::Block {
                bytestream.resize(header.length as usize, header.padding);
                bytestream[crc_offset as usize..(crc_offset + 4) as usize].fill(0);
            }

            // Compute CRC based on selected area
            let crc_val = checksum::calculate_crc(&bytestream, crc_settings);

            let mut crc_bytes: [u8; 4] = match settings.endianness {
                Endianness::Big => crc_val.to_be_bytes(),
                Endianness::Little => crc_val.to_le_bytes(),
            };
            if byte_swap {
                byte_swap_inplace(&mut crc_bytes);
            }

            let crc_address = header.start_address + settings.virtual_offset + crc_offset;
            Some((crc_address, crc_bytes.to_vec()))
        }
        _ => None,
    };

    let used_size = if crc.is_some() {
        ((bytestream.len() as u32).saturating_add(4)).saturating_sub(padding_bytes)
    } else {
        (bytestream.len() as u32).saturating_sub(padding_bytes)
    };

    // Resize to full block if pad_to_end is true
    if pad_to_end {
        bytestream.resize(header.length as usize, header.padding);
    }

    Ok(DataRange {
        start_address: header.start_address + settings.virtual_offset,
        bytestream,
        crc,
        used_size,
        allocated_size,
    })
}

pub fn emit_hex(
    ranges: &[DataRange],
    record_width: usize,
    format: OutputFormat,
) -> Result<String, OutputError> {
    if !(1..=128).contains(&record_width) {
        return Err(OutputError::HexOutputError(
            "Record width must be between 1 and 128".to_string(),
        ));
    }

    // Use bin_file to format output.
    let mut bf = BinFile::new();
    let mut max_end: usize = 0;

    for range in ranges {
        bf.add_bytes(
            range.bytestream.as_slice(),
            Some(range.start_address as usize),
            false,
        )
        .map_err(|e| OutputError::HexOutputError(format!("Failed to add bytes: {}", e)))?;

        let end = (range.start_address as usize).saturating_add(range.bytestream.len());
        if end > max_end {
            max_end = end;
        }

        if let Some((crc_address, crc_bytestream)) = &range.crc {
            bf.add_bytes(crc_bytestream.as_slice(), Some(*crc_address as usize), true)
                .map_err(|e| OutputError::HexOutputError(format!("Failed to add bytes: {}", e)))?;

            let end = (*crc_address as usize).saturating_add(crc_bytestream.len());
            if end > max_end {
                max_end = end;
            }
        }
    }

    match format {
        OutputFormat::Hex => {
            let ihex_format = if max_end <= 0x1_0000 {
                IHexFormat::IHex16
            } else {
                IHexFormat::IHex32
            };
            let lines = bf.to_ihex(Some(record_width), ihex_format).map_err(|e| {
                OutputError::HexOutputError(format!("Failed to generate Intel HEX: {}", e))
            })?;
            Ok(lines.join("\n"))
        }
        OutputFormat::Mot => {
            use bin_file::SRecordAddressLength;
            let addr_len = if max_end <= 0x1_0000 {
                SRecordAddressLength::Length16
            } else if max_end <= 0x100_0000 {
                SRecordAddressLength::Length24
            } else {
                SRecordAddressLength::Length32
            };
            let lines = bf.to_srec(Some(record_width), addr_len).map_err(|e| {
                OutputError::HexOutputError(format!("Failed to generate S-Record: {}", e))
            })?;
            Ok(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::header::CrcLocation;
    use crate::layout::header::Header;
    use crate::layout::settings::Endianness;
    use crate::layout::settings::Settings;
    use crate::layout::settings::{CrcArea, CrcData};

    fn sample_crc_data() -> CrcData {
        CrcData {
            polynomial: 0x04C11DB7,
            start: 0xFFFF_FFFF,
            xor_out: 0xFFFF_FFFF,
            ref_in: true,
            ref_out: true,
            area: CrcArea::Data,
        }
    }

    fn sample_settings() -> Settings {
        Settings {
            endianness: Endianness::Little,
            virtual_offset: 0,
            crc: Some(sample_crc_data()),
            byte_swap: false,
            pad_to_end: false,
        }
    }

    fn sample_header(len: u32) -> Header {
        Header {
            start_address: 0,
            length: len,
            crc_location: Some(CrcLocation::Keyword("end".to_string())),
            padding: 0xFF,
        }
    }

    #[test]
    fn pad_to_end_false_resizes_to_crc_end_only() {
        let settings = sample_settings();
        let header = sample_header(16);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");
        let hex = emit_hex(&[dr], 16, crate::output::args::OutputFormat::Hex)
            .expect("hex generation failed");

        // No in-memory resize when pad_to_end=false; CRC is emitted separately
        assert_eq!(bytestream.len(), 4);

        // And the emitted hex should contain the CRC bytes (endianness applied)
        let crc_loc = header.crc_location.as_ref().unwrap();
        let crc_offset = super::validate_crc_location(4usize, &header, crc_loc).expect("crc loc");
        assert_eq!(crc_offset as usize, 4, "crc should follow payload end");
        let crc_val =
            checksum::calculate_crc(&bytestream[..crc_offset as usize], &sample_crc_data());
        let crc_bytes = match settings.endianness {
            Endianness::Big => crc_val.to_be_bytes(),
            Endianness::Little => crc_val.to_le_bytes(),
        };
        // No byte swap in this test
        use std::fmt::Write;
        let expected_crc_ascii =
            crc_bytes
                .iter()
                .fold(String::new(), |mut s, b| {
                    let _ = write!(s, "{:02X}", b);
                    s
                });
        assert!(
            hex.to_uppercase().contains(&expected_crc_ascii),
            "hex should contain CRC bytes"
        );
    }
    #[test]
    fn pad_to_end_true_resizes_to_full_block() {
        let settings = sample_settings();
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, false, true, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
    }
}
