use std::{collections::BTreeMap, io::Read};

use serde_json::Value;

use super::{Classification, CoreError, labeler::Labeler, visitor::Visitor};

/// Visits JSON dataset content.
pub struct JsonVisitor;

impl Visitor for JsonVisitor {
    /// Inspects JSON data using the provided scientific labelers.
    ///
    /// # Parameters
    /// - `reader`: Input containing JSON data.
    /// - `labelers`: Scientific labelers used to evaluate string values.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the JSON input cannot be parsed.
    fn visit<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
    ) -> Result<Vec<Classification>, CoreError> {
        let value: Value = serde_json::from_reader(reader)?;

        let mut evidence: Vec<BTreeMap<String, EvidenceStats>> =
            (0..labelers.len()).map(|_| BTreeMap::new()).collect();

        visit_value(&value, "$", labelers, &mut evidence);

        let mut classifications = Vec::new();

        for (labeler, locations) in labelers.iter().zip(evidence) {
            classifications.extend(locations.into_iter().map(|(location, stats)| {
                Classification::new(labeler.content_type(), location, stats.inspected, stats.valid)
            }));
        }

        Ok(classifications)
    }
}

/// Classification evidence collected for a JSON location.
#[derive(Debug, Default)]
struct EvidenceStats {
    /// Number of non-empty values inspected.
    inspected: usize,
    /// Number of values accepted by the labeler.
    valid: usize,
}

/// Recursively visits JSON values and accumulates evidence by location.
fn visit_value(
    value: &Value,
    location: &str,
    labelers: &[&dyn Labeler],
    evidence: &mut [BTreeMap<String, EvidenceStats>],
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let child_location = format!("{location}.{key}");
                visit_value(value, &child_location, labelers, evidence);
            }
        }

        Value::Array(values) => {
            let child_location = format!("{location}[]");

            for value in values {
                visit_value(value, &child_location, labelers, evidence);
            }
        }

        Value::String(value) => {
            if value.trim().is_empty() {
                return;
            }

            for (labeler, locations) in labelers.iter().zip(evidence.iter_mut()) {
                let stats = locations.entry(location.to_owned()).or_default();
                stats.inspected += 1;
                if labeler.is_valid(value) {
                    stats.valid += 1;
                }
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    use super::*;
    use crate::{
        ContentType::{MolecularFormula, Smiles},
        labels::{MolecularFormulaLabeler, SmilesLabeler},
    };

    #[test]
    fn visits_json_string_locations() -> Result<(), CoreError> {
        let json = r#"
        {
            "compounds": [
                {
                    "name": "ethanol",
                    "smiles": "CCO"
                },
                {
                    "name": "acetic acid",
                    "smiles": "CC(=O)O"
                },
                {
                    "name": "broken",
                    "smiles": "garbage"
                }
            ],
            "canonical_smiles": [
                "CCO",
                "CC(=O)O",
                "C("
            ]
        }
        "#;

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let classifications = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        let expected = vec![
            Classification::new(Smiles, "$.canonical_smiles[]", 3, 2),
            Classification::new(Smiles, "$.compounds[].name", 3, 0),
            Classification::new(Smiles, "$.compounds[].smiles", 3, 2),
        ];

        assert_eq!(
            classifications, expected,
            "JSON visitor should accumulate evidence by normalized JSON location"
        );

        Ok(())
    }

    #[test]
    fn visits_json_with_multiple_labelers() -> Result<(), CoreError> {
        let json = r#"
    {
        "records": [
            {"smiles": "CCO", "formula": "C2H6O"},
            {"smiles": "C(", "formula": "garbage"}
        ]
    }
    "#;

        let labelers: [&dyn Labeler; 2] = [&SmilesLabeler, &MolecularFormulaLabeler];

        let classifications = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == Smiles
                    && classification.location() == "$.records[].smiles"
                    && classification.inspected() == 2
                    && classification.valid() == 1
            }),
            "SMILES labeler should identify the valid SMILES value"
        );

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == MolecularFormula
                    && classification.location() == "$.records[].formula"
                    && classification.inspected() == 2
                    && classification.valid() == 1
            }),
            "formula labeler should identify the valid molecular formula"
        );

        assert_eq!(classifications.len(), 4);

        Ok(())
    }
}
