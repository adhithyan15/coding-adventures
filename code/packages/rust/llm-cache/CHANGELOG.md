# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-12

### Added

Content-addressed prompt cache for any `LlmClient`. Wraps an inner
client with a `(model, prompt_hash[, schema_name])`-keyed in-memory
hashmap. Built on the deterministic-prompts invariant: every primitive
uses `temperature: 0.0` and is content-addressed via
`llm_primitives::fingerprint_prompt`, so identical inputs produce
identical outputs — caching is sound.

- `CachingClient` — `LlmClient` impl that delegates to an inner
  client on miss and serves from cache on hit.
- `CachingClient::with_capacity(inner, n)` — bounded variant with
  FIFO eviction.
- `CacheStats { hits, misses, entries }` plus a `hit_rate()` helper.
- `CachingClient::clear()` — empty the cache (preserves the
  hit/miss counters so a session's cumulative stats stay intact).
- `complete_json` cache keys include the JSON schema name so the
  same prompt at different roles doesn't collide.
- 10 unit tests cover the hit/miss/eviction/clear paths plus the
  multi-schema and multi-model isolation invariants.
