//! Dataset crawler application and utilities

/// Indexes artifact records
mod index;

/// Crawler implementation for crawling datasets for evaluation
mod crawler;

/// Implementations for external dataset sources
mod sources;

pub use crawler::{CandidateArtifact, Crawler};
pub use index::{DatasetIndex, IndexedArtifact};
pub use sources::HuggingFaceSource;
