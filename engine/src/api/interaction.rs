use crate::models::UserInteraction;
use crate::services::content_interaction::content_interaction;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

pub async fn user_interaction(
    State(pool): State<PgPool>,
    Json(interaction): Json<UserInteraction>,
) -> Response {
    match content_interaction(interaction, &pool).await {
        Ok(_) => (StatusCode::OK, "interaction processed").into_response(),
        Err(e) => {
            eprintln!("engine_upsert error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to process interaction: {e}"),
            )
                .into_response()
        }
    }
}
