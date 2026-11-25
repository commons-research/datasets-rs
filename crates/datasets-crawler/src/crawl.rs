use std::{
    fmt,
    time::{Duration, Instant},
};

use datasets_core::{ArtifactFormat, Classification, ContentType, StopReason, classify_artifact};
use futures::{TryStreamExt, pin_mut};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use self::Stage::{ClassifyArtifact, ListArtifacts, OpenArtifact};
use crate::{
    CrawlerError,
    database::Database,
    source::{DatasetSource, FileFormat, RepositorySummary, SourceArtifact},
};

/// Configuration for a repository-independent crawl.
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Scientific content types to seek.
    pub content_types: Vec<ContentType>,
    /// Optional file-format restrictions.
    pub formats: Option<Vec<FileFormat>>,
    /// Optional maximum number of repositories to successfully inspect.
    pub max_repositories: Option<usize>,
    /// Maximum time a streaming artifact may stall.
    pub stall_timeout: Duration,
}

/// Current state of crawl
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlStatus {
    /// Dataset query has been completed
    Completed,
    /// The crawl has not completed and has been cancelled
    Interrupted,
}

/// Discovers, inspects, classifies datasets from the specified source.
///
/// # Errors
/// - Returns an error if initial searches cannot be completed.
pub async fn crawl<S: DatasetSource>(
    source: &S,
    database: &mut Database,
    config: &CrawlConfig,
    cancellation: &CancellationToken,
) -> Result<CrawlStatus, CrawlerError> {
    let mut session = CrawlSession { source, database, config, cancellation };

    session.run().await
}

/// Holds the state and dependencies for the crawl.
struct CrawlSession<'a, S: DatasetSource> {
    /// Remote source of discoverable and retrievable datasets.
    source: &'a S,
    /// The database where crawl results are stored.
    database: &'a mut Database,
    /// Configuration for crawling data source.
    config: &'a CrawlConfig,
    /// Token for tracking cancellation request of crawl.
    cancellation: &'a CancellationToken,
}

