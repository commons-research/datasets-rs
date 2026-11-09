use std::{collections::HashSet, path::Path};

use datasets_core::{ArtifactFormat, ContentType};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::{CandidateArtifact, Crawler, CrawlerError, DatasetSource, DatasetSummary, FileFormat};
use crate::SourceArtifact;

/// Configuration for a repository-independent crawl.
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Scientific content types to seek.
    pub content_types: Vec<ContentType>,
    /// Optional file-format restrictions, if none provided will crawl supported file types.
    pub formats: Option<Vec<FileFormat>>,
    /// Optional search terms, if none provided will search all available repositories.
    pub search_terms: Option<Vec<String>>,
    /// Maximum number of repositories to inspect, if not provided will crawl all available repositories.
    pub max_repositories: Option<usize>,
}

/// Current state of crawl
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlStatus {
    /// Dataset query has been completed
    Completed,
    /// The crawl has not completed and has been cancelled
    Interrupted,
}

/// Results and non-fatal issues produced by a crawl.
#[derive(Debug)]
pub struct CrawlReport {
    /// Crawler containing the accumulated classification index.
    crawler: Crawler,
    /// Warnings for datasets or files that could not be processed.
    warnings: Vec<String>,
    /// Indicates completion status of the crawl.
    status: CrawlStatus,
}

impl CrawlReport {
    /// Returns the crawler and its accumulated index.
    #[must_use]
    pub const fn crawler(&self) -> &Crawler {
        &self.crawler
    }

    /// Returns warnings generated during the crawl.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    /// Returns status of the crawl.
    #[must_use]
    pub const fn status(&self) -> CrawlStatus {
        self.status
    }
}


/// Statistics collected while inspecting one dataset.
#[derive(Debug, Default)]
struct DatasetStats {
    /// Number of artifacts successfully classified and indexed.
    indexed_artifacts: usize,
    /// Number of artifacts skipped because their format was unsupported.
    skipped_unsupported_format: usize,
    /// Number of artifact downloads that failed.
    download_failures: usize,
    /// Number of artifact classification attempts that failed.
    classification_failures: usize,
}


/// Tracks datasets collected during discovery
#[derive(Debug, Default)]
struct DiscoveryState {
    /// Unique datasets discovered so far
    datasets: Vec<DatasetSummary>,
    /// IDs of datasets, to avoid duplication
    seen: HashSet<String>,
}

/// Statistics collected while processing a response
#[derive(Debug, Default)]
struct SearchResultStats {
    /// Number of datasets rejected by filters
    filtered_out: usize,
    /// Number of datasets already discovered in another search
    duplicate_results: usize,
    /// Dataset limit reached
    reached_limit: bool,
    /// Process interrupted
    interrupted: bool,
}

/// A downloaded artifact ready for classification.
struct DownloadedArtifact<'a> {
    /// Dataset containing the artifact.
    dataset: &'a DatasetSummary,
    /// Repository-relative path to the artifact.
    path: &'a str,
    /// Format selected for inspection.
    format: ArtifactFormat,
    /// Downloaded artifact contents.
    data: Vec<u8>,
}

/// Discovers, inspects, classifies datasets
///
/// # Errors
/// - Returns an error if initial searches cannot be completed.
pub async fn crawl<S: DatasetSource>(
    source: &S,
    config: &CrawlConfig,
    cancellation: &CancellationToken,
) -> Result<CrawlReport, CrawlerError> {

    info!(
        content_types = ?config.content_types,
        formats = ?config.formats,
        search_terms = ?config.search_terms,
        max_repositories = config.max_repositories,
        "starting crawl"
    );


    let mut session = CrawlSession {
        source,
        config,
        cancellation,
        crawler: Crawler::new(&config.content_types),
        warnings: Vec::new(),
    };

    let (datasets, mut status) = session.discover_datasets().await?;

    let discovered_dataset_count = datasets.len();

    info!(dataset_count = discovered_dataset_count, "starting artifact inspection");

    if status == CrawlStatus::Completed {
        for dataset in &datasets {
            if cancellation.is_cancelled() {
                status = CrawlStatus::Interrupted;
                break;
            }
            if session.inspect_dataset(dataset).await == InspectionOutcome::Interrupted {
                status = CrawlStatus::Interrupted;
                break;
            }
        }
    }

    // Catch cancellation arriving just as the final operation completes.
    if cancellation.is_cancelled() {
        status = CrawlStatus::Interrupted;
    }

    let CrawlSession { crawler, warnings, .. } = session;

    info!(
        discovered_dataset_count,
        indexed_artifact_count = crawler.index().artifacts().count(),
        warning_count = warnings.len(),
        crawl_status = ?status,
        "crawl finished"
    );

    Ok(CrawlReport { crawler, warnings, status })
}

