use super::settings::{CrcArea, CrcData};
use serde::Deserialize;

/// Block header defining memory region and optional CRC configuration.
#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    /// Per-header CRC settings. If present, enables CRC for this block.
    /// Settings here override the global `[settings.crc]` values.
    #[serde(default)]
    pub crc: Option<HeaderCrc>,
    #[serde(default = "default_padding")]
    pub padding: u8,
}

/// CRC location: either a keyword ("end", "none") or an absolute address.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum CrcLocation {
    Keyword(String),
    Address(u32),
}

/// Per-header CRC configuration. The `location` field is required; all other
/// fields are optional and override the corresponding `[settings.crc]` values.
#[derive(Debug, Deserialize, Clone)]
pub struct HeaderCrc {
    pub location: CrcLocation,
    pub polynomial: Option<u32>,
    pub start: Option<u32>,
    pub xor_out: Option<u32>,
    pub ref_in: Option<bool>,
    pub ref_out: Option<bool>,
    pub area: Option<CrcArea>,
}

impl HeaderCrc {
    /// Resolves the effective CRC settings by merging with global defaults.
    /// Returns `None` if location is "none" or if required fields are missing.
    pub fn resolve(&self, global: Option<&CrcData>) -> Option<CrcData> {
        // Check if CRC is disabled
        if let CrcLocation::Keyword(kw) = &self.location
            && kw == "none"
        {
            return None;
        }

        // Try to build CrcData from header overrides + global fallback
        let polynomial = self.polynomial.or(global.map(|g| g.polynomial))?;
        let start = self.start.or(global.map(|g| g.start))?;
        let xor_out = self.xor_out.or(global.map(|g| g.xor_out))?;
        let ref_in = self.ref_in.or(global.map(|g| g.ref_in))?;
        let ref_out = self.ref_out.or(global.map(|g| g.ref_out))?;
        let area = self.area.or(global.map(|g| g.area))?;

        Some(CrcData {
            polynomial,
            start,
            xor_out,
            ref_in,
            ref_out,
            area,
        })
    }

    /// Returns the CRC location.
    pub fn location(&self) -> &CrcLocation {
        &self.location
    }
}

fn default_padding() -> u8 {
    0xFF
}
