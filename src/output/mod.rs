pub mod args;
pub mod checksum;
pub mod errors;

use crate::layout::header::{CrcLocation, Header};
use crate::layout::settings::{CrcArea, CrcData, Endianness, Settings};
use crate::output::args::OutputFormat;
use errors::OutputError;

use bin_file::{BinFile, IHexFormat};

#[derive(Debug, Clone)]
pub struct DataRange {
    pub start_address: u32,
    pub bytestream: Vec<u8>,
    pub crc_address: u32,
    pub crc_bytestream: Vec<u8>,
    pub used_size: u32,
    pub allocated_size: u32,
}

fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
}

/// Returns `(crc_offset, crc_settings)` if CRC is enabled, `None` otherwise.
fn resolve_crc(
    length: usize,
    header: &Header,
    settings: &Settings,
) -> Result<Option<(u32, CrcData)>, OutputError> {
    let header_crc = match &header.crc {
        Some(hc) => hc,
        None => return Ok(None), // No CRC configured for this header
    };

    let crc_location = header_crc.location();

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
            "none" => return Ok(None),
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

    // Resolve CRC settings: header overrides merged with global defaults
    let crc_settings = header_crc.resolve(settings.crc.as_ref()).ok_or_else(|| {
        OutputError::HexOutputError(
            "CRC location specified but missing CRC settings (no [settings.crc] or header overrides).".to_string(),
        )
    })?;

    Ok(Some((crc_offset, crc_settings)))
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
        if !bytestream.len().is_multiple_of(2) {
            bytestream.push(header.padding);
        }
        byte_swap_inplace(bytestream.as_mut_slice());
    }

    // Resolve CRC configuration (location + settings) from header + global defaults
    let crc_config = resolve_crc(bytestream.len(), header, settings)?;

    let mut used_size = (bytestream.len() as u32).saturating_sub(padding_bytes);
    let allocated_size = header.length;

    // If CRC is disabled for this block, return early with no CRC
    let Some((crc_offset, crc_settings)) = crc_config else {
        if pad_to_end {
            bytestream.resize(header.length as usize, header.padding);
        }

        return Ok(DataRange {
            start_address: header.start_address + settings.virtual_offset,
            bytestream,
            crc_address: 0,
            crc_bytestream: Vec::new(),
            used_size,
            allocated_size,
        });
    };

    used_size = used_size.saturating_add(4);

    // Padding for CRC alignment (when using keyword location like "end")
    if let Some(hc) = &header.crc
        && let CrcLocation::Keyword(_) = hc.location()
    {
        bytestream.resize(crc_offset as usize, header.padding);
    }

    // Handle block-level CRC modes
    match crc_settings.area {
        CrcArea::BlockZeroCrc | CrcArea::BlockPadCrc | CrcArea::BlockOmitCrc => {
            bytestream.resize(header.length as usize, header.padding);
        }
        CrcArea::Data => {}
    }

    // Zero CRC location for BlockZeroCrc mode
    if crc_settings.area == CrcArea::BlockZeroCrc {
        bytestream[crc_offset as usize..(crc_offset + 4) as usize].fill(0);
    }

    // Compute CRC - omit CRC bytes for BlockOmitCrc mode
    let crc_val = if crc_settings.area == CrcArea::BlockOmitCrc {
        let before = &bytestream[..crc_offset as usize];
        let after = &bytestream[(crc_offset + 4) as usize..];
        let combined: Vec<u8> = [before, after].concat();
        checksum::calculate_crc(&combined, &crc_settings)
    } else {
        checksum::calculate_crc(&bytestream, &crc_settings)
    };

    let mut crc_bytes: [u8; 4] = match settings.endianness {
        Endianness::Big => crc_val.to_be_bytes(),
        Endianness::Little => crc_val.to_le_bytes(),
    };
    if byte_swap {
        byte_swap_inplace(&mut crc_bytes);
    }

    // Resize to full block if pad_to_end is true
    if pad_to_end {
        bytestream.resize(header.length as usize, header.padding);
    }

    Ok(DataRange {
        start_address: header.start_address + settings.virtual_offset,
        bytestream,
        crc_address: header.start_address + settings.virtual_offset + crc_offset,
        crc_bytestream: crc_bytes.to_vec(),
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

        // Only add CRC bytes if CRC is enabled for this block
        if !range.crc_bytestream.is_empty() {
            bf.add_bytes(
                range.crc_bytestream.as_slice(),
                Some(range.crc_address as usize),
                true,
            )
            .map_err(|e| OutputError::HexOutputError(format!("Failed to add bytes: {}", e)))?;
        }

        let end = (range.start_address as usize).saturating_add(range.bytestream.len());
        if end > max_end {
            max_end = end;
        }
        if !range.crc_bytestream.is_empty() {
            let end = (range.crc_address as usize).saturating_add(range.crc_bytestream.len());
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
    use crate::layout::header::{CrcLocation, Header, HeaderCrc};
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

    fn sample_header_crc() -> HeaderCrc {
        HeaderCrc {
            location: CrcLocation::Keyword("end".to_string()),
            polynomial: None,
            start: None,
            xor_out: None,
            ref_in: None,
            ref_out: None,
            area: None,
        }
    }

    fn sample_header(len: u32) -> Header {
        Header {
            start_address: 0,
            length: len,
            crc: Some(sample_header_crc()),
            padding: 0xFF,
        }
    }

    fn header_no_crc(len: u32) -> Header {
        Header {
            start_address: 0,
            length: len,
            crc: None,
            padding: 0xFF,
        }
    }

    #[test]
    fn pad_to_end_false_resizes_to_crc_end_only() {
        let settings = sample_settings();
        let crc_data = sample_crc_data();
        let header = sample_header(16);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");
        let hex = emit_hex(&[dr], 16, crate::output::args::OutputFormat::Hex)
            .expect("hex generation failed");

        // No in-memory resize when pad_to_end=false; CRC is emitted separately
        assert_eq!(bytestream.len(), 4);

        // CRC offset should be 4 (aligned to 4-byte boundary after payload)
        let crc_offset = 4u32;
        let crc_val = checksum::calculate_crc(&bytestream[..crc_offset as usize], &crc_data);
        let crc_bytes = match settings.endianness {
            Endianness::Big => crc_val.to_be_bytes(),
            Endianness::Little => crc_val.to_le_bytes(),
        };
        let expected_crc_ascii = crc_bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();
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

    #[test]
    fn block_zero_crc_zeros_crc_location() {
        let mut crc_data = sample_crc_data();
        crc_data.area = CrcArea::BlockZeroCrc;
        let settings = Settings {
            crc: Some(crc_data),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, false, false, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
        let crc_offset = 4u32; // aligned to 4-byte boundary
        assert_eq!(
            dr.bytestream[crc_offset as usize..(crc_offset + 4) as usize],
            [0, 0, 0, 0],
            "CRC location should be zeroed"
        );
    }

    #[test]
    fn block_pad_crc_includes_padding_at_crc_location() {
        let mut crc_data = sample_crc_data();
        crc_data.area = CrcArea::BlockPadCrc;
        let settings = Settings {
            crc: Some(crc_data),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, false, false, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
        let crc_offset = 4u32;
        assert_eq!(
            dr.bytestream[crc_offset as usize..(crc_offset + 4) as usize],
            [0xFF, 0xFF, 0xFF, 0xFF],
            "CRC location should contain padding value"
        );
    }

    #[test]
    fn block_omit_crc_excludes_crc_bytes_from_calculation() {
        let mut crc_data = sample_crc_data();
        crc_data.area = CrcArea::BlockOmitCrc;
        let settings = Settings {
            crc: Some(crc_data.clone()),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
        let crc_offset = 4u32;

        // Calculate expected CRC by omitting CRC bytes
        let before = &dr.bytestream[..crc_offset as usize];
        let after = &dr.bytestream[(crc_offset + 4) as usize..];
        let combined: Vec<u8> = [before, after].concat();
        let expected_crc = checksum::calculate_crc(&combined, &crc_data);

        // Extract actual CRC from the result
        let actual_crc = match settings.endianness {
            Endianness::Little => u32::from_le_bytes(
                dr.crc_bytestream[..4]
                    .try_into()
                    .expect("CRC bytes should be 4 bytes"),
            ),
            Endianness::Big => u32::from_be_bytes(
                dr.crc_bytestream[..4]
                    .try_into()
                    .expect("CRC bytes should be 4 bytes"),
            ),
        };

        assert_eq!(
            expected_crc, actual_crc,
            "CRC should match calculation with CRC bytes omitted"
        );

        // Verify that including CRC bytes produces a different result
        let crc_with_bytes = checksum::calculate_crc(&dr.bytestream, &crc_data);
        assert_ne!(
            expected_crc, crc_with_bytes,
            "CRC with bytes included should differ from CRC with bytes omitted"
        );
    }

    #[test]
    fn no_crc_config_skips_crc() {
        let settings = Settings {
            crc: None,
            ..sample_settings()
        };
        let header = header_no_crc(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");

        assert!(dr.crc_bytestream.is_empty(), "CRC should be empty");
        assert_eq!(dr.crc_address, 0, "CRC address should be 0");
        assert_eq!(dr.bytestream.len(), 4, "bytestream should not be padded");
    }

    #[test]
    fn crc_location_none_skips_crc() {
        let settings = sample_settings();
        let header = Header {
            crc: Some(HeaderCrc {
                location: CrcLocation::Keyword("none".to_string()),
                ..sample_header_crc()
            }),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");

        assert!(dr.crc_bytestream.is_empty(), "CRC should be empty");
        assert_eq!(dr.crc_address, 0, "CRC address should be 0");
        assert_eq!(dr.bytestream.len(), 4, "bytestream should not be padded");
    }

    #[test]
    fn no_crc_with_pad_to_end() {
        let settings = Settings {
            crc: None,
            ..sample_settings()
        };
        let header = header_no_crc(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, true, 0)
            .expect("data range generation failed");

        assert!(dr.crc_bytestream.is_empty(), "CRC should be empty");
        assert_eq!(
            dr.bytestream.len(),
            32,
            "bytestream should be padded to full block"
        );
    }

    #[test]
    fn crc_location_set_but_settings_missing_errors() {
        let settings = Settings {
            crc: None,
            ..sample_settings()
        };
        // Header has CRC location but no overrides, and no global settings
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let result = bytestream_to_datarange(bytestream, &header, &settings, false, false, 0);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing CRC settings")
        );
    }

    #[test]
    fn header_crc_overrides_global_settings() {
        // Global settings with one polynomial
        let settings = Settings {
            crc: Some(sample_crc_data()),
            ..sample_settings()
        };

        // Header overrides polynomial
        let header = Header {
            crc: Some(HeaderCrc {
                location: CrcLocation::Keyword("end".to_string()),
                polynomial: Some(0x1EDC6F41), // Different polynomial
                start: None,
                xor_out: None,
                ref_in: None,
                ref_out: None,
                area: None,
            }),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");

        // CRC should be computed with the overridden polynomial
        let expected_crc_data = CrcData {
            polynomial: 0x1EDC6F41,
            ..sample_crc_data()
        };
        let expected_crc = checksum::calculate_crc(&bytestream, &expected_crc_data);
        let actual_crc = u32::from_le_bytes(dr.crc_bytestream[..4].try_into().unwrap());
        assert_eq!(expected_crc, actual_crc);
    }

    #[test]
    fn header_crc_fully_specified_no_global() {
        // No global CRC settings
        let settings = Settings {
            crc: None,
            ..sample_settings()
        };

        // Header fully specifies all CRC settings
        let header = Header {
            crc: Some(HeaderCrc {
                location: CrcLocation::Keyword("end".to_string()),
                polynomial: Some(0x04C11DB7),
                start: Some(0xFFFFFFFF),
                xor_out: Some(0xFFFFFFFF),
                ref_in: Some(true),
                ref_out: Some(true),
                area: Some(CrcArea::Data),
            }),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, false, false, 0)
            .expect("data range generation failed");

        // Should succeed and produce a valid CRC
        assert!(!dr.crc_bytestream.is_empty());
        let expected_crc = checksum::calculate_crc(&bytestream, &sample_crc_data());
        let actual_crc = u32::from_le_bytes(dr.crc_bytestream[..4].try_into().unwrap());
        assert_eq!(expected_crc, actual_crc);
    }
}
