use smiles_rs::Smiles;

use super::super::{labeler::Labeler, content::ContentType};


/// Structure for evaluating and labeling SMILES values
pub struct SmilesLabeler;

impl Labeler for SmilesLabeler {
    fn content_type(&self) -> ContentType {
        ContentType::Smiles
    }

    fn is_valid(&self, value: &str) -> bool {
        value.parse::<Smiles>().is_ok()
    }
}