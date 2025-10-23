use crate::layout::settings::CrcData;

/// Hand-rolled CRC32 calculation matching the crc crate's NoTable implementation.
/// This removes the need for static state and allows each block to use its own CRC settings.
pub fn calculate_crc(data: &[u8], crc_settings: &CrcData) -> u32 {
    // Initialize CRC based on ref_in
    let mut crc = if crc_settings.ref_in {
        crc_settings.start.reverse_bits()
    } else {
        crc_settings.start
    };

    // Prepare polynomial
    let poly = if crc_settings.ref_in {
        crc_settings.polynomial.reverse_bits()
    } else {
        crc_settings.polynomial
    };

    // Process each byte
    for &byte in data {
        let idx = if crc_settings.ref_in {
            (crc ^ (byte as u32)) & 0xFF
        } else {
            ((crc >> 24) ^ (byte as u32)) & 0xFF
        };

        // Perform 8 rounds of bitwise CRC calculation
        let mut step = if crc_settings.ref_in { idx } else { idx << 24 };
        if crc_settings.ref_in {
            for _ in 0..8 {
                step = (step >> 1) ^ ((step & 1) * poly);
            }
        } else {
            for _ in 0..8 {
                step = (step << 1) ^ (((step >> 31) & 1) * poly);
            }
        }

        crc = if crc_settings.ref_in {
            step ^ (crc >> 8)
        } else {
            step ^ (crc << 8)
        };
    }

    // Finalize
    if crc_settings.ref_in ^ crc_settings.ref_out {
        crc = crc.reverse_bits();
    }

    crc ^ crc_settings.xor_out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::settings::CrcArea;

    // Verify our CRC32 implementation against the well-known test vector
    // This tests the standard CRC32 settings used in all project examples
    #[test]
    fn test_crc32_standard_test_vector() {
        let crc_settings = CrcData {
            polynomial: 0x04C11DB7,
            start: 0xFFFF_FFFF,
            xor_out: 0xFFFF_FFFF,
            ref_in: true,
            ref_out: true,
            area: CrcArea::Data,
        };

        // The standard CRC32 test vector - "123456789" should produce 0xCBF43926
        // This is the well-known test vector for CRC-32 (used in ZIP, PNG, etc.)
        let test_str = b"123456789";
        let result = calculate_crc(test_str, &crc_settings);
        assert_eq!(
            result, 0xCBF43926,
            "Standard CRC32 test vector failed (expected 0xCBF43926 for \"123456789\")"
        );

        // Test with simple data to ensure the implementation is stable
        let simple_data = vec![0x01, 0x02, 0x03, 0x04];
        let simple_result = calculate_crc(&simple_data, &crc_settings);
        assert_eq!(simple_result, 0xB63CFBCD, "CRC32 for [1,2,3,4] failed");
    }

    #[test]
    fn test_crc32_mpeg2_non_reflected_vector() {
        let crc_settings = CrcData {
            polynomial: 0x04C11DB7,
            start: 0xFFFF_FFFF,
            xor_out: 0x0000_0000,
            ref_in: false,
            ref_out: false,
            area: CrcArea::Data,
        };

        // CRC-32/MPEG-2 parameters (non-reflected) over "123456789" should produce 0x0376E6E7
        let test_str = b"123456789";
        let result = calculate_crc(test_str, &crc_settings);
        assert_eq!(
            result, 0x0376E6E7,
            "CRC32/MPEG-2 test vector failed (expected 0x0376E6E7 for \"123456789\")"
        );
    }
}
