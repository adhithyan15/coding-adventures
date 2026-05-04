# STR01 — File-system Storage Backend

## Overview

Filesystem-backed implementation of `storage_core::StorageBackend`.
The first concrete persistent backend beyond `InMemoryBackend`,
demonstrating that the Vault stack's storage-agnosticism property
is real: a vault can store its records on a local disk, in a
cloud bucket, or in any other byte-addressed medium, and the
backend never sees plaintext.

Implementation lives at `code/packages/rust/storage-fs/`.

## Why this layer exists

`storage_core` defines a trait; `InMemoryBackend` covers tests.
But every real Vault deployment needs persistence. STR-FILE is
the simplest persistent backend (a directory tree on local
disk) and serves three roles:

1. **First user-visible deployment target** — `~/.vault/data` on a
   single user's laptop, à la KeePassXC.
2. **Reference implementation** — the canonical "what does a
   `StorageBackend` need to handle?" that other backend crates
   (S3, GDrive, WebDAV, git) can mirror.
3. **Storage-agnosticism evidence** — the backend has no
   knowledge of what's inside the records (VLT01 sees to that),
   so the same on-disk file works equally well as the body of
   an S3 object, a row in a SQLite blob column, or a git LFS
   pointer.

## On-disk layout

```text
   <root>/
     <hex(namespace)>/
       <hex(key)>          ← single file per record (header + body)
       <hex(key)>.tmp      ← only mid-write; cleaned on init
```

Hex-encoded names so arbitrary key bytes (which the Vault stack
above may treat as opaque) survive the filesystem's allowed-
character rules. `<root>` is caller-supplied.

## Single-file record format

```text
   record_file =
       magic(4) "STRF" ||
       version(1) = 1 ||
       meta_len(4 BE) ||
       meta_json(N) ||
       body(rest)
```

`meta_json` is a JSON object with `revision`, `content_type`,
`created_at`, `updated_at`, and the caller-supplied JSON
`metadata`. The body bytes follow immediately — no length-of-
body field because "all the rest" is the body.

A single file per record means readers see either the whole old
record or the whole new record — never a half-applied write —
because `rename(2)` is atomic w.r.t. concurrent reads.

## Atomic writes

Every `put` writes to `<key>.tmp` first:

1. Open `<key>.tmp` for write+truncate.
2. Write header + meta_json + body.
3. `fsync` the tmp file so the bytes hit the platter.
4. `rename(<key>.tmp, <key>)` — atomic on POSIX.
5. Best-effort `fsync` of the parent directory (some
   filesystems need this for true rename durability).

The crate never exposes a "partial write" to a reader.

## Initialize / crash recovery

`initialize()`:

1. Creates `<root>` if missing.
2. Walks every namespace dir and removes any stranded `.tmp`
   files — these are the result of crashes during steps 1–3
   above and don't represent committed state.
3. Scans every committed record file to find the highest
   revision number, then seeds the in-memory revision counter
   so monotonic numbering survives process restart.

## Public API

```rust
pub struct FsStorageBackend { /* … */ }

impl FsStorageBackend {
    pub fn new(root: impl Into<PathBuf>) -> Self;
}

impl StorageBackend for FsStorageBackend {
    fn initialize(&self) -> Result<(), StorageError>;
    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError>;
    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError>;
    fn delete(&self, namespace: &str, key: &str, if_revision: Option<&Revision>)
        -> Result<(), StorageError>;
    fn list(&self, namespace: &str, options: StorageListOptions)
        -> Result<StoragePage, StorageError>;
    fn stat(&self, namespace: &str, key: &str)
        -> Result<Option<StorageStat>, StorageError>;
    fn acquire_lease(&self, name: &str, ttl_ms: u64)
        -> Result<Option<StorageLease>, StorageError>;
}
```

## Threat model & test coverage

| Threat                                                                         | Defence                                              | Test                                                                |
|--------------------------------------------------------------------------------|------------------------------------------------------|---------------------------------------------------------------------|
| Reader observes a half-applied put                                             | Atomic `rename(2)` after `fsync` of `<key>.tmp`      | covered structurally; specific test would require concurrent reader |
| Crash mid-write strands `<key>.tmp`                                            | `initialize()` cleans `.tmp` files                   | `initialize_removes_stranded_tmp_files`                             |
| Restart resets the revision counter and reuses revisions                       | `initialize()` scans for the highest existing revision and seeds | `restart_picks_up_revision_counter`                                 |
| Concurrent put-of-same-key races                                               | Per-process `Mutex` on writes                        | covered by trait contract; cross-process is out of scope            |
| Wrong CAS revision overwrites                                                  | `if_revision` mismatch → `Conflict`                  | `put_with_wrong_if_revision_conflicts`, `put_with_if_revision_against_missing_record_conflicts`, `delete_with_wrong_if_revision_conflicts` |
| Corrupted file contents (someone edits the bytes)                              | Magic + version check; `Backend` error on mismatch   | `corrupted_magic_returns_backend_error`, `truncated_file_returns_backend_error` |
| Caller supplies arbitrary key bytes                                            | Hex-encoded filenames                                | covered by all roundtrip tests with default chars                   |
| Metadata-validator rejection (must be object)                                  | Defaults missing metadata to empty object            | covered by `put_get_roundtrip` (metadata round-trips)               |
| Backend leaks plaintext                                                        | Stack invariant: VLT01 above; STR-FILE never sees pt | structural — the `body: Vec<u8>` is opaque                          |

## Out of scope (future PRs)

- **Encryption** — VLT01 sealed-store is layered above; this
  crate is content-agnostic.
- **Replication / sync** — VLT10.
- **Cross-process advisory locks** (POSIX `flock`/`lockf`).
- **Cloud-backed implementations** — S3 / GCS / Google Drive /
  WebDAV / git / IPFS / SQLite. Each is a separate sibling
  crate following the same `StorageBackend` trait.
- **GC of orphaned namespace directories.**

## Citations

- POSIX `rename(2)` — atomicity guarantees.
- `fsync(2)` / `fdatasync(2)` — durability.
- VLT00-vault-roadmap.md — STR backend layer purpose.
- VLT01-vault-sealed-store.md — what sits above this layer
  (the source of the ciphertext bytes).
- `storage_core::StorageBackend` — the trait this implements.
