//! Candidate persistence.

use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::domain::candidate::CandidateInput;
use crate::domain::text::fold_query;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: i64,
    pub nin: Option<String>,
    pub first_name_fr: String,
    pub last_name_fr: String,
    pub first_name_ar: String,
    pub last_name_ar: String,
    pub father_name_fr: String,
    pub father_name_ar: String,
    pub mother_name_fr: String,
    pub mother_name_ar: String,
    pub birth_date: Option<String>,
    pub birth_place_fr: String,
    pub birth_place_ar: String,
    pub gender: Option<String>,
    pub blood_group: String,
    pub phone: String,
    pub phone_alt: String,
    pub address_fr: String,
    pub address_ar: String,
    pub commune: String,
    pub wilaya_code: String,
    pub photo_path: Option<String>,
    pub status: String,
    pub notes: String,
    pub registered_at: String,
    pub created_at: String,
    pub updated_at: String,
}

const COLUMNS: &str = "id, nin, first_name_fr, last_name_fr, first_name_ar, last_name_ar, \
     father_name_fr, father_name_ar, mother_name_fr, mother_name_ar, birth_date, \
     birth_place_fr, birth_place_ar, gender, blood_group, phone, phone_alt, \
     address_fr, address_ar, commune, wilaya_code, photo_path, status, notes, \
     registered_at, created_at, updated_at";

