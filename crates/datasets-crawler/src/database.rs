//! Sqlite database initialization for crawler results.
use std::path::Path;

use rusqlite::{Connection, Transaction, params};

use super::{CrawlerError, Failure, RepositoryResult, artifact::ArtifactResult};

/// Manages database storage of crawler results.
pub struct Database {
    /// Sqlite connection used to persist crawl results.
    connection: Connection,
}

impl Database {
    /// Opens the database and creates its tables if they do not exist.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open_database(path: impl AsRef<Path>) -> Result<Self, CrawlerError> {
        let connection = Connection::open(path)?;

        connection.execute_batch(
            "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS crawl_run (
            id                  INTEGER PRIMARY KEY,
            started_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            finished_at         TEXT,
            datasets_crawled    INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS repository (
            id          INTEGER PRIMARY KEY,
            provider      TEXT NOT NULL,
            external_id   TEXT NOT NULL,
            url         TEXT NOT NULL,
            last_crawled_at TEXT,
            UNIQUE (provider, external_id)
        );

        CREATE TABLE IF NOT EXISTS artifact (
            id                  INTEGER PRIMARY KEY,
            repository_id       INTEGER NOT NULL REFERENCES repository(id),
            path                TEXT NOT NULL,
            format              TEXT NOT NULL,
            size_bytes          INTEGER NOT NULL,
            UNIQUE (repository_id, path)
        );

        CREATE TABLE IF NOT EXISTS classification (
            artifact_id     INTEGER NOT NULL REFERENCES artifact(id),
            content_type    TEXT NOT NULL,
            location        TEXT NOT NULL,
            inspected       INTEGER NOT NULL,
            valid           INTEGER NOT NULL,
            PRIMARY KEY (artifact_id, content_type, location)
        );

        CREATE TABLE IF NOT EXISTS failure (
            id              INTEGER PRIMARY KEY,
            repository_id   INTEGER NOT NULL REFERENCES repository(id),
            artifact_id     INTEGER REFERENCES artifact(id),
            stage           TEXT NOT NULL,
            message         TEXT NOT NULL,
            failed_at       TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        ",
        )?;

        Ok(Self { connection })
    }

    /// Starts a new crawl run and returns its database ID.
    ///
    /// # Errors
    /// - Returns an error if the run cannot be inserted.
    pub fn start_run(&self) -> Result<i64, CrawlerError> {
        self.connection.execute("INSERT INTO crawl_run DEFAULT VALUES", [])?;

        Ok(self.connection.last_insert_rowid())
    }

    /// Marks a crawl run as finished.
    ///
    /// # Errors
    /// - Returns an error if the run cannot be updated.
    pub fn finish_run(&self, run_id: i64) -> Result<(), CrawlerError> {
        self.connection.execute(
            "UPDATE crawl_run SET finished_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [run_id],
        )?;

        Ok(())
    }

