use std::error::Error;

use futures::TryStreamExt;
use hf_hub::{
    HFClient, HFResult,
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
};

use super::super::source::{DatasetSource, DatasetSummary, SourceArtifact};

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
    pub fn new() -> HFResult<Self> {
        Ok(Self { client: HFClient::new()? })
    }
}

impl DatasetSource for HuggingFaceSource {
    type Error = Box<dyn Error + Send + Sync>;

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<DatasetSummary>, Self::Error> {
        let datasets: Vec<DatasetInfo> =
            self.client.list_datasets().search(query).limit(limit).send()?.try_collect().await?;

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
        limit: usize,
    ) -> Result<Vec<SourceArtifact>, Self::Error> {
        let (owner, name) = split_id(dataset_id);

        let entries: Vec<RepoTreeEntry> = self
            .client
            .dataset(owner, name)
            .list_tree()
            .recursive(true)
            .expand(true)
            .limit(limit)
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
                    _ => None,
                }
            })
            .collect())
    }

    async fn download_artifact(
        &self,
        dataset_id: &str,
        artifact: &SourceArtifact,
    ) -> Result<Vec<u8>, Self::Error> {
        let (owner, name) = split_id(dataset_id);

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
