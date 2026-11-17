//! Sqlite database initialization for crawler results.
use std::path::Path;

use rusqlite::{Connection, Result};

/// Opens the database and creates its tables if they do not exist.
///
/// # Errors
/// Returns an error if the database cannot be opened or initialized.
pub fn open_database(path: impl AsRef<Path>) -> Result<Connection> {
    let connection = Connection::open(path)?;

    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS repository (
            id          INTEGER PRIMARY KEY,
            source      TEXT NOT NULL,
            source_id   TEXT NOT NULL,
            url         TEXT NOT NULL,
            UNIQUE (source, source_id)
        );

        CREATE TABLE IF NOT EXISTS artifact (
            id                  INTEGER PRIMARY KEY,
            repository_id       INTEGER NOT NULL REFERENCES repository(id),
            path                TEXT NOT NULL,
            format              TEXT NOT NULL,
            size_bytes          INTEGER NOT NULL,
            last_crawled_at     TEXT,
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
