//!

use datasets_core::{ArtifactFormat, Classification};

use super::repository::Failure;

/// Results from one artifact.
#[derive(Debug, Clone)]
pub struct ArtifactResult {
    /// Path of the inspected artifact.
    pub path: String,
    /// Format used to inspect the artifact.
    pub format: ArtifactFormat,
    /// Artifact size in bytes.
    pub size_bytes: u64,
    /// Classification evidence.
    pub classifications: Vec<Classification>,
    /// Failure encountered while inspecting the artifact.
    pub failure: Option<Failure>,
}

/// File available for inspection within remote dataset
#[derive(Debug, Clone)]
pub struct SourceArtifact {
    /// Path identifying the file within source repo
    pub path: String,
    /// File size in bytes
    pub size_bytes: u64,
}
