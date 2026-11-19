use std::io;

use datasets_core::CoreError;
use thiserror::Error;

/// Error handling for crawling datasets
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CrawlerError {
    /// Artifact not classifiable
    #[error("dataset classification failed: {0}")]
    Core(#[from] CoreError),
    /// Operation against HF failed
    #[error("Hugging Face operation failed: {0}")]
    HuggingFace(#[from] hf_hub::HFError),
    /// Error returned from Zenodo client
    #[error("Zenodo request failed: {0}")]
    Zenodo(#[from] zenodo_rs::ZenodoError),
    /// Zenodo ID was invalid
    #[error("invalid Zenodo record ID: {0}")]
    InvalidZenodoRecordId(String),
    /// Reading downloaded artifact failed
    #[error("failed to read downloaded artifact: {0}")]
    Io(#[from] io::Error),
    /// Tokio async runtime failure
    #[error("failed to initialize Tokio runtime: {0}")]
    Runtime(#[source] io::Error),
    /// The selected source is not available in this build
    #[error("source is not available yet: {0}")]
    SourceUnavailable(&'static str),
    /// Parsing file format failed
    #[error("failed to parse file format: {0}")]
    ParseFileFormat(String),
    /// Error operating sqlite database
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    /// Failed to convert an integer
    #[error("integer conversion failed: {0}")]
    IntegerConversion(#[from] std::num::TryFromIntError),
}
