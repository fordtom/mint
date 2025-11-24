use super::entry::ScalarType;
use super::errors::LayoutError;
use super::settings::{EndianBytes, Endianness};
use super::value::DataValue;

macro_rules! impl_try_from_data_value {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<&DataValue> for $t {
            type Error = LayoutError;
            fn try_from(value: &DataValue) -> Result<Self, LayoutError> {
                match value {
                    DataValue::Bool(val) => {
                        let n: u8 = if *val { 1 } else { 0 };
                        Ok(n as $t)
                    }
                    DataValue::U64(val) => Ok(*val as $t),
                    DataValue::I64(val) => Ok(*val as $t),
                    DataValue::F64(val) => Ok(*val as $t),
                    DataValue::Str(_) => {
                        return Err(LayoutError::DataValueExportFailed(
                            "Cannot convert string to scalar type.".to_string(),
                        ));
                    }
                }
            }
        }
    )* }; }

impl_try_from_data_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

pub trait TryFromStrict<T>: Sized {
    fn try_from_strict(value: T) -> Result<Self, LayoutError>;
}

macro_rules! err {
    ($msg:expr) => {
        LayoutError::DataValueExportFailed($msg.to_string())
    };
}

macro_rules! impl_try_from_strict_unsigned {
    ($($t:ty),* $(,)?) => {$(
        impl TryFromStrict<&DataValue> for $t {
            fn try_from_strict(value: &DataValue) -> Result<Self, LayoutError> {
                match value {
                    DataValue::U64(v) => <Self as TryFrom<u64>>::try_from(*v)
                        .map_err(|_| err!(format!("u64 value {} out of range for {}", v, stringify!($t)))),
                    DataValue::I64(v) => {
                        if *v < 0 { return Err(err!("negative integer cannot convert to unsigned in strict mode")); }
                        <Self as TryFrom<u64>>::try_from(*v as u64)
                            .map_err(|_| err!(format!("i64 value {} out of range for {}", v, stringify!($t))))
                    }
                    DataValue::F64(v) => {
                        if !v.is_finite() { return Err(err!("non-finite float cannot convert to integer in strict mode")); }
                        if v.fract() != 0.0 { return Err(err!("float to integer conversion not allowed unless value is an exact integer")); }
                        if *v < 0.0 || *v > (<$t>::MAX as f64) { return Err(err!(format!("float value {} out of range for {}", v, stringify!($t)))); }
                        Ok(*v as $t)
                    }
                    DataValue::Bool(b) => {
                        let n: u8 = if *b { 1 } else { 0 };
                        Ok(n as $t)
                    }
                    DataValue::Str(_) => Err(err!("Cannot convert string to scalar type.")),
                }
            }
        }
    )*};
}

macro_rules! impl_try_from_strict_signed {
    ($($t:ty),* $(,)?) => {$(
        impl TryFromStrict<&DataValue> for $t {
            fn try_from_strict(value: &DataValue) -> Result<Self, LayoutError> {
                match value {
                    DataValue::U64(v) => {
                        <Self as TryFrom<i128>>::try_from(*v as i128)
                            .map_err(|_| err!(format!("u64 value {} out of range for {}", v, stringify!($t))))
                    }
                    DataValue::I64(v) => <Self as TryFrom<i64>>::try_from(*v)
                        .map_err(|_| err!(format!("i64 value {} out of range for {}", v, stringify!($t)))),
                    DataValue::F64(v) => {
                        if !v.is_finite() { return Err(err!("non-finite float cannot convert to integer in strict mode")); }
                        if v.fract() != 0.0 { return Err(err!("float to integer conversion not allowed unless value is an exact integer")); }
                        if *v < (<$t>::MIN as f64) || *v > (<$t>::MAX as f64) { return Err(err!(format!("float value {} out of range for {}", v, stringify!($t)))); }
                        Ok(*v as $t)
                    }
                    DataValue::Bool(b) => {
                        let n: u8 = if *b { 1 } else { 0 };
                        Ok(n as $t)
                    }
                    DataValue::Str(_) => Err(err!("Cannot convert string to scalar type.")),
                }
            }
        }
    )*};
}

