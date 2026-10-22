//! Demo data loader.
//!
//! Fills a database with a plausible mid-sized Algerian auto-école — thirty
//! candidates at every stage, a week of booked sessions, money owed and money
//! taken — so the app can be shown to a school owner and so changes to the
//! screens can be checked against realistic rows rather than three of them.
//!
//!     cargo run --bin seed              # default app database
//!     cargo run --bin seed -- <path>    # a specific file
//!
//! Safe to re-run: it skips seeding if the database already has candidates.
//!
//! Everything dated is generated RELATIVE TO TODAY, so the week grid is full
//! whenever this is run rather than only on the day it was written.

use std::path::PathBuf;

use chrono::{Datelike, Duration, NaiveDate, Utc};

use tariqi_lib::db;
use tariqi_lib::domain::candidate::CandidateInput;

fn default_db_path() -> PathBuf {
    // Mirrors Tauri's app_data_dir for identifier "dz.tariqi.desktop".
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("dz.tariqi.desktop").join("tariqi.db")
}

/// Dinars to centimes. Money is integer centimes everywhere — see CLAUDE.md.
const DA: i64 = 100;

/// last_fr, first_fr, last_ar, first_ar, phone, nin, birth, commune, status
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// Thirty candidates, names in both scripts as the paperwork requires.
///
/// The spread of statuses is deliberate: a real school is mostly people in
/// training, with a tail of finished, stalled and abandoned files that the
/// owner still has to see. A screen that only ever gets "active" rows looks
/// fine until the day someone abandons.
const CANDIDATES: &[Row] = &[
    ("BENALI", "Yacine", "بن علي", "ياسين", "0551 23 45 67", "109 2004 1600 5482 130", "2004-07-19", "Alger-Centre", "active"),
    ("KADRI", "Amine", "قادري", "أمين", "0661 88 21 09", "110 2003 1600 7781 204", "2003-02-11", "Hussein Dey", "active"),
    ("BENAÏSSA", "Leïla", "بن عيسى", "ليلى", "0770 45 12 33", "111 2005 1600 2290 318", "2005-11-03", "Bab Ezzouar", "active"),
    ("MEZIANE", "Sofiane", "مزيان", "سفيان", "0555 10 20 30", "112 2001 1600 3344 551", "2001-05-27", "Kouba", "completed"),
    ("HAMDI", "Nour El Houda", "حمدي", "نور الهدى", "0699 77 65 43", "113 2006 1600 8812 447", "2006-01-15", "El Harrach", "prospect"),
    ("SAÏDI", "Nabil", "سعيدي", "نبيل", "0540 31 22 18", "114 1999 1600 1174 962", "1999-09-08", "Bir Mourad Raïs", "inactive"),
    ("TOUATI", "Imene", "تواتي", "إيمان", "0558 64 90 12", "115 2004 1600 6628 073", "2004-12-30", "Draria", "active"),
    ("ZIANI", "Walid", "زياني", "وليد", "0773 19 54 06", "116 2002 1600 4417 285", "2002-03-22", "Bordj El Kiffan", "withdrawn"),
    ("BOUDJEMAA", "Karim", "بوجمعة", "كريم", "0550 44 17 82", "117 2000 1600 9023 614", "2000-06-14", "Birkhadem", "active"),
    ("LOUNIS", "Samira", "لونيس", "سميرة", "0662 90 11 47", "118 2003 1600 5561 730", "2003-08-02", "Chéraga", "active"),
    ("CHERIF", "Mehdi", "شريف", "مهدي", "0776 23 88 05", "119 2005 1600 3390 158", "2005-04-19", "Dély Ibrahim", "active"),
    ("AMRANI", "Yasmine", "عمراني", "ياسمين", "0553 71 60 29", "120 2004 1600 7714 842", "2004-10-07", "Ben Aknoun", "active"),
    ("BELKACEM", "Rachid", "بلقاسم", "رشيد", "0667 35 02 91", "121 1998 1600 2208 365", "1998-12-25", "Baraki", "completed"),
    ("HADDAD", "Sara", "حداد", "سارة", "0779 48 13 76", "122 2006 1600 6640 219", "2006-02-28", "Bachdjarah", "active"),
    ("MERABET", "Bilal", "مرابط", "بلال", "0556 02 94 38", "123 2002 1600 8871 503", "2002-07-11", "Rouiba", "active"),
    ("KHELIFI", "Amel", "خليفي", "أمال", "0664 57 29 10", "124 2005 1600 4432 687", "2005-09-16", "Reghaïa", "active"),
    ("SLIMANI", "Tarek", "سليماني", "طارق", "0771 86 40 52", "125 2001 1600 1159 974", "2001-01-30", "Dar El Beïda", "inactive"),
    ("BOUKHARI", "Nadia", "بوخاري", "نادية", "0559 13 75 68", "126 2003 1600 9985 241", "2003-11-21", "Hydra", "active"),
    ("ZEROUAL", "Ismail", "زروال", "إسماعيل", "0668 29 51 03", "127 2004 1600 3317 850", "2004-05-05", "El Biar", "active"),
    ("MANSOURI", "Lydia", "منصوري", "ليديا", "0774 60 38 17", "128 2006 1600 7726 492", "2006-03-12", "Bouzareah", "prospect"),
    ("GHANEM", "Riad", "غانم", "رياض", "0552 95 08 44", "129 2000 1600 5503 138", "2000-08-23", "Oued Smar", "active"),
    ("BENYAHIA", "Soumia", "بن يحيى", "سمية", "0665 41 87 26", "130 2005 1600 2264 709", "2005-06-01", "Staoueli", "active"),
    ("TALEB", "Anis", "طالب", "أنيس", "0777 52 16 93", "131 2002 1600 8840 357", "2002-10-18", "Zeralda", "withdrawn"),
    ("DJEBBAR", "Hiba", "جبار", "هبة", "0554 38 72 01", "132 2007 1600 6618 924", "2007-01-09", "Aïn Benian", "active"),
    ("OUALI", "Fouad", "والي", "فؤاد", "0669 74 20 85", "133 1997 1600 1142 563", "1997-04-27", "Les Eucalyptus", "completed"),
    ("BRAHIMI", "Rania", "براهيمي", "رانيا", "0772 09 63 48", "134 2006 1600 9971 206", "2006-08-14", "Kouba", "active"),
    ("MOKRANI", "Zineddine", "مقراني", "زين الدين", "0557 26 49 70", "135 2001 1600 4458 831", "2001-12-03", "El Madania", "active"),
    ("FERHAT", "Meriem", "فرحات", "مريم", "0663 81 05 32", "136 2004 1600 7793 614", "2004-02-20", "Bologhine", "active"),
    ("AÏT ALI", "Sofiane", "آيت علي", "سفيان", "0778 14 57 90", "137 2003 1600 3326 478", "2003-07-08", "Bab El Oued", "inactive"),
    ("BENZEMA", "Khadidja", "بن زمة", "خديجة", "0551 67 93 24", "138 2005 1600 8807 165", "2005-05-31", "Casbah", "active"),
];

