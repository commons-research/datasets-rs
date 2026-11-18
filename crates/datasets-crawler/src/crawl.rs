use std::time::Duration;

use datasets_core::{ArtifactFormat, ContentType, InspectionLimits};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::{
    CrawlerError, DatasetSource, DatasetSummary, FileFormat, SourceArtifact, reader::BlockingReader,
};
use crate::source::RepositorySummary;

/// Configuration for a repository-independent crawl.
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Scientific content types to seek.
    pub content_types: Vec<ContentType>,
    /// Optional file-format restrictions, if none provided will crawl supported
    /// file types.
    pub formats: Option<Vec<FileFormat>>,
    /// Optional search terms, if none provided will search all available
    /// repositories.
    pub search_terms: Option<Vec<String>>,
    /// Maximum number of repositories to inspect, if not provided will crawl
    /// all available repositories.
    pub max_repositories: Option<usize>,
    /// Longest an artifact stream may go without delivering bytes before
    /// abandoning.
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

/// Discovers, inspects, classifies datasets
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
    /// Inspects the supported artifacts in one dataset.
    async fn inspect_repository(
        &self,
        repository: &RepositorySummary,
    ) -> Result<Option<RepositoryResult>, CrawlerError> {
        let Some(artifacts) = self.list_artifacts(dataset).await else {
            return self.inspection_outcome();
        };

        let repository_artifact_count = artifacts.len();

        info!(
            dataset_id = %dataset.id,
            artifact_count = repository_artifact_count,
            "retrieved repository artifact list"
        );

        for artifact in &artifacts {
            if self.cancellation.is_cancelled() {
                break;
            }

            if self.inspect_artifact(dataset, artifact).await == InspectionOutcome::Interrupted {
                break;
            }
        }

        self.inspection_outcome()
    }

    /// Returns current inspection outcome
    fn inspection_outcome(&self) -> InspectionOutcome {
        if self.cancellation.is_cancelled() {
            InspectionOutcome::Interrupted
        } else {
            InspectionOutcome::Completed
        }
    }

    /// Retrieves files available in dataset repository.
    async fn list_artifacts(&mut self, dataset: &DatasetSummary) -> Option<Vec<SourceArtifact>> {
        let result = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                info!(
                    dataset_id = %dataset.id,
                    "interrupted while listing dataset artifacts"
                );

                return None;
            }
            result = self.source.list_artifacts(
                &dataset.id,
            ) => result,
        };

        match result {
            Ok(artifacts) => Some(artifacts),
            Err(error) => {
                warn!(
                    dataset_id = %dataset.id,
                    error = %error,
                    "failed to list dataset artifacts; skipping dataset"
                );

                self.warnings.push(format!("Could not list artifacts for {}: {error}", dataset.id));
                None
            }
        }
    }

    /// Streams one artifact through classification and indexes the result.
    async fn inspect_artifact(
        &mut self,
        dataset: &DatasetSummary,
        artifact: &SourceArtifact,
    ) -> InspectionOutcome {
        let Some(format) = select_artifact_format(&dataset.id, &artifact.path, self.config) else {
            return InspectionOutcome::Completed;
        };

        let source = self.source;
        let cancellation = self.cancellation;

        debug!(
            dataset_id = %dataset.id,
            artifact_path = %artifact.path,
            artifact_size_bytes = artifact.size_bytes,
            "opening artifact stream"
        );

        let opened = tokio::select! {
            biased;

            () = cancellation.cancelled() => {
                info!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    "interrupted while opening artifact"
                );
                return InspectionOutcome::Interrupted;
            }
            opened = source.open_artifact(&dataset.id, artifact) => opened,
        };

        let stream = match opened {
            Ok(stream) => stream,
            Err(error) => {
                warn!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to open artifact; skipping artifact"
                );
                return InspectionOutcome::Completed;
            }
        };

        let reader = BlockingReader::new(stream, cancellation, self.config.stall_timeout);
        let candidate = CandidateArtifact::new(
            dataset.id.clone(),
            dataset.url.clone(),
            artifact.path.clone(),
            format,
        );

        let crawler = &mut self.crawler;
        let result = tokio::task::block_in_place(|| crawler.process(candidate, reader));

        match result {
            Ok(indexed) => {
                let inspection = indexed.inspection();
                info!(
                   dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                   artifact_size_bytes = artifact.size_bytes,
                   bytes_read = inspection.bytes,
                   records = inspection.records,
                   stop = ?inspection.stop,
                    "artifact classified and indexed"
                );
            }

            Err(_) if cancellation.is_cancelled() => {}
            Err(error) => {
                warn!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to classify artifact"
                );
            }
        }
        self.inspection_outcome()
    }

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

        for term in search_terms {
            let mut repositories = self.source.discover_datasets(term)?;

            while let Some(repository) = repositories.try_next().await? {
                if self.cancellation.is_cancelled() {
                    return Ok(CrawlStatus::Interrupted);
                }

                // Check SQLite before investigating.
                if self.database.repository_is_crawled(&repository)? {
                    continue;
                }

                let Some(result) = self.inspect_repository(&repository).await? else {
                    return Ok(CrawlStatus::Interrupted);
                };

                // Persist the completed repository.
                self.database.save_repository(run_id, &result)?;

                repositories_crawled += 1;

                if self.config.max_repositories.is_some_and(|limit| repositories_crawled >= limit) {
                    return Ok(CrawlStatus::Completed);
                }
            }
        }

        Ok(CrawlStatus::Completed)
    }
}

/// Selects an artifact for inspection if its format is supported and permitted.
fn select_artifact_format(artifact_path: &str, config: &CrawlConfig) -> Option<ArtifactFormat> {
    let Some(file_format) = FileFormat::from_path(artifact_path) else {
        return None;
    };

    if config.formats.as_ref().is_some_and(|formats| !formats.contains(&file_format)) {
        return None;
    }

    Some(file_format.artifact_format())
}
