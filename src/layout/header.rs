use super::settings::CrcConfig;
use serde::Deserialize;

/// Block header defining memory region and optional CRC configuration.
#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    /// Per-header CRC settings. Merged with `[settings.crc]` at runtime.
    #[serde(default)]
    pub crc: Option<CrcConfig>,
    #[serde(default = "default_padding")]
    pub padding: u8,
}

fn default_padding() -> u8 {
    0xFF
}
