use std::io::Read;

use super::{classification::Classification, error::CoreError, labeler::Labeler};

/// Visits structured dataset content and produces classification evidence.
pub trait Visitor {
    /// Inspects structured dataset content
    ///
    /// # Errors
    /// - Returns [`CoreError`] if input cannot be inspected
    fn visit<R: Read, L: Labeler>(reader: R, labeler: &L)
    -> Result<Vec<Classification>, CoreError>;
}
