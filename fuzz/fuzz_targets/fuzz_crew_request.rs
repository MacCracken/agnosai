#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the crew request deserialization (same struct the API accepts).
        let _ = serde_json::from_str::<serde_json::Value>(s).and_then(|v| {
            // Simulate the validation path.
            serde_json::from_value::<agnosai::core::CrewSpec>(v)
        });
    }
});
