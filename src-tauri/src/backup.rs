//! Backup and restore.
//!
//! The feature where a bug means a school loses everything, so two things are
//! done deliberately rather than conveniently:
//!
//! 1. **Copies are made with `VACUUM INTO`, never by copying the file.**
//!    The database runs in WAL mode, so the `.db` file on its own is an
//!    incomplete picture — recent writes live in the `-wal` sidecar. Copying
//!    just the `.db` yields a backup that is silently missing the last day's
//!    payments. `VACUUM INTO` asks SQLite itself for a consistent, compact
//!    snapshot while the database is in use.
//!
//! 2. **Every copy is verified before it is called a backup.** It is opened,
//!    integrity-checked, and its row counts compared against the source. An
//!    unverified backup is just a file, and a school only finds out it was
//!    worthless on the day they need it.
//!
//! Restore is staged, not immediate: the chosen file is validated and parked,
//! and swapped in at the next startup before anything opens the database.
//! Swapping a file out from under live connections is how you get a corrupt
//! database on top of the data loss you were already recovering from.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection, SqlitePool};

use crate::error::AppError;

/// How many automatic backups to keep.
pub const KEEP_AUTOMATIC: usize = 10;

/// A backup is made at startup if the newest one is older than this.
pub const AUTO_BACKUP_AFTER_HOURS: i64 = 12;

const PENDING_RESTORE: &str = "pending-restore.db";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub file_name: String,
    /// Taken from the file's modification time, in UTC.
    pub created_at: String,
    pub size_bytes: i64,
    pub candidates: i64,
    pub payments: i64,
    /// Highest migration applied to that file.
    pub schema_version: i64,
}

/// Highest migration this binary knows about.
///
/// A backup from a NEWER version of the app cannot be restored into this one:
/// migrations only go forward, so the database would contain tables and
/// columns this build has never heard of. sqlx would fail at startup with a
/// message about a missing migration, which looks like a corrupt file rather
/// than an out-of-date app.
fn known_schema_version() -> i64 {
    sqlx::migrate!("./migrations")
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0)
}

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn io_err(context: &str, e: std::io::Error) -> AppError {
    AppError::Internal(format!("{context}: {e}"))
}

/// Automatic backups are named `rokhsa-<timestamp>.db`.
const PREFIX: &str = "rokhsa-";

/// What they were called before the app was renamed. Still recognised, so a
/// school that updates keeps its existing backups in the rotation and in the
/// "is one due" check — otherwise the old files would pile up forever and the
/// first start after the update would think no backup had ever been made.
const OLD_PREFIX: &str = "tariqi-";

fn is_backup_name(name: &str) -> bool {
    (name.starts_with(PREFIX) || name.starts_with(OLD_PREFIX)) && name.ends_with(".db")
}

fn timestamp_name(now: DateTime<Utc>) -> String {
    format!("{PREFIX}{}.db", now.format("%Y%m%d-%H%M%S"))
}

