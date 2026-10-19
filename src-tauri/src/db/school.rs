//! The school's own details and its price list.
//!
//! Everything a school needs to make the app theirs: the letterhead that goes
//! on every printed receipt, and the categories and prices they actually
//! charge. Until this is editable in the app, a school is printing somebody
//! else's name on its paperwork.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct School {
    pub name_fr: String,
    pub name_ar: String,
    pub address_fr: String,
    pub address_ar: String,
    pub commune: String,
    pub wilaya_code: String,
    pub phone: String,
    pub email: String,
    pub agrement_number: String,
    pub default_language: String,
    pub receipt_prefix: String,
    pub next_receipt_number: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SchoolInput {
    pub name_fr: String,
    pub name_ar: String,
    pub address_fr: String,
    pub address_ar: String,
    pub commune: String,
    pub wilaya_code: String,
    pub phone: String,
    pub email: String,
    pub agrement_number: String,
    pub default_language: String,
    pub receipt_prefix: String,
    /// Only applied before the first payment — see `update`.
    pub next_receipt_number: i64,
}

const COLUMNS: &str = "name_fr, name_ar, address_fr, address_ar, commune, wilaya_code, \
                       phone, email, agrement_number, default_language, receipt_prefix, \
                       next_receipt_number";

pub async fn get(pool: &SqlitePool) -> Result<School, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM school WHERE id = 1");
    Ok(sqlx::query_as::<_, School>(&sql).fetch_one(pool).await?)
}

/// True once any payment exists, which freezes the receipt counter.
pub async fn has_payments(pool: &SqlitePool) -> Result<bool, AppError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payments")
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
}

fn validate(input: &SchoolInput) -> Result<(), AppError> {
    if input.name_fr.trim().is_empty() && input.name_ar.trim().is_empty() {
        return Err(AppError::field("settings.error.nameRequired", "nameFr"));
    }
    let lang = input.default_language.trim();
    if !lang.is_empty() && lang != "fr" && lang != "ar" {
        return Err(AppError::field(
            "settings.error.languageInvalid",
            "defaultLanguage",
        ));
    }
    if input.next_receipt_number < 1 {
        return Err(AppError::field(
            "settings.error.receiptNumberPositive",
            "nextReceiptNumber",
        ));
    }
    Ok(())
}

pub async fn update(pool: &SqlitePool, input: &SchoolInput) -> Result<School, AppError> {
    validate(input)?;

    let current = get(pool).await?;
    let now = Utc::now().to_rfc3339();

    // The receipt counter is frozen once money has moved. Letting an owner
    // reset it mid-year would produce a duplicate receipt number, and a
    // duplicate is worse than a gap: two candidates holding the same receipt.
    // Before the first payment it is theirs to set, so a school continuing a
    // paper book can carry its numbering over.
    let next_receipt_number = if has_payments(pool).await? {
        if input.next_receipt_number != current.next_receipt_number {
            return Err(AppError::field(
                "settings.error.receiptNumberLocked",
                "nextReceiptNumber",
            ));
        }
        current.next_receipt_number
    } else {
        input.next_receipt_number
    };

    let language = if input.default_language.trim().is_empty() {
        current.default_language.clone()
    } else {
        input.default_language.trim().to_string()
    };

    sqlx::query(
        "UPDATE school SET
            name_fr = ?1, name_ar = ?2, address_fr = ?3, address_ar = ?4,
            commune = ?5, wilaya_code = ?6, phone = ?7, email = ?8,
            agrement_number = ?9, default_language = ?10, receipt_prefix = ?11,
            next_receipt_number = ?12, updated_at = ?13
         WHERE id = 1",
    )
    .bind(input.name_fr.trim())
    .bind(input.name_ar.trim())
    .bind(input.address_fr.trim())
    .bind(input.address_ar.trim())
    .bind(input.commune.trim())
    .bind(input.wilaya_code.trim())
    .bind(input.phone.trim())
    .bind(input.email.trim())
    .bind(input.agrement_number.trim())
    .bind(&language)
    .bind(input.receipt_prefix.trim())
    .bind(next_receipt_number)
    .bind(&now)
    .execute(pool)
    .await?;

    get(pool).await
}

