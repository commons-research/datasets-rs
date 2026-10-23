use clap::{Parser,ValueEnum};
use datasets_core::{ArtifactFormat, ContentType};

/// CLI config for dataset crawler
#[derive(Debug, Parser)]
#[command(
    name = "dataset-crawler",
    version,
    about = "Discover and classify scientific datasets on public repositories"
)]


pub(crate) struct Args {
    /// Dataset source to search.
    #[arg(
        long,
        value_enum,
        default_value = "hf",
        env = "DATASETS_CRAWLER_SOURCE"
    )]
    pub(crate) source: SourceArg,

    /// Scientific content types to seek, comma separated.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "smiles,molecular-formula",
        env = "DATASETS_CRAWLER_CONTENT_TYPES"
    )]
    pub(crate) content_types: Vec<ContentType>,

    /// File formats to inspect, comma separated.
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        default_value = "csv,json",
        env = "DATASETS_CRAWLER_FORMATS"
    )]
    pub(crate) formats: Vec<FileFormatArg>,

    /// Use only the canonical search term for each content type.
    #[arg(long, env = "DATASETS_CRAWLER_STRICT")]
    pub(crate) strict: bool,

    /// Require each additional term to occur in dataset metadata.
    /// Repeat the option to require multiple terms.
    #[arg(
        long = "filter-term",
        action = clap::ArgAction::Append,
        value_delimiter = ',',
        env = "DATASETS_CRAWLER_FILTER_TERMS"
    )]
    pub(crate) filter_terms: Vec<String>,

    /// Maximum number of unique datasets to inspect.
    #[arg(
        long,
        default_value_t = 50,
        env = "DATASETS_CRAWLER_MAX_DATASETS"
    )]
    pub(crate) max_datasets: usize,

    /// Maximum number of results returned per search term.
    #[arg(
        long,
        default_value_t = 10,
        env = "DATASETS_CRAWLER_RETURNS_PER_TERM"
    )]
    pub(crate) returns_per_term: usize,

    /// Maximum number of repository tree entries retrieved per dataset.
    #[arg(
        long,
        default_value_t = 500,
        env = "DATASETS_CRAWLER_MAX_TREE_ENTRIES"
    )]
    pub(crate) max_tree_entries: usize,

    /// Maximum number of artifacts inspected per dataset.
    #[arg(
        long,
        default_value_t = 10,
        env = "DATASETS_CRAWLER_MAX_ARTIFACTS_PER_DATASET"
    )]
    pub(crate) max_artifacts_per_dataset: usize,

    /// Maximum file size in decimal MB.
    #[arg(
        long,
        default_value_t = 10,
        env = "DATASETS_CRAWLER_MAX_FILE_SIZE_MB"
    )]
    pub(crate) max_file_size_mb: u64,
}

impl Args {
    /// Parses crawler config from the CLI and ENV
    #[must_use]
    pub(crate) fn parse_args() -> Self {
        Self::parse()
    }
}

/// Repo source selected by CLI
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum SourceArg {
    /// Hugging Face Hub
    #[value(name = "hf")]
    HuggingFace,
}

/// File formats supported
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum FileFormatArg {
    /// Comma-separated values
    Csv,
    /// JSON
    Json,
}

impl FileFormatArg {
    /// Converts selected file format to artifact format
    #[must_use]
    pub(crate) const fn artifact_format(self) -> ArtifactFormat {
        match self {
            FileFormatArg::Csv => ArtifactFormat::Tabular,
            FileFormatArg::Json => ArtifactFormat::Json,
        }
    }
}