/// Open a database file read-only, without creating it.
async fn open_readonly(path: &Path) -> Result<SqliteConnection, AppError> {
    if !path.exists() {
        return Err(AppError::NotFound);
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(true);
    Ok(SqliteConnection::connect_with(&options).await?)
}

/// Validate a file and read what it contains.
///
/// Used both to verify a freshly written backup and to inspect a file the user
/// has chosen to restore from.
pub async fn inspect(path: &Path) -> Result<BackupInfo, AppError> {
    let mut conn = open_readonly(path).await?;

    let integrity: (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&mut conn)
        .await?;
    if integrity.0 != "ok" {
        return Err(AppError::validation("backup.error.corrupt"));
    }

    // A valid SQLite file that is not one of ours must not be offered as a
    // restore candidate — restoring it would wipe the school's data with
    // something unrelated.
    let candidates: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM candidates")
        .fetch_one(&mut conn)
        .await
        .map_err(|_| AppError::validation("backup.error.notTariqi"))?;

    let payments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payments")
        .fetch_one(&mut conn)
        .await
        .map_err(|_| AppError::validation("backup.error.notTariqi"))?;

    let schema: (Option<i64>,) = sqlx::query_as("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_one(&mut conn)
        .await
        .map_err(|_| AppError::validation("backup.error.notTariqi"))?;
    let schema_version = schema.0.unwrap_or(0);

    conn.close().await.ok();

    // Refuse a file from a newer build rather than let it fail confusingly at
    // the next startup, after the live database has already been swapped out.
    if schema_version > known_schema_version() {
        return Err(AppError::validation("backup.error.newerVersion"));
    }

    let meta = std::fs::metadata(path).map_err(|e| io_err("reading backup metadata", e))?;
    let created: DateTime<Utc> = meta
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());

    Ok(BackupInfo {
        path: path.to_string_lossy().to_string(),
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        created_at: created.to_rfc3339(),
        size_bytes: meta.len() as i64,
        candidates: candidates.0,
        payments: payments.0,
        schema_version,
    })
}

/// The name a file is written under before it is known to be good.
///
/// Ends in `.partial`, not `.db`, so nothing that lists or rotates backups
/// ever mistakes one for a backup.
fn partial_path(dest: &Path) -> PathBuf {
    PathBuf::from(format!("{}.partial", dest.to_string_lossy()))
}

/// Write a verified backup to `dest`.
///
/// The copy is written under a temporary name, verified, and only then
/// renamed onto `dest`. So a file at `dest` always means a good backup, and
/// one that was already there — the morning's USB backup, being overwritten
/// in the afternoon — survives until its replacement is proven good. The
/// first version deleted it before writing: a stick pulled out mid-write
/// left the school with neither copy.
pub async fn write_backup(pool: &SqlitePool, dest: &Path) -> Result<BackupInfo, AppError> {
    if let Some(parent) = dest.parent() {
        // A removed USB stick fails here rather than later, with a message
        // about the destination rather than about the database.
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("[tariqi] cannot reach backup folder {}: {e}", parent.display());
            return Err(AppError::validation("backup.error.destinationUnreachable"));
        }
    }

    // VACUUM INTO refuses to overwrite. A leftover from an earlier write that
    // was cut off is ours to clear; the real `dest` is not touched.
    let partial = partial_path(dest);
    if partial.exists() {
        std::fs::remove_file(&partial).map_err(|e| io_err("clearing an unfinished backup", e))?;
    }

    let source_counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM candidates), (SELECT COUNT(*) FROM payments)",
    )
    .fetch_one(pool)
    .await?;

    // A USB stick pulled mid-write, or a full disk, fails here. Clean up
    // whatever landed and say what actually went wrong — "database error" sends
    // the owner to the wrong problem entirely.
    if let Err(e) = sqlx::query("VACUUM INTO ?1")
        .bind(partial.to_string_lossy().to_string())
        .execute(pool)
        .await
    {
        std::fs::remove_file(&partial).ok();
        eprintln!("[tariqi] backup write to {} failed: {e}", partial.display());
        return Err(AppError::validation("backup.error.writeFailed"));
    }

    let mut info = match inspect(&partial).await {
        Ok(info) => info,
        Err(e) => {
            std::fs::remove_file(&partial).ok();
            return Err(e);
        }
    };

    if info.candidates != source_counts.0 || info.payments != source_counts.1 {
        std::fs::remove_file(&partial).ok();
        return Err(AppError::validation("backup.error.verificationFailed"));
    }

    // Proven good: now, and only now, it takes the real name — replacing an
    // older file of the same name in one step.
    if let Err(e) = retry(|| std::fs::rename(&partial, dest)) {
        std::fs::remove_file(&partial).ok();
        eprintln!("[tariqi] could not put the backup in place at {}: {e}", dest.display());
        return Err(AppError::validation("backup.error.writeFailed"));
    }

    info.path = dest.to_string_lossy().to_string();
    info.file_name = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(info)
}

/// Make an automatic backup in the app's own folder and prune old ones.
pub async fn auto_backup(pool: &SqlitePool, data_dir: &Path) -> Result<BackupInfo, AppError> {
    let dir = backups_dir(data_dir);
    let dest = dir.join(timestamp_name(Utc::now()));
    let info = write_backup(pool, &dest).await?;
    prune(&dir, KEEP_AUTOMATIC, Some(&dest))?;
    Ok(info)
}

