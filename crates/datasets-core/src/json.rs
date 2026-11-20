use std::{
    collections::BTreeMap,
    fmt,
    io::{Cursor, Read},
};

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess};

use super::{
    Classification, CoreError,
    labeler::Labeler,
    visitor::{StopReason, Visit, Visitor},
};

/// Visits JSON dataset content.
pub struct JsonVisitor;

impl Visitor for JsonVisitor {
    /// Inspects JSON or JSON Lines data using the provided
    /// scientific labelers without loading into memory.
    ///
    /// # Errors
    /// - Returns [`CoreError`] if the input fails before any evidence is
    ///   collected.
    fn visit<R: Read>(mut reader: R, labelers: &[&dyn Labeler]) -> Result<Visit, CoreError> {
        let mut sample = Vec::new();
        reader.by_ref().take(64 * 1024).read_to_end(&mut sample)?;

        // Reuse the existing parser for both inspections.
        let inspect = |reader: &mut dyn Read| -> Result<Visit, CoreError> {
            let mut state = WalkState {
                labelers,
                evidence: BTreeMap::new(),
                path: String::from("$"),
                records: 0,
            };

            let mut deserializer = serde_json::Deserializer::from_reader(reader);
            let mut stop = StopReason::EndOfInput;

            while deserializer.end().is_err() {
                let records_before = state.records;
                let walker = Walker { state: &mut state, depth: 0 };

                if let Err(error) = walker.deserialize(&mut deserializer) {
                    if state.evidence.is_empty() {
                        return Err(error.into());
                    }
                    stop = StopReason::Error(error.to_string());
                    break;
                }

                if state.records == records_before {
                    state.records += 1;
                }
                state.path.truncate(1);
            }

            Ok(Visit { classifications: state.classifications(), records: state.records, stop })
        };

        // First pass: inspect the buffered data.
        if let Ok(preview) = inspect(&mut sample.as_slice()) {
            if preview.classifications.iter().all(|c| c.valid() < 5) {
                return Ok(preview);
            }
        }

        // Second pass: replay the buffer, then continue the original stream.
        inspect(&mut Cursor::new(sample).chain(reader))
    }
}

/// Classification evidence collected for a JSON location.
#[derive(Debug, Clone, Copy, Default)]
struct EvidenceStats {
    /// Number of non-empty values inspected.
    inspected: usize,
    /// Number of values accepted by the labeler.
    valid: usize,
}

/// Mutable state shared by every step of the walk.
struct WalkState<'l> {
    /// Labelers applied to every string value.
    labelers: &'l [&'l dyn Labeler],
    /// Evidence per location of the value being visited.
    evidence: BTreeMap<String, Vec<EvidenceStats>>,
    /// Normalized location of value being visited.
    path: String,
    /// Records completed so far.
    records: usize,
}

impl WalkState<'_> {
    /// Evaluates one string value at the current location.
    fn observe(&mut self, value: &str) {
        let value = value.trim();
        if value.is_empty() {
            return;
        }
        if !self.evidence.contains_key(self.path.as_str()) {
            self.evidence
                .insert(self.path.clone(), vec![EvidenceStats::default(); self.labelers.len()]);
        }

        let Some(stats) = self.evidence.get_mut(self.path.as_str()) else {
            return;
        };

        for (labeler, stats) in self.labelers.iter().zip(stats) {
            stats.inspected += 1;
            if labeler.is_valid(value) {
                stats.valid += 1;
            }
        }
    }
    /// Converts the collected evidence into classifications.
    fn classifications(&self) -> Vec<Classification> {
        let mut classifications = Vec::with_capacity(self.labelers.len() * self.evidence.len());

        for (labeler_index, labeler) in self.labelers.iter().enumerate() {
            for (location, stats) in &self.evidence {
                if let Some(stats) = stats.get(labeler_index) {
                    classifications.push(Classification::new(
                        labeler.content_type(),
                        location.clone(),
                        stats.inspected,
                        stats.valid,
                    ));
                }
            }
        }
        classifications
    }
}

