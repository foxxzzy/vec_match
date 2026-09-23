use crate::services::embedding::types::{EmbeddingRequest, EmbeddingResponse};
use crate::utils::api_call::generic_api_call;
use crate::utils::env::load_env;

pub async fn fetch_embedding(input: &str) -> Result<EmbeddingResponse, Box<dyn std::error::Error>> {
    let api_key = load_env("OPEN_AI_API_EMBEDDING_API_KEY".to_string())?;

    let url: String = "https://api.openai.com/v1/embeddings".to_string();

    let body: EmbeddingRequest = EmbeddingRequest {
        input,
        model: "text-embedding-3-small",
    };

    let method: String = "POST".to_string();

    let embedding_response: EmbeddingResponse =
        match generic_api_call(url, method, body, Some(api_key)).await {
            Ok(response) => response,
            Err(error) => return Err(error),
        };

    Ok(embedding_response)
}
