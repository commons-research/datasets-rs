use std::{fmt, str::FromStr};

use datasets_core::ArtifactFormat;
use futures::AsyncRead;
use strum::VariantArray;

use super::error::CrawlerError;

/// File format exposed by dataset source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, VariantArray)]
#[non_exhaustive]
pub enum FileFormat {
    /// Comma-separated values
    Csv,
    /// JSON document
    Json,
    /// Plain text formatted files
    PlainText,
}

impl FileFormat {
    /// Returns the canonical name of the file format
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::PlainText => "plain-text",
        }
    }

    /// Returns structural artifact format
    #[must_use]
    pub const fn artifact_format(self) -> ArtifactFormat {
        match self {
            Self::Csv | Self::PlainText => ArtifactFormat::Tabular,
            Self::Json => ArtifactFormat::Json,
        }
    }

    /// Returns all supported file formats
    #[must_use]
    pub fn all_formats() -> Vec<Self> {
        Self::VARIANTS.to_vec()
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
            "txt" | "text" | "plain-text" => Ok(Self::PlainText),
            _ => Err(CrawlerError::ParseFileFormat(s.to_owned())),
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
    /// Discovers datasets available from the remote source.
    ///
    /// Parameters:
    /// - `query`: Optional source-specific search query. If `None`, discovery
    ///   is unrestricted by search terms.
    /// - `limit`: Optional maximum number of datasets to return.
    ///
    /// Errors:
    /// - Returns an error if dataset discovery fails.
    fn discover_datasets<'a>(
        &'a self,
        query: Option<&'a str>,
        limit: Option<usize>,
    ) -> impl Future<Output = Result<Vec<DatasetSummary>, CrawlerError>> + Send + 'a;

    /// Lists the artifacts available within a dataset.
    ///
    /// Parameters:
    /// - `dataset_id`: Identifier of the dataset whose artifacts should be
    ///   listed.
    ///
    /// Errors:
    /// - Returns an error if artifact listing fails.
    fn list_artifacts<'a>(
        &'a self,
        dataset_id: &'a str,
    ) -> impl Future<Output = Result<Vec<SourceArtifact>, CrawlerError>> + Send + 'a;

    /// Opens a remote artifact for asynchronous reading.
    ///
    /// # Parameters
    /// - `dataset_id`: Identifier of the dataset containing the artifact.
    /// - `artifact`: Metadata describing the artifact to open.
    ///
    /// # Errors
    /// - Returns an error if the artifact cannot be opened.
    fn open_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> impl Future<Output = Result<impl AsyncRead + Send + Unpin + 'a, CrawlerError>> + Send + 'a;
}
