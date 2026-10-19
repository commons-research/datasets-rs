//! Web crawler dataset evaluation application runtime

#![allow(clippy::print_stdout)]
use datasets_core::{Classification, ContentType};
fn main() {
    let classification = Classification::new(ContentType::Smiles, "canonical_smiles", 100, 98);
    println!("{classification:?}");
}
