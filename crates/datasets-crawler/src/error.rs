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
    /// Reading downloaded artifact failed
    #[error("failed to read downloaded artifact: {0}")]
    Io(#[from] io::Error),
    /// Tokio async runtime failure
    #[error("failed to initialize Tokio runtime: {0}")]
    Runtime(#[source] io::Error),
    /// Tracing subscriber init failed
    #[error("failed to initialize logging: {0}")]
    Logging(String),
    /// Parsing file format failed
    #[error("failed to parse file format: {0}")]
    ParseFileFormat(String),
}
