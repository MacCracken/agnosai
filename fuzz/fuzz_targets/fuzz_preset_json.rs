#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz preset spec parsing — same path as definition loading.
        #[derive(serde::Deserialize)]
        struct PresetSpec {
            name: String,
            description: String,
            domain: String,
            size: String,
            version: String,
            agents: Vec<agnosai::core::AgentDefinition>,
        }
        let _ = serde_json::from_str::<PresetSpec>(s);
    }
});