/* ----------------------------- price list ----------------------------- */

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub code: String,
    pub code_ar: String,
    pub name_fr: String,
    pub name_ar: String,
    pub theory_hours_target: i64,
    pub practical_hours_target: i64,
    pub default_price_centimes: i64,
    pub is_offered: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryInput {
    pub code_ar: String,
    pub name_fr: String,
    pub name_ar: String,
    pub theory_hours_target: i64,
    pub practical_hours_target: i64,
    pub default_price_centimes: i64,
    pub is_offered: bool,
}

const CATEGORY_COLUMNS: &str = "code, code_ar, name_fr, name_ar, theory_hours_target, \
     practical_hours_target, default_price_centimes, is_offered, sort_order";

/// Every category, including ones the school does not offer, because settings
/// is where they turn them back on.
pub async fn all_categories(pool: &SqlitePool) -> Result<Vec<Category>, AppError> {
    let sql = format!("SELECT {CATEGORY_COLUMNS} FROM licence_categories ORDER BY sort_order");
    Ok(sqlx::query_as::<_, Category>(&sql).fetch_all(pool).await?)
}

/// Update one category's price and hour targets.
///
/// These are the school's own price list, not a regulatory table. Editing them
/// never changes an existing enrolment: enrolment snapshots price and targets
/// at the moment it is created (see db/enrollments.rs).
pub async fn update_category(
    pool: &SqlitePool,
    code: &str,
    input: &CategoryInput,
) -> Result<Category, AppError> {
    if input.theory_hours_target < 0 || input.practical_hours_target < 0 {
        return Err(AppError::field("settings.error.hoursNegative", "hours"));
    }
    if input.default_price_centimes < 0 {
        return Err(AppError::field("settings.error.priceNegative", "price"));
    }
    if input.name_fr.trim().is_empty() && input.name_ar.trim().is_empty() {
        return Err(AppError::field("settings.error.nameRequired", "nameFr"));
    }

    let result = sqlx::query(
        "UPDATE licence_categories SET
            code_ar = ?1, name_fr = ?2, name_ar = ?3,
            theory_hours_target = ?4, practical_hours_target = ?5,
            default_price_centimes = ?6, is_offered = ?7
         WHERE code = ?8",
    )
    .bind(input.code_ar.trim())
    .bind(input.name_fr.trim())
    .bind(input.name_ar.trim())
    .bind(input.theory_hours_target)
    .bind(input.practical_hours_target)
    .bind(input.default_price_centimes)
    .bind(i64::from(input.is_offered))
    .bind(code)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let sql = format!("SELECT {CATEGORY_COLUMNS} FROM licence_categories WHERE code = ?1");
    Ok(sqlx::query_as::<_, Category>(&sql)
        .bind(code)
        .fetch_one(pool)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{candidates, enrollments, payments, test_pool};
    use crate::domain::candidate::CandidateInput;

    const DA: i64 = 100;

    fn named(name: &str) -> SchoolInput {
        SchoolInput {
            name_fr: name.into(),
            next_receipt_number: 1,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn a_fresh_install_has_an_empty_school() {
        let pool = test_pool().await;
        let s = get(&pool).await.unwrap();
        assert_eq!(s.name_fr, "");
        assert_eq!(s.next_receipt_number, 1);
    }

    #[tokio::test]
    async fn the_school_can_name_itself() {
        let pool = test_pool().await;
        let s = update(
            &pool,
            &SchoolInput {
                name_ar: "مدرسة الأمل".into(),
                phone: "0551 23 45 67".into(),
                agrement_number: "16/2019/0482".into(),
                ..named("Auto-École El Amel")
            },
        )
        .await
        .unwrap();

        assert_eq!(s.name_fr, "Auto-École El Amel");
        assert_eq!(s.name_ar, "مدرسة الأمل");
        assert_eq!(s.agrement_number, "16/2019/0482");
    }

    #[tokio::test]
    async fn a_school_with_only_an_arabic_name_is_accepted() {
        let pool = test_pool().await;
        let s = update(
            &pool,
            &SchoolInput {
                name_ar: "مدرسة الأمل".into(),
                next_receipt_number: 1,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(s.name_ar, "مدرسة الأمل");
    }

    #[tokio::test]
    async fn a_school_with_no_name_at_all_is_rejected() {
        let pool = test_pool().await;
        assert!(update(&pool, &SchoolInput { next_receipt_number: 1, ..Default::default() })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn the_receipt_counter_can_be_set_before_the_first_payment() {
        // A school carrying numbering over from a paper book.
        let pool = test_pool().await;
        let s = update(
            &pool,
            &SchoolInput { next_receipt_number: 250, ..named("El Amel") },
        )
        .await
        .unwrap();
        assert_eq!(s.next_receipt_number, 250);
    }

    #[tokio::test]
    async fn the_receipt_counter_is_frozen_once_money_has_moved() {
        // Resetting it mid-year would hand two candidates the same receipt
        // number, which is worse than a gap.
        let pool = test_pool().await;
        update(&pool, &named("El Amel")).await.unwrap();

        let candidate = candidates::create(
            &pool,
            &CandidateInput {
                last_name_fr: "BENALI".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let e = enrollments::create(
            &pool,
            candidate.id,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        payments::create(
            &pool,
            e.id,
            &payments::PaymentInput {
                amount_centimes: 1_000 * DA,
                paid_on: "2026-10-18".into(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let attempt = update(
            &pool,
            &SchoolInput { next_receipt_number: 1, ..named("El Amel") },
        )
        .await;
        assert!(attempt.is_err(), "counter must be locked after a payment");

        // Everything else still saves, with the counter left alone.
        let ok = update(
            &pool,
            &SchoolInput { next_receipt_number: 2, ..named("El Amel Renamed") },
        )
        .await
        .unwrap();
        assert_eq!(ok.name_fr, "El Amel Renamed");
    }

    #[tokio::test]
    async fn an_unknown_default_language_is_rejected() {
        let pool = test_pool().await;
        assert!(update(
            &pool,
            &SchoolInput { default_language: "en".into(), ..named("El Amel") }
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn the_price_list_is_editable() {
        let pool = test_pool().await;
        let c = update_category(
            &pool,
            "B",
            &CategoryInput {
                name_fr: "Véhicule léger".into(),
                name_ar: "سيارة خفيفة".into(),
                code_ar: "ب".into(),
                theory_hours_target: 30,
                practical_hours_target: 30,
                default_price_centimes: 40_000 * DA,
                is_offered: true,
            },
        )
        .await
        .unwrap();

        assert_eq!(c.default_price_centimes, 40_000 * DA);
        assert_eq!(c.practical_hours_target, 30);
    }

    #[tokio::test]
    async fn a_category_can_be_switched_off_without_being_deleted() {
        let pool = test_pool().await;
        update_category(
            &pool,
            "D",
            &CategoryInput {
                name_fr: "Transport en commun".into(),
                is_offered: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let all = all_categories(&pool).await.unwrap();
        let d = all.iter().find(|c| c.code == "D").unwrap();
        assert_eq!(d.is_offered, 0);
        assert_eq!(all.len(), 7, "it is hidden, not removed");
    }

    #[tokio::test]
    async fn editing_the_price_list_does_not_touch_existing_enrolments() {
        let pool = test_pool().await;
        let candidate = candidates::create(
            &pool,
            &CandidateInput { last_name_fr: "BENALI".into(), ..Default::default() },
        )
        .await
        .unwrap();
        let e = enrollments::create(
            &pool,
            candidate.id,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                practical_hours_target: 30,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        update_category(
            &pool,
            "B",
            &CategoryInput {
                name_fr: "Véhicule léger".into(),
                default_price_centimes: 99_000 * DA,
                practical_hours_target: 5,
                is_offered: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let s = enrollments::summary(&pool, e.id).await.unwrap();
        assert_eq!(s.charges.agreed_price, 35_000 * DA);
        assert_eq!(s.enrollment.practical_hours_target, 30);
    }

    #[tokio::test]
    async fn a_negative_price_is_rejected() {
        let pool = test_pool().await;
        assert!(update_category(
            &pool,
            "B",
            &CategoryInput {
                name_fr: "x".into(),
                default_price_centimes: -1,
                ..Default::default()
            }
        )
        .await
        .is_err());
    }
}
