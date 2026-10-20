//! Crash consistency.
//!
//! `docs/03-STACK.md` promises a power-cut test in the release checklist, and
//! for a long time it was only a promise. This is it.
//!
//! A real process writes payments in a loop and is killed abruptly with
//! TerminateProcess — no unwinding, no flush, no chance to close the database.
//! That is as close to pulling the plug as a test can get without special
//! hardware: it does not prove the DISK honoured its flushes, but it does
//! prove the application and SQLite leave a consistent file behind.
//!
//! What must hold afterwards:
//!   * the database opens and passes an integrity check
//!   * every payment has a receipt number
//!   * receipt numbers are unique and have no gaps
//!   * the school's next-receipt counter agrees with the payments on disk
//!
//! That last one is the real test of atomicity. Allocating the number and
//! inserting the payment happen in one transaction, so if a crash could tear
//! them apart these two would disagree.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use tariqi_lib::db;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tariqi-crash-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test(flavor = "multi_thread")]
async fn killing_the_app_mid_write_leaves_a_consistent_database() {
    let dir = temp_dir("kill");
    let db_path = dir.join("tariqi.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_crash_writer"))
        .arg(&db_path)
        .stdout(Stdio::piped())
        .spawn()
        .expect("crash_writer should start");

    // Wait until it is past setup and actually writing payments.
    {
        let stdout = child.stdout.take().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).expect("readiness line");
        assert_eq!(line.trim(), "ready");
    }
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // The plug comes out here.
    child.kill().expect("kill");
    child.wait().ok();

    // ---- what the school finds when it switches the PC back on ----

    let pool = db::connect(&db_path)
        .await
        .expect("the database must still open after a crash");

    let integrity = db::integrity_check(&pool).await.unwrap();
    assert_eq!(integrity, "ok", "integrity check failed after a crash");

    let payments: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, receipt_number FROM payments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert!(
        !payments.is_empty(),
        "the writer should have committed some payments before being killed"
    );

    // Every payment has a receipt number, and no two share one.
    let mut numbers: Vec<String> = payments.iter().map(|p| p.1.clone()).collect();
    assert!(
        numbers.iter().all(|n| !n.trim().is_empty()),
        "a payment was committed without a receipt number"
    );
    let before = numbers.len();
    numbers.sort();
    numbers.dedup();
    assert_eq!(before, numbers.len(), "duplicate receipt numbers after a crash");

    // Gapless: the sequence runs 1..n with nothing missing.
    let mut sequence: Vec<i64> = payments
        .iter()
        .map(|p| p.1.trim_start_matches('0').parse::<i64>().unwrap_or(0))
        .collect();
    sequence.sort_unstable();
    for (index, value) in sequence.iter().enumerate() {
        assert_eq!(
            *value,
            index as i64 + 1,
            "receipt numbers have a gap at position {index}: {sequence:?}"
        );
    }

    // The counter and the payments agree — proof the allocation and the insert
    // were torn apart by nothing.
    let next: (i64,) = sqlx::query_as("SELECT next_receipt_number FROM school WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        next.0,
        payments.len() as i64 + 1,
        "the receipt counter disagrees with the payments on disk"
    );

    pool.close().await;
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_durability_pragmas_are_actually_in_force() {
    // The crash test above is only meaningful if these are set. They are easy
    // to lose in a refactor and nothing else would notice.
    let dir = temp_dir("pragmas");
    let pool = db::connect(&dir.join("tariqi.db")).await.unwrap();

    let journal: (String,) = sqlx::query_as("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(journal.0.to_lowercase(), "wal");

    // synchronous: 2 = FULL
    let synchronous: (i64,) = sqlx::query_as("PRAGMA synchronous")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(synchronous.0, 2, "synchronous must be FULL for durability");

    let foreign_keys: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(foreign_keys.0, 1, "foreign keys must be on");

    pool.close().await;
    std::fs::remove_dir_all(&dir).ok();
}