/// A page of candidates, optionally filtered by a free-text query.
///
/// The query is folded the same way `search_text` was built, so Arabic matches
/// regardless of vowelling and a phone matches however it was spaced.
pub async fn list(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Candidate>, AppError> {
    let folded = fold_query(query);
    let pattern = format!("%{folded}%");

    let sql = format!(
        "SELECT {COLUMNS} FROM candidates
         WHERE deleted_at IS NULL
           AND (?1 = '' OR search_text LIKE ?2)
         ORDER BY last_name_fr COLLATE NOCASE, first_name_fr COLLATE NOCASE, id
         LIMIT ?3 OFFSET ?4"
    );

    let rows = sqlx::query_as::<_, Candidate>(&sql)
        .bind(&folded)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}

/// Total matching the same filter, so the UI can show "12 of 240".
pub async fn count(pool: &SqlitePool, query: &str) -> Result<i64, AppError> {
    let folded = fold_query(query);
    let pattern = format!("%{folded}%");

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM candidates
         WHERE deleted_at IS NULL AND (?1 = '' OR search_text LIKE ?2)",
    )
    .bind(&folded)
    .bind(&pattern)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Candidate, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM candidates WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Candidate>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// Candidates that look like the one being entered.
///
/// Shown as a WARNING, never a block: two different people genuinely can share
/// a name, and refusing the second one would push the secretary back to paper.
pub async fn find_possible_duplicates(
    pool: &SqlitePool,
    input: &CandidateInput,
    exclude_id: Option<i64>,
) -> Result<Vec<Candidate>, AppError> {
    let nin = input.normalised_nin();

    // Match on individual name TOKENS, not on the whole name as one string.
    //
    // `search_text` is built first-name-then-surname, but people are entered
    // and searched for surname-first just as often. Matching the joined string
    // silently found nothing whenever the order differed — caught by
    // `warns_about_a_likely_duplicate_without_blocking_it`.
    let tokens: Vec<String> = fold_query(&format!(
        "{} {} {} {}",
        input.last_name_fr, input.first_name_fr, input.last_name_ar, input.first_name_ar
    ))
    .split_whitespace()
    .take(6)
    .map(str::to_string)
    .collect();

    if nin.is_none() && tokens.is_empty() {
        return Ok(Vec::new());
    }

    let name_clause = if tokens.is_empty() {
        "0".to_string()
    } else {
        tokens
            .iter()
            .map(|_| "search_text LIKE ?")
            .collect::<Vec<_>>()
            .join(" AND ")
    };

    let sql = format!(
        "SELECT {COLUMNS} FROM candidates
         WHERE deleted_at IS NULL
           AND (? IS NULL OR id <> ?)
           AND (
                 (? IS NOT NULL AND nin_digits = ?)
              OR ({name_clause})
           )
         ORDER BY id
         LIMIT 10"
    );

    let mut q = sqlx::query_as::<_, Candidate>(&sql)
        .bind(exclude_id)
        .bind(exclude_id)
        .bind(nin.as_deref())
        .bind(nin.as_deref());

    for token in &tokens {
        q = q.bind(format!("%{token}%"));
    }

    Ok(q.fetch_all(pool).await?)
}

/// SQLite reports a violated partial unique index by message, not by code, so
/// this is the one place that string-matches it — and only to turn it into an
/// actionable message instead of "database error".
fn map_unique_violation(error: sqlx::Error) -> AppError {
    let text = error.to_string();
    if text.contains("UNIQUE") && text.contains("nin_digits") {
        AppError::Duplicate("candidates.error.ninDuplicate")
    } else {
        AppError::Database(error)
    }
}

pub async fn create(pool: &SqlitePool, input: &CandidateInput) -> Result<Candidate, AppError> {
    let input = input.trimmed();
    crate::domain::candidate::validate(&input)?;

    let now = Utc::now().to_rfc3339();
    let status = if input.status.is_empty() {
        "active".to_string()
    } else {
        input.status.clone()
    };
    let gender = if input.gender.is_empty() {
        None
    } else {
        Some(input.gender.clone())
    };
    let birth_date = if input.birth_date.is_empty() {
        None
    } else {
        Some(input.birth_date.clone())
    };

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO candidates (
            nin, nin_digits, first_name_fr, last_name_fr, first_name_ar, last_name_ar,
            father_name_fr, father_name_ar, mother_name_fr, mother_name_ar,
            birth_date, birth_place_fr, birth_place_ar, gender, blood_group,
            phone, phone_alt, address_fr, address_ar, commune, wilaya_code,
            status, notes, search_text, registered_at, created_at, updated_at
         ) VALUES (
            ?1, ?27, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26
         )
         RETURNING id",
    )
    .bind(input.nin_as_typed())
    .bind(&input.first_name_fr)
    .bind(&input.last_name_fr)
    .bind(&input.first_name_ar)
    .bind(&input.last_name_ar)
    .bind(&input.father_name_fr)
    .bind(&input.father_name_ar)
    .bind(&input.mother_name_fr)
    .bind(&input.mother_name_ar)
    .bind(birth_date)
    .bind(&input.birth_place_fr)
    .bind(&input.birth_place_ar)
    .bind(gender)
    .bind(&input.blood_group)
    .bind(&input.phone)
    .bind(&input.phone_alt)
    .bind(&input.address_fr)
    .bind(&input.address_ar)
    .bind(&input.commune)
    .bind(&input.wilaya_code)
    .bind(&status)
    .bind(&input.notes)
    .bind(input.search_text())
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .bind(input.normalised_nin()) // ?27 — nin_digits
    .fetch_one(pool)
    .await
    .map_err(map_unique_violation)?;

    let created = get(pool, row.0).await?;
    audit(pool, "candidate", row.0, "create", None, Some(&created)).await?;
    Ok(created)
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &CandidateInput,
) -> Result<Candidate, AppError> {
    let input = input.trimmed();
    crate::domain::candidate::validate(&input)?;

    let before = get(pool, id).await?;
    let now = Utc::now().to_rfc3339();

    let status = if input.status.is_empty() {
        before.status.clone()
    } else {
        input.status.clone()
    };
    let gender = if input.gender.is_empty() {
        None
    } else {
        Some(input.gender.clone())
    };
    let birth_date = if input.birth_date.is_empty() {
        None
    } else {
        Some(input.birth_date.clone())
    };

    sqlx::query(
        "UPDATE candidates SET
            nin = ?1, nin_digits = ?26,
            first_name_fr = ?2, last_name_fr = ?3, first_name_ar = ?4,
            last_name_ar = ?5, father_name_fr = ?6, father_name_ar = ?7,
            mother_name_fr = ?8, mother_name_ar = ?9, birth_date = ?10,
            birth_place_fr = ?11, birth_place_ar = ?12, gender = ?13,
            blood_group = ?14, phone = ?15, phone_alt = ?16, address_fr = ?17,
            address_ar = ?18, commune = ?19, wilaya_code = ?20, status = ?21,
            notes = ?22, search_text = ?23, updated_at = ?24
         WHERE id = ?25 AND deleted_at IS NULL",
    )
    .bind(input.nin_as_typed())
    .bind(&input.first_name_fr)
    .bind(&input.last_name_fr)
    .bind(&input.first_name_ar)
    .bind(&input.last_name_ar)
    .bind(&input.father_name_fr)
    .bind(&input.father_name_ar)
    .bind(&input.mother_name_fr)
    .bind(&input.mother_name_ar)
    .bind(birth_date)
    .bind(&input.birth_place_fr)
    .bind(&input.birth_place_ar)
    .bind(gender)
    .bind(&input.blood_group)
    .bind(&input.phone)
    .bind(&input.phone_alt)
    .bind(&input.address_fr)
    .bind(&input.address_ar)
    .bind(&input.commune)
    .bind(&input.wilaya_code)
    .bind(&status)
    .bind(&input.notes)
    .bind(input.search_text())
    .bind(&now)
    .bind(id)
    .bind(input.normalised_nin()) // ?26 — nin_digits
    .execute(pool)
    .await
    .map_err(map_unique_violation)?;

    let after = get(pool, id).await?;
    audit(pool, "candidate", id, "update", Some(&before), Some(&after)).await?;
    Ok(after)
}

