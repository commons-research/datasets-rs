use std::{collections::HashSet, path::Path};

use datasets_core::{ArtifactFormat, ContentType};
use tracing::{debug, info, warn};

use super::{CandidateArtifact, Crawler, CrawlerError, DatasetSource, DatasetSummary};

/// Configuration for a repository-independent crawl.
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Scientific content types to seek.
    pub content_types: Vec<ContentType>,
    /// Artifact formats to inspect.
    pub formats: Vec<ArtifactFormat>,
    /// Search only the canonical term for each content type.
    pub strict: bool,
    /// Additional metadata terms that must all match.
    pub filter_terms: Vec<String>,
    /// Maximum number of unique datasets to inspect.
    pub max_datasets: usize,
    /// Maximum results returned per search term.
    pub results_per_term: usize,
    /// Maximum repository entries retrieved per dataset.
    pub max_tree_entries: usize,
    /// Maximum artifacts inspected per dataset.
    pub max_artifacts_per_dataset: usize,
    /// Maximum permitted artifact size in bytes.
    pub max_file_size_bytes: u64,
}

/// Results and non-fatal issues produced by a crawl.
#[derive(Debug)]
pub struct CrawlReport {
    /// Crawler containing the accumulated classification index.
    crawler: Crawler,
    /// Warnings for datasets or files that could not be processed.
    warnings: Vec<String>,
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
}

/// Discovers, inspects, classifies datasets
///
/// # Errors
/// - Returns an error if initial searches cannot be completed.
pub async fn crawl<S: DatasetSource>(
    source: &S,
    config: &CrawlConfig,
) -> Result<CrawlReport, CrawlerError> {
    info!(
        content_types = ?config.content_types,
        formats = ?config.formats,
        max_datasets = config.max_datasets,
        results_per_term = config.results_per_term,
        max_tree_entries = config.max_tree_entries,
        max_artifacts_per_dataset = config.max_artifacts_per_dataset,
        max_file_size_bytes = config.max_file_size_bytes,
        "starting crawl"
    );

    let datasets = discover_datasets(source, config).await?;
    let discovered_dataset_count = datasets.len();

    info!(dataset_count = discovered_dataset_count, "starting artifact inspection");

    let mut crawler = Crawler::new(&config.content_types);
    let mut warnings = Vec::new();

    for dataset in datasets {
        info!(
            dataset_id = %dataset.id,
            "inspecting discovered dataset"
        );

        let artifacts = match source.list_artifacts(&dataset.id, config.max_tree_entries).await {
            Ok(artifacts) => artifacts,
            Err(error) => {
                warn!(
                    dataset_id = %dataset.id,
                    error = %error,
                    "failed to list dataset artifacts; skipping dataset"
                );

                warnings.push(format!("Could not list artifacts for {}: {error}", dataset.id));

                continue;
            }
        };

        let repository_artifact_count = artifacts.len();

        info!(
            dataset_id = %dataset.id,
            artifact_count = repository_artifact_count,
            "retrieved repository artifact list"
        );

        let mut eligible_artifacts_attempted = 0;
        let mut downloaded_artifacts = 0;
        let mut indexed_artifacts = 0;
        let mut skipped_unsupported_format = 0;
        let mut skipped_oversized = 0;
        let mut download_failures = 0;
        let mut classification_failures = 0;
        let mut artifact_limit_reached = false;

        for artifact in artifacts {
            if eligible_artifacts_attempted >= config.max_artifacts_per_dataset {
                artifact_limit_reached = true;

                debug!(
                    dataset_id = %dataset.id,
                    max_artifacts_per_dataset = config.max_artifacts_per_dataset,
                    "stopping at configured artifact limit"
                );

                break;
            }

            let Some(format) = artifact_format_for_path(&artifact.path, &config.formats) else {
                skipped_unsupported_format += 1;

                debug!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    "skipping unsupported artifact format"
                );

                continue;
            };

            if artifact.size_bytes > config.max_file_size_bytes {
                skipped_oversized += 1;

                debug!(
                    dataset_id = %dataset.id,
                    artifact_path = %artifact.path,
                    artifact_size_bytes = artifact.size_bytes,
                    max_file_size_bytes = config.max_file_size_bytes,
                    "skipping oversized artifact"
                );

                continue;
            }

            eligible_artifacts_attempted += 1;

            debug!(
                dataset_id = %dataset.id,
                artifact_path = %artifact.path,
                artifact_size_bytes = artifact.size_bytes,
                "downloading supported artifact"
            );

            let data = match source.download_artifact(&dataset.id, &artifact).await {
                Ok(data) => {
                    downloaded_artifacts += 1;
                    data
                }
                Err(error) => {
                    download_failures += 1;

                    warn!(
                        dataset_id = %dataset.id,
                        artifact_path = %artifact.path,
                        error = %error,
                        "failed to download artifact; skipping it"
                    );

                    warnings.push(format!(
                        "Could not download {} from {}: {error}",
                        artifact.path, dataset.id
                    ));

                    continue;
                }
            };

            let artifact_path = artifact.path;

            let candidate = CandidateArtifact::new(
                dataset.id.clone(),
                dataset.url.clone(),
                artifact_path.clone(),
                format,
                data,
            );

            match crawler.process(candidate) {
                Ok(()) => {
                    indexed_artifacts += 1;

                    debug!(
                        dataset_id = %dataset.id,
                        artifact_path = %artifact_path,
                        "artifact classified and indexed"
                    );
                }
                Err(error) => {
                    classification_failures += 1;

                    warn!(
                        dataset_id = %dataset.id,
                        artifact_path = %artifact_path,
                        error = %error,
                        "failed to classify artifact"
                    );

                    warnings.push(format!(
                        "Could not classify {} from {}: {error}",
                        artifact_path, dataset.id
                    ));
                }
            }
        }

        info!(
            repository_artifact_count = repository_artifact_count,
            eligible_artifacts_attempted = eligible_artifacts_attempted,
            downloaded_artifacts = downloaded_artifacts,
            indexed_artifacts = indexed_artifacts,
            skipped_unsupported_format = skipped_unsupported_format,
            skipped_oversized = skipped_oversized,
            download_failures = download_failures,
            classification_failures = classification_failures,
            artifact_limit_reached = artifact_limit_reached,
            "finished inspecting dataset"
        );
    }

    info!(
        discovered_dataset_count,
        indexed_artifact_count = crawler.index().artifacts().count(),
        warning_count = warnings.len(),
        "crawl completed"
    );

    Ok(CrawlReport { crawler, warnings })
}

