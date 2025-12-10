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
    pub crc: Option<CrcData>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CrcArea {
    #[serde(rename = "data")]
    Data,
    #[serde(rename = "block_zero_crc")]
    BlockZeroCrc,
    #[serde(rename = "block_pad_crc")]
    BlockPadCrc,
    #[serde(rename = "block_omit_crc")]
    BlockOmitCrc,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CrcData {
    pub polynomial: u32,
    pub start: u32,
    pub xor_out: u32,
    pub ref_in: bool,
    pub ref_out: bool,
    pub area: CrcArea,
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
