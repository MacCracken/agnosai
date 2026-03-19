//! LRU response cache with TTL expiration.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use crate::provider::{InferenceRequest, InferenceResponse};

/// LRU cache for inference responses with per-entry TTL.
pub struct ResponseCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
    default_ttl: Duration,
}

struct CacheEntry {
    response: InferenceResponse,
    inserted_at: Instant,
    ttl: Duration,
    last_accessed: Instant,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() >= self.ttl
    }
}

impl ResponseCache {
    /// Create a new cache with the given maximum size and default TTL.
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            default_ttl,
        }
    }

    /// Get a cached response. Returns `None` if absent or expired.
    /// Updates `last_accessed` on hit.
    pub fn get(&mut self, key: &str) -> Option<&InferenceResponse> {
        // Check expiry first — remove if expired.
        if self.entries.get(key).is_some_and(|e| e.is_expired()) {
            self.entries.remove(key);
            return None;
        }

        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = Instant::now();
            Some(&entry.response)
        } else {
            None
        }
    }

    /// Insert a response with the default TTL. Evicts the LRU entry if at capacity.
    pub fn put(&mut self, key: &str, response: InferenceResponse) {
        self.put_with_ttl(key, response, self.default_ttl);
    }

    /// Insert a response with a custom TTL. Evicts the LRU entry if at capacity.
    pub fn put_with_ttl(&mut self, key: &str, response: InferenceResponse, ttl: Duration) {
        // If replacing an existing key, just overwrite.
        if self.entries.contains_key(key) {
            let now = Instant::now();
            self.entries.insert(
                key.to_string(),
                CacheEntry {
                    response,
                    inserted_at: now,
                    ttl,
                    last_accessed: now,
                },
            );
            return;
        }

        // Evict LRU if at capacity.
        if self.entries.len() >= self.max_size {
            self.evict_lru();
        }

        let now = Instant::now();
        self.entries.insert(
            key.to_string(),
            CacheEntry {
                response,
                inserted_at: now,
                ttl,
                last_accessed: now,
            },
        );
    }

    /// Remove a specific entry. Returns `true` if it existed.
    pub fn invalidate(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of entries (including potentially expired ones).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evict the entry with the oldest `last_accessed` timestamp.
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&lru_key);
        }
    }
}

/// Generate a cache key from an inference request by hashing model + messages + temperature.
pub fn cache_key(request: &InferenceRequest) -> String {
    let mut hasher = DefaultHasher::new();
    request.model.hash(&mut hasher);
    for msg in &request.messages {
        msg.role.hash(&mut hasher);
        msg.content.hash(&mut hasher);
    }
    // Hash temperature as bits to avoid float hashing issues.
    if let Some(t) = request.temperature {
        t.to_bits().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatMessage, InferenceRequest, InferenceResponse, TokenUsage};
    use std::thread::sleep;

    fn dummy_response(content: &str) -> InferenceResponse {
        InferenceResponse {
            content: content.to_string(),
            model: "test".to_string(),
            usage: TokenUsage::default(),
        }
    }

    fn dummy_request(model: &str, content: &str) -> InferenceRequest {
        InferenceRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: content.to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: None,
            stream: false,
        }
    }

    #[test]
    fn insert_and_get() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put("k1", dummy_response("hello"));

        let resp = cache.get("k1").expect("should exist");
        assert_eq!(resp.content, "hello");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn get_missing_returns_none() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn ttl_expiry() {
        let mut cache = ResponseCache::new(10, Duration::from_millis(50));
        cache.put("k1", dummy_response("short-lived"));

        assert!(cache.get("k1").is_some());
        sleep(Duration::from_millis(60));
        assert!(cache.get("k1").is_none());
        // Expired entry should be removed.
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn custom_ttl() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put_with_ttl("k1", dummy_response("custom"), Duration::from_millis(50));

        assert!(cache.get("k1").is_some());
        sleep(Duration::from_millis(60));
        assert!(cache.get("k1").is_none());
    }

    #[test]
    fn lru_eviction() {
        let mut cache = ResponseCache::new(2, Duration::from_secs(60));
        cache.put("k1", dummy_response("first"));
        cache.put("k2", dummy_response("second"));

        // Access k1 so k2 becomes LRU.
        let _ = cache.get("k1");

        // Insert k3 — should evict k2 (least recently accessed).
        cache.put("k3", dummy_response("third"));

        assert!(cache.get("k1").is_some());
        assert!(cache.get("k2").is_none());
        assert!(cache.get("k3").is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn invalidate() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put("k1", dummy_response("val"));

        assert!(cache.invalidate("k1"));
        assert!(!cache.invalidate("k1"));
        assert!(cache.is_empty());
    }

    #[test]
    fn clear() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put("k1", dummy_response("a"));
        cache.put("k2", dummy_response("b"));
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_key_deterministic() {
        let req = dummy_request("gpt-4", "hello");
        let key1 = cache_key(&req);
        let key2 = cache_key(&req);
        assert_eq!(key1, key2);
    }

    #[test]
    fn cache_key_differs_for_different_requests() {
        let req1 = dummy_request("gpt-4", "hello");
        let req2 = dummy_request("gpt-4", "world");
        assert_ne!(cache_key(&req1), cache_key(&req2));

        let req3 = dummy_request("gpt-3.5", "hello");
        assert_ne!(cache_key(&req1), cache_key(&req3));
    }
}
