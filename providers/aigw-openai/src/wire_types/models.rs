use serde::{Deserialize, Serialize};

use super::shared::{JsonObject, json_object_is_empty};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<Model>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[cfg(test)]
mod tests {
    use super::ModelListResponse;

    #[test]
    fn deserialize_models_response_with_extra_fields() {
        let json = r#"{
            "object": "list",
            "data": [{
                "id": "gpt-4.1",
                "object": "model",
                "owned_by": "openai",
                "context_window": 1048576
            }]
        }"#;

        let response: ModelListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.data[0].extra.get("context_window").unwrap(),
            1048576
        );
    }
}
