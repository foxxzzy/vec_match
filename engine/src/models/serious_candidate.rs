use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriousCandidate {
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub position: i32,
    pub score: f64,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SeriousCandidateRow {
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub position: i32,
    pub score: f64,
    pub status: String,
    pub released_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
