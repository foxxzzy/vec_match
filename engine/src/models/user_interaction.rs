/// A reaction accepted by the interaction endpoint and recorded in the ledger.
use crate::models::shared::{PromptId, Reaction, UserID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInteraction {
    pub user_id: UserID,
    pub id: PromptId,
    pub reaction: Reaction,
}
impl Reaction {
    pub fn reaction_to_int(&self) -> i16 {
        match self {
            Reaction::Me => 1,
            Reaction::NotMe => -1,
            Reaction::Skip => 0,
        }
    }
}
