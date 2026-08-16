use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataError {
    #[error("file error: {0}")]
    FileError(String),

    #[error("Excel column not found: {0}")]
    ColumnNotFound(String),

    #[error("retrieval error: {0}")]
    RetrievalError(String),

    #[error("data source error: {0}")]
    MiscError(String),

    #[error("while retrieving '{name}'")]
    WhileRetrieving {
        name: String,
        #[source]
        source: Box<DataError>,
    },
}

impl DataError {
    pub(super) fn while_retrieving<T>(
        name: &str,
        retrieve: impl FnOnce() -> Result<T, Self>,
    ) -> Result<T, Self> {
        retrieve().map_err(|source| Self::WhileRetrieving {
            name: name.to_owned(),
            source: Box::new(source),
        })
    }
}
