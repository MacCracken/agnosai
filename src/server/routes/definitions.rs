use axum::Json;
use serde_json::Value;

/// GET /api/v1/presets — return all built-in preset specifications.
pub async fn list_presets() -> Json<Vec<Value>> {
    #[cfg(feature = "definitions")]
    {
        let presets = crate::definitions::loader::builtin_presets();
        let values: Vec<Value> = presets
            .into_iter()
            .filter_map(|p| serde_json::to_value(p).ok())
            .collect();
        Json(values)
    }
    #[cfg(not(feature = "definitions"))]
    {
        Json(vec![])
    }
}
