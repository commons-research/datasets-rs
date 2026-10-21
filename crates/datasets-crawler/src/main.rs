//! Web crawler dataset evaluation application runtime

#![allow(clippy::print_stdout)]
use datasets_core::ContentType;
use datasets_crawler::HuggingFaceSource;

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

    for dataset in datasets {
        println!("{}", dataset.id);
    }

    Ok(())
}
