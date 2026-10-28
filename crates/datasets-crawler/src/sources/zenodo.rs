use std::path::Path;

use tracing::debug;
use zenodo_rs::{Record, RecordId, RecordQuery, ZenodoClient};

use super::super::{
    error::CrawlerError,
    progress::DownloadProgress,
    source::{DatasetSource, DatasetSummary, FileFormat, SourceArtifact},
};

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
    async fn search<'a>(
        &'a self,
        query: &'a str,
        formats: &'a [FileFormat],
        limit: Option<usize>,
    ) -> Result<Vec<DatasetSummary>, CrawlerError> {
        const PAGE_SIZE: u32 = 100;

        debug!(
            query = %query,
            formats = ?formats,
            limit = ?limit,
            "querying Zenodo record search"
        );

        let mut datasets = Vec::new();
        let mut page_number = 1;

        loop {
            if limit.is_some_and(|maximum| datasets.len() >= maximum) {
                break;
            }

            let page_size = limit.map_or(PAGE_SIZE, |maximum| {
                let remaining = maximum.saturating_sub(datasets.len());
                u32::try_from(remaining).map_or(PAGE_SIZE, |remaining| remaining.min(PAGE_SIZE))
            });

            let record_query = RecordQuery::builder()
                .query(query)
                .published()
                .most_recent()
                .page(page_number)
                .size(page_size)
                .build();

            let page = self.client.search_records(&record_query).await?;
            let records = page.hits;
            let records_on_page = records.len();

            if records.is_empty() {
                break;
            }

            for record in records {
                if record_matches_formats(&record, formats) {
                    datasets.push(dataset_summary(&record));
                }

                if limit.is_some_and(|maximum| datasets.len() >= maximum) {
                    break;
                }
            }

            if limit.is_some_and(|maximum| datasets.len() >= maximum) {
                break;
            }

            if u32::try_from(records_on_page).is_ok_and(|count| count < page_size) {
                break;
            }

            page_number += 1;
        }

        debug!(
            query = %query,
            matching_datasets = datasets.len(),
            "finished Zenodo record search"
        );

        Ok(datasets)
    }

    async fn list_artifacts<'a>(
        &'a self,
        dataset_id: &'a str,
        limit: Option<usize>,
    ) -> Result<Vec<SourceArtifact>, CrawlerError> {
        let record_id = parse_record_id(dataset_id)?;
        debug!(
            dataset_id = %dataset_id,
            limit = ?limit,
            "requesting Zenodo record files"
        );
        let files = self.client.list_record_files(record_id).await?;

        let files = match limit {
            Some(limit) => files.into_iter().take(limit).collect(),
            None => files,
        };
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
