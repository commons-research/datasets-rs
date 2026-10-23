use std::error::Error;

use super::error::CrawlerError;

/// Metadata describing dataset discovered in public repo
#[derive(Debug, Clone)]
pub struct DatasetSummary {
    /// Stable identifier assigned by source
    pub id: String,
    /// URL of the dataset location
    pub url: String,
    /// Combined metadata used for text filtering
    pub searchable_metadata: String,
}

/// File available for inspection within remote dataset
#[derive(Debug, Clone)]
pub struct SourceArtifact {
    /// Path identifying the file within source repo
    pub path: String,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Operations required to discover and retrieve remote datasets
pub trait DatasetSource: Send + Sync {
    /// Searches for datasets matching search term
    ///
    /// # Errors
    /// - Returns error if search fails
    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<DatasetSummary>, CrawlerError>> + Send + 'a;

    /// Lists files available in dataset
    ///
    /// # Errors
    /// - returns error if repo listing fails
    fn list_artifacts<'a>(
        &'a self,
        dataset_id: &'a str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<SourceArtifact>, CrawlerError>> + Send + 'a;

    /// Downloads an artifact and returns content
    ///
    /// # Errors
    /// - Returns error if artifact cannot be retrieved
    fn download_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> impl Future<Output = Result<Vec<u8>, CrawlerError>> + Send + 'a;
}
