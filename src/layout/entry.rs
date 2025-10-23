use super::block::BuildConfig;
use super::errors::LayoutError;
use super::value::ValueSource;
use crate::variant::DataSheet;
use serde::Deserialize;

/// Leaf entry representing an item to add to the flash block.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeafEntry {
    #[serde(rename = "type")]
    pub scalar_type: ScalarType,
    #[serde(flatten, default)]
    size_keys: SizeKeys,
    #[serde(flatten)]
    pub source: EntrySource,
}

/// Scalar type enum derived from 'type' string in leaf entries.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ScalarType {
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "u16")]
    U16,
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "u64")]
    U64,
    #[serde(rename = "i8")]
    I8,
    #[serde(rename = "i16")]
    I16,
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "f32")]
    F32,
    #[serde(rename = "f64")]
    F64,
}

/// Size source enum.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SizeSource {
    OneD(usize),
    TwoD([usize; 2]),
}

/// Helper struct to capture both 'size' and 'SIZE' keys.
#[derive(Debug, Default, Deserialize)]
struct SizeKeys {
    #[serde(rename = "size")]
    size: Option<SizeSource>,
    #[serde(rename = "SIZE")]
    strict_size: Option<SizeSource>,
}

impl SizeKeys {
    fn resolve(&self) -> Result<(Option<SizeSource>, bool), LayoutError> {
        match (&self.size, &self.strict_size) {
            (Some(_), Some(_)) => Err(LayoutError::DataValueExportFailed(
                "Use either 'size' or 'SIZE', not both.".into(),
            )),
            (Some(s), None) => Ok((Some(s.clone()), false)),
            (None, Some(s)) => Ok((Some(s.clone()), true)),
            (None, None) => Ok((None, false)),
        }
    }
}

/// Mutually exclusive source enum.
#[derive(Debug, Deserialize)]
pub enum EntrySource {
    #[serde(rename = "name")]
    Name(String),
    #[serde(rename = "value")]
    Value(ValueSource),
}

impl LeafEntry {
    /// Returns the alignment of the leaf entry.
    pub fn get_alignment(&self) -> usize {
        self.scalar_type.size_bytes()
    }

    pub fn emit_bytes(
        &self,
        data_sheet: Option<&DataSheet>,
        config: &BuildConfig,
    ) -> Result<Vec<u8>, LayoutError> {
        let (size, strict_len) = self.size_keys.resolve()?;
        match size {
            None => self.emit_bytes_single(data_sheet, config),
            Some(SizeSource::OneD(size)) => {
                self.emit_bytes_1d(data_sheet, size, config, strict_len)
            }
            Some(SizeSource::TwoD(size)) => {
                self.emit_bytes_2d(data_sheet, size, config, strict_len)
            }
        }
    }

    fn emit_bytes_single(
        &self,
        data_sheet: Option<&DataSheet>,
        config: &BuildConfig,
    ) -> Result<Vec<u8>, LayoutError> {
        match &self.source {
            EntrySource::Name(name) => {
                let Some(data_sheet) = data_sheet else {
                    return Err(LayoutError::MissingDataSheet(format!(
                        "Field '{}' requires a value from the Excel datasheet, but no datasheet was provided. Use -x to specify an Excel file.",
                        name
                    )));
                };
                let value = data_sheet.retrieve_single_value(name)?;
                value.to_bytes(self.scalar_type, config.endianness, config.strict)
            }
            EntrySource::Value(ValueSource::Single(v)) => {
                v.to_bytes(self.scalar_type, config.endianness, config.strict)
            }
            EntrySource::Value(_) => Err(LayoutError::DataValueExportFailed(
                "Single value expected for scalar type.".to_string(),
            )),
        }
    }

