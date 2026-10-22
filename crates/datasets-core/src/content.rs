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

impl ContentType {
    /// Returns search terms for assisting in discovering datasets 
    /// containing the specified scientific content type.
    #[must_use]
    pub const fn terms(self) -> &'static [&'static str] {
        match self {
            Self::Smiles => &[
                "smiles",
                "canonical smiles",
                "canonical_smiles",
                "isomeric smiles",
                "isomeric_smiles",
                "chemical structures",
                "molecular structures",
                "chemistry"
            ],
            Self::MolecularFormula => &[
                "molecular formula",
                "molecular_formula",
                "chemical formula",
                "chemical_formula",
                "molecular formulas",
                "molecular formulae",
                "chemistry"
            ]
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


#[cfg(test)]
mod tests {

#[cfg(test)]
mod tests {
    use super::super::ContentType;

    #[test]
    fn first_term_is_the_canonical_search_term() {
        assert_eq!(
            ContentType::Smiles.terms().first().copied(),
            Some("smiles"),
            "SMILES should use its display term as its primary search term"
        );

        assert_eq!(
            ContentType::MolecularFormula.terms().first().copied(),
            Some("molecular formula"),
            "molecular formula should use its display term as its primary search term"
        );
    }

    #[test]
    fn all_content_types_have_nonempty_search_terms() {
        for content_type in [
            ContentType::Smiles,
            ContentType::MolecularFormula,
        ] {
            let terms = content_type.terms();

            assert!(
                !terms.is_empty(),
                "every content type should have at least one search term"
            );

            assert!(
                terms.iter().all(|term| !term.trim().is_empty()),
                "search terms should not be empty"
            );
        }
    }
}

}