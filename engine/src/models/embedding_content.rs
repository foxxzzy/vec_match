//this is what a piece of content we pass to the engine will look like, this will always be grabbed from the Postgres DB
use crate::models::Vector;
use crate::models::shared::{Axis, PromptId};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct EmbeddingContent {
    pub id: PromptId,
    pub axis: Axis,
    pub embedding: Vector,
    pub weight: f32,
    pub anchor: bool,
}
