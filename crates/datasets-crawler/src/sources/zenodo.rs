use std::path::Path;

use zenodo_rs::{Record, RecordId, RecordQuery, ZenodoClient};
use tracing::debug;

use super::super::{
    error::CrawlerError,
    source::{
        DatasetSource,
        DatasetSummary,
        FileFormat,
        SourceArtifact
    },
    progress::DownloadProgress
};

/// Source for discovering public datasets from Zenodo
pub struct ZenodoSource {
    /// Client used with Zenodo
    client: ZenodoClient
}

impl ZenodoSource {
    /// Creates an anonymous client for public Zenodo records
    /// 
    /// # Errors
    /// - Returns error if client fails to initialize
    pub fn new() -> Result<Self, CrawlerError> {
        Ok(Self {
            client: ZenodoClient::anonymous()?,
        })
    }
}

impl DatasetSource for ZenodoSource {
    async fn search<'a>(
        &'a self,
        query: &'a str,
        formats: &'a [FileFormat],
        limit: Option<usize>,
    ) -> Result<Vec<DatasetSummary>, CrawlerError>
    {
        const PAGE_SIZE: usize = 100;

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
                (maximum - datasets.len()).min(PAGE_SIZE)
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

            if records_on_page < page_size {
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
    ) -> Result<Vec<SourceArtifact>, CrawlerError>
    {
        let record_id = parse_record_id(dataset_id)?;
        debug!(
            dataset_id = %dataset_id,
            limit = ?limit,
            "requesting Zenodo record files"
        );
        let files = self.client.list_record_files(record_id).await?;

        let files = match limit {
            Some(limit) => files.into_iter().take(limit).collect(),
            None => files
        };
        Ok(files
            .into_iter()
            .map(|file| SourceArtifact {
                path: file.key,
                size_bytes: file.size,
            }).collect()
        )
    }
}