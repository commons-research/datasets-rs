use std::io::Read;

use csv::ReaderBuilder;

use super::{
    super::{Classification, CoreError, labeler::Labeler, visitor::Visitor},
    TabularDialect,
};

/// Visits tabular dataset content
pub struct TabularVisitor;

impl TabularVisitor {
    /// Inspects tabular data using configured labelers and dialect.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the tabular input cannot be parsed.
    pub(crate) fn visit_with_dialect<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
        dialect: TabularDialect,
    ) -> Result<Vec<Classification>, CoreError> {
        let mut csv_reader = ReaderBuilder::new()
            .delimiter(dialect.delimiter())
            .has_headers(dialect.has_headers())
            .from_reader(reader);
        let column_names: Vec<String> = if dialect.has_headers() {
            csv_reader.headers()?.iter().map(str::to_owned).collect()
        } else {
            let column_count = csv_reader.headers()?.len();
            (0..column_count).map(|index| format!("column_{index}")).collect()
        };

        let mut stats = vec![vec![ColumnStats::default(); column_names.len()]; labelers.len()];
        for record in csv_reader.records() {
            let record = record?;
            for (column_index, value) in record.iter().enumerate() {
                if value.trim().is_empty() {
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
        Ok(classifications)
    }
}

impl Visitor for TabularVisitor {
    /// Inspects tabular data using the provided scientific labelers.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the tabular input cannot be parsed.
    fn visit<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
    ) -> Result<Vec<Classification>, CoreError> {
        Self::visit_with_dialect(reader, labelers, TabularDialect::csv())
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

        let classifications = <TabularVisitor as Visitor>::visit(csv.as_bytes(), &labelers)?;

        let expected = vec![
            Classification::new(Smiles, "name", 3, 0),
            Classification::new(Smiles, "smiles", 3, 2),
            Classification::new(Smiles, "canonical_smiles", 3, 2),
        ];

        assert_eq!(classifications, expected);

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

        let classifications = <TabularVisitor as Visitor>::visit(csv.as_bytes(), &labelers)?;

        let expected = vec![
            Classification::new(MolecularFormula, "label", 3, 0),
            Classification::new(MolecularFormula, "formula", 3, 2),
            Classification::new(MolecularFormula, "alternate_formula", 3, 2),
        ];

        assert_eq!(classifications, expected);

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

        let classifications = <TabularVisitor as Visitor>::visit(csv.as_bytes(), &labelers)?;

        let expected = vec![
            Classification::new(Smiles, "first", 2, 1),
            Classification::new(Smiles, "second", 1, 0),
            Classification::new(MolecularFormula, "first", 2, 0),
            Classification::new(MolecularFormula, "second", 1, 1),
        ];

        assert_eq!(classifications, expected);

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

        let dialect = TabularDialect::detect(data.as_bytes())?;

        assert_eq!(dialect.delimiter(), b';');

        let labelers: [&dyn Labeler; 2] = [&SmilesLabeler, &MolecularFormulaLabeler];

        let classifications =
            TabularVisitor::visit_with_dialect(data.as_bytes(), &labelers, dialect)?;

        assert_eq!(classifications.len(), 6);

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == Smiles
                    && classification.location() == "smiles"
                    && classification.inspected() == 2
                    && classification.valid() == 2
            }),
            "SMILES classification should contain the expected evidence"
        );

        assert!(
            classifications.iter().any(|classification| {
                classification.content_type() == MolecularFormula
                    && classification.location() == "formula"
                    && classification.inspected() == 2
                    && classification.valid() == 2
            }),
            "molecular formula classification should contain the expected evidence"
        );

        Ok(())
    }
}
