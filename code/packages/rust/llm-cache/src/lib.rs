// LlmError carries provider identity + detail on every variant —
// the same audit-trail discipline applies here too.
#![allow(clippy::result_large_err)]

//! # llm-cache — content-addressed prompt cache
//!
//! Wraps any [`LlmClient`] with an in-memory cache keyed on the
//! prompt fingerprint. Because every primitive's prompt is
//! deterministic (`temperature: 0.0`, content-addressed via
//! `llm_primitives::fingerprint_prompt`), the same `(model,
//! prompt_hash)` pair always produces the same response — caching
//! it is sound.
//!
//! ## Why this matters
//!
//! For the framework's "small local models do extraordinary work"
//! design principle, the asymmetry between expensive-to-produce
//! model output and cheap-to-compare prompt hashes is the load-
//! bearing economic argument. A demo run that calls `render_node` +
//! `entail` six times per IR document, plus `decompose_text` once,
//! is doing a lot of work that gets repeated across debug iterations,
//! test fixtures, replay scenarios, and the ADJ06 retry loop.
//! Caching turns 7 LLM round-trips into 0 on the second run.
//!
//! ## What v0.1 ships
//!
//! - [`CachingClient`] — wraps a `Box<dyn LlmClient>` and adds a
//!   hashmap keyed on `(model, prompt_hash)`. Implements
//!   `LlmClient` so it drops into any `GatewayConfig` slot.
//! - Cache hits are recorded via [`CacheStats`] so callers can
//!   measure their hit rate.
//! - Optional capacity limit (FIFO eviction) — defaults to
//!   unlimited because the typical demo run has fewer than a few
//!   dozen entries.
//!
//! ## What v0.1 deliberately does NOT do
//!
//! - **Disk persistence.** The cache is in-memory; a process
//!   restart loses everything. Disk persistence (content-addressed
//!   files keyed on `prompt_hash`) is a v0.2 follow-up.
//! - **Cross-call dependency tracking.** Each cache entry is a
//!   single `(request, response)` pair. We don't track that
//!   primitive A's call depends on primitive B's call.
//! - **TTL / staleness.** Entries live until evicted. Deterministic
//!   prompts mean staleness doesn't really apply, but a future
//!   version may add TTL for adversarial use cases.

use std::sync::Mutex;

use llm_gateway::{
    Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, JsonSchema,
    LlmClient, LlmError, ProviderIdentity,
};
use llm_primitives::fingerprint_prompt;

/// One entry in the cache. We store the typed response variants
/// separately so a cache lookup doesn't have to round-trip through
/// JSON.
#[derive(Clone)]
enum CacheEntry {
    Text(CompletionResponse),
    Json(CompletionJsonResponse),
}

#[derive(Default)]
struct CacheState {
    entries: Vec<(String, CacheEntry)>,
    insertions: Vec<String>, // FIFO order for eviction
    hits: u64,
    misses: u64,
}

/// Cache hit/miss telemetry. Returned by
/// [`CachingClient::stats`] without consuming the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Wraps any `LlmClient` with a `(model, prompt_hash)`-keyed cache.
///
/// Usage:
///
/// ```ignore
/// let inner = OllamaClient::new("gemma4:latest");
/// let cached = CachingClient::new(Box::new(inner));
/// // ... register `cached` in a GatewayConfig ...
/// // After running primitives, inspect:
/// let stats = cached.stats();
/// println!("hit rate: {:.0}%", stats.hit_rate() * 100.0);
/// ```
pub struct CachingClient {
    inner: Box<dyn LlmClient>,
    state: Mutex<CacheState>,
    /// Maximum number of entries before FIFO eviction. `None` =
    /// unlimited.
    capacity: Option<usize>,
}

