use bon::Builder;
use serde::{Deserialize, Serialize};

use super::shared::{JsonObject, json_object_is_empty};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: EmbeddingInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    String(String),
    Strings(Vec<String>),
    Tokens(Vec<i64>),
    TokenArrays(Vec<Vec<i64>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<Embedding>,
    pub model: String,
    pub usage: EmbeddingUsage,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub object: String,
    pub embedding: serde_json::Value,
    pub index: u32,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[cfg(test)]
mod tests {
    use super::{EmbeddingInput, EmbeddingResponse};

    #[test]
    fn deserialize_base64_embedding_response() {
        let json = r#"{
            "object": "list",
            "data": [{
                "object": "embedding",
                "embedding": "AQID",
                "index": 0
            }],
            "model": "text-embedding-3-small",
            "usage": {
                "prompt_tokens": 3,
                "total_tokens": 3
            }
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data[0].embedding, "AQID");
    }

    #[test]
    fn deserialize_token_array_input() {
        let input: EmbeddingInput = serde_json::from_str("[1,2,3]").unwrap();
        assert!(matches!(input, EmbeddingInput::Tokens(_)));
    }
}
