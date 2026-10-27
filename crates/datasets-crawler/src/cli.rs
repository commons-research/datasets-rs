use clap::{Parser, ValueEnum};
use datasets_core::{ArtifactFormat, ContentType};
use super::source::FileFormat;

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
    #[arg(
        long,
        value_enum,
        required_unless_present = "default",
        env = "DATASETS_CRAWLER_SOURCE"
    )]
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
    #[arg(
        long,
        value_delimiter = ',',
        required_unless_present = "default",
        env = "DATASETS_CRAWLER_FORMATS"
    )]
    formats: Vec<FileFormat>,

    /// Use only canonical search terms.
    #[arg(long, env = "DATASETS_CRAWLER_STRICT")]
    strict: bool,

    /// Additional metadata terms; all non-empty terms must match.
    #[arg(
        long = "filter-term",
        action = clap::ArgAction::Append,
        value_delimiter = ',',
        env = "DATASETS_CRAWLER_FILTER_TERMS"
    )]
    filter_terms: Vec<String>,

    /// Maximum number of unique datasets to inspect.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_DATASETS")]
    max_datasets: Option<usize>,

    /// Maximum results returned per search term.
    #[arg(long, env = "DATASETS_CRAWLER_RETURNS_PER_TERM")]
    returns_per_term: Option<usize>,

    /// Maximum repository entries retrieved per dataset.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_TREE_ENTRIES")]
    max_tree_entries: Option<usize>,

    /// Maximum artifacts inspected per dataset.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_ARTIFACTS_PER_DATASET")]
    max_artifacts_per_dataset: Option<usize>,

    /// Maximum file size in decimal MB.
    #[arg(long, env = "DATASETS_CRAWLER_MAX_FILE_SIZE_MB")]
    max_file_size_mb: Option<u64>,
}

/// Fully resolved CLI configuration.
pub(crate) struct ResolvedArgs {
    pub(crate) source: SourceArg,
    pub(crate) content_types: Vec<ContentType>,
    pub(crate) formats: Vec<FileFormat>,
    pub(crate) strict: bool,
    pub(crate) filter_terms: Vec<String>,
    pub(crate) max_datasets: Option<usize>,
    pub(crate) returns_per_term: Option<usize>,
    pub(crate) max_tree_entries: Option<usize>,
    pub(crate) max_artifacts_per_dataset: Option<usize>,
    pub(crate) max_file_size_mb: Option<u64>,
}


impl Args {
    /// Parses and resolves crawler configuration.
    #[must_use]
    pub(crate) fn parse_args() -> ResolvedArgs {
        Self::parse().resolve()
    }

    /// Resolves explicitly supplied arguments and optional defaults.
    fn resolve(self) -> ResolvedArgs {
        let source = self
            .source
            .unwrap_or(SourceArg::HuggingFace);

        let content_types = if self.content_types.is_empty() {
            vec![
                ContentType::Smiles,
                ContentType::MolecularFormula,
            ]
        } else {
            self.content_types
        };

        let formats = if self.formats.is_empty() {
            vec![
                FileFormat::Csv,
                FileFormat::Json,
            ]
        } else {
            self.formats
        };

        ResolvedArgs {
            source,
            content_types,
            formats,
            strict: self.strict,
            filter_terms: self.filter_terms,

            max_datasets: self.max_datasets.or(
                self.default.then_some(50),
            ),

            returns_per_term: self.returns_per_term.or(
                self.default.then_some(10),
            ),

            max_tree_entries: self.max_tree_entries.or(
                self.default.then_some(500),
            ),

            max_artifacts_per_dataset:
                self.max_artifacts_per_dataset.or(
                    self.default.then_some(10),
                ),

            max_file_size_mb: self.max_file_size_mb.or(
                self.default.then_some(10),
            ),
        }
    }
}

/// Dataset source selected by the CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SourceArg {
    /// Hugging Face Hub.
    #[value(name = "hf")]
    HuggingFace,
}