impl CachingClient {
    /// Wrap an inner client with an unbounded cache.
    pub fn new(inner: Box<dyn LlmClient>) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: None,
        }
    }

    /// Wrap with a bounded cache (FIFO eviction on overflow).
    pub fn with_capacity(inner: Box<dyn LlmClient>, capacity: usize) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: Some(capacity),
        }
    }

    /// Snapshot of cache telemetry.
    pub fn stats(&self) -> CacheStats {
        let s = self.state.lock().unwrap();
        CacheStats {
            hits: s.hits,
            misses: s.misses,
            entries: s.entries.len(),
        }
    }

    /// Empty the cache (preserves the hit/miss counters).
    pub fn clear(&self) {
        let mut s = self.state.lock().unwrap();
        s.entries.clear();
        s.insertions.clear();
    }

    fn cache_key(&self, req: &CompletionRequest) -> String {
        let model = if req.model.is_empty() {
            self.inner.identity().model_family
        } else {
            req.model.clone()
        };
        format!("{model}|{hash}", hash = fingerprint_prompt(req))
    }

    fn lookup(&self, key: &str) -> Option<CacheEntry> {
        let mut s = self.state.lock().unwrap();
        if let Some(entry) = s
            .entries
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
        {
            s.hits += 1;
            return Some(entry);
        }
        s.misses += 1;
        None
    }

    fn insert(&self, key: String, entry: CacheEntry) {
        let mut s = self.state.lock().unwrap();
        // O(n) but n is small in practice.
        s.entries.retain(|(k, _)| k != &key);
        s.entries.push((key.clone(), entry));
        s.insertions.push(key);
        if let Some(cap) = self.capacity {
            while s.entries.len() > cap {
                let oldest = s.insertions.remove(0);
                s.entries.retain(|(k, _)| k != &oldest);
            }
        }
    }
}

