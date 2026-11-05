//! Candidate persistence.

use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, Sqlite, SqliteConnection, SqlitePool};

use crate::domain::candidate::CandidateInput;
use crate::domain::text::fold_query;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: i64,
    pub nin: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub father_name: String,
    pub mother_name: String,
    pub birth_date: Option<String>,
    pub birth_place: String,
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
    /// When the file was archived, or None. See migration 0011.
    pub archived_at: Option<String>,
    /// The status the list shows: "completed" once all three exams are
    /// passed, see `domain::candidate::shown_status`. Filled by `list` only;
    /// empty elsewhere, and then left out of the JSON (and the audit log).
    #[sqlx(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub shown_status: String,
}

const COLUMNS: &str = "id, nin, first_name, last_name, \
     father_name, mother_name, birth_date, \
     birth_place, gender, blood_group, phone, phone_alt, \
     address_fr, address_ar, commune, wilaya_code, photo_path, status, notes, \
     registered_at, created_at, updated_at, archived_at";

/// A page of candidates, optionally filtered by a free-text query: the
/// current ones, or with `archived` the archived ones.
///
/// The query is folded the same way `search_text` was built, so Arabic matches
/// regardless of vowelling and a phone matches however it was spaced.
pub async fn list(
    pool: &SqlitePool,
    query: &str,
    archived: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Candidate>, AppError> {
    let folded = fold_query(query);
    let pattern = format!("%{folded}%");

    let sql = format!(
        "SELECT {COLUMNS} FROM candidates
         WHERE deleted_at IS NULL
           AND (?1 = '' OR search_text LIKE ?2)
           AND (archived_at IS NOT NULL) = ?5
         ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE, id
         LIMIT ?3 OFFSET ?4"
    );

    let rows = sqlx::query_as::<_, Candidate>(&sql)
        .bind(&folded)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .bind(archived)
        .fetch_all(pool)
        .await?;

    with_shown_status(pool, rows).await
}

/// Fill `shown_status` from each candidate's exam results.
///
/// Uses the same "passed all three" as the board, so the list and the board
/// cannot disagree about who has finished.
async fn with_shown_status(
    pool: &SqlitePool,
    mut rows: Vec<Candidate>,
) -> Result<Vec<Candidate>, AppError> {
    use crate::domain::candidate::{shown_status, EnrolmentFacts};

    let finished = crate::db::exams::finished_enrollment_ids(pool).await?;
    let enrolments: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT id, candidate_id, status FROM enrollments WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    // Start "withdrawn" true and let any enrolment still running clear it, so
    // it holds only when every live enrolment is abandoned.
    let mut by: std::collections::HashMap<i64, EnrolmentFacts> = std::collections::HashMap::new();
    for (enrollment, candidate, status) in enrolments {
        let f = by.entry(candidate).or_insert(EnrolmentFacts {
            enrolled: true,
            withdrawn: true,
            finished: false,
        });
        if status != "withdrawn" {
            f.withdrawn = false;
        }
        if finished.contains(&enrollment) {
            f.finished = true;
        }
    }
    for row in &mut rows {
        row.shown_status = shown_status(by.get(&row.id).copied().unwrap_or_default()).to_string();
    }
    Ok(rows)
}

/// Total matching the same filter as `list`, so the UI can show "12 of 240".
pub async fn count_in(pool: &SqlitePool, query: &str, archived: bool) -> Result<i64, AppError> {
    let folded = fold_query(query);
    let pattern = format!("%{folded}%");

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM candidates
         WHERE deleted_at IS NULL AND (?1 = '' OR search_text LIKE ?2)
           AND (archived_at IS NOT NULL) = ?3",
    )
    .bind(&folded)
    .bind(&pattern)
    .bind(archived)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Every candidate in the books, archived or not.
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

pub async fn get<'e, E>(exec: E, id: i64) -> Result<Candidate, AppError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM candidates WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Candidate>(&sql)
        .bind(id)
        .fetch_optional(exec)
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
        "{} {}",
        input.last_name, input.first_name
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
    let mut tx = pool.begin().await?;
    let created = create_in(&mut tx, input).await?;
    tx.commit().await?;
    Ok(created)
}

