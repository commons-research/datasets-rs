use thiserror::Error;

/// Errors produced while inspecting dataset contents.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// Tabular data could not be parsed.
    #[error("failed to parse tabular data: {0}")]
    Tabular(#[from] csv::Error),

    /// JSON data could not be parsed
    #[error("failed to parse JSON data: {0}")]
    Json(#[from] serde_json::Error),

    /// A string could not be parsed as a supported scientific content type
    #[error("unsupported content type: {value}")]
    ParseContentTypeError {
        /// Value not able to parse
        value: String,
    },
}