/// Walks one JSON value as it is parsed, without building it in memory.
struct Walker<'s, 'l> {
    /// State shared across the walk.
    state: &'s mut WalkState<'l>,
    /// Nesting depth of the value being visited; the top-level value is 0.
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Walker<'_, '_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Walker<'_, '_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_str<E>(self, value: &str) -> Result<(), E> {
        self.state.observe(value);
        Ok(())
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let parent_len = self.state.path.len();
        self.state.path.push_str("[]");

        loop {
            let element = Walker { state: &mut *self.state, depth: self.depth + 1 };
            if seq.next_element_seed(element)?.is_none() {
                break;
            }

            if self.depth == 0 {
                self.state.records += 1;
            }
        }

        self.state.path.truncate(parent_len);
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let parent_len = self.state.path.len();

        while let Some(key) = map.next_key::<String>()? {
            self.state.path.push('.');
            self.state.path.push_str(&key);

            map.next_value_seed(Walker { state: &mut *self.state, depth: self.depth + 1 })?;

            self.state.path.truncate(parent_len);
        }
        Ok(())
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
                {"name": "ethanol", "smiles": "CCO"},
                {"name": "acetic acid", "smiles": "CC(=O)O"},
                {"name": "broken", "smiles": "garbage"}
            ],
             "canonical_smiles": ["CCO", "CC(=O)O", "C("]
        }
        "#;

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        let expected = vec![
            Classification::new(Smiles, "$.canonical_smiles[]", 3, 2),
            Classification::new(Smiles, "$.compounds[].name", 3, 0),
            Classification::new(Smiles, "$.compounds[].smiles", 3, 2),
        ];

        assert_eq!(visit.classifications, expected);
        assert_eq!(visit.records, 1);
        assert!(matches!(visit.stop, StopReason::EndOfInput));

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

        let visit = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        assert!(
            visit.classifications.iter().any(|classification| {
                classification.content_type() == Smiles
                    && classification.location() == "$.records[].smiles"
                    && classification.inspected() == 2
                    && classification.valid() == 1
            }),
            "SMILES labeler should identify the valid SMILES value"
        );

        assert!(
            visit.classifications.iter().any(|classification| {
                classification.content_type() == MolecularFormula
                    && classification.location() == "$.records[].formula"
                    && classification.inspected() == 2
                    && classification.valid() == 1
            }),
            "formula labeler should identify the valid molecular formula"
        );

        assert_eq!(visit.classifications.len(), 4);
        assert_eq!(visit.records, 1);
        assert!(matches!(visit.stop, StopReason::EndOfInput));

        Ok(())
    }

    #[test]
    fn stops_at_record_limit() -> Result<(), CoreError> {
        let json = r#"
        [
            {"smiles": "CCO"},
            {"smiles": "O"},
            {"smiles": "C("}
        ]
        "#;

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        let expected = vec![Classification::new(Smiles, "$[].smiles", 2, 2)];

        assert_eq!(visit.classifications, expected);
        assert_eq!(visit.records, 2);

        Ok(())
    }

    #[test]
    fn returns_partial_evidence_on_parse_error() -> Result<(), CoreError> {
        let json = r#"
        [
            {"smiles": "CCO"},
            {"smiles": }
        ]
        "#;

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        let expected = vec![Classification::new(Smiles, "$[].smiles", 1, 1)];

        assert_eq!(visit.classifications, expected);
        assert_eq!(visit.records, 1);
        assert!(matches!(visit.stop, StopReason::Error(_)));

        Ok(())
    }

    #[test]
    fn visits_json_lines_one_record_per_line() -> Result<(), CoreError> {
        let jsonl = "{\"smiles\": \"CCO\"}\n{\"smiles\": \"O\"}\n{\"smiles\": \"C(\"}\n";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = JsonVisitor::visit(jsonl.as_bytes(), &labelers)?;

        assert_eq!(
            visit.classifications,
            vec![Classification::new(Smiles, "$.smiles", 3, 2)],
            "every line should contribute to the same location"
        );
        assert_eq!(visit.records, 3, "each line is one record");

        Ok(())
    }

    #[test]
    fn keeps_evidence_when_json_is_cut_short() -> Result<(), CoreError> {
        let json = r#"[{"smiles": "CCO"}, {"smiles": "O"}, {"smi"#;

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = JsonVisitor::visit(json.as_bytes(), &labelers)?;

        assert_eq!(
            visit.classifications,
            vec![Classification::new(Smiles, "$[].smiles", 2, 2)],
            "evidence collected before the failure should be kept"
        );
        assert!(
            matches!(visit.stop, StopReason::Error(_)),
            "the failure should be reported as the stop reason"
        );

        Ok(())
    }
}
