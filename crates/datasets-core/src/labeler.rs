//! Outlines requirements for implementing labeling for dataset fields

use super::ContentType;

/// Trait for labeling and returning data types 
pub trait Labeler {
    /// Scientific content detected by this labler
    fn content_type(&self) -> ContentType;
    /// Returns whether value is valid instance of content type
    fn is_valid(&self, value:&str) -> bool;
}