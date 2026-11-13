use clap::{Parser, ValueEnum};
use datasets_core::{ContentType, InspectionLimits};
use datasets_crawler::FileFormat;

/// CLI configuration for the dataset crawler.
#[derive(Debug, Parser)]
#[command(
    name = "dataset-crawler",
    version,
    about = "Discover and classify scientific datasets on public repositories",
    arg_required_else_help = true
)]
pub struct Args {
    /// Use the built-in configuration for unspecified arguments.
    #[arg(long)]
    default: bool,

    /// Dataset source to search.
    #[arg(long, value_enum, required_unless_present = "default", env = "DATASETS_CRAWLER_SOURCE")]
    source: Option<SourceArg>,

    /// Scientific content types to seek, comma separated.
    #[arg(
        long,
        value_delimiter = ',',
        required_unless_present = "default",
        env = "DATASETS_CRAWLER_CONTENT_TYPES"
    )]
    content_types: Vec<ContentType>,

    /// Physical file formats to inspect, comma separated.
    #[arg(long, value_delimiter = ',', env = "DATASETS_CRAWLER_FORMATS")]
    formats: Vec<FileFormat>,

    /// Optional metadata search terms for dataset discovery.
    #[arg(
        long = "search-term",
        action = clap::ArgAction::Append,
        value_delimiter = ',',
        env = "DATASETS_CRAWLER_SEARCH_TERMS"
    )]
    search_terms: Vec<String>,

    /// Maximum number of unique repositories to inspect.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_REPOSITORIES")]
    max_repositories: Option<usize>,

    
    /// Maximum number of records inspected per artifact.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_RECORDS")]
    max_records: Option<usize>,

    /// Maximum decimal megabytes read per artifact.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_ARTIFACT_MB")]
    max_artifact_mb: Option<u64>,
}

/// Fully resolved CLI configuration.
pub struct ResolvedArgs {
    /// Selected dataset source.
    pub source: SourceArg,
    /// Scientific content types to classify.
    pub content_types: Vec<ContentType>,
    /// Optional file-format restrictions.
    pub formats: Option<Vec<FileFormat>>,
    /// Optional search terms for dataset discovery.
    pub search_terms: Option<Vec<String>>,
    /// Maximum number of repositories to inspect.
    pub max_repositories: Option<usize>,
    /// Bounds on how much of each artifact is inspected.
    pub limits: InspectionLimits,
}

impl Args {
    /// Parses and resolves crawler configuration.
    #[must_use]
    pub(super) fn parse_args() -> ResolvedArgs {
        Self::parse().resolve()
    }

    /// Resolves explicitly supplied arguments and optional defaults.
    fn resolve(self) -> ResolvedArgs {
        let source = self.source.unwrap_or(SourceArg::HuggingFace);

        let content_types = if self.content_types.is_empty() {
            vec![ContentType::Smiles, ContentType::MolecularFormula]
        } else {
            self.content_types
        };

        let formats = if self.formats.is_empty() { None } else { Some(self.formats) };

        let search_terms =
            if self.search_terms.is_empty() { None } else { Some(self.search_terms) };

        let max_repositories = self.max_repositories.or_else(|| self.default.then_some(50));

        let limits = InspectionLimits {
            max_records: self.max_records.or_else(|| self.default.then_some(10_000)),
            max_bytes: self.max_artifact_mb.or_else(|| self.default.then_some(64)).map(|megabytes| megabytes.saturating_mul(1_000_000)),
        };
        ResolvedArgs { source, content_types, formats, search_terms, max_repositories, limits }
    }
}

/// Dataset source selected by the CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SourceArg {
    /// Hugging Face Hub.
    #[value(name = "hf")]
    HuggingFace,
    /// Zenodo Open Science data hosting platform
    #[value(name = "zenodo")]
    Zenodo,
}
