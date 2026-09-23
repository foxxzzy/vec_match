// use crate::types::embedding_content_type::EmbeddingContent;
// This represents what a piece of content that we will show the user looks like
use crate::models::shared::{Axis, PromptId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct DisplayContent {
    pub id: PromptId,
    pub axis: Axis,
    pub content: String,
    pub anchor: bool,
    pub details: Option<String>,
}
