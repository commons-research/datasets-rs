//! Policies and abstraction layer for working with a Repository.

use std::fmt;

use super::artifact::ArtifactResult;

/// Metadata describing repository discovered.
#[derive(Debug, Clone)]
pub struct RepositorySummary {
    /// Stable identifier assigned by source
    pub id: String,
    /// URL of the dataset location
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reason that crawling a specific repository has ceased
pub enum RepositoryStop {
    /// All artifacts have been perused
    Exhausted,
    /// No artifact formats were relevant to the required artifact format type
    NoEligibleFormats,
    /// The number of relevant artifact formats were low
    LowContentYield,
    /// Failed to resolve the repository artifact listings
    ListingFailed,
}

/// Cumulative assessment of a repository.
#[derive(Debug, Default)]
pub struct RepositoryAssessment {
    /// Number of artifacts encountered.
    pub artifacts_seen: u32,
    /// Number of artifacts with eligible formats.
    pub artifacts_eligible: u32,
    /// Number of successfully inspected artifacts.
    pub artifacts_inspected: u32,
    /// Number of inspected artifacts containing target content.
    pub artifacts_matched: u32,
}

impl RepositoryAssessment {
    /// Evaluates whether repository inspection should terminate.
    pub fn assess(&self, policy: &RepositoryPolicy) -> Option<RepositoryStop> {
        if self.artifacts_seen >= policy.min_artifacts_seen && self.artifacts_eligible == 0 {
            return Some(RepositoryStop::NoEligibleFormats);
        }

        if self.artifacts_inspected > 0
            && self.artifacts_inspected >= policy.min_artifacts_inspected
        {
            let ratio = f64::from(self.artifacts_matched) / f64::from(self.artifacts_inspected);

            if ratio < policy.min_match_ratio {
                return Some(RepositoryStop::LowContentYield);
            }
        }

        None
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ListArtifacts => write!(f, "list artifacts"),
            Self::OpenArtifact => write!(f, "open artifact"),
            Self::ClassifyArtifact => write!(f, "classify artifact"),
        }
    }
}

/// Results collected while inspecting one repository.
#[derive(Debug, Clone)]
pub struct RepositoryResult {
    /// Repository metadata.
    pub repository: RepositorySummary,
    /// Inspected artifacts.
    pub artifacts: Vec<ArtifactResult>,
    /// Repository-level failures.
    pub failures: Vec<Failure>,
    /// Reason inspection terminated.
    pub stop: RepositoryStop,
}

/// Enum for representing the stage of a Crawl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Artifacts are being listed from the data source
    ListArtifacts,
    /// An artifact is being opened and streamed
    OpenArtifact,
    /// Classification of an artifact is occurring
    ClassifyArtifact,
}

/// A repository- or artifact-level failure.
#[derive(Debug, Clone)]
pub struct Failure {
    /// Operation that failed.
    pub stage: Stage,
    /// Error description.
    pub message: String,
}

/// Policy governing early repository termination.
#[derive(Debug, Clone)]
pub struct RepositoryPolicy {
    /// Minimum artifacts encountered before format assessment.
    pub min_artifacts_seen: u32,
    /// Minimum successful inspections before content assessment.
    pub min_artifacts_inspected: u32,
    /// Minimum acceptable proportion of matching artifacts.
    pub min_match_ratio: f64,
}
