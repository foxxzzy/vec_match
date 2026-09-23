use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Axis, Vector};

#[derive(serde::Deserialize)]
pub struct ExportQuery {
    pub axis: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ExportedContentRow {
    pub prompt_id: Uuid,
    pub axis: String,
    pub content: String,
    pub anchor: Option<bool>,
    pub details: Option<String>,
}

/// Export all content rows for a given axis.
///
/// Example: GET /content/export?axis=ComfortWithCloseness
pub async fn export_content_for_axis(
    State(pool): State<PgPool>,
    Query(query): Query<ExportQuery>,
) -> Response {
    let axis = match Axis::from_text(&query.axis) {
        Ok(a) => a,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    let axis_text = axis.axis_to_text();

    let rows = match sqlx::query_as::<_, ExportedContentRow>(
        r#"
        SELECT prompt_id, axis, content, anchor, details
        FROM content_who_are_they
        WHERE axis = $1
        ORDER BY prompt_id
        "#,
    )
    .bind(axis_text)
    .fetch_all(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error: {}", e),
            )
                .into_response();
        }
    };

    match serde_json::to_string_pretty(&rows) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialize error: {}", e),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct ExportAnchorTypesQuery {
    pub axis: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ExportedAnchorTypeRow {
    pub anchor_id: i64,
    pub created_at: DateTime<Utc>,
    pub type_name: String,
    pub vector: Vector,
    pub axis: String,
}

/// Export all anchor_types rows for a given axis.
///
/// Example: GET /anchor_types/export?axis=ComfortWithCloseness
pub async fn export_anchor_types_for_axis(
    State(pool): State<PgPool>,
    Query(query): Query<ExportAnchorTypesQuery>,
) -> Response {
    let axis = match Axis::from_text(&query.axis) {
        Ok(a) => a,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    let axis_text = axis.axis_to_text();

    let rows = match sqlx::query_as::<_, ExportedAnchorTypeRow>(
        r#"
        SELECT anchor_id, created_at, type_name, vector, axis
        FROM anchor_types
        WHERE axis = $1
        ORDER BY anchor_id
        "#,
    )
    .bind(axis_text)
    .fetch_all(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error: {}", e),
            )
                .into_response();
        }
    };

    match serde_json::to_string_pretty(&rows) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialize error: {}", e),
        )
            .into_response(),
    }
}
