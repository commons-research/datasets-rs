use super::ContentType;

/// Classifies the dataset into results based on sought [`ContentType`]
pub struct Classification {
    /// [`ContentType`] being sought
    content_type: ContentType,
    /// Location of `ContentType` in the dataset
    location: String,
    /// Number of entries inspected total
    inspected: usize,
    /// Number of entries that contain a valid instance of the sought `ContentType`
    valid: usize,
}

impl Classification {
    /// Creates a new classification
    #[must_use]
    pub fn new(content_type: ContentType, location: impl Into<String>, inspected: usize, valid: usize) -> Self {
        Self { content_type, location: location.into(), inspected, valid }
    }
    /// Returns the `ContentType`
    #[must_use]
    pub const fn content_type(&self) -> ContentType {
        self.content_type
    }
    /// Returns the defined content location
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }
    /// Returns the total inspected entries
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }
    /// Returns the total number of valid entries
    #[must_use]
    pub const fn valid(&self) -> usize {
        self.valid
    }
    /// Returns the ratio of valid to total entries
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub const fn valid_ratio(&self) -> Option<f64> {
        if self.inspected == 0 {
            None
        } else {
            Some(self.valid as f64 / self.inspected as f64)
        }
    }
}