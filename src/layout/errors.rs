use thiserror::Error;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("File error: {0}.")]
    FileError(String),

    #[error("Block not found: {0}.")]
    BlockNotFound(String),

    #[error("Data value export failed: {0}.")]
    DataValueExportFailed(String),

    #[error("Invalid block argument: {0}.")]
    InvalidBlockArgument(String),

    #[error("No blocks provided.")]
    NoBlocksProvided,

    #[error("No blocks found in layout file: {0}.")]
    NoBlocksInLayout(String),

    #[error("Missing datasheet: {0}")]
    MissingDataSheet(String),

    #[error("In field '{field}': {source}")]
    InField {
        field: String,
        #[source]
        source: Box<LayoutError>,
    },

    #[error(transparent)]
    Variant(#[from] crate::variant::errors::VariantError),
}
