use std::io::Read;


use super::{inspection::{Visit, InspectionLimits}, error::CoreError, labeler::Labeler};

/// Visits structured dataset content and produces classification evidence.
pub trait Visitor {
    /// Inspects structured dataset content.
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
