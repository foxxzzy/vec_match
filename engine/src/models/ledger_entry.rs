use crate::models::{Axis, PromptId, Reaction, UserID, Vector};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

//this is the ledger we get back from the db when we request it
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LedgerEntryDb {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub user_id: UserID,
    pub prompt_id: PromptId,
    pub axis: Axis,
    pub embedding: Vector,
    pub reaction: Reaction,
    pub weight: f32,
    pub anchor: bool,
}

// this is the ledger we send to the db to insert or update it
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LedgerEntry {
    pub user_id: UserID,
    pub prompt_id: PromptId,
    pub axis: Axis,
    pub embedding: Vector,
    pub reaction: Reaction,
    pub weight: f32,
    pub anchor: bool,
}
