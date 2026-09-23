use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct HardGatedCandidateRow {
    pub user_id: Uuid,
    pub candidate_user_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub distance_km: Option<f64>,
    pub hard_gate_version: Option<i32>,
    pub consumed: bool,
    pub consumed_at: Option<DateTime<Utc>>,
}