/// The Saturday that opens the week `day` falls in.
///
/// Algerian auto-écoles work Saturday to Thursday and rest on Friday, which
/// is also how the week view is laid out.
fn week_start(day: NaiveDate) -> NaiveDate {
    // num_days_from_sunday(): Sunday = 0 … Saturday = 6.
    let since = (day.weekday().num_days_from_sunday() + 1) % 7;
    day - Duration::days(since as i64)
}

fn iso(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

/// A deterministic spread, so the demo looks the same every time it is loaded.
/// Not random: a demo that reshuffles on every run cannot be talked through.
fn pick<T: Copy>(options: &[T], n: usize) -> T {
    options[n % options.len()]
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

    let today = Utc::now().date_naive();
    let saturday = week_start(today);

    // ---------------------------------------------------------------- people
    let mut ids = Vec::new();
    for row in CANDIDATES {
        let input = CandidateInput {
            last_name_fr: row.0.into(),
            first_name_fr: row.1.into(),
            last_name_ar: row.2.into(),
            first_name_ar: row.3.into(),
            phone: row.4.into(),
            nin: row.5.into(),
            birth_date: row.6.into(),
            commune: row.7.into(),
            status: row.8.into(),
            ..Default::default()
        };
        match db::candidates::create(&pool, &input).await {
            Ok(created) => ids.push((created.id, row.8)),
            Err(e) => eprintln!("  ! {} {}: {e}", row.0, row.1),
        }
    }
    println!("  {} candidate(s)", ids.len());

    // ----------------------------------------------------------- enrolments
    // A prospect has not signed up yet, so has no enrolment and no money.
    let categories = ["B", "B", "B", "B", "B", "A", "C1", "B", "B", "A"];
    let prices = [32_000i64, 35_000, 36_000, 38_000, 40_000, 34_000];

    let mut enrollments = Vec::new();
    for (index, (candidate_id, status)) in ids.iter().enumerate() {
        if *status == "prospect" {
            continue;
        }

        // Older files for the finished and abandoned ones, recent for the rest.
        let age_days = match *status {
            "completed" => 150 + (index as i64 % 40),
            "withdrawn" => 110 + (index as i64 % 30),
            "inactive" => 95 + (index as i64 % 25),
            _ => 20 + (index as i64 * 4 % 70),
        };

        let enrollment = db::enrollments::create(
            &pool,
            *candidate_id,
            &db::enrollments::EnrollmentInput {
                category_code: pick(&categories, index).into(),
                agreed_price_centimes: pick(&prices, index) * DA,
                extra_hour_price_centimes: 1_500 * DA,
                theory_hours_target: 30,
                practical_hours_target: 30,
                enrolled_on: iso(today - Duration::days(age_days)),
                status: match *status {
                    "completed" => "passed".into(),
                    "withdrawn" => "withdrawn".into(),
                    _ => "forming".into(),
                },
                ..Default::default()
            },
        )
        .await?;

        enrollments.push((enrollment.id, *candidate_id, *status, index, age_days));
    }
    println!("  {} enrolment(s)", enrollments.len());

    // ------------------------------------------------------- hours already done
    let kinds = ["theory", "theory", "maneuvers", "road", "road", "road"];
    let mut lesson_count = 0;

    for (enrollment_id, _, status, index, age_days) in &enrollments {
        let age_days = *age_days;
        // How far through training this candidate is.
        let sessions = match *status {
            "completed" => 22,
            "withdrawn" => 4,
            "inactive" => 6,
            _ => 3 + (index % 10),
        };

        for n in 0..sessions {
            // Spread backwards from the enrolment date towards today.
            let offset = age_days - (n as i64 * (age_days.max(8) / (sessions as i64 + 1)));
            if offset <= 0 {
                continue;
            }
            db::lessons::create(
                &pool,
                *enrollment_id,
                &db::lessons::LessonInput {
                    kind: pick(&kinds, index + n).into(),
                    occurred_on: iso(today - Duration::days(offset)),
                    minutes: pick(&[60i64, 90, 90, 120], index + n),
                    ..Default::default()
                },
            )
            .await?;
            lesson_count += 1;
        }
    }
    println!("  {lesson_count} session(s) already taught");

    // --------------------------------------------------------------- payments
    let methods = ["cash", "cash", "cash", "baridimob", "cheque", "cash"];

    for (enrollment_id, _, status, index, age_days) in &enrollments {
        let age_days = *age_days;
        let summary = db::enrollments::summary(&pool, *enrollment_id).await?;
        let due = summary.total_due_centimes;

        // Who has paid what. A school's receivables screen is only convincing
        // if some people are square, some are halfway and some never started.
        let fraction = match *status {
            "completed" => 100,
            "withdrawn" => 35,
            "inactive" => 45,
            _ => pick(&[100i64, 70, 55, 40, 85, 0, 60, 25], *index),
        };
        if fraction == 0 {
            continue;
        }

        // Paid in two or three instalments, the way a versement actually works.
        let total = due * fraction / 100;
        let parts = if fraction >= 100 { 3 } else { 2 };
        let mut paid = 0;
        for p in 0..parts {
            let amount = if p == parts - 1 { total - paid } else { total / parts as i64 };
            if amount <= 0 {
                continue;
            }
            let when = age_days - (p as i64 * age_days / (parts as i64 + 1));
            db::payments::create(
                &pool,
                *enrollment_id,
                &db::payments::PaymentInput {
                    amount_centimes: amount,
                    paid_on: iso(today - Duration::days(when.max(0))),
                    method_code: pick(&methods, index + p).into(),
                    ..Default::default()
                },
            )
            .await?;
            paid += amount;
        }
    }

    // ---------------------------------------------------- this week's diary
    // The point of the demo: a full week on the calendar, at believable hours,
    // with a mix of kinds so the colour coding is visible at a glance.
    let times = ["08:00", "09:30", "10:30", "11:00", "13:30", "14:00", "15:30", "16:30"];
    let notes = [
        "créneau puis circulation en ville",
        "panneaux et priorités",
        "démarrages en côte à revoir",
        "circulation en ville, giratoires",
        "stationnement en bataille",
        "autoroute",
        "conduite de nuit",
        "préparation examen",
        "marche arrière en ligne droite",
        "priorités à droite",
    ];

    let active: Vec<_> = enrollments
        .iter()
        .filter(|(_, _, status, _, _)| *status == "active")
        .collect();

    let mut booked = 0;
    for (slot, (enrollment_id, _, _, index, _)) in active.iter().enumerate() {
        // Two or three sessions each, spread over the working week. Friday is
        // the rest day, so nothing lands on day 6.
        for n in 0..(2 + slot % 2) {
            let day_offset = ((slot * 2 + n * 3) % 6) as i64;
            let day = saturday + Duration::days(day_offset);

            db::lessons::schedule(
                &pool,
                *enrollment_id,
                &db::lessons::ScheduleInput {
                    kind: pick(&kinds, slot + n).into(),
                    scheduled_on: iso(day),
                    scheduled_time: pick(&times, slot + n * 3).into(),
                    minutes: pick(&[60i64, 90, 90, 120], slot + n),
                    plan_note: pick(&notes, index + n).into(),
                    ..Default::default()
                },
            )
            .await?;
            booked += 1;
        }
    }

    // One booked with no time at all: some schools say "Thursday" and the
    // candidate turns up when he can. The week grid has a row for these.
    if let Some((enrollment_id, _, _, _, _)) = active.first() {
        db::lessons::schedule(
            &pool,
            *enrollment_id,
            &db::lessons::ScheduleInput {
                kind: "theory".into(),
                scheduled_on: iso(saturday + Duration::days(3)),
                scheduled_time: String::new(),
                minutes: 120,
                plan_note: "révision générale".into(),
                ..Default::default()
            },
        )
        .await?;
        booked += 1;
    }
    println!("  {booked} session(s) booked this week");

    // ---------------------------------------------------------------- expenses
    for (n, (amount, category, description)) in [
        (4_500i64, "fuel", "Plein Clio + Symbol"),
        (12_000, "rent", "Loyer du mois"),
        (3_200, "maintenance", "Vidange Symbol"),
        (2_800, "fuel", "Carburant semaine"),
        (18_000, "insurance", "Assurance flotte"),
        (6_000, "salaries", "Acompte moniteur"),
        (1_400, "supplies", "Fournitures bureau"),
        (5_600, "fuel", "Carburant semaine"),
        (9_000, "maintenance", "Plaquettes de frein"),
        (2_200, "supplies", "Imprimés et registres"),
    ]
    .iter()
    .enumerate()
    {
        db::expenses::create(
            &pool,
            &db::expenses::ExpenseInput {
                category_code: (*category).into(),
                amount_centimes: amount * DA,
                spent_on: iso(today - Duration::days((n as i64 * 3) % 28)),
                description: (*description).into(),
                ..Default::default()
            },
        )
        .await?;
    }

    let r = db::reports::receivables(&pool).await?;
    println!(
        "  receivables: {} enrolment(s) owing {} DA",
        r.enrollments_owing,
        r.total_owed_centimes / DA
    );
    println!("done: {} candidate(s)", db::candidates::count(&pool, "").await?);
    Ok(())
}
