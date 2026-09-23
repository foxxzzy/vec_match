use crate::models::{Axis, EmbeddingContent, PromptId, Vector};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn get_embedded_content(
    pool: &PgPool,
    prompt_id: &PromptId,
) -> Result<EmbeddingContent, sqlx::Error> {
    // Fetch as primitive types that sqlx understands
    let row = sqlx::query(
        r#"SELECT 
            prompt_id, 
            axis, 
            embedding, 
            weight,
            anchor
        FROM embedded_content_who_are_they WHERE prompt_id = $1"#,
    )
    .bind(prompt_id.0)
    .fetch_one(pool)
    .await?;

    let id: Uuid = row.try_get("prompt_id")?;
    let axis_str: String = row.try_get("axis")?;
    let embedding: Vector = row.try_get("embedding")?;
    let weight: f32 = row.try_get("weight")?;
    let anchor: bool = row.try_get("anchor")?;

    // Map axis text to enum
    let axis = Axis::from_text(axis_str.as_str()).map_err(|err| sqlx::Error::ColumnDecode {
        index: "axis".into(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
    })?;

    Ok(EmbeddingContent {
        id: PromptId(id),
        axis,
        embedding,
        weight,
        anchor,
    })
}
