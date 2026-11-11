use std::io::Read;

use super::{classification::Classification, error::CoreError, labeler::Labeler};

/// Visits structured dataset content and produces classification evidence.
pub trait Visitor {
    /// Inspects structured dataset content.
    ///
    /// # Parameters
    /// - `reader`: Input containing the dataset content.
    /// - `labelers`: Scientific labelers used to evaluate the content.
    ///
    /// # Errors
    /// - Returns an error if input cannot be inspected.
    fn visit<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
    ) -> Result<Vec<Classification>, CoreError>;
}
