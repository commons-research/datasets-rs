use std::io::Read;

use csv::ReaderBuilder;

use super::{super::{Classification, CoreError, labeler::Labeler, visitor::Visitor}, TabularDialect};

/// Visits tabular dataset content
pub struct TabularVisitor;

impl TabularVisitor {
    /// Inspects tabular data using a configured [`TabularDialect`].
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the tabular input cannot be parsed.
    pub(crate) fn visit_with_dialect<R: Read, L: Labeler>(
        reader: R,
        labeler: &L,
        dialect: TabularDialect
    ) -> Result<Vec<Classification>, CoreError> {
        let mut reader = ReaderBuilder::new().delimiter(dialect.delimiter()).has_headers(dialect.has_headers()).from_reader(reader);
        let column_names: Vec<String> = if dialect.has_headers() {
            reader.headers()?.iter().map(str::to_owned).collect()
        } else {
            let column_count = reader.headers()?.len();
            (0..column_count).map(|index| format!("column_{index}")).collect()
        };

        let mut inspected = vec![0; column_names.len()];
        let mut valid = vec![0; column_names.len()];
        for record in reader.records() {
            let record = record?;
            for (index, value) in record.iter().enumerate() {
                if value.trim().is_empty() {
                    continue;
                }

                inspected[index] += 1;

                if labeler.is_valid(value) {
                    valid[index] += 1;
                }
            }
        }

        Ok(column_names.into_iter().enumerate().map(|(index,name)| {
            Classification::new(labeler.content_type(), name, inspected[index], valid[index])
        }).collect())
    }
}

impl Visitor for TabularVisitor {
    /// Inspects tabular data using the provided scientific labeler.
    ///
    /// # Errors
    /// - Returns [`CoreError`] when the tabular input cannot be parsed.
    fn visit<R: Read, L: Labeler>(
        reader: R,
        labeler: &L,
    ) -> Result<Vec<Classification>, CoreError> {
        Self::visit_with_dialect(reader, labeler, TabularDialect::csv())
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
    fn visits_each_tabular_column() -> Result<(), CoreError> {
        let csv = "\
name,smiles,canonical_smiles
ethanol,CCO,CCO
acetic acid,CC(=O)O,CC(=O)O
broken,garbage,C(
";

        let classifications = TabularVisitor::visit(csv.as_bytes(), &SmilesLabeler)?;

        let expected = vec![
            Classification::new(Smiles, "name", 3, 0),
            Classification::new(Smiles, "smiles", 3, 2),
            Classification::new(Smiles, "canonical_smiles", 3, 2),
        ];

        assert_eq!(
            classifications, expected,
            "tabular visitor should collect SMILES evidence for every column"
        );

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

        let classifications = TabularVisitor::visit(csv.as_bytes(), &MolecularFormulaLabeler)?;

        let expected = vec![
            Classification::new(MolecularFormula, "label", 3, 0),
            Classification::new(MolecularFormula, "formula", 3, 2),
            Classification::new(MolecularFormula, "alternate_formula", 3, 2),
        ];

        assert_eq!(
            classifications, expected,
            "tabular visitor should collect molecular formula evidence for every column"
        );

        Ok(())
    }
}
