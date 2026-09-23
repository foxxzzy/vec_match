use sqlx::PgPool;
use uuid::Uuid;

use crate::models::match_row::MatchRow;
use crate::models::pair_state::canonical_pair;

/// Create a mutual match row.
pub async fn create_match(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<MatchRow, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, MatchRow>(
        r#"
        INSERT INTO matches (user_low_id, user_high_id, matched_at)
        VALUES ($1, $2, now())
        ON CONFLICT (user_low_id, user_high_id) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await
}

/// Create or reactivate a mutual match row.
pub async fn upsert_active_match(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<MatchRow, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, MatchRow>(
        r#"
        INSERT INTO matches (user_low_id, user_high_id, matched_at)
        VALUES ($1, $2, now())
        ON CONFLICT (user_low_id, user_high_id) DO UPDATE
        SET unmatched_at = NULL,
            matched_at = COALESCE(matches.matched_at, EXCLUDED.matched_at)
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await
}

/// Get a match row for a pair.
pub async fn get_match(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<MatchRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, MatchRow>(
        "SELECT * FROM matches WHERE user_low_id = $1 AND user_high_id = $2",
    )
    .bind(low)
    .bind(high)
    .fetch_optional(pool)
    .await
}

/// Get all matches for a user.
pub async fn get_matches_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<MatchRow>, sqlx::Error> {
    sqlx::query_as::<_, MatchRow>(
        r#"
        SELECT * FROM matches
        WHERE (user_low_id = $1 OR user_high_id = $1)
          AND unmatched_at IS NULL
        ORDER BY matched_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Unmatch a pair (soft delete by setting unmatched_at).
pub async fn unmatch(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<MatchRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, MatchRow>(
        r#"
        UPDATE matches
        SET unmatched_at = now()
        WHERE user_low_id = $1 AND user_high_id = $2
          AND unmatched_at IS NULL
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_optional(pool)
    .await
}

/// Check if a match exists for a pair.
pub async fn match_exists(pool: &PgPool, user_a: Uuid, user_b: Uuid) -> Result<bool, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM matches WHERE user_low_id = $1 AND user_high_id = $2 AND unmatched_at IS NULL)",
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
