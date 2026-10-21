use datasets_core::{ArtifactFormat, ContentType, CoreError, classify_artifact};

use super::{DatasetIndex, IndexedArtifact};

/// An artifact discovered by a dataset source for inspection
#[derive(Debug, Clone)]
pub struct CandidateArtifact {
    dataset_id: String,
    dataset_url: String,
    artifact_ref: String,
    format: ArtifactFormat,
    data: Vec<u8>,
}

impl CandidateArtifact {
    /// Create a candidate artifact
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        format: ArtifactFormat,
        data: Vec<u8>,
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            dataset_url: dataset_url.into(),
            artifact_ref: artifact_ref.into(),
            format,
            data,
        }
    }
}

/// Classifies discovered artifacts and contributes them to index
#[derive(Debug)]
pub struct Crawler {
    content_types: Vec<ContentType>,
    index: DatasetIndex,
}

impl Crawler {
    /// Creates a crawler set to search for the specified content types
    #[must_use]
    pub fn new(content_types: &[ContentType]) -> Self {
        let mut requested = Vec::new();

        for content_type in content_types {
            if !requested.contains(content_type) {
                requested.push(*content_type);
            }
        }
        Self { content_types: requested, index: DatasetIndex::new() }
    }

    /// Does the classifying and indexing of a discovered artifact
    ///
    /// # Errors
    /// - Returns [`CoreError`] if the artifact cannot be classified
    pub fn process(&mut self, artifact: CandidateArtifact) -> Result<(), CoreError> {
        let classifications =
            classify_artifact(&artifact.data, artifact.format, &self.content_types)?;

        self.index.insert(IndexedArtifact::new(
            artifact.dataset_id,
            artifact.dataset_url,
            artifact.artifact_ref,
            classifications,
        ));

        Ok(())
    }

    /// Returns the crawler's current in-memory index
    #[must_use]
    pub const fn index(&self) -> &DatasetIndex {
        &self.index
    }
}
