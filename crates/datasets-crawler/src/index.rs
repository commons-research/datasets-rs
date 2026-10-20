use std::collections::{HashMap, HashSet};

use datasets_core::{Classification, ContentType};

/// An artifact and its scientific classification evidence.
#[derive(Debug,Clone, PartialEq, Eq)]
pub struct IndexedArtifact {
    dataset_id: String,
    dataset_url: String,
    artifact_ref: String,
    classifications: Vec<Classification>,
}

impl IndexedArtifact {
    /// Creates an indexed artifact record
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        classifications: Vec<Classification>
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            dataset_url: dataset_url.into(),
            artifact_ref: artifact_ref.into(),
            classifications
        }
    }

    /// Returns the stable dataset identifier
    #[must_use]
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    /// Returns the dataset overview URL
    #[must_use]
    pub fn dataset_url(&self) -> &str {
        &self.dataset_url
    }

    /// Returns the artifact reference, e.g. repo path or URL
    #[must_use]
    pub fn artifact_ref(&self) -> &str {
        &self.artifact_ref
    }

    /// Returns the classification evidence for this artifact
    #[must_use]
    pub fn classification(&self) -> &[Classification] {
        &self.classifications
    }
}

/// An in-memory index of classified dataset artifacts
#[derive(Debug, Default)]
pub struct DatasetIndex {
    /// The indexed artifacts
    artifacts: Vec<IndexedArtifact>,
    /// Content type hashmap pointing to artifact indices
    by_content_type: HashMap<ContentType, Vec<usize>>
}

impl DatasetIndex {
    /// Creates an empty index
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an artifact and indexes its successfully validated content types
    pub fn insert(&mut self, artifact: IndexedArtifact) {
        let artifact_index = self.artifacts.len();

        let content_types: HashSet<ContentType> = artifact.classification().iter().filter(|classification| classification.valid() > 0).map(Classification::content_type).collect();

        for content_type in content_types {
            self.by_content_type.entry(content_type).or_default().push(artifact_index);
        }
        self.artifacts.push(artifact);
    }

    /// Returns artifacts with at least one valid instance of the sought content type
    #[must_use]
    pub fn search(
        &self,
        content_type: ContentType,
    ) -> impl Iterator<Item = &IndexedArtifact> + '_ {
        self.by_content_type.get(&content_type).into_iter().flatten().map(|&index| &self.artifacts[index])
    }
}