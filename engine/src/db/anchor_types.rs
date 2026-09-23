use sqlx::PgPool;

use crate::models::{Axis, Vector};

#[derive(Debug, sqlx::FromRow)]
pub struct AnchorTypeRow {
    pub type_name: String,
    pub axis: String,
    pub vector: Vector,
}

pub async fn upsert_anchor_type_vector(
    pool: &PgPool,
    type_name: &str,
    axis: &Axis,
    vector: &[f32],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO anchor_types (type_name, axis, vector)
        VALUES ($1, $2, $3)
        ON CONFLICT (type_name)
        DO UPDATE SET axis = EXCLUDED.axis,
                      vector = EXCLUDED.vector
        "#,
    )
    .bind(type_name)
    .bind(axis.axis_to_text())
    .bind(vector)
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch all anchor vectors for a given axis (e.g. "ComfortWithCloseness")
pub async fn get_anchor_types_by_axis(
    pool: &PgPool,
    axis: &Axis,
) -> Result<Vec<AnchorTypeRow>, sqlx::Error> {
    sqlx::query_as::<_, AnchorTypeRow>(
        "SELECT type_name, axis, vector FROM anchor_types WHERE axis = $1",
    )
    .bind(axis.axis_to_text())
    .fetch_all(pool)
    .await
}
