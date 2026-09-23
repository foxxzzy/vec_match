use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub async fn health() -> Response {
    (StatusCode::OK, "OK").into_response()
}

pub async fn db_health(State(pool): State<sqlx::PgPool>) -> Response {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "DB OK").into_response(),
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, format!("DB ERR: {}", e)).into_response(),
    }
}
