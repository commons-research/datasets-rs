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
}