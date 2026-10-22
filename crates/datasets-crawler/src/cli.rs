use clap::{Parser,ValueEnum};

/// CLI config for dataset crawler
#[derive(Debug, Parser)]
#[command(
    name = "dataset-crawler",
    version,
    about = "Discover and classify scientific datasets on public repositories"
)]
struct Args {
    /// Dataset source to search
    #[arg(long, value_enum, default_value = "hf")]
    source: SourceArg,

    /// Scientific content types to seek (comma separated)
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        default_value = "smiles,molecular-formula"
    )]
    content_types: Vec<CliContentType>,

    /// File formats to inspect (comma separated)
    #[arg(long, value_enum, value_delimiter = ',', default_value = "csv,json")]
    formats: Vec<FileFormatArg>,

    /// Use only canonical search term for content type
    #[arg(long)]
    strict: bool,

    /// Require each additional term occur in dataset metadata (all terms must match)
    #[arg(long = "filter-term", action = clap::ArgAction::Append)]
    filter_terms: Vec<String>,

    /// Max num of unique datasets to inspect
    #[arg(long, default_value_t = 50)]
    max_datasets: usize,

    /// Maximum number of results returned from each search term
    #[arg(long, default_value_t = 10)]
    returns_per_term: usize,

    /// Max number of repo tree entries to retrieve per dataset
    #[arg(long, default_value_t = 500)]
    max_tree_entries: usize,

    /// Max num of artifacts to inspect per dataset
    #[arg(long, default_value_t = 10)]
    max_artifacts_per_dataset: usize,

    /// Max file size in decimal MB
    #[arg(long, default_value_t = 10)]
    max_file_size_mb: u64,

}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum SourceArg {
    /// Hugging face 
    #[value(name = "hf")]
    HuggingFace,
}

