use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::{
    interested_pairs, match_questions, match_queue, matches, pair_state, serious_candidates,
};
use crate::matching::pipeline::full_matching_algorithm;
use crate::services::matching_workflow::{
    MatchingWorkflowError, record_queue_decision, submit_final_decision, submit_question_answer,
};
use crate::services::queue_release::release_serious_candidates_to_queue;

#[derive(Debug, Deserialize)]
pub struct FullMatchingRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct MatchedCandidateResponse {
    pub candidate_user_id: Uuid,
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct FullMatchingResponse {
    pub user_id: Uuid,
    pub matched_count: usize,
    pub candidates: Vec<MatchedCandidateResponse>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseQueueRequest {
    pub user_id: Uuid,
    pub slots_needed: i64,
}

#[derive(Debug, Serialize)]
pub struct ReleaseQueueResponse {
    pub user_id: Uuid,
    pub released_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct QueueStatusRequest {
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct QueueDecisionRequest {
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub decision: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuestionRequest {
    pub user_id: Uuid,
    pub question: String,
    pub position: Option<i32>,
    pub is_required: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionsQuery {
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuestionRequest {
    pub question: Option<String>,
    pub position: Option<i32>,
    pub is_required: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitQuestionAnswerRequest {
    pub answered_by_user_id: Uuid,
    pub asked_by_user_id: Uuid,
    pub question_id: Uuid,
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct FinalDecisionRequest {
    pub user_id: Uuid,
    pub other_user_id: Uuid,
    pub decision: String,
}

#[derive(Debug, Deserialize)]
pub struct UnmatchRequest {
    pub user_id: Uuid,
    pub other_user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SeriousCandidatesQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExpireInterestedPairRequest {
    pub user_id: Uuid,
    pub other_user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub user_id: Uuid,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct RowsAffectedResponse {
    pub rows_affected: u64,
}

pub async fn run_full_matching(
    State(pool): State<PgPool>,
    Json(req): Json<FullMatchingRequest>,
) -> Response {
    let user_id = req.user_id;

    match full_matching_algorithm(pool, user_id).await {
        Ok(candidates) => {
            let candidates = candidates
                .into_iter()
                .map(|candidate| MatchedCandidateResponse {
                    candidate_user_id: candidate.candidate_user_id,
                    score: candidate.score,
                })
                .collect::<Vec<_>>();

            (
                StatusCode::OK,
                Json(FullMatchingResponse {
                    user_id,
                    matched_count: candidates.len(),
                    candidates,
                }),
            )
                .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn release_queue_candidates(
    State(pool): State<PgPool>,
    Json(req): Json<ReleaseQueueRequest>,
) -> Response {
    match release_serious_candidates_to_queue(&pool, req.user_id, req.slots_needed).await {
        Ok(released_count) => (
            StatusCode::OK,
            Json(ReleaseQueueResponse {
                user_id: req.user_id,
                released_count,
            }),
        )
            .into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn get_match_queue(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<QueueQuery>,
) -> Response {
    let limit = query.limit.unwrap_or(10);

    match match_queue::fetch_queue_for_user(&pool, user_id, limit).await {
        Ok(queue) => (StatusCode::OK, Json(queue)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn list_serious_candidates(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<SeriousCandidatesQuery>,
) -> Response {
    let limit = query.limit.unwrap_or(25);

    match serious_candidates::fetch_available_candidates(&pool, user_id, limit).await {
        Ok(candidates) => (StatusCode::OK, Json(candidates)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn count_serious_candidates(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Response {
    match serious_candidates::count_available_candidates(&pool, user_id).await {
        Ok(count) => (StatusCode::OK, Json(CountResponse { user_id, count })).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn expire_stale_serious_candidates(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Response {
    match serious_candidates::expire_stale_candidates(&pool, user_id).await {
        Ok(rows_affected) => {
            (StatusCode::OK, Json(RowsAffectedResponse { rows_affected })).into_response()
        }
        Err(e) => db_error_response(e),
    }
}

pub async fn delete_serious_candidate(
    State(pool): State<PgPool>,
    Path((user_id, candidate_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match serious_candidates::delete_candidate(&pool, user_id, candidate_user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "serious candidate not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn update_match_queue_status(
    State(pool): State<PgPool>,
    Json(req): Json<QueueStatusRequest>,
) -> Response {
    if !matches!(
        req.status.as_str(),
        "queued" | "visible" | "consumed" | "expired" | "removed"
    ) {
        return (
            StatusCode::BAD_REQUEST,
            "status must be one of: queued, visible, consumed, expired, removed",
        )
            .into_response();
    }

    match match_queue::update_queue_status(&pool, req.user_id, req.candidate_user_id, &req.status)
        .await
    {
        Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "queue row not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn submit_queue_decision(
    State(pool): State<PgPool>,
    Json(req): Json<QueueDecisionRequest>,
) -> Response {
    match record_queue_decision(&pool, req.user_id, req.candidate_user_id, &req.decision).await {
        Ok(outcome) => (StatusCode::OK, Json(outcome)).into_response(),
        Err(e) => workflow_error_response(e),
    }
}

pub async fn create_match_question(
    State(pool): State<PgPool>,
    Json(req): Json<CreateQuestionRequest>,
) -> Response {
    match match_questions::create_user_match_question(
        &pool,
        req.user_id,
        &req.question,
        req.position.unwrap_or(0),
        req.is_required.unwrap_or(true),
    )
    .await
    {
        Ok(question) => (StatusCode::CREATED, Json(question)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn list_match_questions(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<QuestionsQuery>,
) -> Response {
    let result = if query.include_inactive.unwrap_or(false) {
        match_questions::fetch_user_match_questions(&pool, user_id).await
    } else {
        match_questions::fetch_active_user_match_questions(&pool, user_id).await
    };

    match result {
        Ok(questions) => (StatusCode::OK, Json(questions)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn update_match_question(
    State(pool): State<PgPool>,
    Path((user_id, question_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateQuestionRequest>,
) -> Response {
    match match_questions::update_user_match_question(
        &pool,
        user_id,
        question_id,
        req.question.as_deref(),
        req.position,
        req.is_required,
        req.is_active,
    )
    .await
    {
        Ok(Some(question)) => (StatusCode::OK, Json(question)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "question not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn delete_match_question(
    State(pool): State<PgPool>,
    Path((user_id, question_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match match_questions::delete_user_match_question(&pool, user_id, question_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "question not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn list_interested_pairs(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Response {
    match interested_pairs::fetch_interested_pairs_for_user(&pool, user_id).await {
        Ok(pairs) => (StatusCode::OK, Json(pairs)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn get_interested_pair(
    State(pool): State<PgPool>,
    Path((user_id, other_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match interested_pairs::get_interested_pair(&pool, user_id, other_user_id).await {
        Ok(Some(pair)) => (StatusCode::OK, Json(pair)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "interested pair not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn expire_interested_pair(
    State(pool): State<PgPool>,
    Json(req): Json<ExpireInterestedPairRequest>,
) -> Response {
    match interested_pairs::mark_interested_pair_expired(&pool, req.user_id, req.other_user_id)
        .await
    {
        Ok(Some(pair)) => (StatusCode::OK, Json(pair)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "active interested pair not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn get_pair_state(
    State(pool): State<PgPool>,
    Path((user_id, other_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match pair_state::get_pair_state(&pool, user_id, other_user_id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "pair state not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn list_questions_to_answer(
    State(pool): State<PgPool>,
    Path((answering_user_id, asked_by_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match match_questions::fetch_questions_to_answer(&pool, answering_user_id, asked_by_user_id)
        .await
    {
        Ok(questions) => (StatusCode::OK, Json(questions)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn submit_interested_answer(
    State(pool): State<PgPool>,
    Json(req): Json<SubmitQuestionAnswerRequest>,
) -> Response {
    match submit_question_answer(
        &pool,
        req.answered_by_user_id,
        req.asked_by_user_id,
        req.question_id,
        &req.answer,
    )
    .await
    {
        Ok(outcome) => (StatusCode::OK, Json(outcome)).into_response(),
        Err(e) => workflow_error_response(e),
    }
}

pub async fn list_interested_answers(
    State(pool): State<PgPool>,
    Path((user_id, other_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match match_questions::fetch_pair_question_answers(&pool, user_id, other_user_id).await {
        Ok(answers) => (StatusCode::OK, Json(answers)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn submit_interested_final_decision(
    State(pool): State<PgPool>,
    Json(req): Json<FinalDecisionRequest>,
) -> Response {
    match submit_final_decision(&pool, req.user_id, req.other_user_id, &req.decision).await {
        Ok(outcome) => (StatusCode::OK, Json(outcome)).into_response(),
        Err(e) => workflow_error_response(e),
    }
}

pub async fn list_matches(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Response {
    match matches::get_matches_for_user(&pool, user_id).await {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn get_match(
    State(pool): State<PgPool>,
    Path((user_id, other_user_id)): Path<(Uuid, Uuid)>,
) -> Response {
    match matches::get_match(&pool, user_id, other_user_id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "match not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

pub async fn unmatch_pair(State(pool): State<PgPool>, Json(req): Json<UnmatchRequest>) -> Response {
    match matches::unmatch(&pool, req.user_id, req.other_user_id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "active match not found").into_response(),
        Err(e) => db_error_response(e),
    }
}

fn workflow_error_response(error: MatchingWorkflowError) -> Response {
    match error {
        MatchingWorkflowError::BadRequest(message) => {
            (StatusCode::BAD_REQUEST, message).into_response()
        }
        MatchingWorkflowError::NotFound(message) => {
            (StatusCode::NOT_FOUND, message).into_response()
        }
        MatchingWorkflowError::Db(error) => db_error_response(error),
    }
}

fn db_error_response(error: sqlx::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("db error: {}", error),
    )
        .into_response()
}
