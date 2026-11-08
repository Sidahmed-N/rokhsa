//! Load test data: three years of a busy school.
//!
//! Not a demo (that is `seed`): this exists to check the screens stay fast
//! once a school has used the app for a few years. Nothing is ever deleted
//! (finished candidates are archived, payments are append-only), so the
//! database only grows.
//!
//!     cargo run --release --bin loadgen -- <path> [candidates]
//!
//! Default 900 candidates, 300 a year, each with about thirty taught
//! sessions, three or four payments, an exam fee per attempt and a share of
//! the expenses. Goes through the same db functions as the app, so the rows
//! are what the app would have written. Refuses a database that already has
//! candidates, and never touches the app's own data unless pointed at it.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{Duration, NaiveDate, Utc};

use tariqi_lib::db;
use tariqi_lib::domain::candidate::CandidateInput;

const DA: i64 = 100;

const LAST: &[&str] = &[
    "BENALI", "KADRI", "MEZIANE", "HAMDI", "SAÏDI", "TOUATI", "ZIANI", "BOUDJEMAA", "LOUNIS",
    "CHERIF", "AMRANI", "BELKACEM", "HADDAD", "MERABET", "KHELIFI", "SLIMANI", "بن علي",
    "قادري", "مزيان", "حمدي", "تواتي", "بوجمعة", "شريف", "عمراني",
];
const FIRST: &[&str] = &[
    "Yacine", "Amine", "Leïla", "Sofiane", "Nour", "Nabil", "Imene", "Walid", "Karim", "Samira",
    "Mehdi", "Yasmine", "Rachid", "Sara", "Bilal", "Amel", "ياسين", "أمين", "ليلى", "سارة",
];

