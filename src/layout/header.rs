use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    pub crc_location: CrcLocation,
    #[serde(default = "default_padding")]
    pub padding: u8,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CrcLocation {
    Keyword(String),
    Address(u32),
}

fn default_padding() -> u8 {
    0xFF
}