/// Keep only the newest `keep` automatic backups, counting `protect`.
///
/// Only files matching our own naming are considered, so a copy the owner
/// dropped in the folder by hand is never deleted.
///
/// `protect` is the backup just written, and it is never deleted. "Newest"
/// is judged by the file's date, and on a PC whose clock is wrong — a dead
/// clock battery puts it back in 2000 — the backup made a second ago has the
/// OLDEST date in the folder. The first version deleted it straight after
/// verifying it, every time, while the screen reported success.
fn prune(dir: &Path, keep: usize, protect: Option<&Path>) -> Result<(), AppError> {
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_backup_name(&name) || protect == Some(path.as_path()) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            files.push((path, modified));
        }
    }

    // The protected file takes one of the `keep` places.
    let others = if protect.is_some() { keep.saturating_sub(1) } else { keep };
    files.sort_by(|a, b| b.1.cmp(&a.1));
    for (path, _) in files.into_iter().skip(others) {
        std::fs::remove_file(path).ok();
    }
    Ok(())
}

/// Whether enough time has passed to warrant another automatic backup.
///
/// A newest backup dated in the FUTURE means the clock has gone backwards
/// since it was made. Then nothing about "how long ago" can be trusted, so a
/// backup is due: one backup too many costs a few seconds, and the first
/// version, which compared the dates anyway, made none at all on such a PC.
pub fn is_auto_backup_due(data_dir: &Path, now: DateTime<Utc>) -> bool {
    let dir = backups_dir(data_dir);
    let newest = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| is_backup_name(&e.file_name().to_string_lossy()))
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .max();

    match newest {
        None => true,
        Some(time) => {
            let last: DateTime<Utc> = time.into();
            let age = now - last;
            // A few minutes ahead is ordinary clock drift; more is a clock
            // that was moved back.
            age < chrono::Duration::minutes(-10) || age.num_hours() >= AUTO_BACKUP_AFTER_HOURS
        }
    }
}

/// Every backup in the app's folder, newest first.
pub async fn list(data_dir: &Path) -> Result<Vec<BackupInfo>, AppError> {
    let dir = backups_dir(data_dir);
    let mut out = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(out),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        // A file that will not open is reported by being absent from the list
        // rather than breaking the screen.
        if let Ok(info) = inspect(&path).await {
            out.push(info);
        }
    }

    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// Validate a chosen file and park it to be restored at the next startup.
///
/// Nothing is overwritten here. The swap happens in `apply_pending_restore`,
/// before any connection is opened.
///
/// The copy is made under a temporary name, flushed to disk and checked
/// again before it becomes the pending restore. Validating the SOURCE is not
/// enough: a power cut while a large file comes off a USB stick leaves a
/// truncated copy, and the first version would have swapped that in at the
/// next start in place of the school's database.
pub async fn stage_restore(data_dir: &Path, source: &Path) -> Result<BackupInfo, AppError> {
    let info = inspect(source).await?;

    std::fs::create_dir_all(data_dir).map_err(|e| io_err("creating data folder", e))?;
    let staged = data_dir.join(PENDING_RESTORE);
    let partial = partial_path(&staged);

    // Flushing needs a handle opened for writing on Windows (FlushFileBuffers);
    // a read-only one is refused with "access denied".
    let copied = std::fs::copy(source, &partial)
        .and_then(|_| std::fs::File::options().write(true).open(&partial)?.sync_all());
    if let Err(e) = copied {
        std::fs::remove_file(&partial).ok();
        return Err(io_err("staging the restore", e));
    }
    if let Err(e) = inspect(&partial).await {
        std::fs::remove_file(&partial).ok();
        return Err(e);
    }
    retry(|| std::fs::rename(&partial, &staged)).map_err(|e| {
        std::fs::remove_file(&partial).ok();
        io_err("staging the restore", e)
    })?;

    Ok(info)
}

pub fn pending_restore(data_dir: &Path) -> Option<PathBuf> {
    let staged = data_dir.join(PENDING_RESTORE);
    staged.exists().then_some(staged)
}

pub fn cancel_pending_restore(data_dir: &Path) -> Result<(), AppError> {
    let staged = data_dir.join(PENDING_RESTORE);
    if staged.exists() {
        std::fs::remove_file(&staged).map_err(|e| io_err("cancelling the restore", e))?;
    }
    Ok(())
}

fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", db.to_string_lossy()))
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Retry a file operation for up to two seconds.
///
/// On Windows a file SQLite has just closed can stay locked for a moment
/// after the connection reports closed, and a delete or rename in that window
/// fails. Seen in the tests: the restore's own checkpoint connection still
/// held the `-wal` when the old code deleted it, the delete failed silently,
/// and the sidecar stayed next to the restored file. Startup only, so a
/// blocking sleep is fine.
fn retry<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut attempts = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(e) if attempts >= 40 => return Err(e),
            Err(_) => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

