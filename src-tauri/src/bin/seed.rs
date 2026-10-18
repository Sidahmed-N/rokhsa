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

    for row in &rows {
        match db::candidates::create(&pool, row).await {
            Ok(created) => println!("  + {} {}", created.last_name_fr, created.first_name_fr),
            Err(e) => eprintln!("  ! {} {}: {e}", row.last_name_fr, row.first_name_fr),
        }
    }

    println!("done: {} candidate(s)", db::candidates::count(&pool, "").await?);
    Ok(())
}
