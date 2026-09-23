// This is what will be uploaded to the add content endpoint.

use crate::models::Vector;
use crate::models::shared::{Axis, WeightBuckets};
use crate::models::{ContentMetaData, DisplayContent, EmbeddingContent, PromptId};
use crate::services::embedding::client::fetch_embedding;
use crate::services::embedding::types::EmbeddingResponse;
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct PartialDisplayContent {
    pub content: String,
    #[serde(default, alias = "detail")]
    pub details: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContentUpload {
    pub content: PartialDisplayContent,
    pub meta_data: ContentMetaData,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FlatContentUpload {
    pub content: String,
    pub axis: Axis,
    pub weight_bucket: WeightBuckets,
    #[serde(default)]
    pub anchor: bool,
    #[serde(default, alias = "detail")]
    pub details: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentUploadRequest {
    Flat(FlatContentUpload),
    Nested(ContentUpload),
}

impl ContentUploadRequest {
    pub fn into_content_upload(self) -> ContentUpload {
        match self {
            ContentUploadRequest::Flat(upload) => ContentUpload {
                content: PartialDisplayContent {
                    content: upload.content,
                    details: upload.details,
                },
                meta_data: ContentMetaData {
                    axis: upload.axis,
                    weight_bucket: upload.weight_bucket,
                    anchor: upload.anchor,
                },
            },
            ContentUploadRequest::Nested(upload) => upload,
        }
    }
}

impl ContentUpload {
    pub async fn compute_embedding(&self) -> Result<Vector, Box<dyn Error>> {
        let response: EmbeddingResponse = match fetch_embedding(&self.content.content).await {
            Ok(val) => val,
            Err(e) => return Err(e),
        };

        match response.data.into_iter().next() {
            Some(data) if !data.embedding.is_empty() => Ok(data.embedding),
            Some(_) => Err("Embedding data is empty".into()),
            None => Err("No embedding data returned".into()),
        }
    }

    pub async fn create_embedded_content_struct(
        &self,
        display: &DisplayContent,
    ) -> Result<EmbeddingContent, Box<dyn Error>> {
        let vector_embedding = match self.compute_embedding().await {
            Ok(embedding) => embedding,
            Err(e) => return Err(e),
        };

        let embedding_content = EmbeddingContent {
            id: display.id.clone(),
            axis: display.axis.clone(),
            embedding: vector_embedding,
            weight: self.meta_data.weight_bucket.value(),
            anchor: self.meta_data.anchor,
        };

        Ok(embedding_content)
    }

    pub fn to_display_content(&self) -> DisplayContent {
        DisplayContent {
            id: PromptId(Uuid::new_v4()),
            axis: self.meta_data.axis.clone(),
            content: self.content.content.clone(),
            anchor: self.meta_data.anchor,
            details: self.content.details.clone(),
        }
    }
}
