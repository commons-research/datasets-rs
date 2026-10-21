//! Dataset crawler application and utilities

/// Indexes artifact records
mod index;

/// Crawler implementation for crawling datasets for evaluation
mod crawler;

pub use crawler::{CandidateArtifact, Crawler};
pub use index::{DatasetIndex, IndexedArtifact};