/// `create` inside a transaction the caller owns, so registering a candidate
/// saves their price and first payment in the same one.
pub(crate) async fn create_in(
    conn: &mut SqliteConnection,
    input: &CandidateInput,
) -> Result<Candidate, AppError> {
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
            nin, nin_digits, first_name, last_name,
            father_name, mother_name,
            birth_date, birth_place, gender, blood_group,
            phone, phone_alt, address_fr, address_ar, commune, wilaya_code,
            status, notes, search_text, registered_at, created_at, updated_at
         ) VALUES (
            ?1, ?22, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21
         )
         RETURNING id",
    )
    .bind(input.nin_as_typed())
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.father_name)
    .bind(&input.mother_name)
    .bind(birth_date)
    .bind(&input.birth_place)
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
    .bind(input.normalised_nin()) // ?22 — nin_digits
    .fetch_one(&mut *conn)
    .await
    .map_err(map_unique_violation)?;

    let created = get(&mut *conn, row.0).await?;
    audit(&mut *conn, "candidate", row.0, "create", None, Some(&created)).await?;
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
            nin = ?1, nin_digits = ?21,
            first_name = ?2, last_name = ?3,
            father_name = ?4, mother_name = ?5, birth_date = ?6,
            birth_place = ?7, gender = ?8,
            blood_group = ?9, phone = ?10, phone_alt = ?11, address_fr = ?12,
            address_ar = ?13, commune = ?14, wilaya_code = ?15, status = ?16,
            notes = ?17, search_text = ?18, updated_at = ?19
         WHERE id = ?20 AND deleted_at IS NULL",
    )
    .bind(input.nin_as_typed())
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.father_name)
    .bind(&input.mother_name)
    .bind(birth_date)
    .bind(&input.birth_place)
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
    .bind(input.normalised_nin()) // ?21 — nin_digits
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

