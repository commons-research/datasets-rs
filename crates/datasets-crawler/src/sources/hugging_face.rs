use futures::{AsyncRead, StreamExt, TryStreamExt};
use hf_hub::{
    HFClient,
    progress::{DownloadEvent, ProgressEvent, ProgressHandler},
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
};
use tracing::debug;

use super::super::{
    error::CrawlerError,
    source::{DatasetSource, DatasetSummary, SourceArtifact},
};
use crate::progress::DownloadProgress;

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
    async fn discover_datasets(
        &self,
        query: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<DatasetSummary>, CrawlerError> {
        debug!(
            query = ?query,
            limit = ?limit,
            "discovering Hugging Face datasets"
        );

        let stream = self.client.list_datasets().maybe_search(query).full(true).send()?;

        let datasets: Vec<DatasetInfo> = match limit {
            Some(limit) => stream.take(limit).try_collect().await?,
            None => stream.try_collect().await?,
        };

        debug!(
            query = ?query,
            discovered_datasets = datasets.len(),
            "finished Hugging Face dataset discovery"
        );

        Ok(datasets.iter().map(dataset_summary).collect())
    }

    async fn list_artifacts(&self, dataset_id: &str) -> Result<Vec<SourceArtifact>, CrawlerError> {
        let (owner, name) = split_id(dataset_id);

        let entries: Vec<RepoTreeEntry> = self
            .client
            .dataset(owner, name)
            .list_tree()
            .recursive(true)
            .expand(true)
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

    async fn open_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> Result<impl AsyncRead + Send + Unpin + 'a, CrawlerError> {
        let (owner, name) = split_id(dataset_id);
        debug!(
            dataset_id = %dataset_id,
            artifact_path = %artifact.path,
            "opening Hugging Face artifact stream"
        );
        let progress_message = format!("{dataset_id}/{}", artifact.path);
        let progress = DownloadProgress::new(artifact.size_bytes, progress_message);

        let download_result = self
            .client
            .dataset(owner, name)
            .download_file_stream()
            .filename(artifact.path.clone())
            .progress(HuggingFaceProgressHandler::new(progress.clone(), artifact.path.clone()))
            .send()
            .await;

        let (_, stream) = match download_result {
            Ok(result) => result,
            Err(error) => {
                progress
                    .abandon_with_message(format!("Failed to open {dataset_id}/{}", artifact.path));
                return Err(error.into());
            }
        };
        let failed_progress = progress.clone();
        let failed_dataset_id = dataset_id.to_owned();
        let failed_artifact_path = artifact.path.clone();
        let reader = stream
            .map_err(move |error| {
                failed_progress.abandon_with_message(format!(
                    "failed reading {failed_dataset_id}/{failed_artifact_path}"
                ));
                std::io::Error::other(error)
            })
            .into_async_read();

        Ok(reader)
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

/// Adapts HF download events to shared progress display
struct HuggingFaceProgressHandler {
    /// Shared terminal progress display
    progress: DownloadProgress,
    /// Repository path of the artifact downloading
    artifact_path: String,
}

impl HuggingFaceProgressHandler {
    /// Creates a handler for artifact download
    #[must_use]
    const fn new(progress: DownloadProgress, artifact_path: String) -> Self {
        Self { progress, artifact_path }
    }
}

impl ProgressHandler for HuggingFaceProgressHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        let ProgressEvent::Download(event) = event else {
            return;
        };

        match event {
            DownloadEvent::Start { total_bytes, .. } => {
                self.progress.set_total_bytes(*total_bytes);
            }

            DownloadEvent::Progress { files } => {
                if let Some(file) = files.iter().find(|file| file.filename == self.artifact_path) {
                    self.progress.set_total_bytes(file.total_bytes);
                    self.progress.set_position(file.bytes_completed);
                }
            }

            DownloadEvent::AggregateProgress { bytes_completed, total_bytes, .. } => {
                self.progress.set_total_bytes(*total_bytes);
                self.progress.set_position(*bytes_completed);
            }

            DownloadEvent::Complete => {
                self.progress.finish_with_message(format!("Download {}", self.artifact_path));
            }
        }
    }
}
