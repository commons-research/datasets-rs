//! Core library for working with datasets

/// Module for classifying data
mod classification;
/// Classifies an artifact
mod classifier;
/// Module for handling dataset content
mod content;
/// Error handling
mod error;
/// JSON data traversal and inspection.
mod json;
/// Labeling dataset item requirements
mod labeler;
/// Built-in scientific content labelers.
mod labels;
/// Tabular data traversal and inspection.
mod tabular;
/// Defines visitor functionality
mod visitor;

pub use classification::Classification;
pub use classifier::{ArtifactFormat, classify_artifact};
pub use content::ContentType;
pub use error::CoreError;
