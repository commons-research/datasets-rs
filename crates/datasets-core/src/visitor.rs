use std::io::Read;

use super::{Classification, error::CoreError, labeler::Labeler};

/// Reason the artifact's inspection ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Entire artifact was successfully read.
    EndOfInput,
    /// Reading or parsing failed after some evidence was collected.
    Error(String),
}

/// Evidence collected while visiting one artifact.
#[derive(Debug)]
pub struct Visit {
    /// Evidence collected per content type and location.
    pub classifications: Vec<Classification>,
    /// Number of records inspected.
    pub records: usize,
    /// Reason the visitor stopped.
    pub stop: StopReason,
}

/// Visits structured dataset content and produces classification evidence.
pub trait Visitor {
    /// Inspects structured dataset content.
    ///
    /// # Parameters
    /// - `reader`: Input containing the dataset content.
    /// - `labelers`: Scientific labelers used to evaluate the content.
    ///
    /// # Errors
    /// - Returns an error if nothing could be inspected. A failure after some
    ///   evidence was collected is reported through [`Visit::stop`] instead.
    fn visit<R: Read>(reader: R, labelers: &[&dyn Labeler]) -> Result<Visit, CoreError>;
}
