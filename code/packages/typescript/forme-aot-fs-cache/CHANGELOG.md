# Changelog — @coding-adventures/forme-aot-fs-cache

## 0.1.0 — 2026-05-17

Initial release.  Third FM06 AOT compiler family package and the
first with non-injected filesystem capability.  Implements the
disk-backed `CacheIO` contract consumed by
`forme-aot-incremental-cache`.

### Added

- `createFsCacheIO(options: FsCacheOptions): CacheIO` — disk-backed
  CacheIO over `node:fs`.  Sharded two-level layout
  (`cacheDir/<2hex>/<62hex>.cache`); atomic writes (temp file +
  rename) by default; lazy `cacheDir` validation on first `put`.
- `FsCacheOptions` — `cacheDir: string` (required, must already
  exist) + `atomicWrites?: boolean` (default `true`).

### Spec adherence

Implements FM06 §4 storage backend.  Consumes the `CacheIO`
contract from `forme-aot-incremental-cache` verbatim.  No spec
divergences.

### Behavioural notes

- **Sharded layout.**  256 shard sub-dirs (`00`–`ff`) keep any
  single dir under ~10k entries for caches up to 2.5M keys.  Real
  cache file: `cacheDir/<first 2 hex>/<remaining 62 hex>.cache`.
- **Atomic writes (default on).**  Each `put` writes to a temp
  file (`<final>.tmp.<pid>.<rand>`) then `fs.rename`s onto the
  final path.  POSIX guarantees `rename` is atomic within one
  filesystem.  Failed renames trigger best-effort temp-file
  unlink.
- **`atomicWrites: false`** skips the temp-rename dance — use
  when concurrent writes to the same key are impossible
  (single-process, single-thread, or caller-coordinated).
- **`cacheDir` MUST exist.**  We don't auto-create.  A missing
  `cacheDir` surfaces as a clear `Error` from the first `put`
  rather than a silent mis-write to CWD.  Shard sub-dirs ARE
  created on demand.
- **`list()` ignores non-cache files** (anything not matching
  `^[0-9a-f]{62}\.cache$` in a shard dir), non-shard top-level
  entries, and leftover `.tmp.*` files from crashed previous runs.
- **Multiple `CacheIO` instances over the same `cacheDir` are
  safe** — atomic writes prevent partial-state reads; no
  in-process coordination state.
- **Graceful empty state.**  `get` / `list` on a non-existent
  `cacheDir` return `null` / `[]` respectively (so callers can
  construct the IO before the dir exists).  Only `put` insists on
  a valid dir.

### Security posture

Four concerns explicitly addressed (pre-push review):

- **Path traversal via cache keys.**  `assertValidKey` rejects
  anything outside `/^[0-9a-f]{64}$/` BEFORE touching the
  filesystem.  Keys like `../../etc/passwd` (which don't match
  the regex) throw synchronously.  Defence in depth: `path.join`
  would normalise even if the assertion were bypassed.
- **Atomic-write correctness.**  Temp file + rename per `put`.
  No partial-read window.  Failed renames clean up the temp
  file (best-effort `unlink`).  Tests pin both the happy-path
  "no .tmp leftovers after 50 concurrent puts" and the
  rename-fail cleanup path.
- **Symlink following.**  No `realpath`, no `..` resolution.  All
  IO uses `fs.promises`.  Worst-case attacker control of
  `cacheDir` points us at a directory they already have write
  access to.
- **No tmpdir leakage.**  Temp files live under the same shard
  dir as their final destination — cleaned up automatically when
  `cacheDir` is removed.  No stray `/tmp/*` files.

### Capabilities

`["fs"]` — first FM06 package with non-injected filesystem
access.  Upstream `forme-aot-incremental-cache` stays
`["hash"]`-only by accepting any `CacheIO`; this package wires
that interface to real disk.

### Tests

33 tests in `fs-cache.test.ts`:

- Basic round-trip (6)
- Sharded layout (2)
- Key validation including path-traversal-shaped keys + control
  chars (6)
- `cacheDir` validation (3)
- Atomic writes including 50-way concurrent stress (3)
- `list()` filtering of stray files (3)
- Defensive non-ENOENT error handling (4)
- End-to-end via incremental-cache layer (1)
- Multiple IOs over same dir (1)
- 1 MiB value round-trip stress (1)
- Overwrite (1)
- Randomness-in-temp-name (1)
- Whitelist for stray .cache.tmp.* files (1)

Coverage: **97.95% line / 95.23% branch** — above the FM04 §14.4
≥95% line target.  Uncovered lines: the defensive non-ENOENT arm
of `ensureCacheDirValid` (e.g. EACCES — hard to reproduce
cross-platform).

### v0 simplifications (documented)

- **No eviction policy.**  A future `forme-aot-cache-gc` package
  would walk `list()` + `unlink` stale entries.
- **No compression.**  Entries store raw UTF-8.
- **No `fsync`.**  Power-loss-during-write may corrupt the latest
  entry; acceptable for build caches.
- **Single-filesystem atomic guarantee.**  If `cacheDir` straddles
  a mount point, `rename` atomicity weakens to "almost always
  atomic in practice."
