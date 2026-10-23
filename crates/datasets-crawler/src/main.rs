//! Dataset crawler application entry point.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use datasets_core::ContentType;
use datasets_crawler::{CrawlConfig, Crawler, CrawlerError, HuggingFaceSource, crawl};

/// Command-line configuration.
mod cli;

use cli::{Args, FileFormatArg, SourceArg};

/// Runs the dataset crawler.
///
/// # Errors
/// - Returns an error if initialization or dataset discovery fails.
///
/// # Panics
/// - May panic
#[tokio::main]
async fn main() -> Result<(), CrawlerError> {
    let args = Args::parse_args();
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

    let report = match selected_source {
        SourceArg::HuggingFace => {
            let source = HuggingFaceSource::new()?;
            crawl(&source, &config).await?
        }
    };

    for warning in report.warnings() {
        eprintln!("Warning: {warning}");
    }

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