/// Soft delete. A candidate has financial and legal weight, so the row stays.
///
/// Refused once any payment has been taken: payments are append-only and a
/// receipt has been handed over, so the person they belong to cannot be made
/// to disappear. Without this, deleting a candidate who owes 30 000 DA quietly
/// removes it from the receivables total and the owner is never told. The
/// alternative the UI points at is marking the enrolment `withdrawn`, which
/// takes them off the chase list while keeping the history.
///
/// When it is allowed, it CASCADES to enrolments and lessons in one
/// transaction — otherwise a deleted candidate leaves live enrolments behind
/// and the two screens disagree about whether that money is owed.
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let before = get(pool, id).await?;

    let payments: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM payments p
         JOIN enrollments e ON e.id = p.enrollment_id
         WHERE e.candidate_id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    if payments.0 > 0 {
        return Err(AppError::validation("candidates.error.hasPayments"));
    }

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE lessons SET deleted_at = ?1, updated_at = ?1
         WHERE deleted_at IS NULL
           AND enrollment_id IN (SELECT id FROM enrollments WHERE candidate_id = ?2)",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE enrollments SET deleted_at = ?1, updated_at = ?1
         WHERE candidate_id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE candidates SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    audit(pool, "candidate", id, "delete", Some(&before), None).await?;
    Ok(())
}

