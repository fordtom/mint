use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::{HashMap, HashSet};

use super::args::VariantArgs;
use super::errors::VariantError;
use super::helpers;
use super::DataSource;
use crate::layout::value::{DataValue, ValueSource};

/// Excel-backed data source for variant values.
pub struct ExcelDataSource {
    names: Vec<String>,
    variant_columns: Vec<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl ExcelDataSource {
    pub(crate) fn new(args: &VariantArgs) -> Result<Self, VariantError> {
        let xlsx_path = args.xlsx.as_ref().expect("xlsx path required");

        let mut workbook: Xlsx<_> = open_workbook(xlsx_path)
            .map_err(|_| VariantError::FileError(format!("failed to open file: {}", xlsx_path)))?;

        let main_sheet_name = args.main_sheet.as_deref().unwrap_or("Main");
        let main_sheet = workbook
            .worksheet_range(main_sheet_name)
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

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| {
            row.get(name_index)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default()
        }));
        helpers::warn_duplicate_names(&names);

        let variant_columns = Self::collect_variant_columns(headers, &rows, data_rows, args)?;

        let mut sheets: HashMap<String, Range<Data>> =
            HashMap::with_capacity(workbook.worksheets().len().saturating_sub(1));
        for (name, sheet) in workbook.worksheets() {
            if name != main_sheet_name {
                sheets.insert(name.to_string(), sheet);
            }
        }

        Ok(Self {
            names,
            variant_columns,
            sheets,
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
            if let Some(value) = column.get(index) {
                if !Self::cell_is_empty(value) {
                    return Ok(value);
                }
            }
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

    fn collect_variant_columns(
        headers: &[Data],
        rows: &[&[Data]],
        data_rows: usize,
        args: &VariantArgs,
    ) -> Result<Vec<Vec<Data>>, VariantError> {
        let variants = args.get_variant_list();

        let mut seen = HashSet::new();
        let mut columns = Vec::new();

        for v in variants {
            if seen.insert(v.clone()) {
                let index = headers
                    .iter()
                    .position(|cell| Self::cell_eq_ascii(cell, &v))
                    .ok_or_else(|| VariantError::ColumnNotFound(v.clone()))?;

                columns.push(Self::collect_column(rows, index, data_rows));
            }
        }

        Ok(columns)
    }
}

impl DataSource for ExcelDataSource {
    fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError> {
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

    fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, VariantError> {
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

    fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, VariantError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;
    use std::collections::HashMap;

    fn datasource_with_variant(value: Data) -> ExcelDataSource {
        ExcelDataSource {
            names: vec!["Flag".to_string()],
            variant_columns: vec![vec![value]],
            sheets: HashMap::new(),
        }
    }

    #[test]
    fn retrieve_single_value_accepts_bool_cell() {
        let ds = datasource_with_variant(Data::Bool(true));
        let value = ds.retrieve_single_value("Flag").expect("bool cell");
        match value {
            DataValue::Bool(v) => assert!(v),
            _ => panic!("expected bool value"),
        }
    }
}
