use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::content::save_content_and_prompt;
use crate::models::ContentUploadRequest;

#[derive(Serialize)]
struct UploadContentResponse {
    pub prompt_id: Uuid,
    pub axis: String,
    pub content: String,
    pub details: Option<String>,
    pub anchor: bool,
    pub weight: f32,
}

pub async fn upload_content(
    State(pool): State<PgPool>,
    Json(payload): Json<ContentUploadRequest>,
) -> Response {
    let upload = payload.into_content_upload();

    let display = upload.to_display_content();
    let embed = match upload.create_embedded_content_struct(&display).await {
        Ok(e) => e,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("embedding error: {}", e)).into_response();
        }
    };

    if let Err(e) = save_content_and_prompt(&pool, &display, &embed).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error: {}", e),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(UploadContentResponse {
            prompt_id: display.id.0,
            axis: display.axis.axis_to_text().to_string(),
            content: display.content,
            details: display.details,
            anchor: display.anchor,
            weight: embed.weight,
        }),
    )
        .into_response()
}
