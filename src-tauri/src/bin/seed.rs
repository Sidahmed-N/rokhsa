//! Demo data loader.
//!
//! Fills a database with plausible Algerian candidates so the app can be
//! demonstrated to a school owner, and so changes to the list and search
//! screens can be checked against real-looking rows.
//!
//!     cargo run --bin seed              # default app database
//!     cargo run --bin seed -- <path>    # a specific file
//!
//! Safe to re-run: it skips seeding if the database already has candidates.

use std::path::PathBuf;

use tariqi_lib::db;
use tariqi_lib::domain::candidate::CandidateInput;

fn default_db_path() -> PathBuf {
    // Mirrors Tauri's app_data_dir for identifier "dz.tariqi.desktop".
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("dz.tariqi.desktop").join("tariqi.db")
}

fn candidate(
    last_fr: &str,
    first_fr: &str,
    last_ar: &str,
    first_ar: &str,
    phone: &str,
    nin: &str,
    birth: &str,
    commune: &str,
    status: &str,
) -> CandidateInput {
    CandidateInput {
        last_name_fr: last_fr.into(),
        first_name_fr: first_fr.into(),
        last_name_ar: last_ar.into(),
        first_name_ar: first_ar.into(),
        phone: phone.into(),
        nin: nin.into(),
        birth_date: birth.into(),
        commune: commune.into(),
        status: status.into(),
        ..Default::default()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_db_path);

    println!("seeding {}", path.display());
    let pool = db::connect(&path).await?;

    let existing = db::candidates::count(&pool, "").await?;
    if existing > 0 {
        println!("database already has {existing} candidate(s); nothing to do");
        return Ok(());
    }

    // School details for the receipt letterhead. There is no settings screen
    // yet, so without this every printed receipt is headed "name not set".
    sqlx::query(
        "UPDATE school SET
            name_fr = ?1, name_ar = ?2, address_fr = ?3, address_ar = ?4,
            phone = ?5, agrement_number = ?6, commune = ?7, wilaya_code = ?8,
            receipt_prefix = ?9
         WHERE id = 1",
    )
    .bind("Auto-École El Amel")
    .bind("مدرسة تعليم السياقة الأمل")
    .bind("12, Rue Didouche Mourad, Alger-Centre")
    .bind("12 شارع ديدوش مراد، الجزائر الوسطى")
    .bind("0551 23 45 67")
    .bind("16/2019/0482")
    .bind("Alger-Centre")
    .bind("16")
    .bind("2026-")
    .execute(&pool)
    .await?;
    println!("  school details set");

    let rows = vec![
        candidate("BENALI", "Yacine", "بن علي", "ياسين", "0551 23 45 67",
                  "109 2004 1600 5482 130", "2004-07-19", "Alger-Centre", "active"),
        candidate("KADRI", "Amine", "قادري", "أمين", "0661 88 21 09",
                  "110 2003 1600 7781 204", "2003-02-11", "Hussein Dey", "active"),
        candidate("BENAÏSSA", "Leïla", "بن عيسى", "ليلى", "0770 45 12 33",
                  "", "2005-11-03", "Bab Ezzouar", "active"),
        candidate("MEZIANE", "Sofiane", "مزيان", "سفيان", "0555 10 20 30",
                  "112 2001 1600 3344 551", "2001-05-27", "Kouba", "completed"),
        candidate("HAMDI", "Nour El Houda", "حمدي", "نور الهدى", "0699 77 65 43",
                  "", "2006-01-15", "El Harrach", "prospect"),
        candidate("SAÏDI", "Nabil", "سعيدي", "نبيل", "0540 31 22 18",
                  "", "1999-09-08", "Bir Mourad Raïs", "inactive"),
        candidate("TOUATI", "Imene", "توايتي", "إيمان", "0558 64 90 12",
                  "", "2004-12-30", "Draria", "active"),
        candidate("ZIANI", "Walid", "زياني", "وليد", "0773 19 54 06",
                  "", "2002-03-22", "Bordj El Kiffan", "withdrawn"),
    ];

    let mut ids = Vec::new();
    for row in &rows {
        match db::candidates::create(&pool, row).await {
            Ok(created) => {
                println!("  + {} {}", created.last_name_fr, created.first_name_fr);
                ids.push(created.id);
            }
            Err(e) => eprintln!("  ! {} {}: {e}", row.last_name_fr, row.first_name_fr),
        }
    }

    // Enrolments, hours and payments, so the detail screen shows a realistic
    // mid-training candidate rather than empty tables.
    const DA: i64 = 100;

    if let Some(&benali) = ids.first() {
        let e = db::enrollments::create(
            &pool,
            benali,
            &db::enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                extra_hour_price_centimes: 1_500 * DA,
                theory_hours_target: 30,
                practical_hours_target: 30,
                enrolled_on: "2026-08-04".into(),
                ..Default::default()
            },
        )
        .await?;

        for (kind, date, minutes) in [
            ("theory", "2026-08-06", 120),
            ("theory", "2026-08-11", 120),
            ("theory", "2026-08-18", 180),
            ("maneuvers", "2026-09-02", 90),
            ("maneuvers", "2026-09-09", 90),
            ("road", "2026-09-16", 120),
            ("road", "2026-09-23", 120),
            ("road", "2026-10-07", 90),
        ] {
            db::lessons::create(
                &pool,
                e.id,
                &db::lessons::LessonInput {
                    kind: kind.into(),
                    occurred_on: date.into(),
                    minutes,
                    ..Default::default()
                },
            )
            .await?;
        }

        for (amount, date, method) in [
            (12_000, "2026-08-04", "cash"),
            (10_000, "2026-09-10", "cash"),
            (5_000, "2026-10-12", "baridimob"),
        ] {
            db::payments::create(
                &pool,
                e.id,
                &db::payments::PaymentInput {
                    amount_centimes: amount * DA,
                    paid_on: date.into(),
                    method_code: method.into(),
                    ..Default::default()
                },
            )
            .await?;
        }

        let s = db::enrollments::summary(&pool, e.id).await?;
        println!(
            "  enrolment for candidate {benali}: due {} / paid {} / balance {} centimes",
            s.total_due_centimes, s.paid_centimes, s.balance_centimes
        );
    }

    // A second, fully-paid enrolment for contrast.
    if let Some(&kadri) = ids.get(1) {
        let e = db::enrollments::create(
            &pool,
            kadri,
            &db::enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 32_000 * DA,
                theory_hours_target: 30,
                practical_hours_target: 30,
                enrolled_on: "2026-06-15".into(),
                ..Default::default()
            },
        )
        .await?;
        db::payments::create(
            &pool,
            e.id,
            &db::payments::PaymentInput {
                amount_centimes: 32_000 * DA,
                paid_on: "2026-06-15".into(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await?;
    }

    println!("done: {} candidate(s)", db::candidates::count(&pool, "").await?);
    Ok(())
}