/// Holds the state and dependencies for the crawl.
struct CrawlSession<'a, S: DatasetSource> {
    /// Remote source of discoverable and retrievable datasets.
    source: &'a S,
    /// Configuration for crawling data source.
    config: &'a CrawlConfig,
    /// Token for tracking cancellation request of crawl.
    cancellation: &'a CancellationToken,
    /// Crawler state that maintains index of classifications.
    crawler: Crawler,
    /// Accumulated warnings encountered while crawling.
    warnings: Vec<String>,
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
    async fn inspect_dataset(&mut self, dataset: &DatasetSummary) -> InspectionOutcome {
        info!(
            dataset_id = %dataset.id,
            "inspecting discovered dataset"
        );

        let Some(artifacts) = self.list_artifacts(dataset).await else {
            return self.inspection_outcome();
        };

        let repository_artifact_count = artifacts.len();

        info!(
            dataset_id = %dataset.id,
            artifact_count = repository_artifact_count,
            "retrieved repository artifact list"
        );

        let mut stats = DatasetStats::default();

        for artifact in &artifacts {
            if self.cancellation.is_cancelled() {
                break;
            }

            if self.inspect_artifact(dataset, artifact, &mut stats).await
                == InspectionOutcome::Interrupted
            {
                break;
            }
        }

        log_dataset_stats(&dataset.id, repository_artifact_count, &stats);

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

    /// Inspects and classifies individual artifacts.
    async fn inspect_artifact(
        &mut self,
        dataset: &DatasetSummary,
        artifact: &SourceArtifact,
        stats: &mut DatasetStats,
    ) -> InspectionOutcome {
        let Some(format) = select_artifact_format(
            &dataset.id,
            &artifact.path,
            artifact.size_bytes,
            self.config,
            stats,
        ) else {
            return InspectionOutcome::Completed;
        };
        let Some(data) = self.download_artifact(dataset, artifact, stats).await else {
            return self.inspection_outcome();
        };

        self.classify_and_index(
            DownloadedArtifact { dataset, path: &artifact.path, format, data },
            stats,
        );
        InspectionOutcome::Completed
    }

    /// Downloads an artifact from the remote dataset
    async fn download_artifact(
        &mut self,
        dataset: &DatasetSummary,
        artifact: &SourceArtifact,
        stats: &mut DatasetStats,
    ) -> Option<Vec<u8>> {
        debug!(
            dataset_id = %dataset.id,
            artifact_path = %artifact.path,
            artifact_size_bytes = artifact.size_bytes,
            "downloading supported artifact"
        );

        let result = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                info!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    "interrupted while downloading artifact"
                );
                return None;
            }
            result = self.source.download_artifact(
                &dataset.id,
                artifact
            ) => result,
        };

        match result {
            Ok(data) => {
                Some(data)
            }
            Err(error) => {
                stats.download_failures += 1;
                warn!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to download artifact; skipping artifact"
                );
                self.warnings.push(format!(
                    "Could not download {} from {}: {error}",
                    artifact.path, dataset.id,
                ));
                None
            }
        }
    }

    /// Classifies a downloaded artifact and adds its results to the index.
    fn classify_and_index(&mut self, artifact: DownloadedArtifact<'_>, stats: &mut DatasetStats) {
        let candidate = CandidateArtifact::new(
            artifact.dataset.id.clone(),
            artifact.dataset.url.clone(),
            artifact.path.to_owned(),
            artifact.format,
            artifact.data,
        );

        match self.crawler.process(candidate) {
            Ok(()) => {
                stats.indexed_artifacts += 1;

                debug!(
                    dataset_id = %artifact.dataset.id,
                    artifact_path = %artifact.path,
                    "artifact classified and indexed"
                );
            }
            Err(error) => {
                stats.classification_failures += 1;

                warn!(
                    dataset_id = %artifact.dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to classify artifact"
                );

                self.warnings.push(format!(
                    "Could not classify {} from {}: {error}",
                    artifact.path, artifact.dataset.id
                ));
            }
        }
    }

    /// Searches for datasets using configured scientific search terms
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] if search fails
    async fn discover_datasets(&self) -> Result<(Vec<DatasetSummary>, CrawlStatus), CrawlerError> {
        if self.config.max_repositories == Some(0) {
            info!("repository limit is zero; skipping discovery");
            return Ok((Vec::new(), CrawlStatus::Completed));
        }
        
        let terms: Vec<Option<&str>> = match self.config.search_terms.as_deref() {
            Some(terms) if !terms.is_empty() => {
                terms.iter().map(|term| Some(term.as_str())).collect()
            }
            _ => vec![None],
        };


        info!(
            search_term_count = terms.len(),
            "prepared dataset search terms"
        );

        debug!(search_terms = ?terms, "dataset discovery search terms");

        let mut state = DiscoveryState::default();
        let mut status = CrawlStatus::Completed;

        for term in terms {
            let Some(results) = self.search_term(term).await? else {
                status = CrawlStatus::Interrupted;
                break;
            };
            let result_stats = self.process_search_results(term, results, &mut state);

            if result_stats.interrupted {
                status = CrawlStatus::Interrupted;
                break;
            }

            info!(
                search_term = ?term,
                duplicate_results = result_stats.duplicate_results,
                unique_dataset_count = state.datasets.len(),
                "processed dataset search results"
            );

            if result_stats.reached_limit {
                info!(
                    dataset_count = state.datasets.len(),
                    max_repositories = self.config.max_repositories,
                    "reached configured dataset discovery limit"
                );
                break;
            }
        }
        if self.cancellation.is_cancelled() {
            status = CrawlStatus::Interrupted;
        }

        info!(
            dataset_count = state.datasets.len(),
            crawl_status = ?status,
            "dataset discovery completed"
        );

        Ok((state.datasets, status))
    }

    /// Searches the dataset source using an optional search term.
    ///
    /// # Errors
    /// - Returns [`CrawlerError`] if the search fails.
    async fn search_term(&self, term: Option<&str>) -> Result<Option<Vec<DatasetSummary>>, CrawlerError> {
        info!(
            search_term = ?term,
            "searching dataset source"
        );

        let results = tokio::select! {
            biased;

            () = self.cancellation.cancelled() => {
                info!(
                    search_term = ?term,
                    "interrupted during dataset discovery"
                );
                return Ok(None);
            }
            result = self.source.search(
                term,
                self.config.formats.as_deref().unwrap_or(&[]),
                self.config.max_repositories,
            ) => result?
        };

        info!(
            search_term = ?term,
            result_count = results.len(),
            "dataset search completed"
        );

        Ok(Some(results))
    }

    /// Filters and collects dataset returned from search
    fn process_search_results(
        &self,
        term: &str,
        results: Vec<DatasetSummary>,
        state: &mut DiscoveryState,
    ) -> SearchResultStats {
        let mut search_stats = SearchResultStats::default();

        for dataset in results {
            if self.cancellation.is_cancelled() {
                search_stats.interrupted = true;
                break;
            }

            if !matches_filter_terms(&dataset, &self.config.filter_terms) {
                search_stats.filtered_out += 1;
                debug!(
                    dataset_id = %dataset.id,
                    search_term = %term,
                    "dataset rejected by metadata filters"
                );
                continue;
            }

            if state.seen.insert(dataset.id.clone()) {
                debug!(
                    dataset_id = %dataset.id,
                    search_term = %term,
                    "discovered new dataset"
                );

                state.datasets.push(dataset);
                if self.config.max_datasets.is_some_and(|limit| state.datasets.len() >= limit) {
                    search_stats.reached_limit = true;
                    break;
                }
            } else {
                search_stats.duplicate_results += 1;
                debug!(
                    search_term = %term,
                    "dataset already discovered by another search"
                );
            }
        }
        search_stats
    }
}

