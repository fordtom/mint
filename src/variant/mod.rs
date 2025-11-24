pub mod args;
pub mod errors;
mod helpers;

use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use std::collections::HashMap;

use crate::layout::value::{DataValue, ValueSource};
use errors::VariantError;

pub struct DataSheet {
    names: Vec<String>,
    default_values: Vec<Data>,
    debug_values: Option<Vec<Data>>,
    variant_values: Option<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl DataSheet {
    pub fn new(args: &args::VariantArgs) -> Result<Option<Self>, VariantError> {
        let Some(xlsx_path) = &args.xlsx else {
            return Ok(None);
        };

        let mut workbook: Xlsx<_> = open_workbook(xlsx_path)
            .map_err(|_| VariantError::FileError(format!("failed to open file: {}", xlsx_path)))?;

        let main_sheet = workbook
            .worksheet_range(&args.main_sheet)
            .map_err(|_| VariantError::MiscError("Main sheet not found.".to_string()))?;

        let rows: Vec<_> = main_sheet.rows().collect();
        let (headers, data_rows) = match rows.split_first() {
            Some((hdr, tail)) => (hdr, tail.len()),
            None => {
                return Err(VariantError::RetrievalError(
                    "invalid main sheet format.".to_string(),
                ));
            }
        };

        let name_index = headers
            .iter()
            .position(|cell| Self::cell_eq_ascii(cell, "Name"))
            .ok_or(VariantError::ColumnNotFound("Name".to_string()))?;

        let default_index = headers
            .iter()
            .position(|cell| Self::cell_eq_ascii(cell, "Default"))
            .ok_or(VariantError::ColumnNotFound("Default".to_string()))?;

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| {
            row.get(name_index)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default()
        }));
        helpers::warn_duplicate_names(&names);

        let mut default_values: Vec<Data> = Vec::with_capacity(data_rows);
        default_values.extend(
            rows.iter()
                .skip(1)
                .map(|row| row.get(default_index).cloned().unwrap_or(Data::Empty)),
        );

        let mut debug_values: Option<Vec<Data>> = None;
        if args.debug {
            let debug_index = headers
                .iter()
                .position(|cell| Self::cell_eq_ascii(cell, "Debug"))
                .ok_or(VariantError::ColumnNotFound("Debug".to_string()))?;

            let mut debug_vec: Vec<Data> = Vec::with_capacity(data_rows);
            debug_vec.extend(
                rows.iter()
                    .skip(1)
                    .map(|row| row.get(debug_index).cloned().unwrap_or(Data::Empty)),
            );

            debug_values = Some(debug_vec);
        }

        let mut variant_values: Option<Vec<Data>> = None;
        if let Some(name) = &args.variant {
            let variant_index = headers
                .iter()
                .position(|cell| Self::cell_eq_ascii(cell, name))
                .ok_or(VariantError::ColumnNotFound(name.to_string()))?;

            let mut variant_vec: Vec<Data> = Vec::with_capacity(data_rows);
            variant_vec.extend(
                rows.iter()
                    .skip(1)
                    .map(|row| row.get(variant_index).cloned().unwrap_or(Data::Empty)),
            );

            variant_values = Some(variant_vec);
        };

        let mut sheets: HashMap<String, Range<Data>> =
            HashMap::with_capacity(workbook.worksheets().len().saturating_sub(1));
        for (name, sheet) in workbook.worksheets() {
            if name != args.main_sheet {
                sheets.insert(name.to_string(), sheet);
            }
        }

