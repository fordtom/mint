use serde_json::Value;
use std::collections::HashMap;

use super::args::VariantArgs;
use super::errors::VariantError;
use super::DataSource;
use crate::layout::value::{DataValue, ValueSource};

fn load_json(input: &str) -> Result<HashMap<String, HashMap<String, Value>>, VariantError> {
    let json_str = if input.ends_with(".json") {
        std::fs::read_to_string(input)
            .map_err(|_| VariantError::FileError(format!("failed to open file: {}", input)))?
    } else {
        input.to_string()
    };

    let map: HashMap<String, HashMap<String, Value>> = serde_json::from_str(&json_str)
        .map_err(|e| VariantError::FileError(format!("failed to parse JSON: {}", e)))?;
    Ok(map)
}

/// JSON data source that reads variant data directly from a JSON object.
/// Expected format: `{ "VariantName": { "key1": value1, "key2": value2, ... }, ... }`
/// Result: `Vec<HashMap<String, Value>>` in variant priority order.
pub struct JsonDataSource {
    variant_columns: Vec<HashMap<String, Value>>,
}

impl JsonDataSource {
    pub(crate) fn new(args: &VariantArgs) -> Result<Self, VariantError> {
        let json_str = args
            .json
            .as_ref()
            .ok_or_else(|| VariantError::MiscError("missing json config".to_string()))?;

        let data = load_json(json_str)?;
        let variants = args.get_variant_list();
        let mut variant_columns = Vec::with_capacity(variants.len());

        for variant in &variants {
            let map = data
                .get(variant)
                .ok_or_else(|| {
                    VariantError::RetrievalError(format!(
                        "variant '{}' not found in JSON data",
                        variant
                    ))
                })?
                .clone();
            variant_columns.push(map);
        }

        Ok(JsonDataSource { variant_columns })
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.variant_columns
            .iter()
            .find_map(|map| map.get(name).filter(|v| !v.is_null()))
    }

    fn value_to_data_value(value: &Value) -> Result<DataValue, VariantError> {
        match value {
            Value::Bool(b) => Ok(DataValue::Bool(*b)),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Ok(DataValue::U64(u))
                } else if let Some(i) = n.as_i64() {
                    Ok(DataValue::I64(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::F64(f))
                } else {
                    Err(VariantError::RetrievalError(
                        "unsupported numeric type".to_string(),
                    ))
                }
            }
            Value::String(s) => Ok(DataValue::Str(s.clone())),
            _ => Err(VariantError::RetrievalError(
                "expected scalar value".to_string(),
            )),
        }
    }

    fn parse_delimited_numbers(s: &str) -> Option<Vec<DataValue>> {
        s.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| {
                p.parse::<u64>()
                    .map(DataValue::U64)
                    .ok()
                    .or_else(|| p.parse::<i64>().map(DataValue::I64).ok())
                    .or_else(|| p.parse::<f64>().map(DataValue::F64).ok())
            })
            .collect()
    }
}

impl DataSource for JsonDataSource {
    fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError> {
        let result = (|| {
            let value = self.lookup(name).ok_or_else(|| {
                VariantError::RetrievalError("key not found in any variant".into())
            })?;

            let dv = Self::value_to_data_value(value)?;
            match dv {
                DataValue::Str(_) => Err(VariantError::RetrievalError(
                    "Found non-numeric single value".to_string(),
                )),
                _ => Ok(dv),
            }
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, VariantError> {
        let result = (|| {
            let value = self.lookup(name).ok_or_else(|| {
                VariantError::RetrievalError("key not found in any variant".into())
            })?;

            match value {
                Value::Array(arr) => {
                    let items: Result<Vec<_>, _> =
                        arr.iter().map(Self::value_to_data_value).collect();
                    Ok(ValueSource::Array(items?))
                }
                Value::String(s) => match Self::parse_delimited_numbers(s) {
                    Some(arr) if !arr.is_empty() => Ok(ValueSource::Array(arr)),
                    _ => Ok(ValueSource::Single(DataValue::Str(s.clone()))),
                },
                _ => Err(VariantError::RetrievalError(
                    "expected array or string for 1D array".to_string(),
                )),
            }
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, VariantError> {
        let result = (|| {
            let value = self.lookup(name).ok_or_else(|| {
                VariantError::RetrievalError("key not found in any variant".into())
            })?;

            let Value::Array(outer) = value else {
                return Err(VariantError::RetrievalError(
                    "expected 2D array (array of arrays)".to_string(),
                ));
            };

            outer
                .iter()
                .map(|row_val| {
                    let Value::Array(inner) = row_val else {
                        return Err(VariantError::RetrievalError(
                            "expected array for 2D array row".to_string(),
                        ));
                    };
                    inner.iter().map(Self::value_to_data_value).collect()
                })
                .collect()
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }
}
