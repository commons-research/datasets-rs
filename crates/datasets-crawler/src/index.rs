use std::collections::HashMap;

use datasets_core::{Classification, ContentType, Inspection};

/// Fewest valid values a location needs before it is counted as a match
const MIN_VALID: usize = 2;

/// Smallest share of inspected values that must be valid to be counted as a
/// match.
const MIN_VALID_RATIO: f64 = 0.5;

/// An artifact and its scientific classification evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedArtifact {
    /// Stable identifier of the parent dataset
    dataset_id: String,
    /// URL of the parent dataset
    dataset_url: String,
    /// Reference identifying this artifact within the dataset
    artifact_ref: String,
    /// Evidence collected for this artifact and degree of inspection
    inspection: Inspection,
}

impl IndexedArtifact {
    /// Creates an indexed artifact record
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        inspection: Inspection,
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            dataset_url: dataset_url.into(),
            artifact_ref: artifact_ref.into(),
            inspection,
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

    /// Returns the inspection result, including degree of inspection.
    #[must_use]
    pub const fn inspection(&self) -> &Inspection {
        &self.inspection
    }

    /// Returns all classification evidence for this artifact
    #[must_use]
    pub fn classifications(&self) -> &[Classification] {
        &self.inspection.classifications
    }

    /// Returns the locations with supporting evidence of content type
    pub fn matches(&self, content_type: ContentType) -> impl Iterator<Item = &Classification> + '_ {
        self.classifications().iter().filter(move |classification| {
            classification.content_type() == content_type
                && classification.valid() >= MIN_VALID
                && classification.valid_ratio().is_some_and(|ratio| ratio >= MIN_VALID_RATIO)
        })
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

    /// Adds an artifact and indexes the content types with supported evidence.
    pub fn insert(&mut self, artifact: IndexedArtifact) -> &IndexedArtifact {
        let artifact_index = self.artifacts.len();

        for content_type in ContentType::all_types() {
            if artifact.matches(content_type).next().is_some() {
                self.by_content_type.entry(content_type).or_default().push(artifact_index);
            }
        }

        self.artifacts.push(artifact);
        &self.artifacts[artifact_index]
    }

    /// Returns artifacts with evidence supporting the sought content type.
    pub fn search(&self, content_type: ContentType) -> impl Iterator<Item = &IndexedArtifact> + '_ {
        self.by_content_type
            .get(&content_type)
            .into_iter()
            .flatten()
            .map(|&index| &self.artifacts[index])
    }

    /// Iterates over index artifacts
    pub fn artifacts(&self) -> impl Iterator<Item = &IndexedArtifact> + '_ {
        self.artifacts.iter()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_panics_doc)]

    use datasets_core::StopReason;

    use super::*;

    /// Builds a complete inspection holding the given evidence.
    fn inspection(classifications: Vec<Classification>) -> Inspection {
        Inspection { classifications, records: 3, bytes: 0, stop: StopReason::EndOfInput }
    }

    #[test]
    fn indexes_artifacts_by_validated_content() {
        let mut index = DatasetIndex::new();

        index.insert(IndexedArtifact::new(
            "example-dataset",
            "https://example.org/datasets/example",
            "data.csv",
            inspection(vec![
                Classification::new(ContentType::Smiles, "smiles", 3, 2),
                Classification::new(ContentType::MolecularFormula, "formula", 3, 2),
            ]),
        ));

        index.insert(IndexedArtifact::new(
            "invalid-dataset",
            "https://example.org/datasets/invalid",
            "data.csv",
            inspection(vec![
                Classification::new(ContentType::Smiles, "smiles", 3, 0),
                Classification::new(ContentType::MolecularFormula, "formula", 3, 0),
            ]),
        ));

        let smiles_matches: Vec<_> = index.search(ContentType::Smiles).collect();

        assert_eq!(smiles_matches.len(), 1, "only the valid dataset should match");
        assert_eq!(
            index.search(ContentType::MolecularFormula).count(),
            1,
            "only the valid dataset should match"
        );

        assert_eq!(smiles_matches[0].dataset_id(), "example-dataset", "wrong dataset matched");
        assert_eq!(smiles_matches[0].artifact_ref(), "data.csv", "wrong artifact matched");

        assert_eq!(
            smiles_matches[0].classifications().len(),
            2,
            "search results should preserve all classification evidence"
        );

        assert_eq!(
            index.artifacts().count(),
            2,
            "index should retain both matches and non matches"
        );
    }

    #[test]
    fn ignores_locations_with_only_incidental_matches() {
        let mut index = DatasetIndex::new();

        index.insert(IndexedArtifact::new(
            "text-dataset",
            "https://example.org/datasets/text",
            "data.csv",
            inspection(vec![Classification::new(ContentType::Smiles, "comment", 1000, 7)]),
        ));

        assert_eq!(
            index.search(ContentType::Smiles).count(),
            0,
            "a few accidental parses in a text column should not be a match"
        );
    }

    #[test]
    fn returns_no_matches_for_unindexed_content() {
        let index = DatasetIndex::new();

        assert_eq!(index.search(ContentType::Smiles).count(), 0, "empty index has no matches");
    }
}
