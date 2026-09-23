use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PairStateRow {
    pub user_low_id: Uuid,
    pub user_high_id: Uuid,
    pub low_seen_at: Option<DateTime<Utc>>,
    pub high_seen_at: Option<DateTime<Utc>>,
    pub low_decision: Option<String>,
    pub high_decision: Option<String>,
    pub low_decided_at: Option<DateTime<Utc>>,
    pub high_decided_at: Option<DateTime<Utc>>,
    pub state: String,
    pub matched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Helper to compute canonical ordering for a pair.
pub fn canonical_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b { (a, b) } else { (b, a) }
}

/// Which side of the canonical pair the acting user is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairSide {
    Low,
    High,
}

pub fn pair_side(acting_user: Uuid, other_user: Uuid) -> PairSide {
    let (low, _high) = canonical_pair(acting_user, other_user);
    if acting_user == low {
        PairSide::Low
    } else {
        PairSide::High
    }
}
