pub mod args;
pub mod errors;
mod helpers;

use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use std::collections::{HashMap, HashSet};

use crate::layout::value::{DataValue, ValueSource};
use errors::VariantError;

pub struct DataSheet {
    names: Vec<String>,
    default_values: Vec<Data>,
    variant_columns: Vec<Vec<Data>>,
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

        let default_values = Self::collect_column(&rows, default_index, data_rows);

        let variant_columns = Self::collect_variant_columns(headers, &rows, data_rows, args)?;

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
            variant_columns,
            sheets,
        }))
    }

    pub fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError> {
        let result = (|| match self.retrieve_cell(name)? {
            Data::Int(i) => Ok(DataValue::I64(*i)),
            Data::Float(f) => Ok(DataValue::F64(*f)),
            Data::Bool(b) => Ok(DataValue::Bool(*b)),
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
                            let v = match cell {
                                Data::Int(i) => DataValue::I64(*i),
                                Data::Float(f) => DataValue::F64(*f),
                                Data::Bool(b) => DataValue::Bool(*b),
                                Data::String(s) => DataValue::Str(s.to_owned()),
                                _ => {
                                    return Err(VariantError::RetrievalError(
                                        "Unsupported data type in 1D array".to_string(),
                                    ));
                                }
                            };
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
                match cell {
                    Data::Int(i) => Ok(DataValue::I64(*i)),
                    Data::Float(f) => Ok(DataValue::F64(*f)),
                    Data::Bool(b) => Ok(DataValue::Bool(*b)),
                    _ => Err(VariantError::RetrievalError(
                        "Unsupported data type in 2D array".to_string(),
                    )),
                }
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

        for column in &self.variant_columns {
            if let Some(value) = column.get(index)
                && !Self::cell_is_empty(value)
            {
                return Ok(value);
            }
        }

        if let Some(default) = self.default_values.get(index)
            && !Self::cell_is_empty(default)
        {
            return Ok(default);
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

    fn collect_column(rows: &[&[Data]], index: usize, data_rows: usize) -> Vec<Data> {
        let mut column = Vec::with_capacity(data_rows);
        column.extend(
            rows.iter()
                .skip(1)
                .map(|row| row.get(index).cloned().unwrap_or(Data::Empty)),
        );
        column
    }

    fn parse_variant_stack(raw: &str) -> Vec<String> {
        raw.split('/')
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
            .collect()
    }

    fn collect_variant_columns(
        headers: &[Data],
        rows: &[&[Data]],
        data_rows: usize,
        args: &args::VariantArgs,
    ) -> Result<Vec<Vec<Data>>, VariantError> {
        let mut names: Vec<String> = Vec::new();

        if args.debug {
            eprintln!("Warning: --debug flag is deprecated; include 'Debug' in --variant instead.");
            names.push("Debug".to_string());
        }

        if let Some(raw) = &args.variant {
            names.extend(Self::parse_variant_stack(raw));
        }

        let mut seen = HashSet::new();
        let mut columns = Vec::new();

        for name in names {
            if !seen.insert(name.clone()) {
                continue;
            }

            let index = headers
                .iter()
                .position(|cell| Self::cell_eq_ascii(cell, &name))
                .ok_or_else(|| VariantError::ColumnNotFound(name.clone()))?;

            columns.push(Self::collect_column(rows, index, data_rows));
        }

        Ok(columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn datasheet_with_default(value: Data) -> DataSheet {
        DataSheet {
            names: vec!["Flag".to_string()],
            default_values: vec![value],
            variant_columns: Vec::new(),
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
}