fn iso(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().ok_or("usage: loadgen <path> [candidates]")?);
    let total: usize = args.next().map(|n| n.parse()).transpose()?.unwrap_or(900);

    let pool = db::connect(&path).await?;
    if db::candidates::count(&pool, "").await? > 0 {
        return Err("this database already has candidates; point loadgen at a new file".into());
    }

    let today = Utc::now().date_naive();
    let span = 365 * 3;
    let start = today - Duration::days(span);
    let kinds = ["theory", "maneuvers", "road"];
    let mut sittings: HashMap<(&str, NaiveDate), i64> = HashMap::new();
    let (mut lessons, mut payments, mut entries) = (0usize, 0usize, 0usize);
    let started = std::time::Instant::now();

    for i in 0..total {
        let enrolled = start + Duration::days(i as i64 * span / total as i64);
        let withdrawn = i % 20 == 7;

        let candidate = db::candidates::create(
            &pool,
            &CandidateInput {
                last_name: LAST[i % LAST.len()].into(),
                first_name: format!("{} {}", FIRST[(i / 3) % FIRST.len()], i),
                phone: format!("05{:02} {:02} {:02} {:02}", 50 + i % 50, i % 97, i % 89, i % 83),
                ..Default::default()
            },
        )
        .await?;

        let enrollment = db::enrollments::create(
            &pool,
            candidate.id,
            &db::enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: [35_000, 38_000, 40_000][i % 3] * DA,
                theory_hours_target: 30,
                practical_hours_target: 30,
                enrolled_on: iso(enrolled),
                status: if withdrawn { "withdrawn".into() } else { "forming".into() },
                ..Default::default()
            },
        )
        .await?;
        let price = [35_000, 38_000, 40_000][i % 3] * DA;

        // About thirty sessions over three months, fewer for who abandoned.
        let sessions = if withdrawn { 6 } else { 28 + i % 6 };
        for n in 0..sessions {
            let day = enrolled + Duration::days(1 + n as i64 * 3);
            if day > today {
                break;
            }
            db::lessons::create(
                &pool,
                enrollment.id,
                &db::lessons::LessonInput {
                    kind: ["theory", "theory", "maneuvers", "road", "road"][n % 5].into(),
                    occurred_on: iso(day),
                    minutes: 60,
                    ..Default::default()
                },
            )
            .await?;
            lessons += 1;
        }

        // A first versement at registration, then two more; one in seven
        // stops paying and stays in Créances.
        let stops = withdrawn || i % 7 == 3;
        let plan: &[(i64, i64)] = if stops {
            &[(0, 15_000 * DA)]
        } else {
            &[(0, 15_000 * DA), (30, 10_000 * DA), (60, 0)]
        };
        let mut paid = 0;
        for &(after, amount) in plan {
            let day = enrolled + Duration::days(after);
            if day > today {
                break;
            }
            let amount = if amount == 0 { price - paid } else { amount };
            db::payments::create(
                &pool,
                enrollment.id,
                &db::payments::PaymentInput {
                    amount_centimes: amount,
                    paid_on: iso(day),
                    method_code: if i % 4 == 0 { "baridimob" } else { "cash" }.into(),
                    ..Default::default()
                },
            )
            .await?;
            paid += amount;
            payments += 1;
        }

        // The three exams a month apart, at the sitting held every two weeks;
        // one in five fails once and passes at the next one.
        if !withdrawn {
            let mut day = enrolled + Duration::days(30);
            for (k, kind) in kinds.iter().enumerate() {
                let attempts = if (i + k) % 5 == 0 { 2 } else { 1 };
                for attempt in 0..attempts {
                    let offset = (day - start).num_days();
                    let held = start + Duration::days(offset - offset % 14 + 14);
                    if held > today + Duration::days(20) {
                        break;
                    }
                    let session = match sittings.get(&(*kind, held)) {
                        Some(id) => *id,
                        None => {
                            let s = db::exams::create_session(
                                &pool,
                                &db::exams::ExamSessionInput {
                                    kind: (*kind).into(),
                                    held_on: iso(held),
                                    held_at: "08:00".into(),
                                    place: "Centre d'examen".into(),
                                    ..Default::default()
                                },
                            )
                            .await?;
                            sittings.insert((*kind, held), s.id);
                            s.id
                        }
                    };
                    let entry =
                        db::exams::add_to_session(&pool, session, enrollment.id, 2_000 * DA).await?;
                    entries += 1;
                    if held <= today {
                        let outcome = if attempt + 1 < attempts { "failed" } else { "passed" };
                        db::exams::set_outcome(&pool, entry.id, outcome, "").await?;
                        if i % 9 != 4 {
                            db::payments::create(
                                &pool,
                                enrollment.id,
                                &db::payments::PaymentInput {
                                    amount_centimes: 2_000 * DA,
                                    paid_on: iso(held),
                                    method_code: "cash".into(),
                                    exam_fee_charge_id: entry.fee_charge_id,
                                    ..Default::default()
                                },
                            )
                            .await?;
                            payments += 1;
                        }
                    }
                    day = held + Duration::days(14);
                }
                day += Duration::days(16);
            }
        }

        // Half of those finished more than a year ago are archived.
        if !withdrawn && enrolled < today - Duration::days(365) && i % 2 == 0 {
            db::candidates::set_archived(&pool, &[candidate.id], true).await?;
        }

        if (i + 1) % 100 == 0 {
            println!("  {} candidates ({:.0?})", i + 1, started.elapsed());
        }
    }

    // About fifteen expenses a month.
    let categories = ["fuel", "fuel", "maintenance", "rent", "salaries", "supplies", "insurance"];
    let months = span / 30;
    for m in 0..months {
        for n in 0..15 {
            db::expenses::create(
                &pool,
                &db::expenses::ExpenseInput {
                    category_code: categories[(m as usize + n) % categories.len()].into(),
                    amount_centimes: [3_000, 4_500, 25_000, 12_000, 1_500][n % 5] * DA,
                    spent_on: iso(start + Duration::days(m * 30 + n as i64 * 2)),
                    description: "Dépense".into(),
                    ..Default::default()
                },
            )
            .await?;
        }
    }

    println!(
        "done in {:.0?}: {total} candidates, {lessons} sessions, {payments} payments, \
         {entries} exam entries in {} sittings, {} expenses",
        started.elapsed(),
        sittings.len(),
        months * 15
    );
    Ok(())
}