/// Swap a staged restore into place. Call at startup, BEFORE opening the pool.
///
/// The database being replaced is kept, not deleted: if the restore turns out
/// to be the wrong file, the school has not lost today's work on top of it.
///
/// "Kept" has to mean COMPLETE. The app never closes its pool cleanly, so the
/// last payments may still be in the `-wal` sidecar rather than the `.db`
/// file; the first version moved the `.db` and deleted the sidecar, and the
/// copy it kept was missing them. Now the WAL is first folded into the file
/// (a checkpoint, through `db::connect` as CLAUDE.md requires), and whatever
/// is still in it is moved alongside the kept copy rather than deleted.
///
/// A staged file that fails its check is set aside and the live database is
/// left alone: the app starts on the data it had, never on a broken file.
pub async fn apply_pending_restore(
    data_dir: &Path,
    db_path: &Path,
) -> Result<Option<String>, AppError> {
    let Some(staged) = pending_restore(data_dir) else {
        return Ok(None);
    };

    if let Err(e) = inspect(&staged).await {
        let rejected = data_dir.join(format!(
            "rejected-restore-{}.db",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        eprintln!("[tariqi] staged restore failed its check ({e}); kept aside as {}", rejected.display());
        // Retried: the check just above has only now let go of the file.
        retry(|| std::fs::rename(&staged, &rejected))
            .or_else(|_| retry(|| remove_if_present(&staged)))
            .ok();
        return Ok(None);
    }

    if db_path.exists() {
        // Fold the WAL into the file. Best effort: a database damaged enough
        // to be the reason for this restore may refuse, and then the sidecar
        // is carried along below instead.
        match crate::db::connect(db_path).await {
            Ok(pool) => {
                if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(&pool).await {
                    eprintln!("[tariqi] checkpoint before restore failed: {e}");
                }
                pool.close().await;
            }
            Err(e) => eprintln!("[tariqi] could not open the database being replaced: {e}"),
        }

        let dir = backups_dir(data_dir);
        std::fs::create_dir_all(&dir).map_err(|e| io_err("creating backup folder", e))?;
        let kept = dir.join(format!(
            "replaced-{}.db",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let kept_wal = sidecar(&kept, "-wal");

        // The sidecars are dealt with FIRST, while the database is still in
        // place, so that if Windows will not let go of them the restore can
        // be put off to the next start with nothing changed.
        //
        // Anything the checkpoint could not fold in goes WITH the kept copy:
        // opened later as `replaced-….db`, SQLite finds `replaced-….db-wal`
        // and replays it. Nothing may stay next to the restored file: SQLite
        // would replay it against that file and corrupt it. The shared-memory
        // file is rebuilt on open, so it is simply removed.
        let wal = sidecar(db_path, "-wal");
        let shm = sidecar(db_path, "-shm");
        let wal_has_data = std::fs::metadata(&wal).map(|m| m.len() > 0).unwrap_or(false);
        let cleared = (if wal_has_data {
            retry(|| std::fs::rename(&wal, &kept_wal)).or_else(|_| {
                std::fs::copy(&wal, &kept_wal)?;
                retry(|| remove_if_present(&wal))
            })
        } else {
            retry(|| remove_if_present(&wal))
        })
        .and_then(|_| retry(|| remove_if_present(&shm)));

        if let Err(e) = cleared {
            // The live database has not been touched: postpone, never half-do.
            std::fs::remove_file(&kept_wal).ok();
            eprintln!("[tariqi] restore postponed to the next start, sidecars still in use: {e}");
            return Ok(None);
        }

        retry(|| std::fs::rename(db_path, &kept))
            .or_else(|_| std::fs::copy(db_path, &kept).map(|_| ()))
            .map_err(|e| io_err("setting the current database aside", e))?;
    }

    std::fs::rename(&staged, db_path)
        .or_else(|_| std::fs::copy(&staged, db_path).map(|_| ()))
        .map_err(|e| io_err("putting the restored database in place", e))?;
    std::fs::remove_file(&staged).ok();

    Ok(Some(db_path.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::candidates;
    use crate::domain::candidate::CandidateInput;

    async fn seeded_db(dir: &Path, names: &[&str]) -> (PathBuf, SqlitePool) {
        let path = dir.join("tariqi.db");
        let pool = db::connect(&path).await.unwrap();
        for name in names {
            candidates::create(
                &pool,
                &CandidateInput {
                    last_name: (*name).into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        (path, pool)
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tariqi-test-{tag}-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn a_backup_contains_everything_the_live_database_has() {
        let dir = temp_dir("contents");
        let (_, pool) = seeded_db(&dir, &["BENALI", "KADRI", "SAIDI"]).await;

        let dest = dir.join("out.db");
        let info = write_backup(&pool, &dest).await.unwrap();

        assert_eq!(info.candidates, 3);
        assert!(info.size_bytes > 0);
        assert!(dest.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_backup_includes_writes_still_sitting_in_the_wal() {
        // The reason VACUUM INTO is used instead of copying the file: in WAL
        // mode a recent write lives in the sidecar, and a plain file copy
        // would silently lose it.
        let dir = temp_dir("wal");
        let (db_path, pool) = seeded_db(&dir, &["BENALI"]).await;

        candidates::create(
            &pool,
            &CandidateInput { last_name: "JUST_WRITTEN".into(), ..Default::default() },
        )
        .await
        .unwrap();

        let dest = dir.join("out.db");
        let info = write_backup(&pool, &dest).await.unwrap();
        assert_eq!(info.candidates, 2, "the newest write must be in the backup");

        assert!(db_path.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_backup_can_be_read_back_as_a_working_database() {
        let dir = temp_dir("readback");
        let (_, pool) = seeded_db(&dir, &["BENALI", "KADRI"]).await;
        let dest = dir.join("out.db");
        write_backup(&pool, &dest).await.unwrap();

        let restored = db::connect(&dest).await.unwrap();
        assert_eq!(candidates::count(&restored, "").await.unwrap(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_truncated_file_is_rejected_rather_than_offered_as_a_backup() {
        let dir = temp_dir("corrupt");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        let dest = dir.join("out.db");
        write_backup(&pool, &dest).await.unwrap();

        // Damage it the way a half-finished USB write would.
        let bytes = std::fs::read(&dest).unwrap();
        std::fs::write(&dest, &bytes[..bytes.len() / 3]).unwrap();

        assert!(inspect(&dest).await.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_unrelated_sqlite_file_is_not_accepted_for_restore() {
        let dir = temp_dir("foreign");
        let foreign = dir.join("someone-elses.db");
        let options = SqliteConnectOptions::new()
            .filename(&foreign)
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&options).await.unwrap();
        sqlx::query("CREATE TABLE notes (id INTEGER PRIMARY KEY)")
            .execute(&mut conn)
            .await
            .unwrap();
        conn.close().await.ok();

        assert!(inspect(&foreign).await.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_backup_from_a_newer_version_of_the_app_is_refused() {
        // Otherwise it is accepted, swapped in at startup, and THEN fails —
        // after the working database has already been moved aside.
        let dir = temp_dir("newer");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        let dest = dir.join("from-the-future.db");
        write_backup(&pool, &dest).await.unwrap();
        pool.close().await;

        // Pretend a later release added migration 9999.
        let options = SqliteConnectOptions::new().filename(&dest);
        let mut conn = SqliteConnection::connect_with(&options).await.unwrap();
        sqlx::query(
            "INSERT INTO _sqlx_migrations
               (version, description, installed_on, success, checksum, execution_time)
             VALUES (9999, 'from a newer build', datetime('now'), 1, X'00', 0)",
        )
        .execute(&mut conn)
        .await
        .unwrap();
        conn.close().await.ok();

        let result = inspect(&dest).await;
        assert!(
            matches!(result, Err(AppError::Validation(code, _)) if code.contains("newerVersion")),
            "expected a newer-version refusal, got {result:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_backup_reports_the_schema_it_was_made_with() {
        let dir = temp_dir("schema");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        let dest = dir.join("out.db");
        let info = write_backup(&pool, &dest).await.unwrap();
        assert!(info.schema_version > 0);
        assert_eq!(info.schema_version, known_schema_version());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_unreachable_destination_fails_with_a_useful_message() {
        // The USB stick that was pulled out before the owner clicked save.
        let dir = temp_dir("unreachable");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;

        let gone = PathBuf::from(r"Z:\tariqi-no-such-drive\backup.db");
        let result = write_backup(&pool, &gone).await;

        assert!(
            matches!(result, Err(AppError::Validation(code, _)) if code.contains("destinationUnreachable") || code.contains("writeFailed")),
            "expected a destination error, got {result:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_failed_write_leaves_no_half_finished_file_behind() {
        // A file in the backups folder must always mean a good backup.
        let dir = temp_dir("partial");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;

        // VACUUM INTO refuses a destination that is a directory.
        let blocked = dir.join("not-a-file");
        std::fs::create_dir_all(&blocked).unwrap();
        let target = blocked.join("inner");
        std::fs::create_dir_all(&target).unwrap();

        let result = write_backup(&pool, &target).await;
        assert!(result.is_err(), "writing onto a directory must fail");
        assert!(target.is_dir(), "and must not have replaced it");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn rotation_keeps_the_newest_and_leaves_foreign_files_alone() {
        let dir = temp_dir("prune");
        let backups = backups_dir(&dir);
        std::fs::create_dir_all(&backups).unwrap();

        for i in 0..5 {
            let path = backups.join(format!("tariqi-2026010{i}-000000.db"));
            std::fs::write(&path, b"x").unwrap();
            // Distinct modification times so ordering is deterministic.
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::fs::write(backups.join("my-own-copy.db"), b"x").unwrap();

        prune(&backups, 3, None).unwrap();

        let remaining: Vec<String> = std::fs::read_dir(&backups)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(remaining.iter().filter(|n| n.starts_with("tariqi-")).count(), 3);
        assert!(remaining.contains(&"my-own-copy.db".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// After the rename, old `tariqi-` backups and new `rokhsa-` ones are one
    /// rotation: the oldest go first whatever they are called.
    #[tokio::test]
    async fn rotation_treats_backups_from_before_the_rename_as_backups() {
        let dir = temp_dir("prune-rename");
        let backups = backups_dir(&dir);
        std::fs::create_dir_all(&backups).unwrap();

        for name in [
            "tariqi-20260101-000000.db",
            "tariqi-20260102-000000.db",
            "rokhsa-20260103-000000.db",
            "rokhsa-20260104-000000.db",
        ] {
            std::fs::write(backups.join(name), b"x").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        prune(&backups, 3, None).unwrap();

        let mut remaining: Vec<String> = std::fs::read_dir(&backups)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            ["rokhsa-20260103-000000.db", "rokhsa-20260104-000000.db", "tariqi-20260102-000000.db"]
        );
        assert!(timestamp_name(Utc::now()).starts_with("rokhsa-"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_automatic_backup_is_due_when_there_are_none() {
        let dir = temp_dir("due");
        assert!(is_auto_backup_due(&dir, Utc::now()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_automatic_backup_is_not_due_straight_after_one() {
        let dir = temp_dir("notdue");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        auto_backup(&pool, &dir).await.unwrap();

        assert!(!is_auto_backup_due(&dir, Utc::now()));
        assert!(
            is_auto_backup_due(&dir, Utc::now() + chrono::Duration::hours(13)),
            "but it is due again the next day"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn staging_a_restore_changes_nothing_until_it_is_applied() {
        let dir = temp_dir("stage");
        let (db_path, pool) = seeded_db(&dir, &["BENALI"]).await;

        // A backup taken when there was more data.
        let other = temp_dir("stage-src");
        let (_, other_pool) = seeded_db(&other, &["A", "B", "C"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();

        stage_restore(&dir, &snapshot).await.unwrap();

        // Still the original until startup applies it.
        assert_eq!(candidates::count(&pool, "").await.unwrap(), 1);
        assert!(pending_restore(&dir).is_some());

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&other).ok();
        let _ = db_path;
    }

    #[tokio::test]
    async fn applying_a_restore_swaps_the_database_and_keeps_the_old_one() {
        let dir = temp_dir("apply");
        let (db_path, pool) = seeded_db(&dir, &["ONLY_ONE"]).await;
        pool.close().await;

        let other = temp_dir("apply-src");
        let (_, other_pool) = seeded_db(&other, &["A", "B", "C"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();
        other_pool.close().await;

        stage_restore(&dir, &snapshot).await.unwrap();
        let applied = apply_pending_restore(&dir, &db_path).await.unwrap();
        assert!(applied.is_some());
        assert!(pending_restore(&dir).is_none(), "staging file is consumed");

        let reopened = db::connect(&db_path).await.unwrap();
        assert_eq!(candidates::count(&reopened, "").await.unwrap(), 3);

        // The replaced database is kept, not thrown away.
        let kept: Vec<String> = std::fs::read_dir(backups_dir(&dir))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("replaced-"))
            .collect();
        assert_eq!(kept.len(), 1);

        reopened.close().await;
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&other).ok();
    }

    #[tokio::test]
    async fn applying_a_restore_removes_the_old_wal_sidecar() {
        // A stale WAL would be replayed against the restored file and corrupt it.
        let dir = temp_dir("sidecar");
        let (db_path, pool) = seeded_db(&dir, &["ONLY_ONE"]).await;
        pool.close().await;

        let wal = PathBuf::from(format!("{}-wal", db_path.to_string_lossy()));
        std::fs::write(&wal, b"stale").unwrap();

        let other = temp_dir("sidecar-src");
        let (_, other_pool) = seeded_db(&other, &["A", "B"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();
        other_pool.close().await;

        stage_restore(&dir, &snapshot).await.unwrap();
        apply_pending_restore(&dir, &db_path).await.unwrap();

        assert!(!wal.exists(), "stale WAL must be gone");

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&other).ok();
    }

    #[tokio::test]
    async fn a_cancelled_restore_does_not_apply() {
        let dir = temp_dir("cancel");
        let (db_path, pool) = seeded_db(&dir, &["ONLY_ONE"]).await;

        let other = temp_dir("cancel-src");
        let (_, other_pool) = seeded_db(&other, &["A", "B", "C"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();

        stage_restore(&dir, &snapshot).await.unwrap();
        cancel_pending_restore(&dir).unwrap();

        assert!(apply_pending_restore(&dir, &db_path).await.unwrap().is_none());
        assert_eq!(candidates::count(&pool, "").await.unwrap(), 1);

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&other).ok();
    }

    #[tokio::test]
    async fn listing_reports_what_each_backup_holds() {
        let dir = temp_dir("list");
        let (_, pool) = seeded_db(&dir, &["BENALI", "KADRI"]).await;
        auto_backup(&pool, &dir).await.unwrap();

        let all = list(&dir).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].candidates, 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    fn set_age(path: &Path, days_ago: u64) {
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(days_ago * 86_400);
        std::fs::File::options().write(true).open(path).unwrap().set_modified(when).unwrap();
    }

    /// A PC whose clock battery died boots in 2000, so the backup just made
    /// carries the oldest date in the folder. It must still survive the
    /// rotation that follows it.
    #[tokio::test]
    async fn the_backup_just_written_survives_rotation_even_with_the_oldest_date() {
        let dir = temp_dir("prune-protect");
        let backups = backups_dir(&dir);
        std::fs::create_dir_all(&backups).unwrap();

        for i in 0..KEEP_AUTOMATIC {
            let path = backups.join(format!("rokhsa-202610{:02}-000000.db", i + 1));
            std::fs::write(&path, b"x").unwrap();
            set_age(&path, 10 - i as u64);
        }
        let just_written = backups.join("rokhsa-20000101-000000.db");
        std::fs::write(&just_written, b"x").unwrap();
        set_age(&just_written, 9_000);

        prune(&backups, KEEP_AUTOMATIC, Some(&just_written)).unwrap();

        assert!(just_written.exists(), "the newest backup must never be deleted");
        let count = std::fs::read_dir(&backups).unwrap().count();
        assert_eq!(count, KEEP_AUTOMATIC, "still ten in all, the oldest other one went");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// With the clock moved back, the newest backup is "in the future". The
    /// first version computed a negative age and never made another one.
    #[tokio::test]
    async fn a_backup_is_due_when_the_clock_has_gone_backwards() {
        let dir = temp_dir("due-clock");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        auto_backup(&pool, &dir).await.unwrap();

        let clock_in_2000 = Utc::now() - chrono::Duration::days(9_000);
        assert!(is_auto_backup_due(&dir, clock_in_2000));
        // Ordinary drift of a minute or two is not "backwards".
        assert!(!is_auto_backup_due(&dir, Utc::now() - chrono::Duration::minutes(2)));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Writing over an existing backup: the new one replaces it only once it
    /// is proven good, and nothing half-written is left beside it.
    #[tokio::test]
    async fn overwriting_a_backup_replaces_it_only_with_a_good_copy() {
        let dir = temp_dir("overwrite");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        let dest = dir.join("usb.db");
        write_backup(&pool, &dest).await.unwrap();

        candidates::create(&pool, &CandidateInput { last_name: "KADRI".into(), ..Default::default() })
            .await
            .unwrap();
        let info = write_backup(&pool, &dest).await.unwrap();
        assert_eq!(info.candidates, 2);
        assert_eq!(info.file_name, "usb.db", "reports the real name, not the temporary one");
        assert_eq!(inspect(&dest).await.unwrap().candidates, 2);
        assert!(!partial_path(&dest).exists(), "no temporary file left behind");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The case that used to lose both copies: the second write fails. The
    /// first backup must still be there, intact.
    #[tokio::test]
    async fn a_failed_overwrite_leaves_the_previous_backup_intact() {
        let dir = temp_dir("overwrite-fail");
        let (_, pool) = seeded_db(&dir, &["BENALI"]).await;
        let dest = dir.join("usb.db");
        write_backup(&pool, &dest).await.unwrap();

        // Make the write fail: the temporary name is occupied by a folder.
        std::fs::create_dir_all(partial_path(&dest).join("blocked")).unwrap();
        assert!(write_backup(&pool, &dest).await.is_err());

        assert_eq!(inspect(&dest).await.unwrap().candidates, 1, "the old backup survived");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A restore sets the current database aside. The app never closes its
    /// pool cleanly, so the last payments may live only in the -wal sidecar;
    /// the copy set aside must still contain them.
    #[tokio::test]
    async fn the_database_set_aside_by_a_restore_keeps_writes_still_in_the_wal() {
        let live = temp_dir("aside-live");
        let (live_db, pool) = seeded_db(&live, &["A", "B", "C"]).await;
        let wal_before = std::fs::metadata(sidecar(&live_db, "-wal")).map(|m| m.len()).unwrap_or(0);
        assert!(wal_before > 0, "the test needs writes still in the WAL");

        // Freeze the files as a power cut would leave them: main file stale,
        // the recent writes only in the WAL.
        let dir = temp_dir("aside");
        let db_path = dir.join("tariqi.db");
        std::fs::copy(&live_db, &db_path).unwrap();
        std::fs::copy(sidecar(&live_db, "-wal"), sidecar(&db_path, "-wal")).unwrap();
        pool.close().await;

        let other = temp_dir("aside-src");
        let (_, other_pool) = seeded_db(&other, &["ONLY"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();
        other_pool.close().await;

        stage_restore(&dir, &snapshot).await.unwrap();
        apply_pending_restore(&dir, &db_path).await.unwrap();

        let kept = std::fs::read_dir(backups_dir(&dir))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("replaced-") && n.ends_with(".db")
            })
            .unwrap();
        let reopened = db::connect(&kept).await.unwrap();
        assert_eq!(candidates::count(&reopened, "").await.unwrap(), 3, "nothing lost");
        reopened.close().await;

        let restored = db::connect(&db_path).await.unwrap();
        assert_eq!(candidates::count(&restored, "").await.unwrap(), 1);
        restored.close().await;

        for d in [&live, &dir, &other] {
            std::fs::remove_dir_all(d).ok();
        }
    }

    /// A staged file damaged after staging — a power cut mid-copy — must not
    /// replace the live database. It is set aside and the app starts on the
    /// data it had.
    #[tokio::test]
    async fn a_damaged_staged_restore_is_refused_and_the_live_database_kept() {
        let dir = temp_dir("bad-stage");
        let (db_path, pool) = seeded_db(&dir, &["ONLY_ONE"]).await;
        pool.close().await;

        let other = temp_dir("bad-stage-src");
        let (_, other_pool) = seeded_db(&other, &["A", "B", "C"]).await;
        let snapshot = other.join("snapshot.db");
        write_backup(&other_pool, &snapshot).await.unwrap();
        other_pool.close().await;

        stage_restore(&dir, &snapshot).await.unwrap();
        let staged = pending_restore(&dir).unwrap();
        let bytes = std::fs::read(&staged).unwrap();
        std::fs::write(&staged, &bytes[..bytes.len() / 3]).unwrap();

        assert!(apply_pending_restore(&dir, &db_path).await.unwrap().is_none());
        assert!(pending_restore(&dir).is_none(), "not retried at every start");

        let live = db::connect(&db_path).await.unwrap();
        assert_eq!(candidates::count(&live, "").await.unwrap(), 1, "live data untouched");
        live.close().await;

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&other).ok();
    }
}
