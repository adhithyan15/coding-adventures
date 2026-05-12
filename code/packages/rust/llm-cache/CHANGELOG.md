# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-12

### Added

**Disk persistence.** The cache can now write each entry to a file
keyed on the FNV-1a hash of the cache key, so cache hits survive
process restarts. Built on the deterministic-prompts invariant —
the same `(model, prompt_hash, schema_name)` always produces the
same response, so persisting it is sound.

- `CachingClient::with_disk_persistence(inner, dir)` — disk-backed
  variant with an unbounded in-memory cache on top.
- `CachingClient::with_disk_persistence_and_capacity(inner, dir, n)`
  — both an in-memory bound AND disk persistence.
- Disk hits are promoted to memory on first read so subsequent
  lookups stay fast.
- Best-effort I/O: malformed or unreadable files are silently
  treated as misses (cache is an accelerator, not a database).
- Hand-rolled JSON serialization (no serde derives on llm-gateway
  types) — each entry is a self-describing object with `kind`,
  the provider identity, usage, latency, and the typed response
  payload.

### Verified end-to-end

Wrapping the TSA demo's gateway clients in
`CachingClient::with_disk_persistence` produced a 10× wall-clock
speedup on repeat runs against `gemma4:latest` + `llama3.1:8b`:

- Cold run (9 LLM calls to Ollama): ~60 s
- Warm run (9 disk hits, 0 LLM calls): ~0.5 s (excluding Rust
  binary launch overhead)

5 new tests cover: survive-new-client (disk hit on a fresh
in-memory state), memory-promotion (disk hit promotes to memory),
complete_json round-trip, empty-dir misses, and
corrupted-file-falls-through-and-self-heals.

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