macro_rules! impl_try_from_strict_float_targets {
    ($t:ty) => {
        impl TryFromStrict<&DataValue> for $t {
            fn try_from_strict(value: &DataValue) -> Result<Self, LayoutError> {
                match value {
                    DataValue::F64(v) => {
                        if !v.is_finite() {
                            return Err(err!("non-finite float not allowed in strict mode"));
                        }
                        let out = *v as $t;
                        if out.is_finite() {
                            Ok(out)
                        } else {
                            Err(err!(format!(
                                "float value {} out of range for {}",
                                v,
                                stringify!($t)
                            )))
                        }
                    }
                    DataValue::U64(v) => {
                        let out = (*v as $t);
                        if !out.is_finite() {
                            return Err(err!("integer to float produced non-finite value"));
                        }
                        // exactness check via round-trip
                        if (out as u64) == *v {
                            Ok(out)
                        } else {
                            Err(err!(
                                "lossy integer to float conversion not allowed in strict mode"
                            ))
                        }
                    }
                    DataValue::I64(v) => {
                        let out = (*v as $t);
                        if !out.is_finite() {
                            return Err(err!("integer to float produced non-finite value"));
                        }
                        if (out as i64) == *v {
                            Ok(out)
                        } else {
                            Err(err!(
                                "lossy integer to float conversion not allowed in strict mode"
                            ))
                        }
                    }
                    DataValue::Bool(b) => {
                        let out: $t = if *b { 1.0 } else { 0.0 };
                        Ok(out)
                    }
                    DataValue::Str(_) => Err(err!("Cannot convert string to scalar type.")),
                }
            }
        }
    };
}

impl_try_from_strict_unsigned!(u8, u16, u32, u64);
impl_try_from_strict_signed!(i8, i16, i32, i64);
impl_try_from_strict_float_targets!(f32);
impl TryFromStrict<&DataValue> for f64 {
    fn try_from_strict(value: &DataValue) -> Result<Self, LayoutError> {
        match value {
            DataValue::F64(v) => Ok(*v),
            DataValue::U64(v) => {
                let out = *v as f64;
                if (out as u64) == *v {
                    Ok(out)
                } else {
                    Err(err!(
                        "lossy integer to float conversion not allowed in strict mode"
                    ))
                }
            }
            DataValue::I64(v) => {
                let out = *v as f64;
                if (out as i64) == *v {
                    Ok(out)
                } else {
                    Err(err!(
                        "lossy integer to float conversion not allowed in strict mode"
                    ))
                }
            }
            DataValue::Bool(b) => {
                let out = if *b { 1.0 } else { 0.0 };
                Ok(out)
            }
            DataValue::Str(_) => Err(err!("Cannot convert string to scalar type.")),
        }
    }
}

pub fn convert_value_to_bytes(
    value: &DataValue,
    scalar_type: ScalarType,
    endianness: &Endianness,
    strict: bool,
) -> Result<Vec<u8>, LayoutError> {
    macro_rules! to_bytes {
        ($t:ty) => {{
            let val: $t = if strict {
                <$t as TryFromStrict<&DataValue>>::try_from_strict(value)?
            } else {
                <$t as TryFrom<&DataValue>>::try_from(value)?
            };
            Ok(val.to_endian_bytes(endianness))
        }};
    }

    match scalar_type {
        ScalarType::U8 => to_bytes!(u8),
        ScalarType::I8 => to_bytes!(i8),
        ScalarType::U16 => to_bytes!(u16),
        ScalarType::I16 => to_bytes!(i16),
        ScalarType::U32 => to_bytes!(u32),
        ScalarType::I32 => to_bytes!(i32),
        ScalarType::U64 => to_bytes!(u64),
        ScalarType::I64 => to_bytes!(i64),
        ScalarType::F32 => to_bytes!(f32),
        ScalarType::F64 => to_bytes!(f64),
    }
}
