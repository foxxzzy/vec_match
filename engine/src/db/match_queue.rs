use sqlx::PgPool;
use uuid::Uuid;

use crate::models::match_queue::MatchQueueRow;

/// Insert a batch of ranked candidates into the match queue.
pub async fn enqueue_candidates(
    pool: &PgPool,
    user_id: Uuid,
    candidates: &[(Uuid, i32, f64)], // (candidate_user_id, position, score)
) -> Result<u64, sqlx::Error> {
    let mut total = 0u64;
    for (candidate_id, position, score) in candidates {
        let result = sqlx::query(
            r#"
            INSERT INTO match_queue (user_id, candidate_user_id, position, status, score)
            VALUES ($1, $2, $3, 'queued', $4)
            ON CONFLICT (user_id, candidate_user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(candidate_id)
        .bind(position)
        .bind(score)
        .execute(pool)
        .await?;
        total += result.rows_affected();
    }
    Ok(total)
}

/// Fetch queued/visible candidates for a user, ordered by position.
pub async fn fetch_queue_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<MatchQueueRow>, sqlx::Error> {
    sqlx::query_as::<_, MatchQueueRow>(
        r#"
        SELECT *
        FROM match_queue
        WHERE user_id = $1
          AND status IN ('queued', 'visible')
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

/// Update queue entry status (e.g. queued -> visible -> consumed).
pub async fn update_queue_status(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
    new_status: &str,
) -> Result<Option<MatchQueueRow>, sqlx::Error> {
    sqlx::query_as::<_, MatchQueueRow>(
        r#"
        UPDATE match_queue
        SET status = $3,
            consumed_at = CASE WHEN $3 = 'consumed' THEN now() ELSE consumed_at END
        WHERE user_id = $1
          AND candidate_user_id = $2
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(candidate_user_id)
    .bind(new_status)
    .fetch_optional(pool)
    .await
}

/// Remove a queue entry after a user interacts (like, pass, block) with a candidate.
/// This is the primary method to call when a user makes a decision on someone in their queue.
pub async fn remove_after_interaction(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM match_queue
        WHERE user_id = $1
          AND candidate_user_id = $2
        "#,
    )
    .bind(user_id)
    .bind(candidate_user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Remove a specific queue entry (e.g. after match or block).
pub async fn remove_queue_entry(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM match_queue
        WHERE user_id = $1
          AND candidate_user_id = $2
        "#,
    )
    .bind(user_id)
    .bind(candidate_user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Remove queue entries for a pair in both directions (after match/block).
pub async fn remove_queue_entries_for_pair(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM match_queue
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

/// Count active queue entries for a user.
pub async fn count_queue_for_user(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM match_queue
        WHERE user_id = $1
          AND status IN ('queued', 'visible')
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
