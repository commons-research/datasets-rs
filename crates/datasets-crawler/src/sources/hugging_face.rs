use std::time::{Duration, Instant};

use futures::{AsyncRead, Stream, TryStreamExt, future};
use hf_hub::{
    HFClient,
    progress::{DownloadEvent, ProgressEvent, ProgressHandler},
    repository::{DatasetInfo, RepoTreeEntry},
    split_id,
};
use tracing::info;

use super::super::{
    error::CrawlerError,
    progress::DownloadProgress,
    source::{DatasetSource, RepositorySummary, SourceArtifact},
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
        let client = HFClient::builder()
            .retry_base_delay(Duration::from_secs(30))
            .retry_max_attempts(6)
            .build()?;

        Ok(Self { client })
    }
}

impl DatasetSource for HuggingFaceSource {
    fn discover_datasets(
        &self,
    ) -> Result<impl Stream<Item = Result<RepositorySummary, CrawlerError>> + Send, CrawlerError>
    {
        let stream = self.client.list_datasets().full(true).send()?;

        let stream = stream
            .try_filter(|dataset| future::ready(is_eligible(dataset)))
            .map_ok(|dataset| dataset_summary(&dataset))
            .map_err(CrawlerError::from);

        Ok(stream)
    }

    async fn list_artifacts(&self, dataset_id: &str) -> Result<Vec<SourceArtifact>, CrawlerError> {
        let started = Instant::now();

        info!(repository = dataset_id, "starting repository tree listing");

        let (owner, name) = split_id(dataset_id);
        let dataset = self.client.dataset(owner, name);

        let entries = dataset.list_tree().recursive(true).expand(true).send()?;

        let mut entries_seen = 0usize;

        let artifacts: Vec<SourceArtifact> = entries
            .inspect_ok(|_| {
                entries_seen += 1;

                if entries_seen.is_multiple_of(1000) {
                    info!(
                        repository = dataset_id,
                        entries = entries_seen,
                        elapsed_secs = started.elapsed().as_secs(),
                        "gathering repository tree"
                    );
                }
            })
            .try_filter_map(|entry| {
                future::ready(Ok(match entry {
                    RepoTreeEntry::File { path, size, .. } => {
                        Some(SourceArtifact { path, size_bytes: size })
                    }
                    RepoTreeEntry::Directory { .. } => None,
                }))
            })
            .try_collect()
            .await?;

        info!(
            repository = dataset_id,
            entries = entries_seen,
            files = artifacts.len(),
            elapsed_secs = started.elapsed().as_secs(),
            "repository tree listing completed"
        );

        Ok(artifacts)
    }

    async fn open_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> Result<impl AsyncRead + Send + Unpin + 'a, CrawlerError> {
        let (owner, name) = split_id(dataset_id);
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

    fn provider(&self) -> &'static str {
        "hf"
    }
}

/// Returns whether chosen HF [`DatasetInfo`] is publicly accessible.
fn is_eligible(dataset: &DatasetInfo) -> bool {
    dataset.private != Some(true)
        && dataset.gated.as_ref().is_none_or(|value| value.as_bool() == Some(false))
}

/// Converts Hugging Face metadata into source-independent dataset metadata
fn dataset_summary(dataset: &DatasetInfo) -> RepositorySummary {
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

    RepositorySummary {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

    use futures::{StreamExt, TryStreamExt, stream};

    use super::*;

    #[tokio::test]
    async fn filters_restricted_datasets_before_limit() -> Result<(), Box<dyn std::error::Error>> {
        let fixtures = [
            r#"{"id":"test/a","private":false,"gated":false}"#,
            r#"{"id":"test/b","private":false,"gated":"auto"}"#,
            r#"{"id":"test/c","private":false,"gated":"manual"}"#,
            r#"{"id":"test/d","private":false,"gated":false}"#,
            r#"{"id":"test/e","private":false,"gated":false}"#,
        ];

        let datasets: Vec<DatasetInfo> =
            fixtures.iter().map(|json| serde_json::from_str(json)).collect::<Result<_, _>>()?;

        let stream = stream::iter(datasets.into_iter().map(Ok::<_, std::convert::Infallible>));

        let eligible: Vec<DatasetInfo> = stream
            .try_filter(|dataset| futures::future::ready(is_eligible(dataset)))
            .take(3)
            .try_collect()
            .await?;

        let ids: Vec<&str> = eligible.iter().map(|dataset| dataset.id.as_str()).collect();

        assert_eq!(ids, vec!["test/a", "test/d", "test/e"]);

        Ok(())
    }
}