    /// Verifies whether a given repository has already been visited.
    ///
    /// # Errors
    /// - Returns an error when failing to work with database.
    pub fn repository_is_crawled(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<bool, CrawlerError> {
        let crawled = self.connection.query_row(
            "
            SELECT EXISTS (
                SELECT 1 
                FROM repository
                WHERE provider = ?1
                AND external_id = ?2
                AND last_crawled_at IS NOT NULL
            )
        ",
            [provider, external_id],
            |row| row.get(0),
        )?;

        Ok(crawled)
    }

    /// Saves repository crawl results to the database.
    ///
    /// # Errors
    /// - Returns an error if a database operation fails.
    pub fn save_repository(
        &mut self,
        run_id: i64,
        provider: &str,
        result: &RepositoryResult,
    ) -> Result<(), CrawlerError> {
        let tx = self.connection.transaction()?;

        let repository_id = insert_and_return_repository_id(
            &tx,
            provider,
            &result.repository.id,
            &result.repository.url,
        )?;

        for artifact in &result.artifacts {
            save_artifact(&tx, repository_id, artifact)?;
        }

        for failure in &result.failures {
            insert_failure(&tx, repository_id, None, failure)?;
        }

        let completed = result.failures.is_empty()
            && result.artifacts.iter().all(|artifact| artifact.failure.is_none());

        if completed {
            tx.execute(
                "
            UPDATE repository
            SET last_crawled_at = CURRENT_TIMESTAMP
            WHERE id = ?1
            ",
                [repository_id],
            )?;
        }

        // Count this repository in the current crawl run.
        tx.execute(
            "
        UPDATE crawl_run
        SET datasets_crawled = datasets_crawled + 1
        WHERE id = ?1
        ",
            [run_id],
        )?;

        tx.commit()?;

        Ok(())
    }
}

/// Saves an artifact and its classification results.
///
/// # Errors
/// - Returns an error if a database operation fails.
fn save_artifact(
    tx: &Transaction<'_>,
    repository_id: i64,
    artifact: &ArtifactResult,
) -> Result<(), CrawlerError> {
    let artifact_id = insert_and_return_artifact_id(
        tx,
        repository_id,
        &artifact.path,
        &artifact.format.to_string(),
        artifact.size_bytes,
    )?;

    tx.execute("DELETE FROM classification WHERE artifact_id = ?1", [artifact_id])?;

    for classification in
        artifact.classifications.iter().filter(|classification| classification.valid() > 0)
    {
        tx.execute(
            "
            INSERT INTO classification
                (artifact_id, content_type, location, inspected, valid)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                artifact_id,
                classification.content_type().to_string(),
                classification.location(),
                i64::try_from(classification.inspected())?,
                i64::try_from(classification.valid())?,
            ],
        )?;
    }

    if let Some(failure) = &artifact.failure {
        insert_failure(tx, repository_id, Some(artifact_id), failure)?;
    }

    Ok(())
}

/// Records a repository-level or artifact-level failure.
///
/// # Errors
/// - Returns an error if the insertion fails.
fn insert_failure(
    tx: &Transaction<'_>,
    repository_id: i64,
    artifact_id: Option<i64>,
    failure: &Failure,
) -> Result<(), CrawlerError> {
    tx.execute(
        "
        INSERT INTO failure
            (repository_id, artifact_id, stage, message)
        VALUES (?1, ?2, ?3, ?4)
        ",
        params![repository_id, artifact_id, failure.stage.to_string(), &failure.message,],
    )?;

    Ok(())
}

/// Creates and retrieves the repository's ID
///
/// # Errors
/// - Returns a [`CrawlerError`] if insertion fails.
fn insert_and_return_repository_id(
    tx: &Transaction<'_>,
    provider: &str,
    repo_id: &str,
    url: &str,
) -> Result<i64, CrawlerError> {
    let repository_id: i64 = tx.query_row(
        "
        INSERT INTO repository (provider, external_id, url)
        VALUES (?1, ?2, ?3)
        ON CONFLICT (provider, external_id)
        DO UPDATE SET url = excluded.url
        RETURNING id
        ",
        params![provider, &repo_id, &url],
        |row| row.get(0),
    )?;
    Ok(repository_id)
}

/// Creates and retrieves the artifact's ID
///
/// # Errors
/// - Returns [`CrawlerError`] if insertion fails.
fn insert_and_return_artifact_id(
    tx: &Transaction<'_>,
    repo_id: i64,
    path: &str,
    format: &str,
    size_bytes: u64,
) -> Result<i64, CrawlerError> {
    let artifact_id: i64 = tx.query_row(
        "
            INSERT INTO artifact (repository_id, path, format, size_bytes)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT (repository_id, path)
            DO UPDATE SET
                format = excluded.format,
                size_bytes = excluded.size_bytes
            RETURNING id
            ",
        params![repo_id, path, format, i64::try_from(size_bytes)?,],
        |row| row.get(0),
    )?;
    Ok(artifact_id)
}
