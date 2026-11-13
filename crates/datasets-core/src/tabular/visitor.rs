use std::io::{Cursor, Read};

use csv::{ByteRecord, ReaderBuilder};

use super::{
    super::{
        Classification, CoreError,
        inspection::{InspectionLimits, StopReason, Visit},
        labeler::Labeler,
        visitor::Visitor,
    },
    TabularDialect,
};

/// Number of bytes buffered from start for dialect detection.
const DIALECT_SAMPLE_BYTES: u64 = 64 * 1024;

/// Visits tabular dataset content
pub struct TabularVisitor;

impl TabularVisitor {
    /// Inspects tabular data using configured labelers and dialect.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the no record could be read.
    pub(crate) fn visit_with_dialect<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
        limits: InspectionLimits,
        dialect: TabularDialect,
    ) -> Result<Visit, CoreError> {
        let mut csv_reader = ReaderBuilder::new()
            .delimiter(dialect.delimiter())
            .has_headers(dialect.has_headers())
            .flexible(true)
            .from_reader(reader);

        let column_names: Vec<String> = if dialect.has_headers() {
            csv_reader
                .byte_headers()?
                .iter()
                .map(|name| String::from_utf8_lossy(name).trim().to_owned())
                .collect()
        } else {
            let column_count = csv_reader.byte_headers()?.len();
            (0..column_count).map(|index| format!("column_{index}")).collect()
        };

        let mut stats = vec![vec![ColumnStats::default(); column_names.len()]; labelers.len()];
        let mut record = ByteRecord::new();
        let mut records = 0usize;
        let mut stop = StopReason::EndOfInput;

        loop {
            if limits.max_records.is_some_and(|max_records| records >= max_records) {
                stop = StopReason::RecordLimit;
                break;
            }
            match csv_reader.read_byte_record(&mut record) {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) if records > 0 => {
                    stop = StopReason::Error(error.to_string());
                    break;
                }
                Err(error) => return Err(error.into()),
            }
            records += 1;

            for (column_index, cell) in record.iter().enumerate().take(column_names.len()) {
                let Ok(value) = std::str::from_utf8(cell) else {
                    continue;
                };
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                for (labeler_index, labeler) in labelers.iter().enumerate() {
                    let column_stats = &mut stats[labeler_index][column_index];
                    column_stats.inspected += 1;
                    if labeler.is_valid(value) {
                        column_stats.valid += 1;
                    }
                }
            }
        }

        let mut classifications = Vec::with_capacity(labelers.len() * column_names.len());
        for (labeler_index, labeler) in labelers.iter().enumerate() {
            for (column_index, name) in column_names.iter().enumerate() {
                let column_stats = &stats[labeler_index][column_index];

                classifications.push(Classification::new(
                    labeler.content_type(),
                    name.clone(),
                    column_stats.inspected,
                    column_stats.valid,
                ));
            }
        }

        Ok(Visit { classifications, records, stop })
    }
}

impl Visitor for TabularVisitor {
    /// Detects dialect, then inspects tabular data by column with the
    /// provided scientific labelers.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when no record could be read.
    fn visit<R: Read>(
        mut reader: R,
        labelers: &[&dyn Labeler],
        limits: InspectionLimits,
    ) -> Result<Visit, CoreError> {
        let mut sample = Vec::new();
        reader.by_ref().take(DIALECT_SAMPLE_BYTES).read_to_end(&mut sample)?;
        let may_be_truncated = u64::try_from(sample.len()) == Ok(DIALECT_SAMPLE_BYTES);
        let dialect = TabularDialect::detect_sample(&sample, may_be_truncated)?;

        Self::visit_with_dialect(Cursor::new(sample).chain(reader), labelers, limits, dialect)
    }
}

/// Statistics collected for a single column and labeler.
#[derive(Debug, Default, Clone, Copy)]
struct ColumnStats {
    /// Number of non-empty values inspected.
    inspected: usize,
    /// Number of values accepted by the labeler.
    valid: usize,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

    use std::cell::Cell;

    use super::*;
    use crate::{
        ContentType,
        ContentType::{MolecularFormula, Smiles},
        labels::{MolecularFormulaLabeler, SmilesLabeler},
    };

    /// Test labeler that counts the number of values inspected.
    struct CountingLabeler {
        /// Content type reported by the labeler.
        content_type: ContentType,
        /// Value accepted by the labeler.
        accepted_value: &'static str,
        /// Number of values passed to the labeler.
        calls: Cell<usize>,
    }

    impl Labeler for CountingLabeler {
        fn content_type(&self) -> ContentType {
            self.content_type
        }

        fn is_valid(&self, value: &str) -> bool {
            self.calls.set(self.calls.get() + 1);
            value == self.accepted_value
        }
    }

    #[test]
    fn visits_each_tabular_column_with_smiles_labeler() -> Result<(), CoreError> {
        let csv = "\
name,smiles,canonical_smiles
ethanol,CCO,CCO
acetic acid,CC(=O)O,CC(=O)O
broken,garbage,C(
";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = <TabularVisitor as Visitor>::visit(
            csv.as_bytes(),
            &labelers,
            InspectionLimits::default(),
        )?;

        let expected = vec![
            Classification::new(Smiles, "name", 3, 0),
            Classification::new(Smiles, "smiles", 3, 2),
            Classification::new(Smiles, "canonical_smiles", 3, 2),
        ];

        assert_eq!(visit.classifications, expected);

        Ok(())
    }

