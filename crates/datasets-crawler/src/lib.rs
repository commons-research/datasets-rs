//! Dataset crawler library and application utilities.

/// Shared discovery and crawling workflow.
mod crawl;
/// Handles the indexing database control
mod database;
/// Crawler error definitions
mod error;
/// Indexes classified artifact records.
mod index;
/// Tracking for dataset download progress
mod progress;
/// Bridges asynchronous artifact streams to blocking readers.
mod reader;
/// Common interface for remote dataset sources.
mod source;
/// Implementations for external dataset sources.
mod sources;

pub use crawl::{CrawlConfig, CrawlStatus, crawl};
pub use error::CrawlerError;
pub use index::{DatasetIndex, IndexedArtifact};
pub use source::{DatasetSource, FileFormat, RepositorySummary, SourceArtifact};
// pub use sources::{HuggingFaceSource, ZenodoSource};
pub use sources::HuggingFaceSource;
