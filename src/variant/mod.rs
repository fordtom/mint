pub mod args;
pub mod errors;
mod excel;
mod helpers;

use crate::layout::value::{DataValue, ValueSource};
use errors::VariantError;
use excel::ExcelDataSource;

/// Trait for data sources that provide variant values by name.
pub trait DataSource: Sync {
    /// Retrieves a single numeric or boolean value.
    fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError>;

    /// Retrieves a 1D array (from sheet reference) or a literal string.
    fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, VariantError>;

    /// Retrieves a 2D array from a sheet reference.
    fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, VariantError>;
}

/// Creates a data source from CLI arguments.
///
/// Returns `None` if no data source is configured (e.g., no `--xlsx` provided).
pub fn create_data_source(
    args: &args::VariantArgs,
) -> Result<Option<Box<dyn DataSource>>, VariantError> {
    // We need to check that if one of these is provided, -v also exists
    if args.xlsx.is_some() {
        Ok(Some(Box::new(ExcelDataSource::new(args)?)))
    } else {
        Ok(None)
    }
}
