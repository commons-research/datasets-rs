use std::{collections::BTreeMap, io::Read};

use serde_json::Value;

use super::{Classification, CoreError, labeler::Labeler, visitor::Visitor};

/// Visits JSON dataset content.
pub struct JsonVisitor;

impl Visitor for JsonVisitor {
    /// Inspects JSON data using the provided scientific labeler.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the JSON input cannot be parsed
    fn visit<R: Read, L: Labeler>(
        reader: R,
        labeler: &L,
    ) -> Result<Vec<Classification>, CoreError> {
        let value: Value = serde_json::from_reader(reader)?;

        let mut evidence = BTreeMap::new();

        visit_value(&value, "$", labeler, &mut evidence);
        Ok(evidence
            .into_iter()
            .map(|(location, (inspected, valid))| {
                Classification::new(labeler.content_type(), location, inspected, valid)
            })
            .collect())
    }
}

/// Recursively visits JSON values and accumulates evidence by location
fn visit_value<L: Labeler>(
    value: &Value,
    location: &str,
    labeler: &L,
    evidence: &mut BTreeMap<String, (usize, usize)>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let child_location = format!("{location}.{key}");

                visit_value(value, &child_location, labeler, evidence);
            }
        }

        Value::Array(values) => {
            let child_location = format!("{location}[]");

            for value in values {
                visit_value(value, &child_location, labeler, evidence);
            }
        }

        Value::String(value) => {
            if value.trim().is_empty() {
                return;
            }

            let (inspected, valid) = evidence.entry(location.to_owned()).or_default();

            *inspected += 1;

            if labeler.is_valid(value) {
                *valid += 1;
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    use super::*;
    use crate::{ContentType::Smiles, labels::SmilesLabeler};

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

        let classifications = JsonVisitor::visit(json.as_bytes(), &SmilesLabeler)?;

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
}
