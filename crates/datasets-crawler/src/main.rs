//! Dataset crawler application entry point.

#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(clippy::future_not_send)]
use std::time::Duration;

use datasets_crawler::{
    CrawlConfig, CrawlStatus, CrawlerError, Database, HuggingFaceSource, crawl,
};

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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("datasets_crawler=info,hf_hub=warn")),
        )
        .init();

    let args = Args::parse_args();
    info!("Initializing asynchronous runtime");

    let runtime =
        Builder::new_multi_thread().enable_all().build().map_err(CrawlerError::Runtime)?;

    runtime.block_on(run(args))
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
        max_repositories: args.max_repositories,
        stall_timeout: STALL_TIMEOUT,
    };

    let mut database = Database::open_database("datasets.sqlite")?;

    info!(
        source = ?selected_source,
        content_types = ?config.content_types,
        formats = ?config.formats,
        max_repositories = config.max_repositories,
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
                Ok(source) => crawl(&source, &mut database, &config, &cancellation).await,
                Err(error) => Err(error),
            }
        }
        // update when zenodo implemented
        SourceArg::Zenodo => return Ok(()),
    };

    // The crawl has finished; stop listening for signals.
    signal_task.abort();

    let report = report_result?;

    match report {
        CrawlStatus::Completed => {
            info!("Crawl completed successfully");
        }
        CrawlStatus::Interrupted => {
            warn!("Crawl interrupted; results saved to DB");
        }
    }

    Ok(())
}
