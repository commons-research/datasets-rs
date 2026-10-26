use std::{fmt, str::FromStr};
use datasets_core::ArtifactFormat;
use super::error::CrawlerError;

/// File format exposed by dataset source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileFormat {
    /// Comma-separated values
    Csv,
    /// JSON document
    Json,
}

impl FileFormat {
    /// Returns the canonical name of the file format
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }

    /// Returns expected file extension
    #[must_use]
    pub const fn extension(self) -> &'static str {
        self.as_str()
    }

    /// Returns structural artifact format
    #[must_use]
    pub const fn artifact_format(self) -> ArtifactFormat {
        match self {
            Self::Csv => ArtifactFormat::Tabular,
            Self::Json => ArtifactFormat::Json,
        }
    }
}

impl fmt::Display for FileFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FileFormat {
    type Err = CrawlerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            _ => Err(CrawlerError::ParseFileFormat(s.to_owned(),
            ))
        }
    }
}

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
        formats: &'a [FileFormat],
        limit: Option<usize>,
    ) -> impl Future<Output = Result<Vec<DatasetSummary>, CrawlerError>> + Send + 'a;

    /// Lists files available in dataset
    ///
    /// # Errors
    /// - returns error if repo listing fails
    fn list_artifacts<'a>(
        &'a self,
        dataset_id: &'a str,
        limit: Option<usize>,
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
