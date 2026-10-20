use std::{
    collections::BTreeMap,
    io::Read
};

use serde_json::Value;

use super::{
    Classification,
    CoreError,
    labeler::Labeler,
    visitor::Visitor
};

/// Visits JSON dataset content.
pub struct JsonVisitor;

impl Visitor for JsonVisitor {
    /// Inspects JSON data using the provided scientific labeler.
    /// 
    /// # Errors
    /// - Returns [`CoreError`] when the JSON input cannot be parsed
    fn visit<R: Read, L: Labeler>(
        reader: R,
        labeler: &L
    ) -> Result<Vec<Classification>, CoreError> {
        let value: Value = serde_json::from_reader(reader)?;

        let mut evidence = BTreeMap::new();

        visit_value(
            &value,
            "$",
            labeler,
            &mut evidence
        );
        Ok(evidence.into_iter().map(|(location, (inspected, valid))| {
            Classification::new(labeler.content_type(), location, inspected, valid)
        }).collect())
    }
}

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

                visit_value(
                    value,
                    &child_location,
                    labeler,
                    evidence,
                );
            }
        }

        Value::Array(values) => {
            let child_location = format!("{location}[]");

            for value in values {
                visit_value(
                    value,
                    &child_location,
                    labeler,
                    evidence,
                );
            }
        }

        Value::String(value) => {
            if value.trim().is_empty() {
                return;
            }

            let (inspected, valid) =
                evidence.entry(location.to_owned()).or_default();

            *inspected += 1;

            if labeler.is_valid(value) {
                *valid += 1;
            }
        }

        _ => {}
    }
}