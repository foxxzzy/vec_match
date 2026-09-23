use crate::db::hard_gated_candidates::refill_hard_gated_candidates;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct RefillCandidates {
    user_id: Uuid,
    limit: i32,
}

pub async fn refill_candidates(
    State(pool): State<PgPool>,
    Json(req): Json<RefillCandidates>,
) -> Response {
    match refill_hard_gated_candidates(&pool, req.user_id, req.limit).await {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
