use sqlx::PgPool;
use uuid::Uuid;

use crate::models::interested_pair::InterestedPairRow;
use crate::models::pair_state::{PairSide, canonical_pair, pair_side};

pub async fn get_interested_pair(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<InterestedPairRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairRow>(
        r#"
        SELECT *
        FROM interested_pairs
        WHERE user_low_id = $1
          AND user_high_id = $2
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_optional(pool)
    .await
}

pub async fn create_interested_pair(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<InterestedPairRow, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairRow>(
        r#"
        WITH inserted AS (
        INSERT INTO interested_pairs (user_low_id, user_high_id)
        VALUES ($1, $2)
        ON CONFLICT (user_low_id, user_high_id) DO NOTHING
        RETURNING *
        )
        SELECT *
        FROM inserted

        UNION ALL

        SELECT *
        FROM interested_pairs
        WHERE user_low_id = $1
          AND user_high_id = $2
          AND NOT EXISTS (SELECT 1 FROM inserted)
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await
}

pub async fn get_or_create_interested_pair(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<InterestedPairRow, sqlx::Error> {
    if let Some(row) = get_interested_pair(pool, user_a, user_b).await? {
        return Ok(row);
    }

    create_interested_pair(pool, user_a, user_b).await
}

pub async fn interested_pair_exists(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<bool, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    let row: (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM interested_pairs
            WHERE user_low_id = $1
              AND user_high_id = $2
        )
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn mark_questions_answered(
    pool: &PgPool,
    acting_user: Uuid,
    other_user: Uuid,
) -> Result<InterestedPairRow, sqlx::Error> {
    let current = get_or_create_interested_pair(pool, acting_user, other_user).await?;
    let (low, high) = canonical_pair(acting_user, other_user);
    let side = pair_side(acting_user, other_user);

    let low_questions_answered =
        side == PairSide::Low || current.low_questions_answered_at.is_some();
    let high_questions_answered =
        side == PairSide::High || current.high_questions_answered_at.is_some();
    let new_state = derive_state(
        low_questions_answered,
        high_questions_answered,
        current.low_final_decision.as_deref(),
        current.high_final_decision.as_deref(),
    )?;

    let query = match side {
        PairSide::Low => {
            r#"
            UPDATE interested_pairs
            SET low_questions_answered_at = COALESCE(low_questions_answered_at, now()),
                state = $3,
                updated_at = now()
            WHERE user_low_id = $1
              AND user_high_id = $2
            RETURNING *
            "#
        }
        PairSide::High => {
            r#"
            UPDATE interested_pairs
            SET high_questions_answered_at = COALESCE(high_questions_answered_at, now()),
                state = $3,
                updated_at = now()
            WHERE user_low_id = $1
              AND user_high_id = $2
            RETURNING *
            "#
        }
    };

    sqlx::query_as::<_, InterestedPairRow>(query)
        .bind(low)
        .bind(high)
        .bind(new_state)
        .fetch_one(pool)
        .await
}

pub async fn record_final_decision(
    pool: &PgPool,
    acting_user: Uuid,
    other_user: Uuid,
    decision: &str,
) -> Result<InterestedPairRow, sqlx::Error> {
    validate_final_decision(decision)?;

    let current = get_or_create_interested_pair(pool, acting_user, other_user).await?;
    let (low, high) = canonical_pair(acting_user, other_user);
    let side = pair_side(acting_user, other_user);

    let low_final_decision = match side {
        PairSide::Low => Some(decision),
        PairSide::High => current.low_final_decision.as_deref(),
    };
    let high_final_decision = match side {
        PairSide::Low => current.high_final_decision.as_deref(),
        PairSide::High => Some(decision),
    };
    let new_state = derive_state(
        current.low_questions_answered_at.is_some(),
        current.high_questions_answered_at.is_some(),
        low_final_decision,
        high_final_decision,
    )?;

    let query = match side {
        PairSide::Low => {
            r#"
            UPDATE interested_pairs
            SET low_final_decision = $3,
                state = $4,
                updated_at = now()
            WHERE user_low_id = $1
              AND user_high_id = $2
            RETURNING *
            "#
        }
        PairSide::High => {
            r#"
            UPDATE interested_pairs
            SET high_final_decision = $3,
                state = $4,
                updated_at = now()
            WHERE user_low_id = $1
              AND user_high_id = $2
            RETURNING *
            "#
        }
    };

    sqlx::query_as::<_, InterestedPairRow>(query)
        .bind(low)
        .bind(high)
        .bind(decision)
        .bind(new_state)
        .fetch_one(pool)
        .await
}

pub async fn mark_interested_pair_expired(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<InterestedPairRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairRow>(
        r#"
        UPDATE interested_pairs
        SET state = 'expired',
            updated_at = now()
        WHERE user_low_id = $1
          AND user_high_id = $2
          AND state NOT IN ('confirmed', 'rejected', 'expired')
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_interested_pairs_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<InterestedPairRow>, sqlx::Error> {
    sqlx::query_as::<_, InterestedPairRow>(
        r#"
        SELECT *
        FROM interested_pairs
        WHERE user_low_id = $1
           OR user_high_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_confirmed_interested_pairs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<InterestedPairRow>, sqlx::Error> {
    sqlx::query_as::<_, InterestedPairRow>(
        r#"
        SELECT *
        FROM interested_pairs
        WHERE state = 'confirmed'
        ORDER BY updated_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

fn validate_final_decision(decision: &str) -> Result<(), sqlx::Error> {
    if decision == "yes" || decision == "no" {
        return Ok(());
    }

    Err(sqlx::Error::Protocol(
        format!(
            "invalid interested_pairs final decision '{}'; expected 'yes' or 'no'",
            decision
        )
        .into(),
    ))
}

fn derive_state(
    low_questions_answered: bool,
    high_questions_answered: bool,
    low_final_decision: Option<&str>,
    high_final_decision: Option<&str>,
) -> Result<String, sqlx::Error> {
    if low_final_decision == Some("no") || high_final_decision == Some("no") {
        return Ok("rejected".to_string());
    }

    if low_final_decision == Some("yes") && high_final_decision == Some("yes") {
        return Ok("confirmed".to_string());
    }

    if let Some(decision) = low_final_decision {
        validate_final_decision(decision)?;
    }
    if let Some(decision) = high_final_decision {
        validate_final_decision(decision)?;
    }

    if low_questions_answered && high_questions_answered {
        if low_final_decision == Some("yes") {
            return Ok("awaiting_high_final_decision".to_string());
        }
        if high_final_decision == Some("yes") {
            return Ok("awaiting_low_final_decision".to_string());
        }

        return Ok("awaiting_both_final_decisions".to_string());
    }

    if low_questions_answered {
        return Ok("awaiting_high_answers".to_string());
    }

    if high_questions_answered {
        return Ok("awaiting_low_answers".to_string());
    }

    Ok("awaiting_both_answers".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_state_tracks_answer_progress() {
        assert_eq!(
            derive_state(false, false, None, None).unwrap(),
            "awaiting_both_answers"
        );
        assert_eq!(
            derive_state(true, false, None, None).unwrap(),
            "awaiting_high_answers"
        );
        assert_eq!(
            derive_state(false, true, None, None).unwrap(),
            "awaiting_low_answers"
        );
        assert_eq!(
            derive_state(true, true, None, None).unwrap(),
            "awaiting_both_final_decisions"
        );
    }

    #[test]
    fn derive_state_tracks_final_decisions() {
        assert_eq!(
            derive_state(true, true, Some("yes"), None).unwrap(),
            "awaiting_high_final_decision"
        );
        assert_eq!(
            derive_state(true, true, None, Some("yes")).unwrap(),
            "awaiting_low_final_decision"
        );
        assert_eq!(
            derive_state(true, true, Some("yes"), Some("yes")).unwrap(),
            "confirmed"
        );
        assert_eq!(
            derive_state(true, true, Some("no"), Some("yes")).unwrap(),
            "rejected"
        );
    }
}
