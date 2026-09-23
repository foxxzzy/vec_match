use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct InterestedPairRow {
    pub user_low_id: Uuid,
    pub user_high_id: Uuid,
    pub low_questions_answered_at: Option<DateTime<Utc>>,
    pub high_questions_answered_at: Option<DateTime<Utc>>,
    pub low_final_decision: Option<String>,
    pub high_final_decision: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
