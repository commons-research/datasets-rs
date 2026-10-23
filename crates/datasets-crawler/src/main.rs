//! Web crawler dataset evaluation application runtime

#![allow(clippy::print_stdout, clippy::print_stderr)]
use std::{
    collections::HashSet,
    error::Error,
    fs,
    path::Path
}
use clap::Parser;
use datasets_core::{ArtifactFormat, ContentType};
use datasets_crawler::{CandidateArtifact, Crawler, DatasetIndex, HuggingFaceSource};
use hf_hub::repository::{DatasetInfo, RepoTreeEntry};
/// Command Line Interface settings for the crawler
mod cli;

use cli::{Args, FileFormatArg, SourceArg};

/// Main entry point for the crawler application
///
/// # Panics
/// - Panics when encountering an error and returns it.
///
/// # Errors
/// - Returns an error if unable to reach data source or work with the dataset
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse_args();
    match args.source {
        SourceArg::HuggingFace => crawl_hugging_face(&args).await?,
    }

    Ok(())
}
