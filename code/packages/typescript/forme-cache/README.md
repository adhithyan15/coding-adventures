# @coding-adventures/forme-cache

Persistent cache layer for the Forme orchestrator (FM03 §5).

Three concerns:

- **Storage adapters** — `CacheBackend` interface plus two built-ins.
- **Key derivation** — content-addressed keys via BLAKE2b.
- **Integrity** — every read verifies the payload hash.

## Exports

| Group       | Exports                                                                              |
| ----------- | ------------------------------------------------------------------------------------ |
| Types       | `CacheBackend`, `CacheEntry`, `CacheKeyInput`                                         |
| Backends    | `memoryCache()`, `filesystemCache(root)`                                              |
| Keys        | `cacheKey(input)`, `capabilitySetHash(caps)`, `CACHE_KEY_VERSION`, `CACHE_KEY_DIGEST_BYTES` |
| Integrity   | `computeContentHash(bytes)`, `verifyEntry(entry)`, `makeEntry(bytes, now?)`           |

## Quick reference

```typescript
import {
  cacheKey, makeEntry, memoryCache,
} from "@coding-adventures/forme-cache";

const backend = memoryCache();

const key = cacheKey({
  stageName: "@forme/parse-markdown",
  stageVersion: "0.1.0",
  stageConfig: { gfm: true },
  inputRevision: source.revision,
  capabilities: stage.capabilities,
});

const cached = await backend.get(key);
if (cached) {
  return decode(cached.payload);
}
const result = await stage.run(input, config, ctx);
await backend.put(key, makeEntry(encode(result)));
```

## Cache key format (FM03 §5.2)

```
key = blake2b-256(
  "forme-cache-v1\0"
  || stage.name      || "\0"
  || stage.version   || "\0"
  || canonical_json(config) || "\0"
  || input_revision  || "\0"
  || capability_set_hash
)
```

- The `forme-cache-v1` prefix is a kernel-version barrier — bumping it
  invalidates every entry without a manual flush.
- NUL byte separators prevent adjacent-field collisions.
- Capability sets are sorted before hashing so order doesn't matter.
- Config is canonicalised (RFC 8785) so key-order changes don't break reuse.

## Integrity contract

Every `get` recomputes BLAKE2b over the payload and compares with the stored `contentHash`. Mismatch ⇒ return `null` AND invalidate the entry. The orchestrator treats integrity failures the same as cache misses — the stage re-executes, and the corrupt entry is cleaned up.

## Filesystem layout

```
<root>/
  ab/
    abcdef...01.entry       payload bytes
    abcdef...01.meta        JSON: { writtenMs, sizeBytes, contentHash }
  ...
```

Sharded by first 2 hex chars of the key — keeps any single directory bounded even when the cache holds millions of entries. Writes are two-phase (`.tmp` then `rename`) for atomicity.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets ≥95% line coverage. Filesystem backend is exercised against real `os.tmpdir()` directories with cleanup.
