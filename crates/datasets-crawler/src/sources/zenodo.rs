use std::collections::VecDeque;

use futures::{AsyncRead, Stream, TryStreamExt, stream};
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
    fn discover_datasets(
        &self,
    ) -> Result<impl Stream<Item = Result<RepositorySummary, CrawlerError>> + Send, CrawlerError>
    {
        let repositories = stream::try_unfold(
            (self.client.clone(), 1_u32, VecDeque::<RepositorySummary>::new(), false),
            |(client, page_number, mut pending, finished)| {
                async move {
                    if let Some(repository) = pending.pop_front() {
                        return Ok::<_, ZenodoError>(Some((
                            repository,
                            (client, page_number, pending, finished),
                        )));
                    }

                    if finished {
                        return Ok(None);
                    }

                    let query = RecordQuery::builder()
                        .published()
                        .most_recent()
                        .page(page_number)
                        .size(PAGE_SIZE)
                        .build();

                    let page = client.search_records(&query).await?;

                    let finished = page.hits.len() < PAGE_SIZE as usize;

                    let mut pending =
                        page.hits.iter().map(dataset_summary).collect::<VecDeque<_>>();

                    let Some(repository) = pending.pop_front() else {
                        return Ok(None);
                    };

                    Ok(Some((repository, (client, page_number + 1, pending, finished))))
                }
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

        let files = self.client.list_record_files(record_id).await?;

        Ok(files
            .into_iter()
            .map(|file| SourceArtifact { path: file.key, size_bytes: file.size })
            .collect())
    }

    async fn open_artifact<'a>(
        &'a self,
        dataset_id: &'a str,
        artifact: &'a SourceArtifact,
    ) -> Result<impl AsyncRead + Send + Unpin + 'a, CrawlerError> {
        let record_id = parse_record_id(dataset_id)?;
        let selector = ArtifactSelector::file(record_id, artifact.path.clone());

        let download = self.client.open_artifact(&selector).await?;

        Ok(download.stream.map_err(std::io::Error::other).into_async_read())
    }

    fn provider(&self) -> &'static str {
        "zenodo"
    }
}

/// Converts a Zenodo record into metadata.
fn dataset_summary(record: &Record) -> RepositorySummary {
    let id = record.id.to_string();

    RepositorySummary { id: id.clone(), url: format!("https://zenodo.org/records/{id}") }
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
    use zenodo_rs::RecordId;

    use super::{CrawlerError, parse_record_id};

    #[test]
    fn parses_numeric_record_id() {
        assert!(matches!(parse_record_id("123"), Ok(RecordId(123))));
    }

    #[test]
    fn rejects_invalid_record_id() {
        assert!(matches!(parse_record_id("invalid"), Err(CrawlerError::InvalidZenodoRecordId(_))));
    }
}
