use std::{collections::HashSet, time::Duration};

use datasets_core::{ArtifactFormat, ContentType, InspectionLimits};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::{CandidateArtifact, Crawler, CrawlerError, DatasetSource, DatasetSummary, FileFormat, SourceArtifact, reader::BlockingReader};

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
    /// Bounds on how much of each artifact is read and inspected. 
    pub limits: InspectionLimits,
    /// Longest an artifact stream may go without delivering bytes before abandoning.
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
    /// Number of artifacts with streams that could not be opened.
    open_failures: usize,
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
    /// Number of datasets already discovered in another search
    duplicate_results: usize,
    /// Dataset limit reached
    reached_limit: bool,
    /// Process interrupted
    interrupted: bool,
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
        crawler: Crawler::new(&config.content_types, config.limits),
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

    /// Streams one artifact through classification and indexes the result.
    async fn inspect_artifact(
        &mut self,
        dataset: &DatasetSummary,
        artifact: &SourceArtifact,
        stats: &mut DatasetStats,
    ) -> InspectionOutcome {
        let Some(format) = select_artifact_format(&dataset.id, &artifact.path, self.config, stats)
        else {
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
                stats.open_failures += 1;
                warn!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to open artifact; skipping artifact"
                );
                self.warnings.push(format!("Could not open {} from {}: {error}", artifact.path, dataset.id));
                return InspectionOutcome::Completed;
            }
        };
        
        let reader = BlockingReader::new(stream, cancellation, self.config.stall_timeout);
        let candidate = CandidateArtifact::new(
            dataset.id.clone(),
            dataset.url.clone(),
            artifact.path.to_owned(),
            format,
            );

        let crawler = &mut self.crawler;
        let result = tokio::task::block_in_place(|| crawler.process(candidate, reader));

                    match result {
            Ok(indexed) => {
                stats.indexed_artifacts = 1;
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
                stats.classification_failures += 1;

                warn!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    error = %error,
                    "failed to classify artifact"
                );

                self.warnings.push(format!(
                    "Could not classify {} from {}: {error}",
                    artifact.path, dataset.id
                ));
            }
        }
        self.inspection_outcome()
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

        info!(search_term_count = terms.len(), "prepared dataset search terms");

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
    async fn search_term(
        &self,
        term: Option<&str>,
    ) -> Result<Option<Vec<DatasetSummary>>, CrawlerError> {
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
            result = self.source.discover_datasets(
                term,
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

    /// Collects unique datasets returned by a search.
    fn process_search_results(
        &self,
        term: Option<&str>,
        results: Vec<DatasetSummary>,
        state: &mut DiscoveryState,
    ) -> SearchResultStats {
        let mut search_stats = SearchResultStats::default();

        for dataset in results {
            if self.cancellation.is_cancelled() {
                search_stats.interrupted = true;
                break;
            }
            if state.seen.insert(dataset.id.clone()) {
                debug!(
                    dataset_id = %dataset.id,
                    search_term = ?term,
                    "discovered new dataset"
                );

                state.datasets.push(dataset);

                if self.config.max_repositories.is_some_and(|limit| state.datasets.len() >= limit) {
                    search_stats.reached_limit = true;
                    break;
                }
            } else {
                search_stats.duplicate_results += 1;
                debug!(search_term = ?term, "dataset already discovered by another search");
            }
        }
        search_stats
    }
}

/// Selects an artifact for inspection if its format is supported and permitted.
fn select_artifact_format(
    dataset_id: &str,
    artifact_path: &str,
    config: &CrawlConfig,
    stats: &mut DatasetStats,
) -> Option<ArtifactFormat> {
    let Some(file_format) = FileFormat::from_path(artifact_path) else {
        stats.skipped_unsupported_format += 1;
        debug!(dataset_id, artifact_path, "skipping unsupported artifact format");
        return None;
    };

    if config.formats.as_ref().is_some_and(|formats| !formats.contains(&file_format)) {
        debug!(
            dataset_id,
            artifact_path,
            file_format = %file_format,
            "skipping artifact due to format restriction"
        );
        return None;
    }

    Some(file_format.artifact_format())
}

/// Logs the inspection statistics for a dataset.
fn log_dataset_stats(dataset_id: &str, repository_artifact_count: usize, stats: &DatasetStats) {
    info!(
        dataset_id,
        repository_artifact_count,
        indexed_artifacts = stats.indexed_artifacts,
        skipped_unsupported_format = stats.skipped_unsupported_format,
        open_failures = stats.open_failures,
        classification_failures = stats.classification_failures,
        "finished inspecting dataset"
    );
}


#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use datasets_core::StopReason;
    use futures::{AsyncRead, io::Cursor};

    use super::*;

    /// One artifact served by the mock source.
    #[derive(Clone)]
    enum MockContent {
        /// Bytes delivered normally.
        Bytes(Vec<u8>),
        /// A stream that never delivers anything.
        Stalled,
    }

    /// Reader served by the mock source.
    enum MockReader {
        /// Reads from memory.
        Bytes(Cursor<Vec<u8>>),
        /// Never becomes ready.
        Stalled,
    }

    impl AsyncRead for MockReader {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            match self.get_mut() {
                Self::Bytes(cursor) => Pin::new(cursor).poll_read(cx, buf),
                Self::Stalled => Poll::Pending,
            }
        }
    }

    /// In-memory dataset source with one dataset.
    struct MockSource {
        /// Files in the dataset, by path.
        files: Vec<(&'static str, MockContent)>,
    }

    impl DatasetSource for MockSource {
        async fn discover_datasets(
            &self,
            _query: Option<&str>,
            _limit: Option<usize>,
        ) -> Result<Vec<DatasetSummary>, CrawlerError> {
            Ok(vec![DatasetSummary {
                id: "mock/dataset".to_owned(),
                url: "https://example.org/mock/dataset".to_owned(),
                searchable_metadata: String::new(),
            }])
        }

        async fn list_artifacts(
            &self,
            _dataset_id: &str,
        ) -> Result<Vec<SourceArtifact>, CrawlerError> {
            Ok(self
                .files
                .iter()
                .map(|(path, content)| {
                    let size_bytes = match content {
                        MockContent::Bytes(bytes) => u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                        MockContent::Stalled => 0,
                    };
                    SourceArtifact { path: (*path).to_owned(), size_bytes }
                })
                .collect())
        }

        async fn open_artifact<'a>(
            &'a self,
            _dataset_id: &'a str,
            artifact: &'a SourceArtifact,
        ) -> Result<impl AsyncRead + Send + Unpin + 'a, CrawlerError> {
            let content = self
                .files
                .iter()
                .find(|(path, _)| *path == artifact.path)
                .map(|(_, content)| content.clone())
                .ok_or_else(|| CrawlerError::ParseFileFormat(artifact.path.clone()))?;

            Ok(match content {
                MockContent::Bytes(bytes) => MockReader::Bytes(Cursor::new(bytes)),
                MockContent::Stalled => MockReader::Stalled,
            })
        }
    }

    /// Crawl configuration used by the tests.
    fn config(limits: InspectionLimits) -> CrawlConfig {
        CrawlConfig {
            content_types: vec![ContentType::Smiles],
            formats: None,
            search_terms: None,
            max_repositories: None,
            limits,
            stall_timeout: Duration::from_millis(200),
        }
    }

    /// A CSV file with a header and `rows` SMILES rows.
    fn smiles_csv(rows: usize) -> MockContent {
        MockContent::Bytes(format!("name,smiles\n{}", "ethanol,CCO\n".repeat(rows)).into_bytes())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn crawls_streamed_artifacts_into_the_index() -> Result<(), CrawlerError> {
        let source = MockSource {
            files: vec![
                ("data.csv", smiles_csv(100)),
                (
                    "records.jsonl",
                    MockContent::Bytes(b"{\"smiles\": \"CCO\"}\n{\"smiles\": \"O\"}\n".to_vec()),
                ),
                ("README.md", MockContent::Bytes(b"# readme".to_vec())),
            ],
        };

        let report =
            crawl(&source, &config(InspectionLimits::UNLIMITED), &CancellationToken::new()).await?;

        assert_eq!(report.status(), CrawlStatus::Completed, "crawl should complete");
        assert!(report.warnings().is_empty(), "no warnings expected: {:?}", report.warnings());

        let matches: Vec<_> = report.crawler().index().search(ContentType::Smiles).collect();
        assert_eq!(matches.len(), 2, "the CSV and the JSON Lines file should both match");
        assert!(
            matches.iter().all(|artifact| artifact.inspection().is_complete()),
            "small artifacts should be inspected completely"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stops_reading_at_the_record_limit() -> Result<(), CrawlerError> {
        let source = MockSource { files: vec![("data.csv", smiles_csv(100_000))] };
        let limits = InspectionLimits { max_records: Some(50), max_bytes: None };

        let report = crawl(&source, &config(limits), &CancellationToken::new()).await?;

        let indexed: Vec<_> = report.crawler().index().artifacts().collect();
        assert_eq!(indexed.len(), 1, "the artifact should be indexed");

        let inspection = indexed[0].inspection();
        assert_eq!(inspection.records, 50, "only the limited number of records should be read");
        assert_eq!(inspection.stop, StopReason::RecordLimit, "the record limit should stop it");
        assert!(
            inspection.bytes < 200_000,
            "most of the 1.2 MB artifact should never be read, read {}",
            inspection.bytes
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stalled_artifact_is_skipped_with_a_warning() -> Result<(), CrawlerError> {
        let source = MockSource {
            files: vec![("stalled.csv", MockContent::Stalled), ("data.csv", smiles_csv(10))],
        };

        let report =
            crawl(&source, &config(InspectionLimits::UNLIMITED), &CancellationToken::new()).await?;

        assert_eq!(report.status(), CrawlStatus::Completed, "a stalled file is not fatal");
        assert_eq!(report.warnings().len(), 1, "the stalled artifact should be reported");
        assert_eq!(
            report.crawler().index().search(ContentType::Smiles).count(),
            1,
            "the artifact after the stalled one should still be indexed"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancellation_interrupts_a_stalled_read() -> Result<(), CrawlerError> {
        let source = MockSource { files: vec![("stalled.csv", MockContent::Stalled)] };
        let mut config = config(InspectionLimits::UNLIMITED);
        config.stall_timeout = Duration::from_hours(1);

        let cancellation = CancellationToken::new();
        let trigger = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            trigger.cancel();
        });

        let report = crawl(&source, &config, &cancellation).await?;

        assert_eq!(report.status(), CrawlStatus::Interrupted, "crawl should report interruption");
        assert!(report.warnings().is_empty(), "cancellation is not a warning");

        Ok(())
    }
}
