use std::{fmt, str::FromStr};

use strum::VariantArray;

use super::CoreError;
use crate::{
    labeler::Labeler,
    labels::{MolecularFormulaLabeler, SmilesLabeler},
};

/// Scientific content type that may be detected within a dataset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, VariantArray)]
#[non_exhaustive]
pub enum ContentType {
    /// [SMILES](https://en.wikipedia.org/wiki/Simplified_Molecular_Input_Line_Entry_System) representation
    Smiles,
    /// [Molecular Formula](https://en.wikipedia.org/wiki/Chemical_formula)
    MolecularFormula,
}

impl ContentType {
    /// Returns all supported scientific content types
    #[must_use]
    pub fn all_types() -> Vec<Self> {
        Self::VARIANTS.to_vec()
    }
    /// Returns the `Labeler` implementation associated with the content type.
    #[must_use]
    pub fn labeler(&self) -> &'static dyn Labeler {
        match self {
            Self::MolecularFormula => &MolecularFormulaLabeler,
            Self::Smiles => &SmilesLabeler,
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Smiles => f.write_str("smiles"),
            Self::MolecularFormula => f.write_str("molecular formula"),
        }
    }
}

impl FromStr for ContentType {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "smiles" => Ok(Self::Smiles),

            "molecular-formula" | "molecular_formula" | "molecular formula" => {
                Ok(Self::MolecularFormula)
            }

            _ => Err(CoreError::ParseContentTypeError { value: value.to_owned() }),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_panics_doc)]

    use super::ContentType;

    #[test]
    fn parses_content_types() {
        assert!(matches!("smiles".parse::<ContentType>(), Ok(ContentType::Smiles)));

        assert!(matches!(
            "molecular_formula".parse::<ContentType>(),
            Ok(ContentType::MolecularFormula)
        ));

        assert!(
            "bullshit".parse::<ContentType>().is_err(),
            "unsupported content types should fail parsing"
        );
    }
}
