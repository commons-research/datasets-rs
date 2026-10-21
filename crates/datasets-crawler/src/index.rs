use std::collections::{HashMap, HashSet};

use datasets_core::{Classification, ContentType};

/// An artifact and its scientific classification evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedArtifact {
    /// Stable identifier of the parent dataset
    dataset_id: String,
    /// URL of the parent dataset
    dataset_url: String,
    /// Reference identifying this artifact within the dataset
    artifact_ref: String,
    /// Scientific classification evidence collected for this artifact
    classifications: Vec<Classification>,
}

impl IndexedArtifact {
    /// Creates an indexed artifact record
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        classifications: Vec<Classification>,
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            dataset_url: dataset_url.into(),
            artifact_ref: artifact_ref.into(),
            classifications,
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
    pub fn classifications(&self) -> &[Classification] {
        &self.classifications
    }
}

/// An in-memory index of classified dataset artifacts
#[derive(Debug, Default)]
pub struct DatasetIndex {
    /// The indexed artifacts
    artifacts: Vec<IndexedArtifact>,
    /// Content type hashmap pointing to artifact indices
    by_content_type: HashMap<ContentType, Vec<usize>>,
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

        let content_types: HashSet<ContentType> = artifact
            .classifications()
            .iter()
            .filter(|classification| classification.valid() > 0)
            .map(Classification::content_type)
            .collect();

        for content_type in content_types {
            self.by_content_type.entry(content_type).or_default().push(artifact_index);
        }
        self.artifacts.push(artifact);
    }

    /// Returns artifacts with at least one valid instance of the sought content
    /// type
    pub fn search(&self, content_type: ContentType) -> impl Iterator<Item = &IndexedArtifact> + '_ {
        self.by_content_type
            .get(&content_type)
            .into_iter()
            .flatten()
            .map(|&index| &self.artifacts[index])
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn indexes_artifacts_by_validated_content() {
        let mut index = DatasetIndex::new();

        index.insert(IndexedArtifact::new(
            "example-dataset",
            "https://example.org/datasets/example",
            "data.csv",
            vec![
                Classification::new(ContentType::Smiles, "smiles", 3, 2),
                Classification::new(ContentType::MolecularFormula, "formula", 3, 2),
            ],
        ));

        index.insert(IndexedArtifact::new(
            "invalid-dataset",
            "https://example.org/datasets/invalid",
            "data.csv",
            vec![
                Classification::new(ContentType::Smiles, "smiles", 3, 0),
                Classification::new(ContentType::MolecularFormula, "formula", 3, 0),
            ],
        ));

        let smiles_matches: Vec<_> = index.search(ContentType::Smiles).collect();

        assert_eq!(smiles_matches.len(), 1);
        assert_eq!(index.search(ContentType::MolecularFormula).count(), 1);

        assert_eq!(smiles_matches[0].dataset_id(), "example-dataset");
        assert_eq!(smiles_matches[0].artifact_ref(), "data.csv");

        assert_eq!(
            smiles_matches[0].classifications().len(),
            2,
            "search results should preserve all classification evidence"
        );
    }

    #[test]
    fn returns_no_matches_for_unindexed_content() {
        let index = DatasetIndex::new();

        assert_eq!(index.search(ContentType::Smiles).count(), 0);
    }
}
