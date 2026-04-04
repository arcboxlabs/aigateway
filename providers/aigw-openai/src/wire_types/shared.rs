use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type JsonObject = BTreeMap<String, Value>;

pub fn json_object_is_empty(value: &JsonObject) -> bool {
    value.is_empty()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: String,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Value>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[cfg(test)]
mod tests {
    use super::ApiErrorResponse;

    #[test]
    fn deserialize_error_body_with_extra_fields() {
        let json = r#"{
            "error": {
                "message": "bad request",
                "type": "invalid_request_error",
                "param": "model",
                "code": "missing",
                "request_id": "req_123"
            }
        }"#;

        let error: ApiErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(error.error.kind.as_deref(), Some("invalid_request_error"));
        assert_eq!(error.error.extra.get("request_id").unwrap(), "req_123");
    }
}
