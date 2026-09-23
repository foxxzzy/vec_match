use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchRow {
    pub user_low_id: Uuid,
    pub user_high_id: Uuid,
    pub matched_at: DateTime<Utc>,
    pub chat_thread_id: Option<Uuid>,
    pub unmatched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
