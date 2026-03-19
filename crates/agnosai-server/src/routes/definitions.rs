use axum::Json;
use serde_json::Value;

pub async fn list_presets() -> Json<Vec<Value>> {
    // Placeholder — return empty array.
    Json(vec![])
}
