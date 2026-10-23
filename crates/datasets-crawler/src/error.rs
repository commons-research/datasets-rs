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
    /// Reading downloaded artifact failed
    #[error("failed to read downloaded artifact: {0}")]
    Io(#[from] io::Error),
    /// Tokio async runtime failure
    #[error("failed to initialize Tokio runtime: {0}")]
    Runtime(#[source] io::Error),
}
