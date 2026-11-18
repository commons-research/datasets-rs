//! Dataset crawler application entry point.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::time::Duration;

use datasets_core::ContentType;
use datasets_crawler::{CrawlConfig, CrawlStatus, Crawler, CrawlerError, HuggingFaceSource, crawl};

/// Command-line configuration.
mod cli;

use cli::{Args, SourceArg};
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use self::cli::ResolvedArgs;

/// Longest an artifact stream may stall before being abandoned.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

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
async fn run(args: ResolvedArgs) -> Result<(), CrawlerError> {
    let selected_source = args.source;

    let config = CrawlConfig {
        content_types: args.content_types,
        formats: args.formats,
        search_terms: args.search_terms,
        max_repositories: args.max_repositories,
        stall_timeout: STALL_TIMEOUT,
    };

    info!(
        source = ?selected_source,
        content_types = ?config.content_types,
        formats = ?config.formats,
        search_terms = ?config.search_terms,
        max_repositories = config.max_repositories,
        limits = ?config.limits,
        "starting crawl",
    );

    // Create the single cancellation token for this crawl.
    let cancellation = CancellationToken::new();

    // Give the signal handler a handle to the same cancellation state.
    let signal_cancellation = cancellation.clone();

    // Listen for Ctrl+C independently of the crawling operation.
    let signal_task = tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                warn!("Ctrl+C received; requesting graceful shutdown");
                signal_cancellation.cancel();
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    "failed to listen for Ctrl+C"
                );
            }
        }
    });

    // Run the selected source.
    let report_result = match selected_source {
        SourceArg::HuggingFace => {
            match HuggingFaceSource::new() {
                Ok(source) => crawl(&source, &config, &cancellation).await,
                Err(error) => Err(error),
            }
        }
        // update when zenodo implemented
        SourceArg::Zenodo => return Ok(()),
    };

    // The crawl has finished; stop listening for signals.
    signal_task.abort();

    let report = report_result?;

    match report.status() {
        CrawlStatus::Completed => {
            info!("Crawl completed successfully");
        }
        CrawlStatus::Interrupted => {
            warn!("Crawl interrupted; results saved to DB");
        }
    }

    Ok(())
}
