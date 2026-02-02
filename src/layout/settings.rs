use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub endianness: Endianness,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Endianness {
    Little,
    Big,
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
