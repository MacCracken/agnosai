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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_presets_returns_array() {
        let Json(presets) = list_presets().await;
        // With "definitions" feature enabled, we expect built-in presets.
        // Without it, we get an empty vec. Either way it must be a Vec.
        #[cfg(feature = "definitions")]
        assert!(
            !presets.is_empty(),
            "expected non-empty presets with definitions feature"
        );
        #[cfg(not(feature = "definitions"))]
        assert!(
            presets.is_empty(),
            "expected empty presets without definitions feature"
        );
    }
}
