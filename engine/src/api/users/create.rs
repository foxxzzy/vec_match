use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::db::users::{create_full_user as db_create_full_user, create_user as db_create_user};
use crate::models::user::{CreateFullUserRequest, CreateUserRequest};

pub async fn create_user(
    State(pool): State<PgPool>,
    Json(req): Json<CreateUserRequest>,
) -> Response {
    match db_create_user(&pool, &req).await {
        Ok(full) => (StatusCode::CREATED, Json(full)).into_response(),
        Err(e) => map_create_error("create_user", e),
    }
}

pub async fn create_full_user(
    State(pool): State<PgPool>,
    Json(req): Json<CreateFullUserRequest>,
) -> Response {
    match db_create_full_user(&pool, &req).await {
        Ok(full) => (StatusCode::CREATED, Json(full)).into_response(),
        Err(e) => map_create_error("create_full_user", e),
    }
}

fn map_create_error(context: &str, e: sqlx::Error) -> Response {
    match &e {
        sqlx::Error::Database(db_err) => {
            let code = db_err
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            match code.as_str() {
                "23505" => (StatusCode::CONFLICT, db_err.message().to_string()).into_response(),
                "23503" | "23514" | "22P02" => {
                    (StatusCode::BAD_REQUEST, db_err.message().to_string()).into_response()
                }
                _ => {
                    eprintln!("{context} error ({code}): {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create user: {e}"),
                    )
                        .into_response()
                }
            }
        }
        _ => {
            eprintln!("create_user error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create user: {e}"),
            )
                .into_response()
        }
    }
}