    fn emit_bytes_1d(
        &self,
        data_sheet: Option<&DataSheet>,
        size: usize,
        config: &BuildConfig,
        strict_len: bool,
    ) -> Result<Vec<u8>, LayoutError> {
        let elem = self.scalar_type.size_bytes();
        let total_bytes = size
            .checked_mul(elem)
            .ok_or(LayoutError::DataValueExportFailed(
                "Array size overflow".into(),
            ))?;
        let mut out = Vec::with_capacity(total_bytes);

        match &self.source {
            EntrySource::Name(name) => {
                let Some(data_sheet) = data_sheet else {
                    return Err(LayoutError::MissingDataSheet(format!(
                        "Field '{}' requires a value from the Excel datasheet, but no datasheet was provided. Use -x to specify an Excel file.",
                        name
                    )));
                };
                match data_sheet.retrieve_1d_array_or_string(name)? {
                    ValueSource::Single(v) => {
                        if !matches!(self.scalar_type, ScalarType::U8) {
                            return Err(LayoutError::DataValueExportFailed(
                                "Strings should have type u8.".to_string(),
                            ));
                        }
                        out.extend(v.string_to_bytes()?);
                    }
                    ValueSource::Array(v) => {
                        for v in v {
                            out.extend(v.to_bytes(
                                self.scalar_type,
                                config.endianness,
                                config.strict,
                            )?);
                        }
                    }
                }
            }
            EntrySource::Value(ValueSource::Array(v)) => {
                for v in v {
                    out.extend(v.to_bytes(self.scalar_type, config.endianness, config.strict)?);
                }
            }
            EntrySource::Value(ValueSource::Single(v)) => {
                if !matches!(self.scalar_type, ScalarType::U8) {
                    return Err(LayoutError::DataValueExportFailed(
                        "Strings should have type u8.".to_string(),
                    ));
                }
                out.extend(v.string_to_bytes()?);
            }
        }

        if out.len() > total_bytes {
            return Err(LayoutError::DataValueExportFailed(
                "Array/string is larger than defined size.".to_string(),
            ));
        }
        if strict_len && out.len() < total_bytes {
            return Err(LayoutError::DataValueExportFailed(
                "Array/string is smaller than defined size (strict SIZE).".to_string(),
            ));
        }
        while out.len() < total_bytes {
            out.push(config.padding);
        }
        Ok(out)
    }

    fn emit_bytes_2d(
        &self,
        data_sheet: Option<&DataSheet>,
        size: [usize; 2],
        config: &BuildConfig,
        strict_len: bool,
    ) -> Result<Vec<u8>, LayoutError> {
        match &self.source {
            EntrySource::Name(name) => {
                let Some(data_sheet) = data_sheet else {
                    return Err(LayoutError::MissingDataSheet(format!(
                        "Field '{}' requires a value from the Excel datasheet, but no datasheet was provided. Use -x to specify an Excel file.",
                        name
                    )));
                };
                let data = data_sheet.retrieve_2d_array(name)?;

                let rows = size[0];
                let cols = size[1];

                let elem = self.scalar_type.size_bytes();
                let total_elems =
                    rows.checked_mul(cols)
                        .ok_or(LayoutError::DataValueExportFailed(
                            "2D size overflow".into(),
                        ))?;
                let total_bytes =
                    total_elems
                        .checked_mul(elem)
                        .ok_or(LayoutError::DataValueExportFailed(
                            "2D byte count overflow".into(),
                        ))?;

                if data.iter().any(|row| row.len() != cols) {
                    return Err(LayoutError::DataValueExportFailed(
                        "2D array column count mismatch.".to_string(),
                    ));
                }

                if data.len() > rows {
                    return Err(LayoutError::DataValueExportFailed(
                        "2D array row count greater than defined size.".to_string(),
                    ));
                }

                if strict_len && data.len() < rows {
                    return Err(LayoutError::DataValueExportFailed(
                        "2D array row count smaller than defined size (strict SIZE).".to_string(),
                    ));
                }

                let mut out = Vec::with_capacity(total_bytes);
                for row in data {
                    for v in row {
                        out.extend(v.to_bytes(
                            self.scalar_type,
                            config.endianness,
                            config.strict,
                        )?);
                    }
                }

                while out.len() < total_bytes {
                    out.push(config.padding);
                }

                Ok(out)
            }
            EntrySource::Value(_) => Err(LayoutError::DataValueExportFailed(
                "2D arrays within the layout file are not supported.".to_string(),
            )),
        }
    }
}

impl ScalarType {
    /// Returns the size of the scalar type in bytes.
    pub fn size_bytes(&self) -> usize {
        match self {
            ScalarType::U8 | ScalarType::I8 => 1,
            ScalarType::U16 | ScalarType::I16 => 2,
            ScalarType::U32 | ScalarType::I32 | ScalarType::F32 => 4,
            ScalarType::U64 | ScalarType::I64 | ScalarType::F64 => 8,
        }
    }
}
