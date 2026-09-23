use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::users;

pub async fn get_user(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Response {
    match users::get_user(&pool, user_id).await {
        Ok(full) => (StatusCode::OK, Json(full)).into_response(),
        Err(sqlx::Error::RowNotFound) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            eprintln!("get_user error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch user: {e}"),
            )
                .into_response()
        }
    }
}

pub async fn get_user_profile(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Response {
    match users::get_user_profile(&pool, user_id).await {
        Ok(profile) => (StatusCode::OK, Json(profile)).into_response(),
        Err(sqlx::Error::RowNotFound) => {
            (StatusCode::NOT_FOUND, "Profile not found").into_response()
        }
        Err(e) => {
            eprintln!("get_user_profile error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch profile: {e}"),
            )
                .into_response()
        }
    }
}

pub async fn get_user_preferences(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Response {
    match users::get_user_preferences(&pool, user_id).await {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(sqlx::Error::RowNotFound) => {
            (StatusCode::NOT_FOUND, "Preferences not found").into_response()
        }
        Err(e) => {
            eprintln!("get_user_preferences error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch preferences: {e}"),
            )
                .into_response()
        }
    }
}

pub async fn get_user_computed(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Response {
    match users::get_user_computed(&pool, user_id).await {
        Ok(Some(computed)) => (StatusCode::OK, Json(computed)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Computed data not found").into_response(),
        Err(e) => {
            eprintln!("get_user_computed error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch computed: {e}"),
            )
                .into_response()
        }
    }
}
