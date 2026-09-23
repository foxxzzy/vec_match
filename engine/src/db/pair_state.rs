use sqlx::PgPool;
use uuid::Uuid;

use crate::models::pair_state::{PairSide, PairStateRow, canonical_pair, pair_side};

/// Get the pair state row for two users, if it exists.
pub async fn get_pair_state(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<PairStateRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, PairStateRow>(
        "SELECT * FROM pair_state WHERE user_low_id = $1 AND user_high_id = $2",
    )
    .bind(low)
    .bind(high)
    .fetch_optional(pool)
    .await
}

/// Create a new pair_state row with initial state 'unseen'.
pub async fn create_pair_state(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<PairStateRow, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    sqlx::query_as::<_, PairStateRow>(
        r#"
        INSERT INTO pair_state (user_low_id, user_high_id, state)
        VALUES ($1, $2, 'unseen')
        ON CONFLICT (user_low_id, user_high_id) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await
}

/// Get or create the pair_state row for two users.
pub async fn get_or_create_pair_state(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<PairStateRow, sqlx::Error> {
    if let Some(row) = get_pair_state(pool, user_a, user_b).await? {
        return Ok(row);
    }
    create_pair_state(pool, user_a, user_b).await
}

/// Record that a user has seen the other user.
pub async fn mark_seen(
    pool: &PgPool,
    acting_user: Uuid,
    other_user: Uuid,
) -> Result<PairStateRow, sqlx::Error> {
    let (low, high) = canonical_pair(acting_user, other_user);
    let side = pair_side(acting_user, other_user);

    // Ensure the row exists
    get_or_create_pair_state(pool, acting_user, other_user).await?;

    let query = match side {
        PairSide::Low => {
            r#"
            UPDATE pair_state
            SET low_seen_at = COALESCE(low_seen_at, now()),
                updated_at = now()
            WHERE user_low_id = $1 AND user_high_id = $2
            RETURNING *
            "#
        }
        PairSide::High => {
            r#"
            UPDATE pair_state
            SET high_seen_at = COALESCE(high_seen_at, now()),
                updated_at = now()
            WHERE user_low_id = $1 AND user_high_id = $2
            RETURNING *
            "#
        }
    };

    sqlx::query_as::<_, PairStateRow>(query)
        .bind(low)
        .bind(high)
        .fetch_one(pool)
        .await
}

/// Record a decision (like, pass, block) and derive the new state.
pub async fn record_decision(
    pool: &PgPool,
    acting_user: Uuid,
    other_user: Uuid,
    decision: &str, // "like", "pass", "block"
) -> Result<PairStateRow, sqlx::Error> {
    let (low, high) = canonical_pair(acting_user, other_user);
    let side = pair_side(acting_user, other_user);

    // Ensure the row exists and mark seen
    let current = mark_seen(pool, acting_user, other_user).await?;

    // Determine new derived state
    let other_decision = match side {
        PairSide::Low => current.high_decision.as_deref(),
        PairSide::High => current.low_decision.as_deref(),
    };
    let new_state = derive_state(decision, other_decision, side);

    let is_match = new_state == "matched";

    let query = match side {
        PairSide::Low => {
            r#"
            UPDATE pair_state
            SET low_decision = $3,
                low_decided_at = COALESCE(low_decided_at, now()),
                state = $4,
                matched_at = CASE WHEN $5 THEN COALESCE(matched_at, now()) ELSE matched_at END,
                updated_at = now()
            WHERE user_low_id = $1 AND user_high_id = $2
            RETURNING *
            "#
        }
        PairSide::High => {
            r#"
            UPDATE pair_state
            SET high_decision = $3,
                high_decided_at = COALESCE(high_decided_at, now()),
                state = $4,
                matched_at = CASE WHEN $5 THEN COALESCE(matched_at, now()) ELSE matched_at END,
                updated_at = now()
            WHERE user_low_id = $1 AND user_high_id = $2
            RETURNING *
            "#
        }
    };

    sqlx::query_as::<_, PairStateRow>(query)
        .bind(low)
        .bind(high)
        .bind(decision)
        .bind(&new_state)
        .bind(is_match)
        .fetch_one(pool)
        .await
}

/// Check if a pair_state row exists for two users.
pub async fn pair_state_exists(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<bool, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pair_state WHERE user_low_id = $1 AND user_high_id = $2)",
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// ── State derivation helpers ────────────────────────────────────

/// Derive the pair state given the acting user's decision, the other side's
/// existing decision, and which side (low/high) is acting.
fn derive_state(acting_decision: &str, other_decision: Option<&str>, side: PairSide) -> String {
    if acting_decision == "block" || other_decision == Some("block") {
        return "blocked".to_string();
    }
    if acting_decision == "pass" || other_decision == Some("pass") {
        return "rejected".to_string();
    }
    match (acting_decision, other_decision) {
        ("like", Some("like")) => "matched".to_string(),
        ("like", _) => match side {
            PairSide::Low => "pending_low_like".to_string(),
            PairSide::High => "pending_high_like".to_string(),
        },
        _ => "unseen".to_string(),
    }
}
