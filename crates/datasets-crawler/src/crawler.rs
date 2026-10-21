use datasets_core::{
    ArtifactFormat,
    ContentType,
    CoreError,
    classify_artifact
};

use super::{DatasetIndex, IndexedArtifact};

/// An artifact discovered by a dataset source for inspection
#[derive(Debug, Clone)]
pub struct CandidateArtifact {
    dataset_id: String,
    dataset_url: String,
    artifact_ref: String,
    format: ArtifactFormat,
    data: Vec<u8>
}

impl CandidateArtifact {
    /// Create a candidate artifact
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        format: ArtifactFormat,
        data: Vec<u8>
    ) -> Self {
        Self { dataset_id: dataset_id.into(), dataset_url: dataset_url.into(), artifact_ref: artifact_ref.into(), format, data }
    }
}