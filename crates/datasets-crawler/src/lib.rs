//! Dataset crawler library and application utilities.

/// Shared discovery and crawling workflow.
mod crawl;
/// Crawler implementation for classifying discovered artifacts.
mod crawler;
/// Crawler error definitions
mod error;
/// Indexes classified artifact records.
mod index;
/// Tracking for dataset download progress
mod progress;
/// Common interface for remote dataset sources.
mod source;
/// Implementations for external dataset sources.
mod sources;

pub use crawl::{CrawlConfig, CrawlReport, crawl};
pub use crawler::{CandidateArtifact, Crawler};
pub use error::CrawlerError;
pub use index::{DatasetIndex, IndexedArtifact};
pub use source::{DatasetSource, DatasetSummary, FileFormat, SourceArtifact};
pub use sources::{HuggingFaceSource, ZenodoSource};
