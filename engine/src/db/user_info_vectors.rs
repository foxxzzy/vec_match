use crate::models::{Axis, UserID, UserInfoVectors, Vector};
use sqlx::{FromRow, PgPool, postgres::PgQueryResult};
use uuid::Uuid;

#[derive(Debug, FromRow)]
struct UserInfoVectorsRow {
    pub user_id: Uuid,
    pub axis: String,
    pub vector: Vector,
}

pub async fn insert_user_info_vectors(
    pool: &PgPool,
    user_info: &UserInfoVectors,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_info_vectors (user_id, axis, vector) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, axis) DO UPDATE SET vector = EXCLUDED.vector",
    )
    .bind(&user_info.user_id.0)
    .bind(user_info.axis.axis_to_text())
    .bind(&user_info.embedding)
    .execute(pool)
    .await
}

pub async fn get_user_info_vectors_by_user_id(
    pool: &PgPool,
    user_id: &UserID,
) -> Result<Vec<UserInfoVectors>, sqlx::Error> {
    let rows: Vec<UserInfoVectorsRow> =
        sqlx::query_as("SELECT user_id, axis, vector FROM user_info_vectors WHERE user_id = $1")
            .bind(&user_id.0)
            .fetch_all(pool)
            .await?;

    rows.into_iter()
        .map(|row| {
            let axis = Axis::from_text(&row.axis).map_err(|_| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid axis stored in DB: {}", row.axis),
                )))
            })?;

            Ok(UserInfoVectors {
                user_id: UserID(row.user_id),
                axis,
                embedding: row.vector,
            })
        })
        .collect()
}
