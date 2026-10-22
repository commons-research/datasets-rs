//! Web crawler dataset evaluation application runtime

#![allow(clippy::print_stdout, clippy::print_stderr)]
use std::{
    collections::HashSet,
    error::Error,
    fs,
    path::Path
}
use clap::{Parser,ValueEnum};
use datasets_core::{ArtifactFormat, ContentType};
use datasets_crawler::{CandidateArtifact, Crawler, DatasetIndex, HuggingFaceSource};
use hf_hub::repository::{DatasetInfo, RepoTreeEntry};
/// Command Line Interface settings for the crawler
mod cli;


/// Main entry point for the crawler application
///
/// # Panics
/// - Panics when encountering an error and returns it.
///
/// # Errors
/// - Returns an error if unable to reach data source or work with the dataset
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = HuggingFaceSource::new()?;
    let query = ContentType::Smiles.to_string();
    let datasets = source.search(&query, 10).await?;

    let Some(dataset) = datasets.first() else {
        println!("No datasets found.");
        return Ok(());
    };

    println!("Dataset: {}", dataset.id);

    let entries = source.list_tree(&dataset.id, 50000).await?;

    for entry in entries {
        if let RepoTreeEntry::File { path, size, .. } = entry {
            println!("{path} ({size} bytes)");
        }
    }

    Ok(())
}
