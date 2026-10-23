//! Dataset crawler library and application utilities.

/// Shared discovery and crawling workflow.
mod crawl;
/// Crawler implementation for classifying discovered artifacts.
mod crawler;
/// Indexes classified artifact records.
mod index;
/// Common interface for remote dataset sources.
mod source;
/// Implementations for external dataset sources.
mod sources;

pub use crawl::{CrawlConfig, CrawlReport, crawl};
pub use crawler::{CandidateArtifact, Crawler};
pub use index::{DatasetIndex, IndexedArtifact};
pub use source::{DatasetSource, DatasetSummary, SourceArtifact};
pub use sources::HuggingFaceSource;
