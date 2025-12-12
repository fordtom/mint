use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub endianness: Endianness,
    #[serde(default = "default_offset")]
    pub virtual_offset: u32,
    #[serde(default)]
    pub byte_swap: bool,
    #[serde(default)]
    pub pad_to_end: bool,
    pub crc: Option<CrcConfig>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrcArea {
    #[default]
    #[serde(rename = "data")]
    Data,
    #[serde(rename = "block_zero_crc")]
    BlockZeroCrc,
    #[serde(rename = "block_pad_crc")]
    BlockPadCrc,
    #[serde(rename = "block_omit_crc")]
    BlockOmitCrc,
}

/// CRC location: keyword or absolute address.
/// - `"end_data"`: CRC placed after data (4-byte aligned)
/// - `"end_block"`: CRC in final 4 bytes of block
/// - `0x8FF0`: Absolute address within block
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum CrcLocation {
    Keyword(String),
    Address(u32),
}

/// Unified CRC configuration used in both `[settings.crc]` and `[header.crc]`.
/// All fields are optional; header values override settings values.
/// At settings level, `location` should be "end" or "none" (not an address).
#[derive(Debug, Deserialize, Clone, Default)]
pub struct CrcConfig {
    pub location: Option<CrcLocation>,
    pub polynomial: Option<u32>,
    pub start: Option<u32>,
    pub xor_out: Option<u32>,
    pub ref_in: Option<bool>,
    pub ref_out: Option<bool>,
    pub area: Option<CrcArea>,
}

impl CrcConfig {
    /// Merge this config with a base config. Self takes precedence.
    pub fn resolve(&self, base: Option<&CrcConfig>) -> CrcConfig {
        CrcConfig {
            location: self
                .location
                .clone()
                .or_else(|| base.and_then(|b| b.location.clone())),
            polynomial: self.polynomial.or_else(|| base.and_then(|b| b.polynomial)),
            start: self.start.or_else(|| base.and_then(|b| b.start)),
            xor_out: self.xor_out.or_else(|| base.and_then(|b| b.xor_out)),
            ref_in: self.ref_in.or_else(|| base.and_then(|b| b.ref_in)),
            ref_out: self.ref_out.or_else(|| base.and_then(|b| b.ref_out)),
            area: self.area.or_else(|| base.and_then(|b| b.area)),
        }
    }

    /// Check if CRC is disabled (location not set).
    pub fn is_disabled(&self) -> bool {
        self.location.is_none()
    }

    /// Returns true if all required CRC parameters are present.
    pub fn is_complete(&self) -> bool {
        self.polynomial.is_some()
            && self.start.is_some()
            && self.xor_out.is_some()
            && self.ref_in.is_some()
            && self.ref_out.is_some()
            && self.area.is_some()
    }
}

fn default_offset() -> u32 {
    0
}

pub trait EndianBytes {
    fn to_endian_bytes(self, endianness: &Endianness) -> Vec<u8>;
}

macro_rules! impl_endian_bytes {
    ($($t:ty),* $(,)?) => {$(
        impl EndianBytes for $t {
            fn to_endian_bytes(self, e: &Endianness) -> Vec<u8> {
                match e {
                    Endianness::Little => self.to_le_bytes().to_vec(),
                    Endianness::Big => self.to_be_bytes().to_vec(),
                }
            }
        }
    )*};
}
impl_endian_bytes!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
