use std::{collections::BTreeMap, io::Read};

use serde_json::Value;

use super::{Classification, CoreError, labeler::Labeler, visitor::Visitor};

/// Visits JSON dataset content.
pub struct JsonVisitor;


impl Visitor for JsonVisitor {
    /// Inspects JSON data using the provided scientific labelers.
    ///
    /// Parameters:
    /// - `reader`: Input containing JSON data.
    /// - `labelers`: Scientific labelers used to evaluate string values.
    ///
    /// Errors:
    /// - Returns [`CoreError`] when the JSON input cannot be parsed.
    fn visit<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
    ) -> Result<Vec<Classification>, CoreError> {
        let value: Value = serde_json::from_reader(reader)?;

        let mut evidence: Vec<BTreeMap<String, (usize, usize)>> =
            (0..labelers.len()).map(|_| BTreeMap::new()).collect();

        visit_value(&value, "$", labelers, &mut evidence);

        let mut classifications = Vec::new();

        for (labeler, locations) in labelers.iter().zip(evidence) {
            classifications.extend(
                locations.into_iter().map(|(location, (inspected, valid))| {
                    Classification::new(
                        labeler.content_type(),
                        location,
                        inspected,
                        valid,
                    )
                }),
            );
        }

        Ok(classifications)
    }
}



/// Recursively visits JSON values and accumulates evidence by location.
fn visit_value(
    value: &Value,
    location: &str,
    labelers: &[&dyn Labeler],
    evidence: &mut [BTreeMap<String, (usize, usize)>],
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
                let (inspected, valid) =
                    locations.entry(location.to_owned()).or_default();

                *inspected += 1;

                if labeler.is_valid(value) {
                    *valid += 1;
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
}
