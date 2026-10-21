use futures::TryStreamExt;
use hf_hub::{
    HFClient, HFResult,
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
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
    pub fn new() -> HFResult<Self> {
        Ok(Self { client: HFClient::new()? })
    }

    /// Searches HF for datasets matching provided query
    ///
    /// # Errors
    /// - Returns an error if the HF Dataset listing(s) cannot be retrieved
    pub async fn search(&self, query: &str, limit: usize) -> HFResult<Vec<DatasetInfo>> {
        self.client.list_datasets().search(query).limit(limit).send()?.try_collect().await
    }

    /// Lists entries contained within a Hugging Face dataset repository
    ///
    /// # Errors
    /// - Returns an error if repo tree cannot be retrieved
    pub async fn list_tree(&self, dataset_id: &str, limit: usize) -> HFResult<Vec<RepoTreeEntry>> {
        let (owner, name) = split_id(dataset_id);

        self.client
            .dataset(owner, name)
            .list_tree()
            .recursive(true)
            .expand(true)
            .limit(limit)
            .send()?
            .try_collect()
            .await
    }
}
