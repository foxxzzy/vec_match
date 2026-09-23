use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::users;
use crate::models::user::{UpdatePreferencesRequest, UpdateProfileRequest, UpdateUserRequest};

pub async fn update_user(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Response {
    match users::update_user(&pool, user_id, &req).await {
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        Err(sqlx::Error::RowNotFound) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            eprintln!("update_user error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update user: {e}"),
            )
                .into_response()
        }
    }
}

pub async fn update_profile(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateProfileRequest>,
) -> Response {
    match users::update_profile(&pool, user_id, &req).await {
        Ok(profile) => (StatusCode::OK, Json(profile)).into_response(),
        Err(sqlx::Error::RowNotFound) => {
            (StatusCode::NOT_FOUND, "Profile not found").into_response()
        }
        Err(e) => {
            eprintln!("update_profile error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update profile: {e}"),
            )
                .into_response()
        }
    }
}

pub async fn update_preferences(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Response {
    match users::update_preferences(&pool, user_id, &req).await {
        Ok(prefs) => (StatusCode::OK, Json(prefs)).into_response(),
        Err(sqlx::Error::RowNotFound) => {
            (StatusCode::NOT_FOUND, "Preferences not found").into_response()
        }
        Err(e) => {
            eprintln!("update_preferences error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update preferences: {e}"),
            )
                .into_response()
        }
    }
}
