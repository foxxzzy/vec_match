use crate::models::{Axis, UserID, Vector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserInfoVectors {
    pub user_id: UserID,
    pub axis: Axis,
    pub embedding: Vector,
}
