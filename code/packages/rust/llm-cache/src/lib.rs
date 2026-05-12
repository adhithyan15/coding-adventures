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

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use llm_gateway::{
    Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, FinishReason,
    JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
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

// ---------------------------------------------------------------------------
// Disk persistence (v0.2)
// ---------------------------------------------------------------------------
//
// Each cache entry is one file under `<dir>/<sha-like>.json`. The
// file name is derived from the cache key (FNV-1a hash of `model |
// prompt_hash [| schema_name]`) so collisions are extremely
// unlikely. The file contents are a JSON record with enough
// information to reconstruct the typed response without needing
// serde derives on `CompletionResponse` / `CompletionJsonResponse`.
//
// This module intentionally stays minimal: no compression, no
// concurrent-write coordination, no atomic-rename. The cache is a
// best-effort accelerator, not a database — corrupted entries are
// silently treated as misses.

/// Compute the on-disk file name for a cache key. Uses the same
/// FNV-1a 64-bit hash as `fingerprint_prompt` so the filenames are
/// short, ASCII, and never need escaping.
fn key_to_filename(key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}.json", h)
}

/// Serialize a `CompletionResponse` as JSON we can write to disk.
fn serialize_text(resp: &CompletionResponse) -> serde_json::Value {
    serde_json::json!({
        "kind": "text",
        "text": resp.text,
        "model": resp.model,
        "usage": {
            "input_tokens": resp.usage.input_tokens,
            "output_tokens": resp.usage.output_tokens,
            "cached_tokens": resp.usage.cached_tokens,
        },
        "finish_reason": match resp.finish_reason {
            FinishReason::Stop => "Stop",
            FinishReason::MaxTokens => "MaxTokens",
            FinishReason::Refusal => "Refusal",
            FinishReason::Other => "Other",
        },
        "provider_id": serialize_provider(&resp.provider_id),
        "latency_ms": resp.latency_ms,
    })
}

fn serialize_json(resp: &CompletionJsonResponse) -> serde_json::Value {
    serde_json::json!({
        "kind": "json",
        "raw_text": resp.raw_text,
        "parsed": resp.parsed,
        "schema_valid": resp.schema_valid,
        "model": resp.model,
        "usage": {
            "input_tokens": resp.usage.input_tokens,
            "output_tokens": resp.usage.output_tokens,
            "cached_tokens": resp.usage.cached_tokens,
        },
        "provider_id": serialize_provider(&resp.provider_id),
        "latency_ms": resp.latency_ms,
        "polyfill_used": resp.polyfill_used,
    })
}

fn serialize_provider(p: &ProviderIdentity) -> serde_json::Value {
    serde_json::json!({
        "vendor": p.vendor,
        "model_family": p.model_family,
        "model_version": p.model_version,
        "endpoint": p.endpoint,
    })
}

fn deserialize_provider(v: &serde_json::Value) -> Option<ProviderIdentity> {
    Some(ProviderIdentity {
        vendor: v.get("vendor")?.as_str()?.to_string(),
        model_family: v.get("model_family")?.as_str()?.to_string(),
        model_version: v.get("model_version")?.as_str()?.to_string(),
        endpoint: v
            .get("endpoint")
            .and_then(|x| x.as_str())
            .map(str::to_string),
    })
}

