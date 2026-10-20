use std::io::Read;

use super::{labeler::Labeler, classification::Classification, error::CoreError};


pub trait Visitor {
    /// Inspects structured dataset content
    /// 
    /// # Error
    /// - Returns [`CoreError`] if input cannot be inspected
    fn visit<R: Read, L: Labeler>(
        reader: R,
        labeler: &L
    ) -> Result<Vec<Classification>, CoreError>;
}