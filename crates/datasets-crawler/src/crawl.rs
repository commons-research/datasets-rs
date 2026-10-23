use std::{collections::HashSet, error::Error, path::Path};

use datasets_core::{ArtifactFormat, ContentType};

use crate::{CandidateArtifact, Crawler, DatasetSource, DatasetSummary};

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
) -> Result<CrawlReport, Box<dyn Error + Send + Sync>> {
    let datasets = discover_datasets(source, config).await?;
    let mut crawler = Crawler::new(&config.content_types);
    let mut warnings = Vec::new();

    for dataset in datasets {
        let artifacts = match source.list_artifacts(&dataset.id, config.max_tree_entries).await {
            Ok(artifacts) => artifacts,
            Err(error) => {
                warnings.push(format!("Could not list artifacts for {}: {error}", dataset.id));
                continue;
            }
        };

        let mut inspected_artifacts = 0;

        for artifact in artifacts {
            if inspected_artifacts >= config.max_artifacts_per_dataset {
                break;
            }

            let Some(format) = artifact_format_for_path(&artifact.path, &config.formats) else {
                continue;
            };

            if artifact.size_bytes > config.max_file_size_bytes {
                continue;
            }

            inspected_artifacts += 1;

            let data = match source.download_artifact(&dataset.id, &artifact).await {
                Ok(data) => data,
                Err(error) => {
                    warnings.push(format!(
                        "Could not download {} from {}: {error}",
                        artifact.path, dataset.id
                    ));
                    continue;
                }
            };

            let candidate = CandidateArtifact::new(
                dataset.id.clone(),
                dataset.url.clone(),
                artifact.path,
                format,
                data,
            );

            if let Err(error) = crawler.process(candidate) {
                warnings
                    .push(format!("Could not classify an artifact from {}: {error}", dataset.id));
            }
        }
    }

    Ok(CrawlReport { crawler, warnings })
}

/// Searches for datasets using the configured scientific search terms.
async fn discover_datasets<S: DatasetSource>(
    source: &S,
    config: &CrawlConfig,
) -> Result<Vec<DatasetSummary>, Box<dyn Error + Send + Sync>> {
    if config.max_datasets == 0 {
        return Ok(Vec::new());
    }

    let mut datasets = Vec::new();
    let mut seen = HashSet::new();

    for term in search_terms(&config.content_types, config.strict) {
        let results = source.search(&term, config.results_per_term).await?;

        for dataset in results {
            if !matches_filter_terms(&dataset, &config.filter_terms) {
                continue;
            }

            if seen.insert(dataset.id.clone()) {
                datasets.push(dataset);

                if datasets.len() >= config.max_datasets {
                    return Ok(datasets);
                }
            }
        }
    }

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
