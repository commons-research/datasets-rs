//! Dataset crawler application and utilities

/// Indexes artifact records
mod index;

/// Crawler implementation for crawling datasets for evaluation
mod crawler;

/// Implementations for external dataset sources
mod sources;
/// Source metadata definitions and trait
mod source;

pub use crawler::{CandidateArtifact, Crawler};
pub use index::{DatasetIndex, IndexedArtifact};
pub use sources::HuggingFaceSource;
pub use source::{SourceArtifact, DatasetSummary, DatasetSource};