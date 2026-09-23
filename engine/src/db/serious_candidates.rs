use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::serious_candidate::{SeriousCandidate, SeriousCandidateRow};

// ── Insert / Upsert ─────────────────────────────────────────────

/// Upsert a batch of backend-ranked candidates into serious_candidates.
/// If a row already exists for (user_id, candidate_user_id) and is still
/// 'available', the score and position are updated. Rows in any other
/// status are left untouched (ON CONFLICT ... DO UPDATE only when available).
pub async fn upsert_serious_candidates(
    pool: &PgPool,
    candidates: &[SeriousCandidate],
) -> Result<u64, sqlx::Error> {
    let mut total = 0u64;
    for candidate in candidates {
        let result = sqlx::query(
            r#"
            INSERT INTO serious_candidates
                (user_id, candidate_user_id, position, score, status, expires_at)
            VALUES ($1, $2, $3, $4, 'available', $5)
            ON CONFLICT (user_id, candidate_user_id) DO UPDATE
            SET position   = EXCLUDED.position,
                score      = EXCLUDED.score,
                expires_at = EXCLUDED.expires_at
            WHERE serious_candidates.status = 'available'
            "#,
        )
        .bind(candidate.user_id)
        .bind(candidate.candidate_user_id)
        .bind(candidate.position)
        .bind(candidate.score)
        .bind(candidate.expires_at)
        .execute(pool)
        .await?;
        total += result.rows_affected();
    }
    Ok(total)
}

// ── Read ────────────────────────────────────────────────────────

/// Fetch available (not expired, not released/removed) serious candidates
/// for a user, ordered by position ascending (best first).
pub async fn fetch_available_candidates(
   pool: &PgPool,
   user_id: Uuid,
   limit: i64,
) -> Result<Vec<SeriousCandidateRow>, sqlx::Error> {
   sqlx::query_as::<_, SeriousCandidateRow>(
       r#"
       SELECT
           user_id,
           candidate_user_id,
           position,
           score::double precision AS score,
           status,
           released_at,
           expires_at,
           created_at
       FROM serious_candidates
       WHERE user_id = $1
         AND status = 'available'
         AND (expires_at IS NULL OR expires_at > now())
       ORDER BY position ASC
       LIMIT $2
       "#,
   )
   .bind(user_id)
   .bind(limit)
   .fetch_all(pool)
   .await
}

/// Count available serious candidates for a user.
pub async fn count_available_candidates(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM serious_candidates
        WHERE user_id = $1
          AND status = 'available'
          AND (expires_at IS NULL OR expires_at > now())
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Get a single serious candidate row.
pub async fn get_serious_candidate(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
) -> Result<Option<SeriousCandidateRow>, sqlx::Error> {
    sqlx::query_as::<_, SeriousCandidateRow>(
        "SELECT * FROM serious_candidates WHERE user_id = $1 AND candidate_user_id = $2",
    )
    .bind(user_id)
    .bind(candidate_user_id)
    .fetch_optional(pool)
    .await
}

// ── Status updates ──────────────────────────────────────────────

/// Mark specific serious candidates as released (moved to match_queue).
pub async fn mark_released(
    pool: &PgPool,
    user_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE serious_candidates
        SET status = 'released',
            released_at = now()
        WHERE user_id = $1
          AND candidate_user_id = ANY($2)
          AND status = 'available'
        "#,
    )
    .bind(user_id)
    .bind(candidate_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Mark specific serious candidates as expired.
pub async fn mark_expired(
    pool: &PgPool,
    user_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE serious_candidates
        SET status = 'expired'
        WHERE user_id = $1
          AND candidate_user_id = ANY($2)
          AND status = 'available'
        "#,
    )
    .bind(user_id)
    .bind(candidate_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Mark specific serious candidates as removed.
pub async fn mark_removed(
    pool: &PgPool,
    user_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE serious_candidates
        SET status = 'removed'
        WHERE user_id = $1
          AND candidate_user_id = ANY($2)
          AND status = 'available'
        "#,
    )
    .bind(user_id)
    .bind(candidate_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Expire all stale rows whose expires_at has passed.
pub async fn expire_stale_candidates(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE serious_candidates
        SET status = 'expired'
        WHERE user_id = $1
          AND status = 'available'
          AND expires_at IS NOT NULL
          AND expires_at <= now()
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ── Delete / cleanup ────────────────────────────────────────────

/// Delete a single serious candidate row.
pub async fn delete_candidate(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM serious_candidates WHERE user_id = $1 AND candidate_user_id = $2")
            .bind(user_id)
            .bind(candidate_user_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// Delete all serious candidates for a user.
pub async fn delete_all_for_user(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM serious_candidates WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete expired/removed rows older than a cutoff timestamp.
pub async fn cleanup_old_candidates(
    pool: &PgPool,
    cutoff: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM serious_candidates
        WHERE status IN ('expired', 'removed')
          AND created_at < $1
        "#,
    )
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Remove serious candidate rows for a specific pair in both directions.
/// Used when a pair gets blocked/matched/rejected and should be cleared
/// from both users' shortlists.
pub async fn remove_pair_from_shortlists(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE serious_candidates
        SET status = 'removed'
        WHERE (user_id = $1 AND candidate_user_id = $2)
           OR (user_id = $2 AND candidate_user_id = $1)
        "#,
    )
    .bind(user_a)
    .bind(user_b)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
