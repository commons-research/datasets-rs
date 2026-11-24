

use std::collections::VecDeque;
use futures::{stream, AsyncRead,Stream,TryStreamExt};
use zenodo_rs::{ArtifactSelector, Record, RecordId, RecordQuery, ZenodoClient, ZenodoError};

use super::super::{
    error::CrawlerError,
    source::{DatasetSource, RepositorySummary, SourceArtifact},
};

/// Number of records to request from Zenodo per search page.
const PAGE_SIZE: u32 = 20;

/// Source for discovering public datasets from Zenodo
pub struct ZenodoSource {
    /// Client used with Zenodo
    client: ZenodoClient,
}

impl ZenodoSource {
    /// Creates an anonymous client for public Zenodo records
    ///
    /// # Errors
    /// - Returns error if client fails to initialize
    pub fn new() -> Result<Self, CrawlerError> {
        Ok(Self { client: ZenodoClient::anonymous()? })
    }
}

impl DatasetSource for ZenodoSource {
    async fn discover_datasets<'a>(
        &'a self,
    ) -> Result<impl Stream<Item = Result<RepositorySummary, CrawlerError>> + Send, CrawlerError> {
        let repositories = stream::try_unfold((
            self.client.clone(),
            1_u32,
            VecDeque::<RepositorySummary>::new(),
            false,
        ), |(client, page_number, mut pending, finished)| async move {
            if let Some(repository) = pending.pop_front() {
                return Ok::<_, ZenodoError>(Seom((
                    repository,
                    (client, page_number, pending, finished),
                )));
            }
            if finished {
                return Ok(None);
            }

            let query = RecordQuery::builder().published().most_recent().page(page_number).size(PAGE_SIZE).build();

            let page = client.search_records(&query).await?;
            let finished = page.hits.len() < PAGE_SIZE as usize;

            let mut pending = page.hits.into_iter().map(dataset_summary).collect::<VecDeque<_>>();
 Ok(Some((
                    repository,
                    (client, page_number + 1, pending, finished),
                )))
            },
        )
        .map_err(CrawlerError::Zenodo);

        Ok(repositories)
    }

    async fn list_artifacts<'a>(
        &'a self,
        dataset_id: &'a str,
    ) -> Result<Vec<SourceArtifact>, CrawlerError> {
        let record_id = parse_record_id(dataset_id)?;
        debug!(
            dataset_id = %dataset_id,
            "requesting Zenodo record files"
        );
        let files = self.client.list_record_files(record_id).await?;

        Ok(files
            .into_iter()
            .map(|file| SourceArtifact { path: file.key, size_bytes: file.size })
            .collect())
    }

    async fn download_artifact(
        &self,
        dataset_id: &str,
        artifact: &SourceArtifact,
    ) -> Result<Vec<u8>, CrawlerError> {
        let record_id = parse_record_id(dataset_id)?;

        debug!(
            dataset_id = %dataset_id,
            artifact_path = %artifact.path,
            "requesting Zenodo artifact download"
        );

        // Use a temporary directory so the downloaded artifact is cleaned
        // up automatically after it has been read into memory.
        let temp_dir = tempfile::tempdir()?;
        let destination = temp_dir.path().join("downloaded-artifact");

        let progress =
            DownloadProgress::new(artifact.size_bytes, format!("{dataset_id}/{}", artifact.path));

        let download_result = self
            .client
            .download_record_file_by_key_to_path_with_progress(
                record_id,
                &artifact.path,
                &destination,
                progress.progress_bar(),
            )
            .await;

        match download_result {
            Ok(_) => {
                progress.finish_with_message(format!(
                    "Downloaded Zenodo record {dataset_id}/{}",
                    artifact.path
                ));
            }
            Err(error) => {
                progress.abandon_with_message(format!(
                    "Download failed: {dataset_id}/{}",
                    artifact.path
                ));
                return Err(error.into());
            }
        }

        let bytes = tokio::fs::read(&destination).await?;

        debug!(
            dataset_id = %dataset_id,
            artifact_path = %artifact.path,
            downloaded_bytes = bytes.len(),
            "loaded downloaded Zenodo artifact into memory"
        );

        Ok(bytes)
    }
    
    fn open_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> impl Future<Output = Result<impl futures::prelude::AsyncRead + Send + Unpin + 'a, CrawlerError>> + Send + 'a {
        todo!()
    }
    
    fn provider(&self) -> &'static str {
        todo!()
    }
}

/// Converts a Zenodo record into metadata.
fn dataset_summary(record: &Record) -> DatasetSummary {
    let id = record.id.to_string();
    let metadata = &record.metadata;

    let mut searchable_metadata = metadata.title.clone();

    if let Some(description) = &metadata.description_html {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(description);
    }

    if !metadata.keywords.is_empty() {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(&metadata.keywords.join(" "));
    }

    if !metadata.formats.is_empty() {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(&metadata.formats.join(" "));
    }

    if let Some(notes) = &metadata.notes {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(notes);
    }

    if let Some(journal_title) = &metadata.journal_title {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(journal_title);
    }

    if let Some(conference_title) = &metadata.conference_title {
        searchable_metadata.push(' ');
        searchable_metadata.push_str(conference_title);
    }

    DatasetSummary {
        id: id.clone(),
        url: format!("https://zenodo.org/records/{id}"),
        searchable_metadata,
    }
}

/// Checks whether a record's file metadata matches the requested formats.
///
/// Records without embedded file metadata are retained rather than rejected;
/// their actual files can be inspected by the crawler afterward.
fn record_matches_formats(record: &Record, formats: &[FileFormat]) -> bool {
    formats.is_empty()
        || record.files.is_empty()
        || record.files.iter().any(|file| file_matches_formats(&file.key, formats))
}

/// Checks a file's extension against the requested formats.
fn file_matches_formats(path: &str, formats: &[FileFormat]) -> bool {
    if formats.is_empty() {
        return true;
    }

    let Some(extension) = Path::new(path).extension().and_then(|value| value.to_str()) else {
        return false;
    };

    formats.iter().any(|format| extension.eq_ignore_ascii_case(format.as_str()))
}

/// Parses the numeric Zenodo record identifier used by this source.
///
/// # Errors
/// - Returns [`CrawlerError::InvalidZenodoRecordId`] if unable to parse id
fn parse_record_id(dataset_id: &str) -> Result<RecordId, CrawlerError> {
    dataset_id
        .parse::<u64>()
        .map(RecordId)
        .map_err(|_| CrawlerError::InvalidZenodoRecordId(dataset_id.to_owned()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn matches_requested_file_extensions() {
        assert!(file_matches_formats("data.csv", &[FileFormat::Csv]));
        assert!(file_matches_formats("data.json", &[FileFormat::Json]));
        assert!(file_matches_formats("data.CSV", &[FileFormat::Csv]));
    }

    #[test]
    fn rejects_unsupported_file_extensions() {
        assert!(!file_matches_formats("README", &[FileFormat::Csv, FileFormat::Json]));
    }
}
