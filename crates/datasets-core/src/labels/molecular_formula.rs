use super::super::{labeler::Labeler, content::ContentType};
use std::str::FromStr;
use molecular_formulas::prelude::ChemicalFormula;


/// Structure for evaluating and labeling Molecular Formula values
pub struct MolecularFormulaLabeler;

impl Labeler for MolecularFormulaLabeler {
    fn content_type(&self) -> crate::ContentType {
        ContentType::MolecularFormula
    }

    fn is_valid(&self, value:&str) -> bool {
        ChemicalFormula::<u32, i32>::from_str(value).is_ok()
    }
}