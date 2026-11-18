use std::time::Duration;

use datasets_core::{ArtifactFormat, Classification, ContentType, classify_artifact};
use futures::{TryStreamExt, pin_mut};
use tokio_util::sync::CancellationToken;

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

/// Outcome of inspecting a dataset / artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectionOutcome {
    /// Inspection completed without interruption
    Completed,
    /// Inspection was interrupted.
    Interrupted,
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
    /// # Errors:
    /// - Returns [`CrawlerError`] when failing to interact with repository.
    async fn run_repositories(&mut self, run_id: i64) -> Result<CrawlStatus, CrawlerError> {
        let mut repositories_crawled = 0usize;

        // No search terms. Discovery is unrestricted.
        let mut repositories = self.source.discover_datasets()?;
        pin_mut!(repositories);
        while let Some(repository) = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                return Ok(CrawlStatus::Interrupted);
            }

            repository = repositories.try_next() => repository?,
        } {
            if self.database.repository_is_crawled(&repository)? {
                continue;
            }

            let Some(result) = self.inspect_repository(&repository).await? else {
                return Ok(CrawlStatus::Interrupted);
            };

            // Persist only after the repository has finished.
            self.database.save_repository(run_id, &result)?;

            repositories_crawled += 1;

            if self.config.max_repositories.is_some_and(|limit| repositories_crawled >= limit) {
                break;
            }
        }

        Ok(CrawlStatus::Completed)
    }

    /// Inspects the supported artifacts in one dataset.
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
                        return Ok(Some(RepositoryResult {
                            repository: repository.clone(),
                            artifacts: Vec::new(),
                            failures: vec![Failure {
                                artifact_path: None,
                                stage: "list_artifacts",
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

        for artifact in artifacts {
            if self.cancellation.is_cancelled() {
                return Ok(None);
            }

            let Some(format) = select_artifact_format(&artifact.path, self.config) else {
                continue;
            };

            match self.inspect_artifact(repository, &artifact, format).await? {
                Some(artifact_result) => result.artifacts.push(artifact_result),
                None => return Ok(None),
            }
        }

        Ok(Some(result))
    }

    /// Inspects one supported artifact.
    ///
    /// # Errors:
    /// - Returns [`CrawlerError`] when interacting with artifact fails.
    async fn inspect_artifact(
        &self,
        repository: &RepositorySummary,
        artifact: &SourceArtifact,
        format: ArtifactFormat,
    ) -> Result<Option<ArtifactResult>, CrawlerError> {
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
                            records_inspected: None,
                            classifications: Vec::new(),
                            failure: Some(Failure {
                                artifact_path: Some(artifact.path.clone()),
                                stage: "open_artifact",
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

        match result {
            Ok(visit) => {
                Ok(Some(ArtifactResult {
                    path: artifact.path.clone(),
                    format,
                    size_bytes: artifact.size_bytes,
                    records_inspected: Some(visit.records),
                    classifications: visit.classifications,
                    failure: None,
                }))
            }

            Err(_) if self.cancellation.is_cancelled() => Ok(None),

            Err(error) => {
                Ok(Some(ArtifactResult {
                    path: artifact.path.clone(),
                    format,
                    size_bytes: artifact.size_bytes,
                    records_inspected: None,
                    classifications: Vec::new(),
                    failure: Some(Failure {
                        artifact_path: Some(artifact.path.clone()),
                        stage: "classify_artifact",
                        message: error.to_string(),
                    }),
                }))
            }
        }
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
pub(crate) struct RepositoryResult {
    pub repository: RepositorySummary,
    pub artifacts: Vec<ArtifactResult>,
    pub failures: Vec<Failure>,
}

/// Results from one artifact.
pub(crate) struct ArtifactResult {
    pub path: String,
    pub format: ArtifactFormat,
    pub size_bytes: u64,
    pub records_inspected: Option<usize>,
    pub classifications: Vec<Classification>,
    pub failure: Option<Failure>,
}

/// A repository- or artifact-level failure.
pub(crate) struct Failure {
    pub artifact_path: Option<String>,
    pub stage: &'static str,
    pub message: String,
}