    #[test]
    fn visits_tabular_columns_with_formula_labeler() -> Result<(), CoreError> {
        let csv = "\
label,formula,alternate_formula
sample_one,C6H12O6,C6H12O6
sample_two,H2O,H2O
sample_three,garbage,C(
";

        let labelers: [&dyn Labeler; 1] = [&MolecularFormulaLabeler];

        let visit = <TabularVisitor as Visitor>::visit(
            csv.as_bytes(),
            &labelers,
            InspectionLimits::default(),
        )?;

        let expected = vec![
            Classification::new(MolecularFormula, "label", 3, 0),
            Classification::new(MolecularFormula, "formula", 3, 2),
            Classification::new(MolecularFormula, "alternate_formula", 3, 2),
        ];

        assert_eq!(visit.classifications, expected);

        Ok(())
    }

    #[test]
    fn visits_each_non_empty_cell_with_every_labeler() -> Result<(), CoreError> {
        let csv = "\
first,second
alpha,
beta,gamma
";

        let smiles_labeler =
            CountingLabeler { content_type: Smiles, accepted_value: "alpha", calls: Cell::new(0) };

        let formula_labeler = CountingLabeler {
            content_type: MolecularFormula,
            accepted_value: "gamma",
            calls: Cell::new(0),
        };

        let labelers: [&dyn Labeler; 2] = [&smiles_labeler, &formula_labeler];

        let visit = <TabularVisitor as Visitor>::visit(
            csv.as_bytes(),
            &labelers,
            InspectionLimits::default(),
        )?;

        let expected = vec![
            Classification::new(Smiles, "first", 2, 1),
            Classification::new(Smiles, "second", 1, 0),
            Classification::new(MolecularFormula, "first", 2, 0),
            Classification::new(MolecularFormula, "second", 1, 1),
        ];

        assert_eq!(visit.classifications, expected);

        assert_eq!(smiles_labeler.calls.get(), 3);
        assert_eq!(formula_labeler.calls.get(), 3);

        Ok(())
    }

    #[test]
    fn visits_multiple_labelers_with_detected_semicolon_dialect() -> Result<(), CoreError> {
        let data = "\
name;smiles;formula
ethanol;CCO;C2H6O
water;O;H2O
";

        let dialect = TabularDialect::detect_sample(data.as_bytes(), false)?;

        assert_eq!(dialect.delimiter(), b';');

        let labelers: [&dyn Labeler; 2] = [&SmilesLabeler, &MolecularFormulaLabeler];

        let visit = TabularVisitor::visit_with_dialect(
            data.as_bytes(),
            &labelers,
            InspectionLimits::default(),
            dialect,
        )?;

        assert_eq!(visit.classifications.len(), 6);

        assert!(
            visit.classifications.iter().any(|classification| {
                classification.content_type() == Smiles
                    && classification.location() == "smiles"
                    && classification.inspected() == 2
                    && classification.valid() == 2
            }),
            "SMILES classification should contain the expected evidence"
        );

        assert!(
            visit.classifications.iter().any(|classification| {
                classification.content_type() == MolecularFormula
                    && classification.location() == "formula"
                    && classification.inspected() == 2
                    && classification.valid() == 2
            }),
            "molecular formula classification should contain the expected evidence"
        );

        Ok(())
    }

    #[test]
    fn stops_inspection_at_record_limit() -> Result<(), CoreError> {
        let csv = "\
name,smiles
ethanol,CCO
water,O
broken,C(
";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let limits = InspectionLimits { max_records: Some(2), ..InspectionLimits::default() };

        let visit = TabularVisitor::visit(csv.as_bytes(), &labelers, limits)?;

        let expected = vec![
            Classification::new(Smiles, "name", 2, 0),
            Classification::new(Smiles, "smiles", 2, 2),
        ];

        assert_eq!(visit.classifications, expected);
        assert_eq!(visit.records, 2);
        assert!(matches!(visit.stop, StopReason::RecordLimit));

        Ok(())
    }

        #[test]
    fn stops_at_record_limit() -> Result<(), CoreError> {
        let csv = "name,smiles\na,CCO\nb,O\nc,CC\nd,N\n";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];
        let limits = InspectionLimits { max_records: Some(2), max_bytes: None };

        let visit = TabularVisitor::visit(csv.as_bytes(), &labelers, limits)?;

        assert_eq!(visit.records, 2, "only two records should be inspected");
        assert_eq!(visit.stop, StopReason::RecordLimit, "the record limit should end the visit");
        assert!(
            visit.classifications.contains(&Classification::new(Smiles, "smiles", 2, 2)),
            "evidence should cover only the inspected records"
        );

        Ok(())
    }

    #[test]
    fn tolerates_ragged_rows_and_invalid_utf8() -> Result<(), CoreError> {
        let csv = b"name,smiles\na,CCO,extra,cells\nb\n\xff\xfe,O\n";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = TabularVisitor::visit(&csv[..], &labelers, InspectionLimits::UNLIMITED)?;

        assert_eq!(visit.records, 3, "every row should be read");
        assert_eq!(visit.stop, StopReason::EndOfInput, "ragged rows should not end the visit");
        assert!(
            visit.classifications.contains(&Classification::new(Smiles, "smiles", 2, 2)),
            "valid cells in ragged rows should still be counted"
        );

        Ok(())
    }

    #[test]
    fn inspects_single_column_file_with_header() -> Result<(), CoreError> {
        let csv = "smiles\nCCO\nO\nCC\n";

        let labelers: [&dyn Labeler; 1] = [&SmilesLabeler];

        let visit = TabularVisitor::visit(csv.as_bytes(), &labelers, InspectionLimits::UNLIMITED)?;

        assert_eq!(
            visit.classifications,
            vec![Classification::new(Smiles, "smiles", 3, 3)],
            "a single-column file should keep its header as the location"
        );

        Ok(())
    }

}
