use std::io::Read;

use super::{
    ContentType, CoreError,
    inspection::{BudgetReader, Inspection, InspectionLimits, StopReason},
    json::JsonVisitor,
    labeler::Labeler,
    tabular::TabularVisitor,
    visitor::Visitor,
};

/// Structure of an artifact being inspected.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ArtifactFormat {
    /// Tabular data such as CSV.
    Tabular,
    /// JSON data.
    Json,
}

/// Configures artifact classification
#[derive(Debug, Clone)]
pub struct Classifier {
    /// The associated format for this dataset
    format: ArtifactFormat,
    /// All [`ContentType`] that should be sought and classified in the dataset.
    content_types: Vec<ContentType>,
    /// BOunds on how much of the artifact is inspected.
    limits: InspectionLimits,
}

impl Classifier {
    /// Creates a classifier for an artifact format
    #[must_use]
    pub const fn new(format: ArtifactFormat) -> Self {
        Self { format, content_types: Vec::new(), limits: InspectionLimits::UNLIMITED }
    }

    /// Adds a scientific content type to evaluate
    #[must_use]
    pub fn with(mut self, content_type: ContentType) -> Self {
        if !self.content_types.contains(&content_type) {
            self.content_types.push(content_type);
        }
        self
    }

    /// Bounds how much of the artifact is inspected.
    #[must_use]
    pub const fn limits(mut self, limits: InspectionLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Classifies artifact data using the configured content types
    ///
    /// # Errors
    /// - Returns [`CoreError`] if the artifact cannot be inspected
    pub fn classify<R: Read>(&self, reader: R) -> Result<Inspection, CoreError> {
        classify_artifact(reader, self.format, &self.content_types, self.limits)
    }
}

/// Classifies an artifact for the requested scientific content types.
///
/// # Errors
/// - Returns [`CoreError`] if nothing in the artifact could be inspected.
pub fn classify_artifact<R: Read>(
    reader: R,
    format: ArtifactFormat,
    content_types: &[ContentType],
    limits: InspectionLimits,
) -> Result<Inspection, CoreError> {
    let labelers: Vec<&dyn Labeler> = content_types.iter().map(ContentType::labeler).collect();

    let mut reader = BudgetReader::new(reader, limits.max_bytes);

    let visit = match format {
        ArtifactFormat::Tabular => TabularVisitor::visit(&mut reader, &labelers, limits)?,
        ArtifactFormat::Json => JsonVisitor::visit(&mut reader, &labelers, limits)?,
    };

    let stop = if reader.limit_hit() && visit.stop != StopReason::RecordLimit {
        StopReason::ByteLimit
    } else {
        visit.stop
    };

    Ok(Inspection {
        classifications: visit.classifications,
        records: visit.records,
        bytes: reader.bytes_read(),
        stop,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    use super::{super::tabular::TabularDialect, *};
    #[test]
    fn classifies_tabular_artifact_for_requested_content_types() -> Result<(), CoreError> {
        let csv = "\
name,structure,formula
ethanol,CCO,C2H6O
water,O,H2O
broken,C(,garbage
";

        let classifications = classify_artifact(
            csv.as_bytes(),
            ArtifactFormat::Tabular,
            &[ContentType::Smiles, ContentType::MolecularFormula],
            InspectionLimits::UNLIMITED,
        )?
        .classifications;

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == ContentType::Smiles
                    && classification.location() == "structure"
                    && classification.inspected() == 3
                    && classification.valid() == 2
            }),
            "structure column should contain SMILES evidence"
        );

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == ContentType::MolecularFormula
                    && classification.location() == "formula"
                    && classification.inspected() == 3
                    && classification.valid() == 2
            }),
            "formula column should contain molecular formula evidence"
        );

        Ok(())
    }

    #[test]
    fn classifies_only_requested_content_types() -> Result<(), CoreError> {
        let csv = "\
name,structure,formula
ethanol,CCO,C2H6O
water,O,H2O
";

        let classifications = classify_artifact(
            csv.as_bytes(),
            ArtifactFormat::Tabular,
            &[ContentType::Smiles],
            InspectionLimits::UNLIMITED,
        )?
        .classifications;

        assert!(
            classifications
                .iter()
                .all(|classification| classification.content_type() == ContentType::Smiles),
            "classifier should only evaluate requested content types"
        );

        Ok(())
    }

    #[test]
    fn builder_classifies_multiple_content_types() -> Result<(), CoreError> {
        let csv = "\
name,structure,formula
ethanol,CCO,C2H6O
water,O,H2O
broken,C(,garbage
";

        let classifications = Classifier::new(ArtifactFormat::Tabular)
            .with(ContentType::Smiles)
            .with(ContentType::MolecularFormula)
            .classify(csv.as_bytes())?
            .classifications;

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == ContentType::Smiles
                    && classification.location() == "structure"
                    && classification.inspected() == 3
                    && classification.valid() == 2
            }),
            "builder should classify SMILES content"
        );

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == ContentType::MolecularFormula
                    && classification.location() == "formula"
                    && classification.inspected() == 3
                    && classification.valid() == 2
            }),
            "builder should classify molecular formula content"
        );

        Ok(())
    }

    #[test]
    fn builder_classifies_only_requested_content_type() -> Result<(), CoreError> {
        let json = r#"
        {
            "records": [
                {"smiles": "CCO", "formula": "C2H6O"},
                {"smiles": "C(", "formula": "garbage"}
            ]
        }
        "#;

        let classifications = Classifier::new(ArtifactFormat::Json)
            .with(ContentType::Smiles)
            .classify(json.as_bytes())?
            .classifications;

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == ContentType::Smiles
                    && classification.location() == "$.records[].smiles"
                    && classification.inspected() == 2
                    && classification.valid() == 1
            }),
            "builder should classify SMILES at the normalized JSON location"
        );

        assert!(
            classifications
                .iter()
                .all(|classification| classification.content_type() == ContentType::Smiles),
            "builder should not classify unrequested content types"
        );

        Ok(())
    }

    #[test]
    fn detects_dialect_with_incomplete_final_record() -> Result<(), CoreError> {
        let sample = b"name;smiles;formula\n\
ethanol;CCO;C2H6O\n\
water;O;H2O\n\
broken;\"unfinished quoted field\n\
still inside the quoted field";

        let dialect = TabularDialect::detect_sample(sample, true)?;

        assert_eq!(dialect.delimiter(), b';');
        assert!(dialect.has_headers());

        Ok(())
    }

    #[test]
    fn detects_dialect_with_truncated_utf8() -> Result<(), CoreError> {
        let sample = b"name;smiles;formula\n\
ethanol;CCO;C2H6O\n\
water;O;H2O\n\
broken;\xC3";

        let dialect = TabularDialect::detect_sample(sample, true)?;

        assert_eq!(dialect.delimiter(), b';');
        assert!(dialect.has_headers());

        Ok(())
    }

    #[test]
    fn detects_complete_sample_without_trailing_newline() -> Result<(), CoreError> {
        let sample = b"name;smiles\nethanol;CCO\nwater;O";

        let dialect = TabularDialect::detect_sample(sample, false)?;

        assert_eq!(dialect.delimiter(), b';');
        assert!(dialect.has_headers());

        Ok(())
    }

    #[test]
    fn reports_byte_limit_and_bytes_read() -> Result<(), CoreError> {
        let csv = format!("name,smiles\n{}", "compound,CCO\n".repeat(1000));

        let limits = InspectionLimits { max_records: None, max_bytes: Some(200) };

        let inspection = classify_artifact(
            csv.as_bytes(),
            ArtifactFormat::Tabular,
            &[ContentType::Smiles],
            limits,
        )?;

        assert_eq!(inspection.bytes, 200, "exactly the byte budget should be read");
        assert_eq!(inspection.stop, StopReason::ByteLimit, "the byte limit should end the visit");
        assert!(!inspection.is_complete(), "a truncated inspection is not complete");
        assert!(inspection.records < 1000, "only a prefix of the records should be inspected");

        Ok(())
    }

    #[test]
    fn byte_limit_keeps_evidence_from_truncated_json() -> Result<(), CoreError> {
        let json = format!("[{}{{\"smiles\": \"O\"}}]", "{\"smiles\": \"CCO\"},".repeat(1000));

        let limits = InspectionLimits { max_records: None, max_bytes: Some(210) };

        let inspection = classify_artifact(
            json.as_bytes(),
            ArtifactFormat::Json,
            &[ContentType::Smiles],
            limits,
        )?;

        assert_eq!(inspection.stop, StopReason::ByteLimit, "the byte limit should end the visit");
        assert!(inspection.records >= 9, "records before the cut should be inspected");

        Ok(())
    }
}