/// Searches for datasets using the configured scientific search terms.
///
/// # Errors
/// - Returns [`CrawlerError`] if unable to search for datasets
async fn discover_datasets<S: DatasetSource>(
    source: &S,
    config: &CrawlConfig,
) -> Result<Vec<DatasetSummary>, CrawlerError> {
    if config.max_datasets == 0 {
        info!("dataset limit is zero; skipping discovery");
        return Ok(Vec::new());
    }

    let terms = search_terms(&config.content_types, config.strict);

    info!(search_term_count = terms.len(), strict = config.strict, "prepared dataset search terms");

    debug!(search_terms = ?terms, "dataset discovery search terms");

    let mut datasets = Vec::new();
    let mut seen = HashSet::new();

    for term in terms {
        info!(
            search_term = %term,
            result_limit = config.results_per_term,
            "searching dataset source"
        );

        let results = source.search(&term, config.results_per_term).await?;
        let result_count = results.len();

        info!(
            search_term = %term,
            result_count,
            "dataset search completed"
        );

        let mut filtered_out = 0;
        let mut duplicate_results = 0;
        let mut reached_limit = false;

        for dataset in results {
            if !matches_filter_terms(&dataset, &config.filter_terms) {
                filtered_out += 1;

                debug!(
                    dataset_id = %dataset.id,
                    search_term = %term,
                    "dataset rejected by metadata filters"
                );

                continue;
            }

            if seen.insert(dataset.id.clone()) {
                debug!(
                    dataset_id = %dataset.id,
                    search_term = %term,
                    "discovered new dataset"
                );

                datasets.push(dataset);

                if datasets.len() >= config.max_datasets {
                    reached_limit = true;
                    break;
                }
            } else {
                duplicate_results += 1;

                debug!(
                    search_term = %term,
                    "dataset already discovered by another search"
                );
            }
        }

        info!(
            search_term = %term,
            filtered_out,
            duplicate_results,
            unique_dataset_count = datasets.len(),
            "processed dataset search results"
        );

        if reached_limit {
            info!(
                dataset_count = datasets.len(),
                max_datasets = config.max_datasets,
                "reached configured dataset discovery limit"
            );

            break;
        }
    }

    info!(dataset_count = datasets.len(), "dataset discovery completed");

    Ok(datasets)
}

/// Builds unique search terms, interleaving terms across content types.
fn search_terms(content_types: &[ContentType], strict: bool) -> Vec<String> {
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

/// Maps a supported file extension to its configured artifact format.
fn artifact_format_for_path(path: &str, formats: &[ArtifactFormat]) -> Option<ArtifactFormat> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();

    formats.iter().copied().find(|format| {
        match *format {
            ArtifactFormat::Tabular => extension == "csv",
            ArtifactFormat::Json => extension == "json",
        }
    })
}
