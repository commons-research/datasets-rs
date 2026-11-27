//! Dataset crawler library and application utilities.
#![allow(clippy::future_not_send)]
/// Manages and tracks crawl progress at the artifact level.
mod artifact;
/// Shared discovery and crawling workflow.
mod crawl;
/// Handles the indexing database control
mod database;
/// Crawler error definitions
mod error;
/// Tracking for dataset download progress
mod progress;
/// Bridges asynchronous artifact streams to blocking readers.
mod reader;
/// Manages and tracks crawl progress at the repository level.
mod repository;
/// Common interface for remote dataset sources.
mod source;
/// Implementations for external dataset sources.
mod sources;

pub use crawl::{CrawlConfig, CrawlStatus, crawl};
pub use database::Database;
pub use error::CrawlerError;
pub use repository::{Failure, RepositoryAssessment, RepositoryPolicy, RepositoryResult};
pub use source::{DatasetSource, FileFormat};
pub use sources::{HuggingFaceSource, ZenodoSource};
