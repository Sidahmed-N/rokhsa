//! SQLite connection and migrations.

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::SqlitePool;

use crate::error::AppError;

pub mod candidates;
pub mod enrollments;
pub mod exams;
pub mod expenses;
pub mod health;
pub mod lessons;
pub mod payments;
pub mod reports;
pub mod school;

/// Open (creating if needed) the school's database and bring it up to date.
///
/// Pragma choices matter more than usual here, because these machines lose
/// power mid-operation:
///
/// * `journal_mode = WAL` — readers don't block the writer, so a report can run
///   while a payment is being saved.
/// * `synchronous = FULL` — durability over throughput. A lost receipt is far
///   worse than a slow one, and the write volume of one driving school is
///   trivial for any disk.
/// * `foreign_keys = ON` — SQLite leaves this OFF by default, which is a
///   classic source of silent orphan rows.
/// * `busy_timeout` — wait rather than fail if something else holds the lock.
pub async fn connect(path: &Path) -> Result<SqlitePool, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("cannot create data directory: {e}")))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(10));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(15))
        .connect_with(options)
        .await?;

    // Migrations are embedded in the binary at compile time, so a school's
    // install needs no migration files on disk.
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Integrity check, surfaced later in a health/diagnostics screen.
///
/// The sale is one-time, so every support call costs more than it earns — the
/// owner needs to be able to answer "is my data OK?" without phoning anyone.
pub async fn integrity_check(pool: &SqlitePool) -> Result<String, AppError> {
    let result: (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(pool)
        .await?;
    Ok(result.0)
}

#[cfg(test)]
pub(crate) async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("in-memory database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations apply cleanly");
    pool
}
