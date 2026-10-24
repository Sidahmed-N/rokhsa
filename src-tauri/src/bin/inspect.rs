//! Row counts and an integrity check for a database file.
//!
//! A diagnostic, not a feature: when a screen says "0 candidates" the only
//! way to know whether that is the screen or the data is to ask the file.
//!
//!     cargo run --bin inspect              # default app database
//!     cargo run --bin inspect -- <path>
//!
//! RUN IT WITH THE APP CLOSED, and never open this database any other way.
//! An earlier version of this file opened a second connection with its own
//! options — no WAL, no busy timeout — alongside the running app, and
//! corrupted the database. SQLite will not protect you from two connections
//! disagreeing about the journal mode. So there is exactly one way in here:
//! `db::connect`, the same call the app makes, with the same pragmas.
//!
//! Note this therefore APPLIES PENDING MIGRATIONS. That is deliberate — a
//! migration that fails here is one that would fail in the app — but it does
//! mean this is not a read-only tool. Do not point it at a backup you want
//! to keep untouched; copy it first.

use std::path::PathBuf;

use sqlx::SqlitePool;

use tariqi_lib::db;

fn default_db_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("dz.tariqi.desktop").join("tariqi.db")
}

async fn count(pool: &SqlitePool, table: &str) {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    match sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await {
        Ok(n) => println!("  {table:<18} {n}"),
        Err(e) => println!("  {table:<18} -- {e}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `--backups` asks the same question the restore screen asks, through the
    // same code, so a backup the screen refuses to show says why here.
    if args.first().map(String::as_str) == Some("--backups") {
        let data_dir = default_db_path().parent().unwrap().to_path_buf();
        let dir = data_dir.join("backups");
        println!("{}", dir.display());

        // The exact call the restore screen makes.
        match tariqi_lib::backup::list(&data_dir).await {
            Ok(list) => {
                println!("  backup::list returned {}", list.len());
                for info in &list {
                    println!("    {} candidates={}", info.file_name, info.candidates);
                }
            }
            Err(e) => println!("  backup::list FAILED {e}"),
        }
        println!("  --- per file ---");
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            match tariqi_lib::backup::inspect(&path).await {
                Ok(info) => println!(
                    "  {name:<26} ok   candidates={} payments={} schema={}",
                    info.candidates, info.payments, info.schema_version
                ),
                Err(e) => println!("  {name:<26} REJECTED  {e}"),
            }
        }
        return Ok(());
    }

    let path = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(default_db_path);

    println!("{}", path.display());

    let pool = match db::connect(&path).await {
        Ok(pool) => {
            println!("  connect()          ok (migrations applied)");
            pool
        }
        Err(e) => {
            println!("  connect()          FAILED: {e}");
            return Ok(());
        }
    };

    for table in [
        "candidates",
        "enrollments",
        "lessons",
        "payments",
        "exam_sessions",
        "exam_results",
        "exam_fees",
        "exam_fee_charges",
    ] {
        count(&pool, table).await;
    }

    match db::integrity_check(&pool).await {
        Ok(result) => println!("  integrity_check    {result}"),
        Err(e) => println!("  integrity_check    -- {e}"),
    }

    let applied: Result<Vec<(i64,)>, _> =
        sqlx::query_as("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await;
    if let Ok(rows) = applied {
        let versions: Vec<String> = rows.iter().map(|r| r.0.to_string()).collect();
        println!("  migrations         {}", versions.join(", "));
    }

    pool.close().await;
    Ok(())
}