impl<S: DatasetSource> CrawlSession<'_, S> {
    /// Runs the crawl over the repositories found from source.
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] if search fails
    async fn run(&mut self) -> Result<CrawlStatus, CrawlerError> {
        let run_id = self.database.start_run()?;

        let result = self.run_repositories(run_id).await;

        self.database.finish_run(run_id)?;

        result
    }

    /// Inspects the current repository found.
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] when failing to interact with repository.
    async fn run_repositories(&mut self, run_id: i64) -> Result<CrawlStatus, CrawlerError> {
        if self.config.max_repositories == Some(0) {
            return Ok(CrawlStatus::Completed);
        }

        let mut repositories_crawled = 0usize;

        let repositories = self.source.discover_datasets()?;
        pin_mut!(repositories);
        while let Some(repository) = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                return Ok(CrawlStatus::Interrupted);
            }

            repository = repositories.try_next() => repository?,
        } {
            if self.database.repository_is_crawled(self.source.provider(), &repository.id)? {
                continue;
            }

            info!(repository = %repository.id, "inspecting repository");

            let Some(result) = self.inspect_repository(&repository).await? else {
                return Ok(CrawlStatus::Interrupted);
            };

            // Persist only after the repository has finished.
            self.database.save_repository(run_id, self.source.provider(), &result)?;

            info!(
                repository = %repository.id,
                artifacts = result.artifacts.len(),
                failures = result.failures.len(),
                "repository saved"
            );

            repositories_crawled += 1;

            if self.config.max_repositories.is_some_and(|limit| repositories_crawled >= limit) {
                break;
            }
        }

        Ok(CrawlStatus::Completed)
    }

    /// Inspects the supported artifacts in one dataset.
    ///
    /// # Errors
    /// - Returns a [`CrawlerError`] when failing to inspect repository.
    async fn inspect_repository(
        &self,
        repository: &RepositorySummary,
    ) -> Result<Option<RepositoryResult>, CrawlerError> {
        let artifacts = tokio::select! {
                biased;

                () = self.cancellation.cancelled() => {
                    return Ok(None);
                }

                result = self.source.list_artifacts(&repository.id) => {
                    match result {
                        Ok(artifacts) => artifacts,
                        Err(error) => {
                                warn!(
            repository = %repository.id,
            error = %error,
            "repository tree listing failed"
        );
                            return Ok(Some(RepositoryResult {
                                repository: repository.clone(),
                                artifacts: Vec::new(),
                                failures: vec![Failure {
                                    stage: ListArtifacts,
                                    message: error.to_string(),
                                }],
                            }));
                        }
                    }
                }
            };

        let mut result = RepositoryResult {
            repository: repository.clone(),
            artifacts: Vec::new(),
            failures: Vec::new(),
        };

        let supported: Vec<_> = artifacts
            .into_iter()
            .filter_map(|artifact| {
                select_artifact_format(&artifact.path, self.config).map(|format| (artifact, format))
            })
            .collect();

        let stride = supported.len().div_ceil(100).max(1);

        for (artifact, format) in supported.into_iter().step_by(stride) {
            if self.cancellation.is_cancelled() {
                return Ok(None);
            }

            match self.inspect_artifact(repository, &artifact, format).await? {
                Some(artifact_result) => result.artifacts.push(artifact_result),
                None => return Ok(None),
            }
        }

        Ok(Some(result))
    }

    /// Inspects one supported artifact.
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] when interacting with artifact fails.
    async fn inspect_artifact(
        &self,
        repository: &RepositorySummary,
        artifact: &SourceArtifact,
        format: ArtifactFormat,
    ) -> Result<Option<ArtifactResult>, CrawlerError> {
        let started = Instant::now();

        info!(
            repository = %repository.id,
            artifact = %artifact.path,
            size_bytes = artifact.size_bytes,
            "opening artifact"
        );

        let stream = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                return Ok(None);
            }

            result = self.source.open_artifact(&repository.id, artifact) => {
                match result {
                    Ok(stream) => stream,
                    Err(error) => {

                        return Ok(Some(ArtifactResult {
                            path: artifact.path.clone(),
                            format,
                            size_bytes: artifact.size_bytes,
                            classifications: Vec::new(),
                            failure: Some(Failure {
                                stage: OpenArtifact,
                                message: error.to_string(),
                            }),
                        }));
                    }
                }
            }
        };

        let reader = super::reader::BlockingReader::new(
            stream,
            self.cancellation,
            self.config.stall_timeout,
        );

        let result = tokio::task::block_in_place(|| {
            classify_artifact(reader, format, &self.config.content_types)
        });

        if self.cancellation.is_cancelled() {
            return Ok(None);
        }

        info!(
            repository = %repository.id,
            artifact = %artifact.path,
            elapsed_secs = started.elapsed().as_secs(),
            "artifact inspection finished"
        );

        let make_result = |classifications, failure| {
            ArtifactResult {
                path: artifact.path.clone(),
                format,
                size_bytes: artifact.size_bytes,
                classifications,
                failure,
            }
        };

        let failed = |stage, message| make_result(Vec::new(), Some(Failure { stage, message }));

        let result = match result {
            Ok(visit) => {
                let failure = match visit.stop {
                    StopReason::EndOfInput => None,
                    StopReason::Error(error) => {
                        Some(Failure { stage: ClassifyArtifact, message: error.to_string() })
                    }
                };

                make_result(visit.classifications, failure)
            }
            Err(_) if self.cancellation.is_cancelled() => return Ok(None),
            Err(error) => failed(ClassifyArtifact, error.to_string()),
        };

        Ok(Some(result))
    }
}

/// Selects an artifact format if the file is supported and permitted.
fn select_artifact_format(artifact_path: &str, config: &CrawlConfig) -> Option<ArtifactFormat> {
    let file_format = FileFormat::from_path(artifact_path)?;

    if config.formats.as_ref().is_some_and(|formats| !formats.contains(&file_format)) {
        return None;
    }

    Some(file_format.artifact_format())
}

/// Results collected while inspecting one repository.
pub struct RepositoryResult {
    /// Repository metadata.
    pub repository: RepositorySummary,
    /// Inspected artifacts.
    pub artifacts: Vec<ArtifactResult>,
    /// Repository-level failures.
    pub failures: Vec<Failure>,
}

/// Results from one artifact.
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

/// A repository- or artifact-level failure.
pub struct Failure {
    /// Operation that failed.
    pub stage: Stage,
    /// Error description.
    pub message: String,
}

/// Enum for representing the stage of a Crawl.
pub enum Stage {
    ListArtifacts,
    OpenArtifact,
    ClassifyArtifact,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stage::ListArtifacts => write!(f, "list artifacts"),
            Stage::OpenArtifact => write!(f, "open artifact"),
            Stage::ClassifyArtifact => write!(f, "classify artifact"),
        }
    }
}
