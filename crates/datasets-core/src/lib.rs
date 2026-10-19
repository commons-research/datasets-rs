//! Core library for working with datasets

/// Module for classifying data
mod classification;
/// Module for handling dataset content
mod content;
/// Error handling
mod error;
/// Labeling dataset item requirements
mod labeler;
mod labels;

pub use content::ContentType;
pub use classification::Classification;
pub use error::CoreError;