        Ok(Some(Self {
            names,
            default_values,
            debug_values,
            variant_values,
            sheets,
        }))
    }

    pub fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError> {
        let result = (|| match self.retrieve_cell(name)? {
            Data::Int(i) => Ok(DataValue::I64(*i)),
            Data::Float(f) => Ok(DataValue::F64(*f)),
            Data::Bool(b) => Ok(DataValue::Bool(*b)),
            Data::String(s) => Self::parse_bool_string(s)
                .map(DataValue::Bool)
                .ok_or_else(|| {
                    VariantError::RetrievalError("Found non-numeric single value".to_string())
                }),
            _ => Err(VariantError::RetrievalError(
                "Found non-numeric single value".to_string(),
            )),
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    pub fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, VariantError> {
        let result = (|| {
            let Data::String(cell_string) = self.retrieve_cell(name)? else {
                return Err(VariantError::RetrievalError(
                    "Expected string value for 1D array or string".to_string(),
                ));
            };

            // Check if the value starts with '#' to indicate a sheet reference
            if let Some(sheet_name) = cell_string.strip_prefix('#') {
                let sheet = self.sheets.get(sheet_name).ok_or_else(|| {
                    let available: Vec<_> = self.sheets.keys().map(|s| s.as_str()).collect();
                    VariantError::RetrievalError(format!(
                        "Sheet not found: '{}'. Available sheets: {}",
                        sheet_name,
                        available.join(", ")
                    ))
                })?;

                let mut out = Vec::new();

                for row in sheet.rows().skip(1) {
                    match row.first() {
                        Some(cell) if !Self::cell_is_empty(cell) => {
                            let v = Self::convert_sheet_cell(cell, true, "1D array")?;
                            out.push(v);
                        }
                        _ => break,
                    }
                }
                return Ok(ValueSource::Array(out));
            }

            // No '#' prefix, treat as a literal string
            Ok(ValueSource::Single(DataValue::Str(cell_string.to_owned())))
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    pub fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, VariantError> {
        let result = (|| {
            let Data::String(cell_string) = self.retrieve_cell(name)? else {
                return Err(VariantError::RetrievalError(
                    "Expected string value for 2D array".to_string(),
                ));
            };

            let sheet_name = cell_string.strip_prefix('#').ok_or_else(|| {
                VariantError::RetrievalError(format!(
                    "2D array reference must start with '#' prefix, got: {}",
                    cell_string
                ))
            })?;

            let sheet = self.sheets.get(sheet_name).ok_or_else(|| {
                let available: Vec<_> = self.sheets.keys().map(|s| s.as_str()).collect();
                VariantError::RetrievalError(format!(
                    "Sheet not found: '{}'. Available sheets: {}",
                    sheet_name,
                    available.join(", ")
                ))
            })?;

            let convert = |cell: &Data| -> Result<DataValue, VariantError> {
                Self::convert_sheet_cell(cell, false, "2D array")
            };

            let mut rows = sheet.rows();
            let hdrs = rows.next().ok_or_else(|| {
                VariantError::RetrievalError("No headers found in 2D array".to_string())
            })?;
            let width = hdrs.iter().take_while(|c| !Self::cell_is_empty(c)).count();
            if width == 0 {
                return Err(VariantError::RetrievalError(
                    "Detected zero width 2D array".to_string(),
                ));
            }

            let mut out = Vec::new();

            'outer: for row in rows {
                if row.first().is_none_or(Self::cell_is_empty) {
                    break;
                }

                let mut vals = Vec::with_capacity(width);
                for col in 0..width {
                    let Some(cell) = row.get(col) else {
                        break 'outer;
                    };
                    if Self::cell_is_empty(cell) {
                        break 'outer;
                    };
                    vals.push(convert(cell)?);
                }
                out.push(vals);
            }

            Ok(out)
        })();

        result.map_err(|e| VariantError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    fn retrieve_cell(&self, name: &str) -> Result<&Data, VariantError> {
        let index =
            self.names
                .iter()
                .position(|n| n == name)
                .ok_or(VariantError::RetrievalError(
                    "index not found in data sheet".to_string(),
                ))?;

        if let Some(v) = [
            self.debug_values.as_ref().and_then(|v| v.get(index)),
            self.variant_values.as_ref().and_then(|v| v.get(index)),
            self.default_values.get(index),
        ]
        .iter()
        .flatten()
        .find(|d| !Self::cell_is_empty(d))
        {
            return Ok(v);
        }

        Err(VariantError::RetrievalError(
            "data not found in any variant column".to_string(),
        ))
    }

    fn cell_eq_ascii(cell: &Data, target: &str) -> bool {
        match cell {
            Data::String(s) => s.trim().eq_ignore_ascii_case(target),
            _ => false,
        }
    }

    fn cell_is_empty(cell: &Data) -> bool {
        match cell {
            Data::Empty => true,
            Data::String(s) => s.trim().is_empty(),
            _ => false,
        }
    }

    fn parse_bool_string(input: &str) -> Option<bool> {
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("true") {
            Some(true)
        } else if trimmed.eq_ignore_ascii_case("false") {
            Some(false)
        } else {
            None
        }
    }

    fn convert_sheet_cell(
        cell: &Data,
        allow_plain_strings: bool,
        context: &str,
    ) -> Result<DataValue, VariantError> {
        match cell {
            Data::Int(i) => Ok(DataValue::I64(*i)),
            Data::Float(f) => Ok(DataValue::F64(*f)),
            Data::Bool(b) => Ok(DataValue::Bool(*b)),
            Data::String(s) => {
                if let Some(b) = Self::parse_bool_string(s) {
                    Ok(DataValue::Bool(b))
                } else if allow_plain_strings {
                    Ok(DataValue::Str(s.to_owned()))
                } else {
                    Err(VariantError::RetrievalError(format!(
                        "Unsupported data type in {}",
                        context
                    )))
                }
            }
            _ => Err(VariantError::RetrievalError(format!(
                "Unsupported data type in {}",
                context
            ))),
        }
    }

    // TODO: retrieve sheets by name, data format to be decided
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn datasheet_with_default(value: Data) -> DataSheet {
        DataSheet {
            names: vec!["Flag".to_string()],
            default_values: vec![value],
            debug_values: None,
            variant_values: None,
            sheets: HashMap::new(),
        }
    }

    #[test]
    fn retrieve_single_value_accepts_bool_cell() {
        let ds = datasheet_with_default(Data::Bool(true));
        let value = ds.retrieve_single_value("Flag").expect("bool cell");
        match value {
            DataValue::Bool(v) => assert!(v),
            _ => panic!("expected bool value"),
        }
    }

    #[test]
    fn retrieve_single_value_accepts_bool_string() {
        let ds = datasheet_with_default(Data::String(" False ".to_string()));
        let value = ds.retrieve_single_value("Flag").expect("bool string");
        match value {
            DataValue::Bool(v) => assert!(!v),
            _ => panic!("expected bool value"),
        }
    }

    #[test]
    fn convert_sheet_cell_handles_bool_strings() {
        let bool_cell = Data::Bool(false);
        let value =
            DataSheet::convert_sheet_cell(&bool_cell, true, "1D array").expect("bool cell");
        match value {
            DataValue::Bool(v) => assert!(!v),
            _ => panic!("expected bool"),
        }

        let bool_str = Data::String("TRUE".to_string());
        let value =
            DataSheet::convert_sheet_cell(&bool_str, false, "2D array").expect("bool string");
        match value {
            DataValue::Bool(v) => assert!(v),
            _ => panic!("expected bool"),
        }

        let plain_str = Data::String("Hello".to_string());
        let value =
            DataSheet::convert_sheet_cell(&plain_str, true, "1D array").expect("plain string");
        match value {
            DataValue::Str(s) => assert_eq!(s, "Hello"),
            _ => panic!("expected string"),
        }

        let err = DataSheet::convert_sheet_cell(&plain_str, false, "2D array")
            .expect_err("should reject non-bool string in 2D arrays");
        match err {
            VariantError::RetrievalError(msg) => {
                assert!(msg.contains("2D array"), "unexpected message: {}", msg)
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
