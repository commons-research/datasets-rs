use std::error::Error;

/// Metadata describing dataset discovered in public repo
#[derive(Debug, Clone)]
pub(crate) struct DatasetSummary {
    /// Stable identifier assigned by source
    pub(crate) id: String,
    /// URL of the dataset location
    pub(crate) url: String,
    /// Combined metadata used for text filtering
    pub(crate) searchable_metadata: String,
}

/// File available for inspection within remote dataset
#[derive(Debug, Clone)]
pub(crate) struct SourceArtifact {
    /// Path identifying the file within source repo
    pub(crate) path: String,
    /// File size in bytes
    pub(crate) size_bytes: u64,
}

/// Operations required to discover and retrieve remote datasets
#[allow(async_fn_in_trait)]
pub(crate) trait DatasetSource {
    /// Error type returned by source
    type Error: Error + Send + Sync + 'static;

    /// Searches for datasets matching search term
    /// 
    /// # Errors
    /// - Returns error if search fails
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DatasetSummary>, Self::Error>;

    /// Lists files available in dataset
    /// 
    /// # Errors
    /// - returns error if repo listing fails
    async fn list_artifacts(
        &self,
        dataset_id: &str,
        limit: usize,
    ) -> Result<Vec<SourceArtifact>, Self::Error>;

    /// Downloads an artifact and returns content
    /// 
    /// # Errors
    /// - Returns error if artifact cannot be retrieved
    async fn download_artifact(
        &self,
        dataset_id: &str,
        artifact: &SourceArtifact,
    ) -> Result<Vec<u8>, Self::Error>;
}