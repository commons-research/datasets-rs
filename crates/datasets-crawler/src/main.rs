//! Web crawler dataset evaluation application runtime

#![allow(clippy::print_stdout)]
use datasets_core::ContentType;
use datasets_crawler::HuggingFaceSource;
use hf_hub::repository::RepoTreeEntry;

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
