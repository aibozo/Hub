use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    #[serde(default)]
    pub params: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub ok: bool,
    #[serde(default)]
    pub result: JsonValue,
    #[serde(default)]
    pub error: Option<String>,
}

impl ToolResponse {
    pub fn ok(result: JsonValue) -> Self { Self { ok: true, result, error: None } }
    pub fn err(msg: impl Into<String>) -> Self { Self { ok: false, result: JsonValue::Null, error: Some(msg.into()) } }
}

