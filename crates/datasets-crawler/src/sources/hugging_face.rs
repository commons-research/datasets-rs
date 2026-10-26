use futures::TryStreamExt;
use hf_hub::{
    HFClient,
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
};
use tracing::debug;

use super::super::{
    error::CrawlerError,
    source::{DatasetSource, DatasetSummary, SourceArtifact},
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
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<DatasetSummary>, CrawlerError> {
        debug!(query = %query, limit, "querying Hugging Face dataset search");
        let datasets: Vec<DatasetInfo> =
            self
                .client
                .list_datasets()
                .maybe_limit(limit)
                .search(query)
                .send()?
                .try_collect()
            .await?;

        Ok(datasets
            .into_iter()
            .map(|dataset| {
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
                    url: format!("https://huggingface.co/datasets/{}", dataset.id),
                    id: dataset.id,
                    searchable_metadata,
                }
            })
            .collect())
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
