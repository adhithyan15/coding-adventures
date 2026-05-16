# Changelog — @coding-adventures/forme-cache

## 0.1.0 — 2026-05-15

Initial release. FM03 §5 — persistent cache layer for the orchestrator.

### Added

- `CacheBackend` storage adapter interface: `get`, `put`,
  `invalidate`, optional `invalidatePrefix`, `gc`, `dispose`.
- `CacheEntry` — `{ writtenMs, sizeBytes, payload, contentHash }`.
- `memoryCache()` — in-process backend, lost on dispose.  Suitable
  for tests and the orchestrator's watch-mode "front cache"
  (FM03 §7.5).
- `filesystemCache(root)` — disk-backed backend, sharded by first 2
  hex chars of the key.  Two-phase writes (`.tmp` + `rename`) for
  atomicity.  Keys < 2 chars are rejected up front.
- `cacheKey(input)` — FM03 §5.2 derivation.  BLAKE2b-256 hex digest
  over magic-prefixed, NUL-separated concatenation of `(stage.name,
  stage.version, canonical_json(config), input_revision,
  capability_set_hash)`.  Same inputs ⇒ same key; any single-field
  change ⇒ different key.
- `capabilitySetHash(caps)` — order-insensitive hash of a capability
  set; sorts then hashes.  Doesn't mutate the input array.
- `CACHE_KEY_VERSION = "forme-cache-v1"` — magic prefix that lets
  us flush the cache by bump.
- `CACHE_KEY_DIGEST_BYTES = 32`.
- `computeContentHash(bytes)` — BLAKE2b-256 hex digest, same
  function used for content addressing throughout the kernel.
- `verifyEntry(entry)` — recomputes the payload digest and compares
  with the stored `contentHash`; also checks `sizeBytes` matches
  `payload.byteLength`.
- `makeEntry(payload, now?)` — convenience constructor that
  populates derived fields consistently.

### Spec adherence

No deliberate divergences from FM03 §5.  Hash algorithm matches
forme-identity's choice of BLAKE2b (FM03 originally specified BLAKE3;
the monorepo has no BLAKE3 yet).  The on-disk shard size (2-char
prefix) is a v0 implementation choice not pinned by the spec.

### Notes

- Integrity verification is always-on, even in the in-memory backend.
  The discipline keeps observable behaviour identical across backends
  so swap-in-and-out doesn't change anything except the storage cost.
- Filesystem `gc` uses meta-file mtime rather than `writtenMs` from
  the JSON.  This means a `touch *.meta` (e.g. by an external tool)
  resets the GC clock — usually a feature, occasionally a footgun.
