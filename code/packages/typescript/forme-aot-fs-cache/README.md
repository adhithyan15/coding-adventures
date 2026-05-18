# @coding-adventures/forme-aot-fs-cache

Disk-backed `CacheIO` implementation for
[`forme-aot-incremental-cache`](../forme-aot-incremental-cache).
FM06 §4 storage backend.

Third package of the FM06 family and the first FM06 package with
non-injected filesystem access (`capabilities: ["fs"]`).  The
upstream `forme-aot-incremental-cache` stays `["hash"]`-only by
accepting any `CacheIO`; this package is what wires that interface
to real disk for production use.

## Quick start

```ts
import { createIncrementalCache } from "@coding-adventures/forme-aot-incremental-cache";
import { createFsCacheIO } from "@coding-adventures/forme-aot-fs-cache";
import { mkdir } from "node:fs/promises";

await mkdir(".forme-cache", { recursive: true });

const cache = createIncrementalCache(createFsCacheIO({
  cacheDir: ".forme-cache",
}));

const result = await cache.sliceWithCache(doc, pages, {
  activeContexts: ["screen"],
});
```

## Storage layout

```
cacheDir/
  ab/                                              ← first 2 hex chars of cache key
    cd<...60 more hex chars>.cache                 ← cache entry (one file per key)
  ef/
    01<...60 more hex chars>.cache
  ...
```

Two-level sharded layout — 256 sub-dirs (`00`–`ff`).  At 2.5M
total keys each sub-dir holds ~10k entries, comfortably under
typical filesystem dir limits (ext4: 64k, NTFS practically
unbounded but slow above 300k, APFS: practically unbounded).

## Options

```ts
interface FsCacheOptions {
  cacheDir: string;          // must already exist
  atomicWrites?: boolean;    // default true
}
```

### `cacheDir` (required)

Directory under which cache entries are stored.  MUST exist before
calling `createFsCacheIO` — we don't auto-create.  A missing
`cacheDir` is a configuration error worth surfacing loudly (you
get a clear `Error` from the first `put` rather than a silent
mis-write into wherever Node's CWD happens to be).

Shard sub-dirs (256 of them) ARE created on demand.

### `atomicWrites` (default `true`)

`put` writes to a temp file then `fs.rename`s onto the final path.
POSIX guarantees `rename` is atomic within a single filesystem, so
concurrent puts for the same key never leave a partial-write reader.
Cost: one extra file create per write.  Set to `false` to skip the
temp-rename dance when concurrent writes are impossible
(single-process, single-thread, or caller-coordinated).

## Capabilities — `["fs"]`

First FM06 package with non-injected filesystem access.  Reads /
writes cache entries under the caller-supplied `cacheDir`.  No
network, no shell, no env beyond what `node:fs` / `node:path`
themselves consult.

## Security posture

Four concerns addressed:

1. **Path traversal via cache keys.**  `assertValidKey` rejects
   anything outside `/^[0-9a-f]{64}$/` BEFORE touching the
   filesystem — so a key like `../../etc/passwd` (which doesn't
   match the regex) throws synchronously.  Defence in depth:
   `path.join(cacheDir, key.slice(0, 2), ...)` would normalise
   even if the assertion were bypassed.
2. **Atomic-write correctness.**  Each `put` uses
   `writeFile(temp) + rename(temp → final)`.  Concurrent puts to
   the same key never produce a partial-read.  Failed renames
   trigger best-effort temp-file unlink for cleanup.  Tests
   pin both the happy-path "no .tmp leftovers" and the
   failure-cleanup paths.
3. **Symlink following.**  All file IO uses `fs.promises`; we
   never call `realpath`, never resolve `..`, never follow
   symlinks intentionally.  The worst an attacker controlling the
   `cacheDir` could do is point it at a directory they already
   have write access to — and they'd need that anyway.
4. **No tmpdir leakage.**  Temp files live under the same shard
   dir as the final file (`<file>.tmp.<pid>.<rand>`), so they're
   cleaned up automatically when the cache dir is removed.  No
   stray `/tmp/*` files left behind by crashed processes (other
   than the .tmp.* leftovers in the cache dir itself, which
   `list()` ignores).

## Tests

33 tests in `fs-cache.test.ts`:

- Basic round-trip (put → get; missing key → null; list returns
  all keys sorted+frozen; empty cache; reflective put → list)
- Sharded layout (entries in `cacheDir/<2hex>/<rest>.cache`;
  different prefixes → different shard dirs)
- Key validation (rejects short / non-hex / uppercase /
  path-traversal-shaped / empty / NUL-byte keys)
- `cacheDir` validation (gracefully empty on missing for get/list;
  clear error on missing for put; rejects file-as-dir)
- Atomic writes (no .tmp leftover on success; `atomicWrites: false`
  works; concurrent puts to same key are race-safe; 50 concurrent
  puts leave no stray temp files)
- List filtering (ignores non-`.cache` files, non-shard dirs,
  README at top level, leftover .tmp.* files)
- Defensive non-ENOENT error handling (EISDIR on read; ENOTDIR
  on shard; cacheDir-is-file → list throws; rename-fails-cleanup
  path)
- End-to-end via incremental cache layer (JSON entry round-trip)
- Multiple IOs over same dir see each other's writes
- 1 MiB value round-trip stress test
- Overwrite (second put replaces value)

Coverage: **97.95% line / 95.23% branch** — above the FM04 §14.4
≥95% line target.  Uncovered lines are the defensive non-ENOENT
arm of `ensureCacheDirValid` (e.g. EACCES — hard to reproduce
cross-platform without elevated test infrastructure).

## Spec adherence

Implements FM06 §4 storage backend.  Consumes the `CacheIO` contract
from `forme-aot-incremental-cache` verbatim.  No spec divergences.

## v0 simplifications

- **No eviction policy.**  Caches grow unbounded; the AOT compiler
  driving this is expected to call `list()` + selective `unlink`
  at its own cadence (a future `forme-aot-cache-gc` package).
- **No compression.**  Entries store as raw UTF-8.  Compression
  layers would slot in as a `CacheIO` decorator.
- **Single-filesystem assumption.**  POSIX `rename` atomicity holds
  within one filesystem.  If `cacheDir` straddles a mount point
  (rare for build artefacts) the atomic-write guarantee weakens
  to "almost always atomic in practice."
- **No fsync.**  We don't call `fdatasync` on cache entries —
  power-loss-during-write may corrupt the latest entry.  Acceptable
  for build caches (rebuild on next run); not acceptable for
  primary data stores.