/// Selects an artifact for inspection if its format and size are supported.
fn select_artifact_format(
    dataset_id: &str,
    artifact_path: &str,
    artifact_size_bytes: u64,
    config: &CrawlConfig,
    stats: &mut DatasetStats,
) -> Option<ArtifactFormat> {
    let Some(format) = artifact_format_for_path(artifact_path, &config.formats) else {
        stats.skipped_unsupported_format += 1;

        debug!(dataset_id, artifact_path, "skipping unsupported artifact format");

        return None;
    };

    if config.max_file_size_bytes.is_some_and(|limit| artifact_size_bytes > limit) {
        stats.skipped_oversized += 1;
        debug!(
            dataset_id,
            artifact_path,
            artifact_size_bytes,
            max_file_size_bytes = config.max_file_size_bytes,
            "skipping oversized artifact"
        );

        return None;
    }

    Some(format)
}

/// Logs the inspection statistics for a dataset.
fn log_dataset_stats(dataset_id: &str, repository_artifact_count: usize, stats: &DatasetStats) {
    info!(
        dataset_id,
        repository_artifact_count,
        eligible_artifacts_attempted = stats.eligible_artifacts_attempted,
        downloaded_artifacts = stats.downloaded_artifacts,
        indexed_artifacts = stats.indexed_artifacts,
        skipped_unsupported_format = stats.skipped_unsupported_format,
        skipped_oversized = stats.skipped_oversized,
        download_failures = stats.download_failures,
        classification_failures = stats.classification_failures,
        artifact_limit_reached = stats.artifact_limit_reached,
        "finished inspecting dataset"
    );
}

