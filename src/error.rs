use thiserror::Error;

use crate::layout::errors::LayoutError;
use crate::output::errors::OutputError;
use crate::variant::errors::VariantError;

#[derive(Debug, Error)]
pub enum NvmError {
    #[error(transparent)]
    Layout(#[from] LayoutError),

    #[error(transparent)]
    Variant(#[from] VariantError),

    #[error(transparent)]
    Output(#[from] OutputError),

    #[error("While building block '{block_name}' from '{layout_file}': {source}")]
    InBlock {
        block_name: String,
        layout_file: String,
        #[source]
        source: Box<NvmError>,
    },
}
