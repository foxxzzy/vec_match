use sqlx::PgPool;
use uuid::Uuid;

use crate::models::hard_gated_candidate::HardGatedCandidateRow;

/// Call the Supabase DB function to refill hard-gated candidates for a user.
pub async fn refill_hard_gated_candidates(
    pool: &PgPool,
    user_id: Uuid,
    limit: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT public.refill_gated_matches_for_user($1, $2)")
        .bind(user_id)
        .bind(limit)
        .execute(pool)
        .await?;
    Ok(())
}

/// Fetch all unconsumed hard-gated candidates for a user.
pub async fn fetch_unconsumed_candidates(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<HardGatedCandidateRow>, sqlx::Error> {
    sqlx::query_as::<_, HardGatedCandidateRow>(
        r#"
        SELECT *
        FROM hard_gated_candidates
        WHERE user_id = $1
          AND consumed = false
          AND (expires_at IS NULL OR expires_at > now())
        ORDER BY generated_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Mark specific hard-gated candidates as consumed.
pub async fn mark_candidates_consumed(
    pool: &PgPool,
    user_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE hard_gated_candidates
        SET consumed = TRUE,
            consumed_at = now()
        WHERE user_id = $1
          AND candidate_user_id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(candidate_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Delete consumed hard-gated candidates for a user (cleanup).
pub async fn delete_consumed_candidates(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM hard_gated_candidates
        WHERE user_id = $1
          AND consumed = true
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Delete specific hard-gated candidate rows by candidate IDs.
pub async fn delete_candidates(
    pool: &PgPool,
    user_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM hard_gated_candidates
        WHERE user_id = $1
          AND candidate_user_id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(candidate_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Count unconsumed hard-gated candidates for a user.
pub async fn count_unconsumed_candidates(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM hard_gated_candidates
        WHERE user_id = $1
          AND consumed = false
          AND (expires_at IS NULL OR expires_at > now())
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
