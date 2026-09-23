use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::{interested_pairs, match_questions, match_queue, matches, pair_state};
use crate::models::interested_pair::InterestedPairRow;
use crate::models::match_question::InterestedPairQuestionAnswerRow;
use crate::models::match_row::MatchRow;
use crate::models::pair_state::PairStateRow;

#[derive(Debug)]
pub enum MatchingWorkflowError {
    BadRequest(String),
    NotFound(String),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for MatchingWorkflowError {
    fn from(value: sqlx::Error) -> Self {
        MatchingWorkflowError::Db(value)
    }
}

#[derive(Debug, Serialize)]
pub struct QueueDecisionOutcome {
    pub pair_state: PairStateRow,
    pub interested_pair: Option<InterestedPairRow>,
    pub removed_queue_rows: u64,
}

#[derive(Debug, Serialize)]
pub struct QuestionAnswerOutcome {
    pub answer: InterestedPairQuestionAnswerRow,
    pub interested_pair: Option<InterestedPairRow>,
    pub match_row: Option<MatchRow>,
}

#[derive(Debug, Serialize)]
pub struct FinalDecisionOutcome {
    pub interested_pair: InterestedPairRow,
    pub match_row: Option<MatchRow>,
}

pub async fn record_queue_decision(
    pool: &PgPool,
    acting_user: Uuid,
    candidate_user: Uuid,
    decision: &str,
) -> Result<QueueDecisionOutcome, MatchingWorkflowError> {
    validate_queue_decision(decision)?;

    let pair_state =
        pair_state::record_decision(pool, acting_user, candidate_user, decision).await?;

    let mut removed_queue_rows = 0;
    let mut interested_pair = None;

    if pair_state.state == "matched" {
        removed_queue_rows +=
            match_queue::remove_queue_entries_for_pair(pool, acting_user, candidate_user).await?;

        let pair =
            interested_pairs::get_or_create_interested_pair(pool, acting_user, candidate_user)
                .await?;

        interested_pair = Some(
            advance_no_required_question_sides(pool, acting_user, candidate_user, pair).await?,
        );
    } else if pair_state.state == "rejected" || pair_state.state == "blocked" {
        removed_queue_rows +=
            match_queue::remove_queue_entries_for_pair(pool, acting_user, candidate_user).await?;
    } else {
        let removed =
            match_queue::remove_after_interaction(pool, acting_user, candidate_user).await?;
        if removed {
            removed_queue_rows += 1;
        }
    }

    Ok(QueueDecisionOutcome {
        pair_state,
        interested_pair,
        removed_queue_rows,
    })
}

pub async fn submit_question_answer(
    pool: &PgPool,
    answered_by_user: Uuid,
    asked_by_user: Uuid,
    question_id: Uuid,
    answer: &str,
) -> Result<QuestionAnswerOutcome, MatchingWorkflowError> {
    let pair = ensure_interested_pair_exists(pool, answered_by_user, asked_by_user).await?;
    ensure_interested_pair_can_accept_answers(&pair)?;
    ensure_question_can_be_answered(pool, asked_by_user, question_id).await?;

    let (answer_row, maybe_pair) =
        match_questions::upsert_question_answer_and_mark_complete_if_ready(
            pool,
            answered_by_user,
            asked_by_user,
            question_id,
            answer,
        )
        .await?;

    let match_row = if let Some(pair) = maybe_pair.as_ref() {
        create_match_if_confirmed(pool, pair).await?
    } else {
        None
    };

    Ok(QuestionAnswerOutcome {
        answer: answer_row,
        interested_pair: maybe_pair,
        match_row,
    })
}

pub async fn submit_final_decision(
    pool: &PgPool,
    acting_user: Uuid,
    other_user: Uuid,
    decision: &str,
) -> Result<FinalDecisionOutcome, MatchingWorkflowError> {
    validate_final_decision(decision)?;

    let current = ensure_interested_pair_exists(pool, acting_user, other_user).await?;
    if current.state == "rejected" || current.state == "expired" {
        return Err(MatchingWorkflowError::BadRequest(format!(
            "cannot submit a final decision for an interested pair in '{}' state",
            current.state
        )));
    }

    if current.state == "confirmed" {
        if decision != "yes" {
            return Err(MatchingWorkflowError::BadRequest(
                "confirmed interested pairs cannot be changed to no".to_string(),
            ));
        }

        let match_row = create_match_if_confirmed(pool, &current).await?;
        return Ok(FinalDecisionOutcome {
            interested_pair: current,
            match_row,
        });
    }

    if current.low_questions_answered_at.is_none() || current.high_questions_answered_at.is_none() {
        return Err(MatchingWorkflowError::BadRequest(
            "both users must answer required questions before final decisions".to_string(),
        ));
    }

    let interested_pair =
        interested_pairs::record_final_decision(pool, acting_user, other_user, decision).await?;
    let match_row = create_match_if_confirmed(pool, &interested_pair).await?;

    Ok(FinalDecisionOutcome {
        interested_pair,
        match_row,
    })
}

async fn advance_no_required_question_sides(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
    fallback: InterestedPairRow,
) -> Result<InterestedPairRow, MatchingWorkflowError> {
    let mut latest = fallback;

    if let Some(pair) =
        match_questions::mark_questions_answered_if_required_complete(pool, user_a, user_b).await?
    {
        latest = pair;
    }

    if let Some(pair) =
        match_questions::mark_questions_answered_if_required_complete(pool, user_b, user_a).await?
    {
        latest = pair;
    }

    Ok(latest)
}

async fn ensure_interested_pair_exists(
    pool: &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<InterestedPairRow, MatchingWorkflowError> {
    match interested_pairs::get_interested_pair(pool, user_a, user_b).await? {
        Some(row) => Ok(row),
        None => Err(MatchingWorkflowError::NotFound(
            "interested pair does not exist for these users".to_string(),
        )),
    }
}

async fn ensure_question_can_be_answered(
    pool: &PgPool,
    asked_by_user: Uuid,
    question_id: Uuid,
) -> Result<(), MatchingWorkflowError> {
    let Some(question) = match_questions::get_user_match_question(pool, question_id).await? else {
        return Err(MatchingWorkflowError::NotFound(
            "question does not exist".to_string(),
        ));
    };

    if question.user_id != asked_by_user {
        return Err(MatchingWorkflowError::BadRequest(
            "question does not belong to asked_by_user_id".to_string(),
        ));
    }

    if !question.is_active {
        return Err(MatchingWorkflowError::BadRequest(
            "question is not active".to_string(),
        ));
    }

    Ok(())
}

fn ensure_interested_pair_can_accept_answers(
    pair: &InterestedPairRow,
) -> Result<(), MatchingWorkflowError> {
    if pair.state == "rejected" || pair.state == "expired" || pair.state == "confirmed" {
        return Err(MatchingWorkflowError::BadRequest(format!(
            "cannot answer questions for an interested pair in '{}' state",
            pair.state
        )));
    }

    Ok(())
}

async fn create_match_if_confirmed(
    pool: &PgPool,
    interested_pair: &InterestedPairRow,
) -> Result<Option<MatchRow>, MatchingWorkflowError> {
    if interested_pair.state != "confirmed" {
        return Ok(None);
    }

    let match_row = matches::upsert_active_match(
        pool,
        interested_pair.user_low_id,
        interested_pair.user_high_id,
    )
    .await?;

    Ok(Some(match_row))
}

fn validate_queue_decision(decision: &str) -> Result<(), MatchingWorkflowError> {
    if decision == "like" || decision == "pass" || decision == "block" {
        return Ok(());
    }

    Err(MatchingWorkflowError::BadRequest(
        "decision must be one of: like, pass, block".to_string(),
    ))
}

fn validate_final_decision(decision: &str) -> Result<(), MatchingWorkflowError> {
    if decision == "yes" || decision == "no" {
        return Ok(());
    }

    Err(MatchingWorkflowError::BadRequest(
        "final decision must be one of: yes, no".to_string(),
    ))
}
