use super::{
    Classification, CoreError,
    json::JsonVisitor,
    labels::{MolecularFormulaLabeler, SmilesLabeler},
    tabular::TabularVisitor,
    visitor::Visitor,
};
use crate::{ContentType, labeler::Labeler};

/// Structure of an artifact being inspected.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ArtifactFormat {
    /// Tabular data such as CSV.
    Tabular,
    /// JSON data.
    Json,
}

/// Classifies an artifact for the requested scientific content types.
///
/// # Errors
///
/// Returns [`CoreError`] if the artifact cannot be inspected.
pub fn classify_artifact(
    data: &[u8],
    format: ArtifactFormat,
    content_types: &[ContentType],
) -> Result<Vec<Classification>, CoreError> {
    let mut classifications = Vec::new();

    for content_type in content_types {
        let results = match content_type {
            ContentType::Smiles => classify_with_labeler(data, format, &SmilesLabeler)?,

            ContentType::MolecularFormula => {
                classify_with_labeler(data, format, &MolecularFormulaLabeler)?
            }
        };

        classifications.extend(results);
    }

    Ok(classifications)
}

/// Classifies an artifact using a single scientific labeler.
///
/// # Errors
/// - Returns [`CoreError`] if the artifact cannot be inspected.
fn classify_with_labeler<L: Labeler>(
    data: &[u8],
    format: ArtifactFormat,
    labeler: &L,
) -> Result<Vec<Classification>, CoreError> {
    match format {
        ArtifactFormat::Tabular => TabularVisitor::visit(data, labeler),
        ArtifactFormat::Json => JsonVisitor::visit(data, labeler),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    use super::*;
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
        )?;

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

        let classifications =
            classify_artifact(csv.as_bytes(), ArtifactFormat::Tabular, &[ContentType::Smiles])?;

        assert!(
            classifications
                .iter()
                .all(|classification| classification.content_type() == ContentType::Smiles),
            "classifier should only evaluate requested content types"
        );

        Ok(())
    }
}
