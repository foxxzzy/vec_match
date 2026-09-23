//! OpenAI embeddings request and response payloads.
use crate::models::Vector;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Data {
    pub object: String,
    pub embedding: Vector,
    pub index: i64,
}

#[derive(Deserialize, Debug)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<Data>,
    pub usage: Option<Usage>,
}

#[derive(Serialize)]
pub struct EmbeddingRequest<'a> {
    pub input: &'a str,
    pub model: &'a str,
}
