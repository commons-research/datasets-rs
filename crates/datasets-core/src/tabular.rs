use std::io::Read;
use csv::Reader;

use super::{
    Classification,
    CoreError,
    labeler::Labeler,
};



/// Visits tabular dataset content
pub struct TabularVisitor;

impl TabularVisitor {
    /// Inspects tabular data using the provided scientific labeler.
    /// 
    /// # Errors
    /// - Returns [`CoreError`] when the tabular input cannot be parsed.
    pub fn visit<R: Read>(
        reader: R,
        labeler: &impl Labeler,
    ) -> Result<Vec<Classification>, CoreError>{
        let mut reader = Reader::from_reader(reader);
        let headers = reader.headers()?.clone();

        let mut inspected = vec![0; headers.len()];
        let mut valid = vec![0; headers.len()];

        for record in reader.records() {
            let record = record?;

            for (index, value) in record.iter().enumerate() {
                if value.trim().is_empty() {
                    continue;
                }
                inspected[index] += 1; 

                if labeler.is_valid(value) {
                    valid[index] +=1;
                }
            }
        }
        let classifications = headers.iter().enumerate().map(|(index, header)|{
            Classification::new(labeler.content_type(), header, inspected[index], valid[index])
        }).collect();
        Ok(classifications)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentType::Smiles, labels::SmilesLabeler};

    #[test]
    fn visits_each_tabular_column() -> Result<(), CoreError> {
        let csv = "\
name,smiles,canonical_smiles
ethanol,CCO,CCO
acetic acid,CC(=O)O,CC(=O)O
broken,garbage,C(
";

        let classifications =
            TabularVisitor::visit(csv.as_bytes(), &SmilesLabeler)?;

        let expected = vec![
            Classification::new(Smiles, "name", 3, 0),
            Classification::new(Smiles, "smiles", 3, 2),
            Classification::new(Smiles, "canonical_smiles", 3, 2),
        ];

        assert_eq!(
            classifications,
            expected,
            "tabular visitor should collect SMILES evidence for every column"
        );

        Ok(())
    }
}