async fn audit(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: i64,
    action: &str,
    before: Option<&Candidate>,
    after: Option<&Candidate>,
) -> Result<(), AppError> {
    let to_json = |value: Option<&Candidate>| {
        value.and_then(|v| serde_json::to_string(v).ok())
    };

    sqlx::query(
        "INSERT INTO audit_log (at, entity_type, entity_id, action, before_json, after_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(entity_type)
    .bind(entity_id.to_string())
    .bind(action)
    .bind(to_json(before))
    .bind(to_json(after))
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;

    fn input(last_fr: &str, first_fr: &str) -> CandidateInput {
        CandidateInput {
            last_name_fr: last_fr.into(),
            first_name_fr: first_fr.into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn migrations_apply_and_seed_the_categories() {
        let pool = test_pool().await;
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM licence_categories")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, 7);
    }

    #[tokio::test]
    async fn creates_and_reads_back_a_candidate() {
        let pool = test_pool().await;
        let created = create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        assert_eq!(created.last_name_fr, "BENALI");
        assert_eq!(created.status, "active");

        let fetched = get(&pool, created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
    }

    #[tokio::test]
    async fn finds_an_arabic_name_typed_without_vowels() {
        let pool = test_pool().await;
        create(
            &pool,
            &CandidateInput {
                last_name_ar: "بْن عَلِي".into(),
                last_name_fr: "BENALI".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let found = list(&pool, "بن علي", 20, 0).await.unwrap();
        assert_eq!(found.len(), 1, "vowelled name should match bare query");
    }

    #[tokio::test]
    async fn finds_a_phone_typed_without_spaces() {
        let pool = test_pool().await;
        create(
            &pool,
            &CandidateInput {
                phone: "0551 23 45 67".into(),
                ..input("BENALI", "Yacine")
            },
        )
        .await
        .unwrap();

        assert_eq!(list(&pool, "0551234567", 20, 0).await.unwrap().len(), 1);
        assert_eq!(list(&pool, "0551 23", 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn search_is_case_and_accent_insensitive() {
        let pool = test_pool().await;
        create(&pool, &input("Benaïssa", "Léïla")).await.unwrap();
        assert_eq!(list(&pool, "BENAISSA", 20, 0).await.unwrap().len(), 1);
        assert_eq!(list(&pool, "leila", 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_empty_query_returns_everyone() {
        let pool = test_pool().await;
        create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        create(&pool, &input("KADRI", "Amine")).await.unwrap();
        assert_eq!(list(&pool, "", 20, 0).await.unwrap().len(), 2);
        assert_eq!(count(&pool, "").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn rejects_a_second_candidate_with_the_same_nin() {
        let pool = test_pool().await;
        let nin = "109 2004 1600 5482 130"; // 18 digits
        create(
            &pool,
            &CandidateInput { nin: nin.into(), ..input("BENALI", "Yacine") },
        )
        .await
        .unwrap();

        let second = create(
            &pool,
            &CandidateInput { nin: nin.into(), ..input("KADRI", "Amine") },
        )
        .await;

        assert!(matches!(second, Err(AppError::Duplicate(_))));
    }

    #[tokio::test]
    async fn keeps_the_nin_exactly_as_the_school_typed_it() {
        // An official identity number must print the way the secretary wrote
        // it, so it can be compared against the paper document.
        let pool = test_pool().await;
        let typed = "109 2004 1600 5482 130";
        let created = create(
            &pool,
            &CandidateInput { nin: typed.into(), ..input("BENALI", "Yacine") },
        )
        .await
        .unwrap();
        assert_eq!(created.nin.as_deref(), Some(typed));
    }

    #[tokio::test]
    async fn the_same_nin_spaced_differently_is_still_a_duplicate() {
        let pool = test_pool().await;
        create(
            &pool,
            &CandidateInput {
                nin: "109 2004 1600 5482 130".into(),
                ..input("BENALI", "Yacine")
            },
        )
        .await
        .unwrap();

        let second = create(
            &pool,
            &CandidateInput {
                nin: "109200416005482130".into(), // same number, no spaces
                ..input("KADRI", "Amine")
            },
        )
        .await;

        assert!(matches!(second, Err(AppError::Duplicate(_))));
    }

    #[tokio::test]
    async fn a_nin_is_searchable_however_it_was_typed() {
        let pool = test_pool().await;
        create(
            &pool,
            &CandidateInput {
                nin: "109 2004 1600 5482 130".into(),
                ..input("BENALI", "Yacine")
            },
        )
        .await
        .unwrap();
        assert_eq!(list(&pool, "109200416005482130", 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn allows_many_candidates_with_no_nin() {
        // A unique index over NULLs would break this; the partial index does not.
        let pool = test_pool().await;
        create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        create(&pool, &input("KADRI", "Amine")).await.unwrap();
        create(&pool, &input("SAIDI", "Nabil")).await.unwrap();
        assert_eq!(count(&pool, "").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn a_deleted_candidate_frees_their_nin_for_reuse() {
        let pool = test_pool().await;
        let nin = "109 2004 1600 5482 130"; // 18 digits
        let first = create(
            &pool,
            &CandidateInput { nin: nin.into(), ..input("BENALI", "Yacine") },
        )
        .await
        .unwrap();

        soft_delete(&pool, first.id).await.unwrap();

        // Re-registering the same person must work after a mistaken delete.
        assert!(create(
            &pool,
            &CandidateInput { nin: nin.into(), ..input("BENALI", "Yacine") },
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn soft_delete_hides_the_row_but_keeps_it() {
        let pool = test_pool().await;
        let created = create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        soft_delete(&pool, created.id).await.unwrap();

        assert_eq!(count(&pool, "").await.unwrap(), 0);
        assert!(matches!(get(&pool, created.id).await, Err(AppError::NotFound)));

        let still_there: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM candidates")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(still_there.0, 1, "row must survive for the audit trail");
    }

    #[tokio::test]
    async fn update_changes_the_row_and_refreshes_the_search_text() {
        let pool = test_pool().await;
        let created = create(&pool, &input("BENALI", "Yacine")).await.unwrap();

        update(&pool, created.id, &input("KADRI", "Amine"))
            .await
            .unwrap();

        assert_eq!(list(&pool, "BENALI", 20, 0).await.unwrap().len(), 0);
        assert_eq!(list(&pool, "KADRI", 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn warns_about_a_likely_duplicate_without_blocking_it() {
        let pool = test_pool().await;
        create(&pool, &input("BENALI", "Yacine")).await.unwrap();

        let dupes = find_possible_duplicates(&pool, &input("BENALI", "Yacine"), None)
            .await
            .unwrap();
        assert_eq!(dupes.len(), 1);

        // ...and the second registration still succeeds: two people can share a name.
        assert!(create(&pool, &input("BENALI", "Yacine")).await.is_ok());
    }

    #[tokio::test]
    async fn duplicate_check_excludes_the_row_being_edited() {
        let pool = test_pool().await;
        let created = create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        let dupes =
            find_possible_duplicates(&pool, &input("BENALI", "Yacine"), Some(created.id))
                .await
                .unwrap();
        assert!(dupes.is_empty());
    }

    #[tokio::test]
    async fn every_change_is_written_to_the_audit_log() {
        let pool = test_pool().await;
        let created = create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        update(&pool, created.id, &input("BENALI", "Yassine")).await.unwrap();
        soft_delete(&pool, created.id).await.unwrap();

        let actions: Vec<(String,)> =
            sqlx::query_as("SELECT action FROM audit_log ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        let actions: Vec<String> = actions.into_iter().map(|a| a.0).collect();
        assert_eq!(actions, vec!["create", "update", "delete"]);
    }

    #[tokio::test]
    async fn invalid_input_is_rejected_before_touching_the_database() {
        let pool = test_pool().await;
        let bad = CandidateInput { nin: "123".into(), ..input("BENALI", "Yacine") };
        assert!(create(&pool, &bad).await.is_err());
        assert_eq!(count(&pool, "").await.unwrap(), 0);
    }
}