fn deserialize_usage(v: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        input_tokens: v
            .get("input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        output_tokens: v
            .get("output_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        cached_tokens: v
            .get("cached_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
    }
}

fn deserialize_finish_reason(s: &str) -> FinishReason {
    match s {
        "Stop" => FinishReason::Stop,
        "MaxTokens" => FinishReason::MaxTokens,
        "Refusal" => FinishReason::Refusal,
        _ => FinishReason::Other,
    }
}

fn deserialize_entry(v: &serde_json::Value) -> Option<CacheEntry> {
    let kind = v.get("kind")?.as_str()?;
    match kind {
        "text" => {
            let provider = deserialize_provider(v.get("provider_id")?)?;
            Some(CacheEntry::Text(CompletionResponse {
                text: v.get("text")?.as_str()?.to_string(),
                model: v.get("model")?.as_str()?.to_string(),
                usage: deserialize_usage(v.get("usage").unwrap_or(&serde_json::Value::Null)),
                finish_reason: deserialize_finish_reason(
                    v.get("finish_reason").and_then(|x| x.as_str()).unwrap_or("Other"),
                ),
                provider_id: provider,
                latency_ms: v.get("latency_ms").and_then(|x| x.as_u64()).unwrap_or(0),
            }))
        }
        "json" => {
            let provider = deserialize_provider(v.get("provider_id")?)?;
            Some(CacheEntry::Json(CompletionJsonResponse {
                raw_text: v.get("raw_text")?.as_str()?.to_string(),
                parsed: v.get("parsed").cloned().unwrap_or(serde_json::Value::Null),
                schema_valid: v.get("schema_valid").and_then(|x| x.as_bool()).unwrap_or(false),
                model: v.get("model")?.as_str()?.to_string(),
                usage: deserialize_usage(v.get("usage").unwrap_or(&serde_json::Value::Null)),
                provider_id: provider,
                latency_ms: v.get("latency_ms").and_then(|x| x.as_u64()).unwrap_or(0),
                polyfill_used: v
                    .get("polyfill_used")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
            }))
        }
        _ => None,
    }
}

/// Best-effort load from disk. Returns `None` if the file doesn't
/// exist, is unreadable, or is malformed.
fn try_load_disk(dir: &Path, key: &str) -> Option<CacheEntry> {
    let path = dir.join(key_to_filename(key));
    let bytes = std::fs::read(&path).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    deserialize_entry(&v)
}

/// Best-effort write to disk. Errors are silently swallowed —
/// failing to persist a cache entry should never break the demo.
fn try_save_disk(dir: &Path, key: &str, entry: &CacheEntry) {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(key_to_filename(key));
    let v = match entry {
        CacheEntry::Text(r) => serialize_text(r),
        CacheEntry::Json(r) => serialize_json(r),
    };
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = std::fs::write(&path, s);
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
    /// Directory under which entries are persisted. `None` = no
    /// disk persistence (v0.1 behaviour).
    disk_dir: Option<PathBuf>,
}

impl CachingClient {
    /// Wrap an inner client with an unbounded in-memory cache.
    pub fn new(inner: Box<dyn LlmClient>) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: None,
            disk_dir: None,
        }
    }

    /// Wrap with a bounded in-memory cache (FIFO eviction on overflow).
    pub fn with_capacity(inner: Box<dyn LlmClient>, capacity: usize) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: Some(capacity),
            disk_dir: None,
        }
    }

    /// Wrap with disk persistence. On lookup misses in memory, the
    /// cache checks `dir/<hash>.json` and reconstitutes the entry if
    /// present. On insert, the entry is also written to disk. The
    /// directory is created on demand. Disk failures are silently
    /// treated as cache misses — the cache is a best-effort
    /// accelerator, not a database.
    pub fn with_disk_persistence(
        inner: Box<dyn LlmClient>,
        dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: None,
            disk_dir: Some(dir.into()),
        }
    }

    /// Disk-backed cache with both a memory bound AND persistence.
    pub fn with_disk_persistence_and_capacity(
        inner: Box<dyn LlmClient>,
        dir: impl Into<PathBuf>,
        capacity: usize,
    ) -> Self {
        Self {
            inner,
            state: Mutex::new(CacheState::default()),
            capacity: Some(capacity),
            disk_dir: Some(dir.into()),
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
        // Memory first.
        {
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
        }
        // Disk fallback. On a disk hit, promote the entry to
        // memory so subsequent lookups stay fast and the hit-rate
        // counter reflects the memory-hit cost on later calls.
        if let Some(dir) = &self.disk_dir {
            if let Some(entry) = try_load_disk(dir, key) {
                let mut s = self.state.lock().unwrap();
                s.entries.retain(|(k, _)| k != key);
                s.entries.push((key.to_string(), entry.clone()));
                s.insertions.push(key.to_string());
                if let Some(cap) = self.capacity {
                    while s.entries.len() > cap {
                        let oldest = s.insertions.remove(0);
                        s.entries.retain(|(k, _)| k != &oldest);
                    }
                }
                s.hits += 1;
                return Some(entry);
            }
        }
        let mut s = self.state.lock().unwrap();
        s.misses += 1;
        None
    }

    fn insert(&self, key: String, entry: CacheEntry) {
        {
            let mut s = self.state.lock().unwrap();
            // O(n) but n is small in practice.
            s.entries.retain(|(k, _)| k != &key);
            s.entries.push((key.clone(), entry.clone()));
            s.insertions.push(key.clone());
            if let Some(cap) = self.capacity {
                while s.entries.len() > cap {
                    let oldest = s.insertions.remove(0);
                    s.entries.retain(|(k, _)| k != &oldest);
                }
            }
        }
        if let Some(dir) = &self.disk_dir {
            try_save_disk(dir, &key, &entry);
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

    // ----- Disk persistence (v0.2) -----

    fn tmp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        // Include the process id + nanos so concurrent tests don't
        // collide. Bare `name` would clash on a `cargo test --jobs`.
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!("llm-cache-test-{name}-{pid}-{nanos}"));
        // Best-effort clean previous run.
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn disk_persistence_survives_a_new_caching_client() {
        let dir = tmp_dir("survive");
        // First client populates the cache.
        {
            let inner = CountingClient::new();
            let cached = CachingClient::with_disk_persistence(Box::new(inner), &dir);
            let _ = cached.complete(req_with_text("hello")).unwrap();
        }
        // Second client (fresh in-memory state) should serve the
        // request from disk on the first call.
        let inner2 = CountingClient::new();
        let calls2 = Arc::clone(&inner2.calls);
        let cached2 = CachingClient::with_disk_persistence(Box::new(inner2), &dir);
        let _ = cached2.complete(req_with_text("hello")).unwrap();
        assert_eq!(calls2.load(Ordering::SeqCst), 0, "should hit disk, not call inner");
        assert_eq!(cached2.stats().hits, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_persistence_promotes_to_memory_on_first_disk_hit() {
        let dir = tmp_dir("promote");
        {
            let inner = CountingClient::new();
            let cached = CachingClient::with_disk_persistence(Box::new(inner), &dir);
            let _ = cached.complete(req_with_text("hello")).unwrap();
        }
        let inner2 = CountingClient::new();
        let calls2 = Arc::clone(&inner2.calls);
        let cached2 = CachingClient::with_disk_persistence(Box::new(inner2), &dir);
        // First call: disk hit, promoted to memory.
        let _ = cached2.complete(req_with_text("hello")).unwrap();
        // Second call: memory hit (the disk path is bypassed).
        let _ = cached2.complete(req_with_text("hello")).unwrap();
        assert_eq!(calls2.load(Ordering::SeqCst), 0);
        let s = cached2.stats();
        assert_eq!(s.hits, 2);
        assert_eq!(s.entries, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_persistence_survives_complete_json_responses() {
        let dir = tmp_dir("json");
        let schema = JsonSchema {
            name: "Test".into(),
            schema_json: "{}".into(),
        };
        {
            let inner = CountingClient::new();
            let cached = CachingClient::with_disk_persistence(Box::new(inner), &dir);
            let _ = cached.complete_json(req_with_text("q"), &schema).unwrap();
        }
        let inner2 = CountingClient::new();
        let json_calls2 = Arc::clone(&inner2.json_calls);
        let cached2 = CachingClient::with_disk_persistence(Box::new(inner2), &dir);
        let resp = cached2.complete_json(req_with_text("q"), &schema).unwrap();
        assert_eq!(json_calls2.load(Ordering::SeqCst), 0);
        assert_eq!(resp.parsed, serde_json::json!({ "ok": true }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_persistence_misses_when_dir_is_empty() {
        let dir = tmp_dir("miss-empty");
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::with_disk_persistence(Box::new(inner), &dir);
        let _ = cached.complete(req_with_text("hello")).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cached.stats().hits, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_persistence_treats_corrupted_files_as_misses() {
        let dir = tmp_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        // Drop a malformed file into the dir; the cache must NOT
        // crash and MUST fall through to calling the inner client.
        let inner = CountingClient::new();
        let calls = Arc::clone(&inner.calls);
        let cached = CachingClient::with_disk_persistence(Box::new(inner), &dir);
        let req = req_with_text("hello");
        let key = cached.cache_key(&req);
        let bad_path = dir.join(key_to_filename(&key));
        std::fs::write(&bad_path, b"not valid json").unwrap();
        let _ = cached.complete(req).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // After the call, the cache should have rewritten the file
        // with a valid entry.
        let written = std::fs::read(&bad_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&written).unwrap();
        assert_eq!(parsed.get("kind").and_then(|x| x.as_str()), Some("text"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_to_filename_is_deterministic_and_short() {
        let a = key_to_filename("alpha");
        let b = key_to_filename("alpha");
        assert_eq!(a, b);
        assert_eq!(a.len(), "0123456789abcdef.json".len());
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
