use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;

use crate::core::AgentDefinition;

pub async fn list_definitions() -> Json<Vec<Value>> {
    // Placeholder — return empty array.
    Json(vec![])
}

pub async fn create_definition(Json(def): Json<AgentDefinition>) -> (StatusCode, Json<Value>) {
    // Placeholder — accept and echo back.
    let value = serde_json::to_value(&def).unwrap_or(Value::Null);
    (StatusCode::CREATED, Json(value))
}
