use std::fmt;

/// Scientific content type that may be detected within a dataset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContentType {
    /// [SMILES](https://en.wikipedia.org/wiki/Simplified_Molecular_Input_Line_Entry_System) representation
    Smiles,
    /// [Molecular Formula](https://en.wikipedia.org/wiki/Chemical_formula)
    MolecularFormula,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Smiles => f.write_str("smiles"),
            Self::MolecularFormula => f.write_str("molecular formula"),
        }
    }
}