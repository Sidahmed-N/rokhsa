//! Writes payments until it is killed. Used by tests/crash_consistency.rs.
//!
//! Not part of the shipped application. It exists so the crash test can be a
//! real process that dies mid-write, rather than a simulation of one.

use std::path::PathBuf;

use tariqi_lib::db;
use tariqi_lib::domain::candidate::CandidateInput;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: crash_writer <database path>"),
    );

    let pool = db::connect(&path).await?;

    let candidate = db::candidates::create(
        &pool,
        &CandidateInput {
            last_name: "CRASHTEST".into(),
            ..Default::default()
        },
    )
    .await?;

    let enrollment = db::enrollments::create(
        &pool,
        candidate.id,
        &db::enrollments::EnrollmentInput {
            category_code: "B".into(),
            agreed_price_centimes: 100_000_000,
            ..Default::default()
        },
    )
    .await?;

    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();

    // Signal readiness so the test kills us mid-write, not during setup.
    println!("ready");
    use std::io::Write;
    std::io::stdout().flush().ok();

    loop {
        db::payments::create(
            &pool,
            enrollment.id,
            &db::payments::PaymentInput {
                amount_centimes: 100,
                paid_on: today.clone(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await?;
    }
}
