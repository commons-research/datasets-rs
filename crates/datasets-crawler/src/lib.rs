//! Dataset crawler application and utilities

/// Indexes artifact records
mod index;

/// Crawler implementation for crawling datasets for evaluation
mod crawler;

/// Specific implementations for existing data lakes to crawl
mod sources;

pub use crawler::{CandidateArtifact, Crawler};
pub use index::{DatasetIndex, IndexedArtifact};
pub use sources::HuggingFaceSource;