/// Archive candidates, or bring them back.
///
/// Only the flag changes. Enrolments, sessions, payments and exam results are
/// untouched, so the money history, Créances and Caisse read exactly what they
/// read before. All in one transaction: archiving a whole board column must
/// not stop halfway through. Already-archived files keep their original date.
pub async fn set_archived(pool: &SqlitePool, ids: &[i64], archived: bool) -> Result<(), AppError> {
    let mut befores = Vec::with_capacity(ids.len());
    for &id in ids {
        befores.push(get(pool, id).await?);
    }

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;
    for &id in ids {
        let sql = if archived {
            "UPDATE candidates SET archived_at = ?1, updated_at = ?1
             WHERE id = ?2 AND deleted_at IS NULL AND archived_at IS NULL"
        } else {
            "UPDATE candidates SET archived_at = NULL, updated_at = ?1
             WHERE id = ?2 AND deleted_at IS NULL AND archived_at IS NOT NULL"
        };
        sqlx::query(sql).bind(&now).bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;

    let action = if archived { "archive" } else { "unarchive" };
    for before in befores {
        if before.archived_at.is_some() == archived {
            continue;
        }
        let after = get(pool, before.id).await?;
        audit(pool, "candidate", before.id, action, Some(&before), Some(&after)).await?;
    }
    Ok(())
}

async fn audit<'e, E>(
    exec: E,
    entity_type: &str,
    entity_id: i64,
    action: &str,
    before: Option<&Candidate>,
    after: Option<&Candidate>,
) -> Result<(), AppError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
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
    .execute(exec)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;

    fn input(last_fr: &str, first_fr: &str) -> CandidateInput {
        CandidateInput {
            last_name: last_fr.into(),
            first_name: first_fr.into(),
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
        assert_eq!(created.last_name, "BENALI");
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
                last_name: "بْن عَلِي".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let found = list(&pool, "بن علي", false, 20, 0).await.unwrap();
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

        assert_eq!(list(&pool, "0551234567", false, 20, 0).await.unwrap().len(), 1);
        assert_eq!(list(&pool, "0551 23", false, 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn search_is_case_and_accent_insensitive() {
        let pool = test_pool().await;
        create(&pool, &input("Benaïssa", "Léïla")).await.unwrap();
        assert_eq!(list(&pool, "BENAISSA", false, 20, 0).await.unwrap().len(), 1);
        assert_eq!(list(&pool, "leila", false, 20, 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_empty_query_returns_everyone() {
        let pool = test_pool().await;
        create(&pool, &input("BENALI", "Yacine")).await.unwrap();
        create(&pool, &input("KADRI", "Amine")).await.unwrap();
        assert_eq!(list(&pool, "", false, 20, 0).await.unwrap().len(), 2);
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
        assert_eq!(list(&pool, "109200416005482130", false, 20, 0).await.unwrap().len(), 1);
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

        assert_eq!(list(&pool, "BENALI", false, 20, 0).await.unwrap().len(), 0);
        assert_eq!(list(&pool, "KADRI", false, 20, 0).await.unwrap().len(), 1);
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

    /* ------------- "Terminé" once all three exams are passed ------------- */

    /// Enrol a candidate and put him through the three exams, passing each.
    /// Returns (candidate, the circulation entry) so a test can undo the last.
    /// Returns (candidate, enrolment, the circulation entry).
    async fn through_all_three(pool: &SqlitePool) -> (i64, i64, i64) {
        use crate::db::{enrollments, exams};
        let c = create(pool, &input("BENALI", "Yacine")).await.unwrap();
        let e = enrollments::create(
            pool,
            c.id,
            &enrollments::EnrollmentInput { category_code: "B".into(), ..Default::default() },
        )
        .await
        .unwrap();
        let mut last = 0;
        for (kind, day) in [("theory", "2026-08-10"), ("maneuvers", "2026-09-10"), ("road", "2026-10-10")] {
            let session = exams::create_session(
                pool,
                &exams::ExamSessionInput { kind: kind.into(), held_on: day.into(), ..Default::default() },
            )
            .await
            .unwrap();
            let entry = exams::add_to_session(pool, session.id, e.id, 0).await.unwrap();
            exams::set_outcome(pool, entry.id, "passed", "").await.unwrap();
            last = entry.id;
        }
        (c.id, e.id, last)
    }

    async fn abandon(pool: &SqlitePool, enrollment: i64) {
        use crate::db::enrollments;
        let e = enrollments::get(pool, enrollment).await.unwrap();
        enrollments::revise(
            pool,
            enrollment,
            &enrollments::EnrollmentRevision {
                enrolled_on: e.enrolled_on,
                agreed_price_centimes: e.agreed_price_centimes,
                discount_centimes: e.discount_centimes,
                discount_reason: e.discount_reason,
                withdrawn: true,
            },
        )
        .await
        .unwrap();
    }

    async fn shown(pool: &SqlitePool, id: i64) -> String {
        list(pool, "", false, 50, 0)
            .await
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .shown_status
    }

    #[tokio::test]
    async fn the_list_says_completed_once_all_three_are_passed() {
        let pool = test_pool().await;
        let (id, _, road) = through_all_three(&pool).await;
        assert_eq!(shown(&pool, id).await, "completed");
        // Nothing was written on the candidate.
        assert_eq!(get(&pool, id).await.unwrap().status, "active");

        // A pass undone puts it back.
        crate::db::exams::set_outcome(&pool, road, "failed", "").await.unwrap();
        assert_eq!(shown(&pool, id).await, "active");
    }

    #[tokio::test]
    async fn abandoning_the_enrolment_shows_on_the_list() {
        let pool = test_pool().await;
        let (id, enrollment, _) = through_all_three(&pool).await;
        abandon(&pool, enrollment).await;
        assert_eq!(shown(&pool, id).await, "withdrawn");
    }

    #[tokio::test]
    async fn an_enrolled_candidate_is_in_training() {
        use crate::db::enrollments;
        let pool = test_pool().await;
        let c = create(&pool, &input("HAMDI", "Sara")).await.unwrap();
        assert_eq!(shown(&pool, c.id).await, "prospect", "no enrolment yet");
        enrollments::create(
            &pool,
            c.id,
            &enrollments::EnrollmentInput { category_code: "B".into(), ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(shown(&pool, c.id).await, "active");
    }
}
