use futures::{StreamExt, TryStreamExt, future};
use hf_hub::{
    HFClient,
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
};
use tracing::debug;

use super::super::{
    error::CrawlerError,
    source::{DatasetSource, DatasetSummary, FileFormat, SourceArtifact},
};

/// Structure for discovering datasets hosted on Hugging Face
pub struct HuggingFaceSource {
    /// Client used to communicate with HF Hub
    client: HFClient,
}

impl HuggingFaceSource {
    /// Creates a Hugging Face dataset source
    ///
    /// # Errors
    /// - Returns error if HF Client cannot be created
    pub fn new() -> Result<Self, CrawlerError> {
        Ok(Self { client: HFClient::new()? })
    }
}

impl DatasetSource for HuggingFaceSource {
    async fn search(
        &self,
        query: &str,
        formats: &[FileFormat],
        limit: Option<usize>,
    ) -> Result<Vec<DatasetSummary>, CrawlerError> {
        debug!(
            query = %query,
            formats = ?formats,
            limit = ?limit,
            "querying Hugging Face dataset search"
        );

        let stream = self.client.list_datasets().search(query).full(true).send()?.try_filter(|dataset| {
                let matches = dataset_matches_formats(dataset, formats);

                if !matches {
                    debug!(
                        dataset_id = %dataset.id,
                        requested_formats = ?formats,
                        "dataset rejected by Hugging Face format metadata"
                    );
                }
                future::ready(matches)
        });

        let datasets: Vec<DatasetInfo> = match limit {
            Some(limit) => stream.take(limit).try_collect().await?,
            None => stream.try_collect().await?
        };

        debug!(
            query = %query,
            matching_datasets = datasets.len(),
            "finished Hugging Face metadata filtering"
        );

        Ok(datasets.iter().map(dataset_summary).collect())
    }

    async fn list_artifacts(
        &self,
        dataset_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SourceArtifact>, CrawlerError> {
        let (owner, name) = split_id(dataset_id);

        debug!(dataset_id = %dataset_id, limit, "requesting Hugging Face repo tree");

        let entries: Vec<RepoTreeEntry> = self
            .client
            .dataset(owner, name)
            .list_tree()
            .recursive(true)
            .expand(true)
            .maybe_limit(limit)
            .send()?
            .try_collect()
            .await?;

        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                match entry {
                    RepoTreeEntry::File { path, size, .. } => {
                        Some(SourceArtifact { path, size_bytes: size })
                    }
                    RepoTreeEntry::Directory { .. } => None,
                }
            })
            .collect())
    }

    async fn download_artifact(
        &self,
        dataset_id: &str,
        artifact: &SourceArtifact,
    ) -> Result<Vec<u8>, CrawlerError> {
        let (owner, name) = split_id(dataset_id);

        debug!(dataset_id = %dataset_id, artifact_path = %artifact.path, "requesting Hugging Face artifact download");

        let cached_path = self
            .client
            .dataset(owner, name)
            .download_file()
            .filename(artifact.path.clone())
            .send()
            .await?;

        Ok(tokio::fs::read(cached_path).await?)
    }
}

/// Converts Hugging Face metadata into source-independent dataset metadata
fn dataset_summary(dataset: &DatasetInfo) -> DatasetSummary {
    let mut searchable_metadata = dataset.id.clone();

    if let Some(description) = &dataset.description {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(description);
    }

    if let Some(tags) = &dataset.tags {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(&tags.join(" "));
    }

    if let Some(card_data) = &dataset.card_data {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(&card_data.to_string());
    }

    DatasetSummary {
        id: dataset.id.clone(),
        url: format!("https://huggingface.co/datasets/{}", dataset.id),
        searchable_metadata,
    }
}

/// Checks HF format metadata against required file formats
fn dataset_matches_formats(dataset: &DatasetInfo, requested_formats: &[FileFormat]) -> bool {
    if requested_formats.is_empty() {
        return true;
    }
    let Some(tags) = &dataset.tags else {
        return true;
    };


    let mut format_tags = tags.iter().filter_map(|tag| tag.strip_prefix("format:"));

    let Some(first_tag) = format_tags.next() else {
        return true;
    };

    requested_formats.iter().any(|format| {
        first_tag == format.as_str() || format_tags.clone().any(|tag| tag == format.as_str())
    })
}
