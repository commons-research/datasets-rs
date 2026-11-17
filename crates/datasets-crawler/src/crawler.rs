use std::io::Read;

use datasets_core::{ArtifactFormat, ContentType, InspectionLimits, classify_artifact};

use super::{CrawlerError, DatasetIndex, IndexedArtifact};

/// An artifact discovered by a dataset source for inspection
#[derive(Debug, Clone)]
pub struct CandidateArtifact {
    /// Stable identifier of the parent dataset
    dataset_id: String,
    /// URL of the parent dataset
    dataset_url: String,
    /// Reference identifying this artifact within the dataset
    artifact_ref: String,
    /// Structural format of the artifact
    format: ArtifactFormat,
}

impl CandidateArtifact {
    /// Create a candidate artifact
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        dataset_url: impl Into<String>,
        artifact_ref: impl Into<String>,
        format: ArtifactFormat,
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            dataset_url: dataset_url.into(),
            artifact_ref: artifact_ref.into(),
            format,
        }
    }
}

/// Classifies discovered artifacts and contributes them to index
#[derive(Debug)]
pub struct Crawler {
    /// Scientific content types sought
    content_types: Vec<ContentType>,
    /// Bounds on how much of each artifact to inspect.
    limits: InspectionLimits,
    /// In-memory index populated with classified artifacts
    index: DatasetIndex,
}

impl Crawler {
    /// Creates a crawler set to search for the specified content types
    #[must_use]
    pub fn new(content_types: &[ContentType], limits: InspectionLimits) -> Self {
        let mut requested = Vec::new();

        for content_type in content_types {
            if !requested.contains(content_type) {
                requested.push(*content_type);
            }
        }
        Self { content_types: requested, limits, index: DatasetIndex::new() }
    }

    /// Classifies an artifact as it is read and indexes the result.
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] if the artifact cannot be classified
    pub fn process<R: Read>(
        &mut self,
        artifact: CandidateArtifact,
        reader: R,
    ) -> Result<&IndexedArtifact, CrawlerError> {
        let inspection =
            classify_artifact(reader, artifact.format, &self.content_types, self.limits)?;

        Ok(self.index.insert(IndexedArtifact::new(
            artifact.dataset_id,
            artifact.dataset_url,
            artifact.artifact_ref,
            inspection,
        )))
    }

    /// Returns the crawler's current in-memory index
    #[must_use]
    pub const fn index(&self) -> &DatasetIndex {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn processes_artifact_into_searchable_index() -> Result<(), CrawlerError> {
        let csv = "\
name,structure,formula
ethanol,CCO,C2H6O
water,O,H2O
broken,C(,garbage
";

        let artifact = CandidateArtifact::new(
            "example-dataset",
            "https://example.org/datasets/example",
            "data.csv",
            ArtifactFormat::Tabular,
        );

        let mut crawler = Crawler::new(
            &[ContentType::Smiles, ContentType::MolecularFormula],
            InspectionLimits::UNLIMITED,
        );

        let indexed = crawler.process(artifact, csv.as_bytes())?;
        assert_eq!(indexed.inspection().records, 3, "all three rows should be inspected");

        let smiles_matches: Vec<_> = crawler.index().search(ContentType::Smiles).collect();

        assert_eq!(
            smiles_matches.len(),
            1,
            "crawler should index the artifact as containing SMILES"
        );

        assert_eq!(
            crawler.index().search(ContentType::MolecularFormula).count(),
            1,
            "crawler should index the artifact as containing molecular formulae"
        );

        assert_eq!(smiles_matches[0].dataset_id(), "example-dataset", "wrong dataset indexed");
        Ok(())
    }
}
