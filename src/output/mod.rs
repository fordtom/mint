pub mod args;
pub mod checksum;
pub mod errors;

use crate::layout::header::Header;
use crate::layout::settings::{CrcArea, CrcConfig, CrcLocation, Endianness, Settings};
use crate::output::args::OutputFormat;
use errors::OutputError;

use bin_file::{BinFile, IHexFormat};

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
    pub crc_address: u32,
    pub crc_bytestream: Vec<u8>,
    pub used_size: u32,
    pub allocated_size: u32,
}

/// Resolves CRC config from header + settings, validates location, returns offset + config.
fn resolve_crc(
    length: usize,
    header: &Header,
    settings: &Settings,
) -> Result<Option<(u32, CrcConfig)>, OutputError> {
    // Merge header CRC with settings CRC
    let resolved = header
        .crc
        .as_ref()
        .map(|hc| hc.resolve(settings.crc.as_ref()))
        .unwrap_or_else(|| settings.crc.clone().unwrap_or_default());

    // Check if CRC is disabled
    if resolved.is_disabled() {
        return Ok(None);
    }

    let location = resolved.location.as_ref().ok_or_else(|| {
        OutputError::HexOutputError("CRC enabled but no location specified.".to_string())
    })?;

    // Absolute addresses must come from header, not settings
    if let CrcLocation::Address(_) = location {
        let header_has_location = header.crc.as_ref().is_some_and(|hc| hc.location.is_some());
        if !header_has_location {
            return Err(OutputError::HexOutputError(
                "Absolute CRC address not allowed in [settings.crc]; use [header.crc] instead."
                    .to_string(),
            ));
        }
    }

    let crc_offset = match location {
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
            "end_data" => (length as u32 + 3) & !3,
            "end_block" => {
                let offset = header.length.saturating_sub(4);
                if offset < length as u32 {
                    return Err(OutputError::HexOutputError(
                        "CRC at end_block overlaps with payload data.".to_string(),
                    ));
                }
                offset
            }
            _ => {
                return Err(OutputError::HexOutputError(format!(
                    "Invalid CRC location: '{}'. Use 'end_data', 'end_block', or an address.",
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

    // Verify all CRC parameters are present
    if !resolved.is_complete() {
        return Err(OutputError::HexOutputError(
            "CRC location specified but missing CRC parameters (polynomial, start, etc)."
                .to_string(),
        ));
    }

    Ok(Some((crc_offset, resolved)))
}

pub fn bytestream_to_datarange(
    mut bytestream: Vec<u8>,
    header: &Header,
    settings: &Settings,
    padding_bytes: u32,
) -> Result<DataRange, OutputError> {
    if bytestream.len() > header.length as usize {
        return Err(OutputError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    // Apply byte swap for word-addressing mode BEFORE CRC calculation
    if settings.word_addressing {
        if !bytestream.len().is_multiple_of(2) {
            bytestream.push(header.padding);
        }
        byte_swap_inplace(&mut bytestream);
    }

    // Resolve CRC configuration (location + settings) from header + global defaults
    let crc_config = resolve_crc(bytestream.len(), header, settings)?;

    let mut used_size = (bytestream.len() as u32).saturating_sub(padding_bytes);

    // Address multiplier for word-addressing mode (2x for 16-bit words)
    let addr_mult: u32 = if settings.word_addressing { 2 } else { 1 };

    // If CRC is disabled for this block, return early with no CRC
    let Some((crc_offset, crc_settings)) = crc_config else {
        return Ok(DataRange {
            start_address: header.start_address * addr_mult + settings.virtual_offset,
            bytestream,
            crc_address: 0,
            crc_bytestream: Vec::new(),
            used_size,
            allocated_size: header.length * addr_mult,
        });
    };

    used_size = used_size.saturating_add(4);

    let area = crc_settings.area.unwrap(); // Safe: is_complete() verified
    let is_end_block = matches!(
        &crc_settings.location,
        Some(CrcLocation::Keyword(kw)) if kw == "end_block"
    );

    // Prepare bytestream and compute CRC based on area
    let crc_val = match area {
        CrcArea::Data => {
            // For end_data: pad to crc_offset before CRC calculation (aligning the CRC to be appended to the struct)
            // For end_block: CRC over raw data, pad afterwards
            if !is_end_block {
                bytestream.resize(crc_offset as usize, header.padding);
            }
            let crc = checksum::calculate_crc(&bytestream, &crc_settings);
            if is_end_block {
                bytestream.resize(crc_offset as usize, header.padding);
            }
            crc
        }
        CrcArea::BlockZeroCrc => {
            // Pad to full block, zero CRC location, then calculate
            bytestream.resize(header.length as usize, header.padding);
            bytestream[crc_offset as usize..(crc_offset + 4) as usize].fill(0);
            checksum::calculate_crc(&bytestream, &crc_settings)
        }
        CrcArea::BlockPadCrc => {
            // Pad to full block (CRC location contains padding), then calculate
            bytestream.resize(header.length as usize, header.padding);
            checksum::calculate_crc(&bytestream, &crc_settings)
        }
        CrcArea::BlockOmitCrc => {
            // Pad to full block, calculate CRC excluding CRC bytes
            bytestream.resize(header.length as usize, header.padding);
            let before = &bytestream[..crc_offset as usize];
            let after = &bytestream[(crc_offset + 4) as usize..];
            let combined: Vec<u8> = [before, after].concat();
            checksum::calculate_crc(&combined, &crc_settings)
        }
    };

    let mut crc_bytes: [u8; 4] = match settings.endianness {
        Endianness::Big => crc_val.to_be_bytes(),
        Endianness::Little => crc_val.to_le_bytes(),
    };

    // Swap CRC bytes for word-addressing mode (bytestream already swapped above)
    if settings.word_addressing {
        byte_swap_inplace(&mut crc_bytes);
    }

    Ok(DataRange {
        start_address: header.start_address * addr_mult + settings.virtual_offset,
        bytestream,
        crc_address: (header.start_address + crc_offset) * addr_mult + settings.virtual_offset,
        crc_bytestream: crc_bytes.to_vec(),
        used_size,
        allocated_size: header.length * addr_mult,
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

/// Represents an output file to be written.
#[derive(Debug, Clone)]
pub struct OutputFile {
    pub ranges: Vec<DataRange>,
    pub format: OutputFormat,
    pub record_width: usize,
}

impl OutputFile {
    /// Render this file's contents as a hex/mot string.
    pub fn render(&self) -> Result<String, OutputError> {
        emit_hex(&self.ranges, self.record_width, self.format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::header::Header;
    use crate::layout::settings::Endianness;
    use crate::layout::settings::Settings;
    use crate::layout::settings::{CrcArea, CrcConfig, CrcLocation};

    fn sample_crc_config() -> CrcConfig {
        CrcConfig {
            location: Some(CrcLocation::Keyword("end_data".to_string())),
            polynomial: Some(0x04C11DB7),
            start: Some(0xFFFF_FFFF),
            xor_out: Some(0xFFFF_FFFF),
            ref_in: Some(true),
            ref_out: Some(true),
            area: Some(CrcArea::Data),
        }
    }

    fn sample_settings() -> Settings {
        Settings {
            endianness: Endianness::Little,
            virtual_offset: 0,
            word_addressing: false,
            crc: Some(sample_crc_config()),
        }
    }

    fn sample_header(len: u32) -> Header {
        Header {
            start_address: 0,
            length: len,
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Keyword("end_data".to_string())),
                ..Default::default()
            }),
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
        let crc_config = sample_crc_config();
        let header = sample_header(16);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");
        let hex = emit_hex(&[dr], 16, crate::output::args::OutputFormat::Hex)
            .expect("hex generation failed");

        // No in-memory resize when pad_to_end=false; CRC is emitted separately
        assert_eq!(bytestream.len(), 4);

        // CRC offset should be 4 (aligned to 4-byte boundary after payload)
        let crc_val = checksum::calculate_crc(&bytestream[..4], &crc_config);
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
    fn block_zero_crc_zeros_crc_location() {
        let mut crc_config = sample_crc_config();
        crc_config.area = Some(CrcArea::BlockZeroCrc);
        let settings = Settings {
            crc: Some(crc_config),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
        let crc_offset = 4u32;
        assert_eq!(
            dr.bytestream[crc_offset as usize..(crc_offset + 4) as usize],
            [0, 0, 0, 0],
            "CRC location should be zeroed"
        );
    }

    #[test]
    fn block_pad_crc_includes_padding_at_crc_location() {
        let mut crc_config = sample_crc_config();
        crc_config.area = Some(CrcArea::BlockPadCrc);
        let settings = Settings {
            crc: Some(crc_config),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, 0)
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
        let mut crc_config = sample_crc_config();
        crc_config.area = Some(CrcArea::BlockOmitCrc);
        let settings = Settings {
            crc: Some(crc_config.clone()),
            ..sample_settings()
        };
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        assert_eq!(dr.bytestream.len(), header.length as usize);
        let crc_offset = 4u32;

        // Calculate expected CRC by omitting CRC bytes
        let before = &dr.bytestream[..crc_offset as usize];
        let after = &dr.bytestream[(crc_offset + 4) as usize..];
        let combined: Vec<u8> = [before, after].concat();
        let expected_crc = checksum::calculate_crc(&combined, &crc_config);

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
        let crc_with_bytes = checksum::calculate_crc(&dr.bytestream, &crc_config);
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
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        assert!(dr.crc_bytestream.is_empty(), "CRC should be empty");
        assert_eq!(dr.crc_address, 0, "CRC address should be 0");
        assert_eq!(dr.bytestream.len(), 4, "bytestream should not be padded");
    }

    #[test]
    fn end_block_places_crc_at_block_end() {
        let settings = sample_settings();
        let header = Header {
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Keyword("end_block".to_string())),
                ..Default::default()
            }),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        // CRC should be at offset 28 (block length 32 - 4)
        assert_eq!(dr.crc_address, 28);
        assert!(!dr.crc_bytestream.is_empty());
    }

    #[test]
    fn crc_location_set_but_settings_missing_errors() {
        let settings = Settings {
            crc: None,
            ..sample_settings()
        };
        // Header has CRC location but no param overrides, and no global settings
        let header = sample_header(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let result = bytestream_to_datarange(bytestream, &header, &settings, 0);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing CRC parameters")
        );
    }

    #[test]
    fn header_crc_overrides_global_settings() {
        let settings = sample_settings();

        // Header overrides polynomial
        let header = Header {
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Keyword("end_data".to_string())),
                polynomial: Some(0x1EDC6F41), // Different polynomial
                ..Default::default()
            }),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        // CRC should be computed with the overridden polynomial
        let mut expected_config = sample_crc_config();
        expected_config.polynomial = Some(0x1EDC6F41);
        let expected_crc = checksum::calculate_crc(&bytestream, &expected_config);
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
            crc: Some(sample_crc_config()),
            ..sample_header(32)
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        // Should succeed and produce a valid CRC
        assert!(!dr.crc_bytestream.is_empty());
        let expected_crc = checksum::calculate_crc(&bytestream, &sample_crc_config());
        let actual_crc = u32::from_le_bytes(dr.crc_bytestream[..4].try_into().unwrap());
        assert_eq!(expected_crc, actual_crc);
    }

    #[test]
    fn settings_location_end_with_header_inheriting() {
        // Settings specifies location = "end_data" as default
        let settings = Settings {
            crc: Some(sample_crc_config()),
            ..sample_settings()
        };

        // Header has no crc section - should inherit from settings
        let header = header_no_crc(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream.clone(), &header, &settings, 0)
            .expect("data range generation failed");

        // Should use CRC from settings
        assert!(!dr.crc_bytestream.is_empty());
    }

    #[test]
    fn settings_absolute_address_rejected() {
        // Settings with absolute address - should be rejected
        let settings = Settings {
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Address(0x1000)),
                ..sample_crc_config()
            }),
            ..sample_settings()
        };

        // Header has no crc section - inherits from settings
        let header = header_no_crc(32);

        let bytestream = vec![1u8, 2, 3, 4];
        let result = bytestream_to_datarange(bytestream, &header, &settings, 0);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Absolute CRC address not allowed in [settings.crc]")
        );
    }

    #[test]
    fn header_absolute_address_allowed() {
        let settings = sample_settings();

        // Header specifies absolute address - should work
        let header = Header {
            start_address: 0,
            length: 32,
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Address(28)),
                ..Default::default()
            }),
            padding: 0xFF,
        };

        let bytestream = vec![1u8, 2, 3, 4];
        let dr = bytestream_to_datarange(bytestream, &header, &settings, 0)
            .expect("data range generation failed");

        assert_eq!(dr.crc_address, 28);
        assert!(!dr.crc_bytestream.is_empty());
    }

    #[test]
    fn end_block_overlap_with_data_errors() {
        let settings = sample_settings();

        // Block length is 16, CRC at end_block means offset 12
        // But data is 16 bytes, which would overlap
        let header = Header {
            start_address: 0,
            length: 16,
            crc: Some(CrcConfig {
                location: Some(CrcLocation::Keyword("end_block".to_string())),
                ..Default::default()
            }),
            padding: 0xFF,
        };

        let bytestream = vec![1u8; 16]; // Data fills entire block
        let result = bytestream_to_datarange(bytestream, &header, &settings, 0);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("overlaps with payload")
        );
    }
}
