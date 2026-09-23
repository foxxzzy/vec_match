use sqlx::PgPool;
use uuid::Uuid;

use crate::db::interested_pairs::mark_questions_answered;
use crate::models::interested_pair::InterestedPairRow;
use crate::models::match_question::{
    InterestedPairAnswerWithQuestionRow, InterestedPairQuestionAnswerRow,
    MatchQuestionForAnswerRow, UserMatchQuestionRow,
};
use crate::models::pair_state::canonical_pair;

pub async fn create_user_match_question(
    pool: &PgPool,
    user_id: Uuid,
    question: &str,
    position: i32,
    is_required: bool,
) -> Result<UserMatchQuestionRow, sqlx::Error> {
    validate_non_empty("question", question)?;

    sqlx::query_as::<_, UserMatchQuestionRow>(
        r#"
        INSERT INTO user_match_questions (user_id, question, position, is_required)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(question)
    .bind(position)
    .bind(is_required)
    .fetch_one(pool)
    .await
}

pub async fn get_user_match_question(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Option<UserMatchQuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, UserMatchQuestionRow>(
        r#"
        SELECT *
        FROM user_match_questions
        WHERE question_id = $1
        "#,
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_user_match_questions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserMatchQuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, UserMatchQuestionRow>(
        r#"
        SELECT *
        FROM user_match_questions
        WHERE user_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_active_user_match_questions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserMatchQuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, UserMatchQuestionRow>(
        r#"
        SELECT *
        FROM user_match_questions
        WHERE user_id = $1
          AND is_active = true
        ORDER BY position ASC, created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn update_user_match_question(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
    question: Option<&str>,
    position: Option<i32>,
    is_required: Option<bool>,
    is_active: Option<bool>,
) -> Result<Option<UserMatchQuestionRow>, sqlx::Error> {
    if let Some(question) = question {
        validate_non_empty("question", question)?;
    }

    sqlx::query_as::<_, UserMatchQuestionRow>(
        r#"
        UPDATE user_match_questions
        SET question = COALESCE($3, question),
            position = COALESCE($4, position),
            is_required = COALESCE($5, is_required),
            is_active = COALESCE($6, is_active),
            updated_at = now()
        WHERE user_id = $1
          AND question_id = $2
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(question_id)
    .bind(question)
    .bind(position)
    .bind(is_required)
    .bind(is_active)
    .fetch_optional(pool)
    .await
}

pub async fn deactivate_user_match_question(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<Option<UserMatchQuestionRow>, sqlx::Error> {
    update_user_match_question(pool, user_id, question_id, None, None, None, Some(false)).await
}

pub async fn delete_user_match_question(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM user_match_questions
        WHERE user_id = $1
          AND question_id = $2
        "#,
    )
    .bind(user_id)
    .bind(question_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn fetch_questions_to_answer(
    pool: &PgPool,
    answering_user_id: Uuid,
    asked_by_user_id: Uuid,
) -> Result<Vec<MatchQuestionForAnswerRow>, sqlx::Error> {
    let (low, high) = canonical_pair(answering_user_id, asked_by_user_id);

    sqlx::query_as::<_, MatchQuestionForAnswerRow>(
        r#"
        SELECT
            q.question_id,
            q.user_id AS asked_by_user_id,
            q.question,
            q.position,
            q.is_required,
            a.answer AS existing_answer,
            a.updated_at AS answered_at
        FROM user_match_questions q
        LEFT JOIN interested_pair_question_answers a
          ON a.user_low_id = $3
         AND a.user_high_id = $4
         AND a.question_id = q.question_id
         AND a.answered_by_user_id = $1
        WHERE q.user_id = $2
          AND q.is_active = true
        ORDER BY q.position ASC, q.created_at ASC
        "#,
    )
    .bind(answering_user_id)
    .bind(asked_by_user_id)
    .bind(low)
    .bind(high)
    .fetch_all(pool)
    .await
}

pub async fn upsert_question_answer(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
    question_id: Uuid,
    answer: &str,
) -> Result<InterestedPairQuestionAnswerRow, sqlx::Error> {
    validate_non_empty("answer", answer)?;

    let (low, high) = canonical_pair(answered_by_user_id, asked_by_user_id);

    sqlx::query_as::<_, InterestedPairQuestionAnswerRow>(
        r#"
        INSERT INTO interested_pair_question_answers (
            user_low_id,
            user_high_id,
            question_id,
            asked_by_user_id,
            answered_by_user_id,
            answer
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (user_low_id, user_high_id, question_id) DO UPDATE
        SET answer = EXCLUDED.answer,
            answered_by_user_id = EXCLUDED.answered_by_user_id,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(low)
    .bind(high)
    .bind(question_id)
    .bind(asked_by_user_id)
    .bind(answered_by_user_id)
    .bind(answer)
    .fetch_one(pool)
    .await
}

pub async fn upsert_question_answer_and_mark_complete_if_ready(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
    question_id: Uuid,
    answer: &str,
) -> Result<(InterestedPairQuestionAnswerRow, Option<InterestedPairRow>), sqlx::Error> {
    let answer_row = upsert_question_answer(
        pool,
        answered_by_user_id,
        asked_by_user_id,
        question_id,
        answer,
    )
    .await?;

    let maybe_pair =
        mark_questions_answered_if_required_complete(pool, answered_by_user_id, asked_by_user_id)
            .await?;

    Ok((answer_row, maybe_pair))
}

pub async fn mark_questions_answered_if_required_complete(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
) -> Result<Option<InterestedPairRow>, sqlx::Error> {
    if has_answered_required_questions(pool, answered_by_user_id, asked_by_user_id).await? {
        let row = mark_questions_answered(pool, answered_by_user_id, asked_by_user_id).await?;
        return Ok(Some(row));
    }

    Ok(None)
}

pub async fn has_answered_required_questions(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let remaining =
        count_unanswered_required_questions(pool, answered_by_user_id, asked_by_user_id).await?;
    Ok(remaining == 0)
}

pub async fn count_unanswered_required_questions(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let (low, high) = canonical_pair(answered_by_user_id, asked_by_user_id);

    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM user_match_questions q
        WHERE q.user_id = $1
          AND q.is_active = true
          AND q.is_required = true
          AND NOT EXISTS (
              SELECT 1
              FROM interested_pair_question_answers a
              WHERE a.user_low_id = $3
                AND a.user_high_id = $4
                AND a.question_id = q.question_id
                AND a.answered_by_user_id = $2
          )
        "#,
    )
    .bind(asked_by_user_id)
    .bind(answered_by_user_id)
    .bind(low)
    .bind(high)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn fetch_pair_question_answers(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Vec<InterestedPairAnswerWithQuestionRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairAnswerWithQuestionRow>(
        r#"
        SELECT
            a.answer_id,
            a.user_low_id,
            a.user_high_id,
            a.question_id,
            q.question,
            a.asked_by_user_id,
            a.answered_by_user_id,
            a.answer,
            a.created_at,
            a.updated_at
        FROM interested_pair_question_answers a
        JOIN user_match_questions q ON q.question_id = a.question_id
        WHERE a.user_low_id = $1
          AND a.user_high_id = $2
        ORDER BY q.position ASC, a.created_at ASC
        "#,
    )
    .bind(low)
    .bind(high)
    .fetch_all(pool)
    .await
}

pub async fn fetch_answers_for_question_owner(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
    asked_by_user_id: Uuid,
) -> Result<Vec<InterestedPairAnswerWithQuestionRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairAnswerWithQuestionRow>(
        r#"
        SELECT
            a.answer_id,
            a.user_low_id,
            a.user_high_id,
            a.question_id,
            q.question,
            a.asked_by_user_id,
            a.answered_by_user_id,
            a.answer,
            a.created_at,
            a.updated_at
        FROM interested_pair_question_answers a
        JOIN user_match_questions q ON q.question_id = a.question_id
        WHERE a.user_low_id = $1
          AND a.user_high_id = $2
          AND a.asked_by_user_id = $3
        ORDER BY q.position ASC, a.created_at ASC
        "#,
    )
    .bind(low)
    .bind(high)
    .bind(asked_by_user_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_answers_written_by_user(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
    answered_by_user_id: Uuid,
) -> Result<Vec<InterestedPairAnswerWithQuestionRow>, sqlx::Error> {
    let (low, high) = canonical_pair(user_a, user_b);

    sqlx::query_as::<_, InterestedPairAnswerWithQuestionRow>(
        r#"
        SELECT
            a.answer_id,
            a.user_low_id,
            a.user_high_id,
            a.question_id,
            q.question,
            a.asked_by_user_id,
            a.answered_by_user_id,
            a.answer,
            a.created_at,
            a.updated_at
        FROM interested_pair_question_answers a
        JOIN user_match_questions q ON q.question_id = a.question_id
        WHERE a.user_low_id = $1
          AND a.user_high_id = $2
          AND a.answered_by_user_id = $3
        ORDER BY q.position ASC, a.created_at ASC
        "#,
    )
    .bind(low)
    .bind(high)
    .bind(answered_by_user_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_question_answer(
    pool: &PgPool,
    answered_by_user_id: Uuid,
    asked_by_user_id: Uuid,
    question_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let (low, high) = canonical_pair(answered_by_user_id, asked_by_user_id);

    let result = sqlx::query(
        r#"
        DELETE FROM interested_pair_question_answers
        WHERE user_low_id = $1
          AND user_high_id = $2
          AND question_id = $3
          AND asked_by_user_id = $4
          AND answered_by_user_id = $5
        "#,
    )
    .bind(low)
    .bind(high)
    .bind(question_id)
    .bind(asked_by_user_id)
    .bind(answered_by_user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

fn validate_non_empty(field_name: &str, value: &str) -> Result<(), sqlx::Error> {
    if !value.trim().is_empty() {
        return Ok(());
    }

    Err(sqlx::Error::Protocol(
        format!("{} cannot be empty", field_name).into(),
    ))
}
