//! Sqlite database initialization for crawler results.
use std::path::Path;

use rusqlite::{Connection, Result};

use crate::CrawlerError;

/// Manages database storage of crawler results.
pub struct Database {
    connection: Connection,
}

impl Database {
    /// Opens the database and creates its tables if they do not exist.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, CrawlerError> {
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
            records_inspected   INTEGER,
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

        Ok(connection)
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
}
