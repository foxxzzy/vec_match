use crate::models::shared::{Axis, WeightBuckets};
use serde::{Deserialize, Serialize};

// this is the extra meta data, very important, needed.
#[derive(Clone, Serialize, Deserialize)]
pub struct ContentMetaData {
    pub axis: Axis,
    pub weight_bucket: WeightBuckets,
    pub anchor: bool,
}