/// Builds unique search terms, interleaving terms across content types.
fn search_terms(content_types: &[ContentType]) -> Vec<String> {
    let mut terms = Vec::new();
    let mut seen = HashSet::new();

    if strict {
        for content_type in content_types {
            let term = content_type.to_string();

            if seen.insert(term.clone()) {
                terms.push(term);
            }
        }

        return terms;
    }

    let max_terms = content_types
        .iter()
        .map(|content_type| content_type.terms().len())
        .max()
        .unwrap_or_default();

    for term_index in 0..max_terms {
        for content_type in content_types {
            if let Some(term) = content_type.terms().get(term_index) {
                let term = (*term).to_owned();

                if seen.insert(term.clone()) {
                    terms.push(term);
                }
            }
        }
    }

    terms
}

/// Checks whether all non-empty filter terms occur in dataset metadata.
fn matches_filter_terms(dataset: &DatasetSummary, filter_terms: &[String]) -> bool {
    let metadata = format!("{} {}", dataset.id, dataset.searchable_metadata).to_lowercase();

    filter_terms
        .iter()
        .filter(|term| !term.trim().is_empty())
        .all(|term| metadata.contains(&term.trim().to_lowercase()))
}

/// Maps a path to its configured artifact format.
fn artifact_format_for_path(path: &str, formats: &[FileFormat]) -> Option<ArtifactFormat> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();

    formats.iter().find_map(|format| {
        match (*format, extension.as_str()) {
            (FileFormat::Csv, "csv") => Some(ArtifactFormat::Tabular),
            (FileFormat::Json, "json") => Some(ArtifactFormat::Json),
            _ => None,
        }
    })
}
