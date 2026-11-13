use std::io::Read;

use super::{
    error::CoreError,
    inspection::{InspectionLimits, Visit},
    labeler::Labeler,
};

/// Visits structured dataset content and produces classification evidence.
pub trait Visitor {
    /// Inspects structured dataset content.
    ///
    /// # Parameters
    /// - `reader`: Input containing the dataset content.
    /// - `labelers`: Scientific labelers used to evaluate the content.
    /// - `limits`: Bounds on how much of the input is inspected.
    /// 
    /// # Errors
    /// - Returns an error if nothing could be inspected. A failure after some
    ///   evidence was collected is reported through [`Visit::stop`] instead.
    fn visit<R: Read>(
        reader: R,
        labelers: &[&dyn Labeler],
        limits: InspectionLimits,
    ) -> Result<Visit, CoreError>;
}