impl LlmClient for CachingClient {
    fn identity(&self) -> ProviderIdentity {
        self.inner.identity()
    }

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let key = self.cache_key(&req);
        if let Some(CacheEntry::Text(resp)) = self.lookup(&key) {
            return Ok(resp);
        }
        let resp = self.inner.complete(req)?;
        self.insert(key, CacheEntry::Text(resp.clone()));
        Ok(resp)
    }

    fn complete_json(
        &self,
        req: CompletionRequest,
        schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError> {
        // Key includes the schema name so `entail` and
        // `judge_plausibility` calling the same model with
        // structurally similar prompts don't collide on a false
        // cache hit.
        let key = format!("{}|{}", self.cache_key(&req), schema.name);
        if let Some(CacheEntry::Json(resp)) = self.lookup(&key) {
            return Ok(resp);
        }
        let resp = self.inner.complete_json(req, schema)?;
        self.insert(key, CacheEntry::Json(resp.clone()));
        Ok(resp)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, FinishReason, Message, MessageContent, MockLlmClient, ProviderIdentity, Role,
        TokenUsage,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Test client that counts every underlying call.
    struct CountingClient {
        identity: ProviderIdentity,
        calls: Arc<AtomicUsize>,
        json_calls: Arc<AtomicUsize>,
        response_text: String,
        response_json: serde_json::Value,
    }

    impl CountingClient {
        fn new() -> Self {
            Self {
                identity: ProviderIdentity {
                    vendor: "mock".into(),
                    model_family: "fake".into(),
                    model_version: "1".into(),
                    endpoint: None,
                },
                calls: Arc::new(AtomicUsize::new(0)),
                json_calls: Arc::new(AtomicUsize::new(0)),
                response_text: "ack".into(),
                response_json: serde_json::json!({ "ok": true }),
            }
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
        fn json_calls(&self) -> usize {
            self.json_calls.load(Ordering::SeqCst)
        }
    }

    impl LlmClient for CountingClient {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionResponse {
                text: self.response_text.clone(),
                model: "fake".into(),
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
                provider_id: self.identity.clone(),
                latency_ms: 0,
            })
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            self.json_calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionJsonResponse {
                raw_text: self.response_json.to_string(),
                parsed: self.response_json.clone(),
                schema_valid: true,
                model: "fake".into(),
                usage: TokenUsage::default(),
                provider_id: self.identity.clone(),
                latency_ms: 0,
                polyfill_used: false,
            })
        }
    }

    fn req_with_text(text: &str) -> CompletionRequest {
        CompletionRequest {
            model: "fake".into(),
            system: None,
            messages: vec![Message {
                role: Role::User,
                content: MessageContent::Text(text.into()),
            }],
            temperature: 0.0,
            max_tokens: None,
            stop_sequences: Vec::new(),
            seed: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn second_identical_call_is_a_cache_hit() {
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::new(Box::new(inner));
        let _ = cached.complete(req_with_text("hello")).unwrap();
        let _ = cached.complete(req_with_text("hello")).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let s = cached.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
        assert_eq!(s.entries, 1);
    }

    #[test]
    fn different_prompts_get_distinct_entries() {
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::new(Box::new(inner));
        let _ = cached.complete(req_with_text("hello")).unwrap();
        let _ = cached.complete(req_with_text("world")).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let s = cached.stats();
        assert_eq!(s.hits, 0);
        assert_eq!(s.misses, 2);
        assert_eq!(s.entries, 2);
    }

    #[test]
    fn complete_json_cache_is_independent_per_schema() {
        let inner = CountingClient::new();
        let json_calls = Arc::clone(&inner.json_calls);
        let cached = CachingClient::new(Box::new(inner));
        let schema_a = JsonSchema {
            name: "A".into(),
            schema_json: "{}".into(),
        };
        let schema_b = JsonSchema {
            name: "B".into(),
            schema_json: "{}".into(),
        };
        let _ = cached.complete_json(req_with_text("hi"), &schema_a).unwrap();
        let _ = cached.complete_json(req_with_text("hi"), &schema_a).unwrap();
        let _ = cached.complete_json(req_with_text("hi"), &schema_b).unwrap();
        // Two underlying calls: schema_a (cached on second call) and schema_b.
        assert_eq!(json_calls.load(Ordering::SeqCst), 2);
        let s = cached.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 2);
    }

    #[test]
    fn capacity_evicts_oldest_entry() {
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::with_capacity(Box::new(inner), 2);
        let _ = cached.complete(req_with_text("a")).unwrap(); // miss
        let _ = cached.complete(req_with_text("b")).unwrap(); // miss
        let _ = cached.complete(req_with_text("c")).unwrap(); // miss, evicts "a"
        let _ = cached.complete(req_with_text("a")).unwrap(); // miss (was evicted)
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        let s = cached.stats();
        assert_eq!(s.entries, 2);
    }

    #[test]
    fn clear_resets_entries_but_keeps_stats() {
        let inner = CountingClient::new();
        let cached = CachingClient::new(Box::new(inner));
        let _ = cached.complete(req_with_text("a")).unwrap();
        let _ = cached.complete(req_with_text("a")).unwrap();
        cached.clear();
        let s = cached.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
        assert_eq!(s.entries, 0);
        // After clear, the next call is a miss again.
        let _ = cached.complete(req_with_text("a")).unwrap();
        assert_eq!(cached.stats().misses, 2);
    }

    #[test]
    fn hit_rate_is_zero_when_no_calls() {
        let inner = CountingClient::new();
        let cached = CachingClient::new(Box::new(inner));
        assert_eq!(cached.stats().hit_rate(), 0.0);
    }

    #[test]
    fn hit_rate_is_computed_correctly() {
        let inner = CountingClient::new();
        let cached = CachingClient::new(Box::new(inner));
        let _ = cached.complete(req_with_text("a")).unwrap(); // miss
        let _ = cached.complete(req_with_text("a")).unwrap(); // hit
        let _ = cached.complete(req_with_text("a")).unwrap(); // hit
        let r = cached.stats().hit_rate();
        assert!((r - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn identity_delegates_to_inner_client() {
        let inner = MockLlmClient::new().with_identity(ProviderIdentity {
            vendor: "anthropic".into(),
            model_family: "claude-opus".into(),
            model_version: "4-7".into(),
            endpoint: None,
        });
        let cached = CachingClient::new(Box::new(inner));
        let id = cached.identity();
        assert_eq!(id.vendor, "anthropic");
        assert_eq!(id.model_family, "claude-opus");
    }

    #[test]
    fn cache_key_includes_model_so_same_prompt_different_model_misses() {
        // Distinct CompletionRequest.model values produce distinct
        // cache keys even on byte-identical prompts.
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::new(Box::new(inner));
        let mut req1 = req_with_text("hello");
        req1.model = "model-a".into();
        let mut req2 = req_with_text("hello");
        req2.model = "model-b".into();
        let _ = cached.complete(req1).unwrap();
        let _ = cached.complete(req2).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
