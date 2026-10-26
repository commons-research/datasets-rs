//! Dataset crawler application entry point.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use datasets_core::ContentType;
use datasets_crawler::{CrawlConfig, Crawler, CrawlerError, HuggingFaceSource, crawl};

/// Command-line configuration.
mod cli;

use cli::{Args, FileFormatArg, SourceArg};
use tokio::runtime::Builder;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Runs the dataset crawler.
///
/// # Errors
/// - Returns an error if initialization or dataset discovery fails.
fn main() -> Result<(), CrawlerError> {
    let args = Args::parse_args();
    init_logging()?;
    info!("Initializing asynchronous runtime");

    let runtime =
        Builder::new_multi_thread().enable_all().build().map_err(CrawlerError::Runtime)?;

    runtime.block_on(run(args))
}

/// Initializes tracing on stderr, honors `RUST_LOG`
///
/// # Errors
/// - Returns [`CrawlerError::Logging`] if subscriber fails
fn init_logging() -> Result<(), CrawlerError> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("datasets_crawler=info,hf_hub=warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| CrawlerError::Logging(error.to_string()))?;

    Ok(())
}

/// Executes crawler async
///
/// # Errors
/// - Returns error if init or dataset search fails
async fn run(args: Args) -> Result<(), CrawlerError> {
    let selected_source = args.source;

    let config = CrawlConfig {
        content_types: args.content_types,
        formats: args.formats.into_iter().map(FileFormatArg::artifact_format).collect(),
        strict: args.strict,
        filter_terms: args.filter_terms,
        max_datasets: args.max_datasets,
        results_per_term: args.returns_per_term,
        max_tree_entries: args.max_tree_entries,
        max_artifacts_per_dataset: args.max_artifacts_per_dataset,
        max_file_size_bytes: args.max_file_size_mb.saturating_mul(1_000_000),
    };

    info!(
        source = ?selected_source,
        content_type = ?config.content_types,
        formats = ?config.formats,
        strict = config.strict,
        max_datasets = config.max_datasets,
        results_per_term = config.results_per_term,
        max_tree_entries = config.max_tree_entries,
        max_artifacts_per_dataset = config.max_artifacts_per_dataset,
        max_file_size_bytes = config.max_file_size_bytes,
        "starting crawl",
    );

    let report = match selected_source {
        SourceArg::HuggingFace => {
            let source = HuggingFaceSource::new()?;
            crawl(&source, &config).await?
        }
    };

    info!(
        index_artifacts = report.crawler().index().artifacts().count(),
        warning_count = report.warnings().len(),
        "crawl concluded",
    );

    print_results(report.crawler(), &config.content_types);

    Ok(())
}

/// Prints indexed artifacts and their classification evidence.
fn print_results(crawler: &Crawler, content_types: &[ContentType]) {
    for content_type in content_types {
        println!("\nResults for {content_type}:");

        let mut found = false;

        for artifact in crawler.index().search(*content_type) {
            found = true;

            println!("- {} ({})", artifact.dataset_id(), artifact.dataset_url());
            println!("  Artifact: {}", artifact.artifact_ref());

            for classification in artifact.classifications().iter().filter(|classification| {
                classification.content_type() == *content_type && classification.valid() > 0
            }) {
                println!(
                    "  {}: {}/{} valid entries",
                    classification.location(),
                    classification.valid(),
                    classification.inspected()
                );
            }
        }

        if !found {
            println!("  No matching artifacts found.");
        }
    }
}
