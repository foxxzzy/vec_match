use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchQueueRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub position: i32,
    pub status: String,
    pub score: Option<f64>,
    pub released_at: Option<DateTime<Utc>>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
