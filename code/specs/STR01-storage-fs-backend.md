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
5. `fsync` the parent directory so the rename itself is durable.

The crate never exposes a "partial write" to a reader.

`delete` fsyncs the same parent directory after `remove_file`, for the
identical reason: an unlink is a directory-entry change too, and a crash
before that entry's disappearance is committed can resurrect the file. This
closes a real gap flagged in an earlier security review (`delete` used to do
no fsync at all) — see "Directory-fsync durability" below for what depends on
it, and `vault-pm-application-storage-core`'s `supersede_generation`, which
independently overwrites sensitive bodies through the fsync-durable `put`
path *before* calling `delete`, for the confidentiality property that does
not depend on this.

### Directory-fsync durability: hard failure, split by platform

Step 5's failure handling used to be uniformly best-effort ("if that fails we
don't error — some filesystems don't support directory fsync"), discarding
the `Result` outright. That was inconsistent with `vault-pm-local-host`,
which hard-fails the identical parent-directory `fsync` for `vault-pm.toml`
(`sync_directory` in `unix.rs`) — flagged as an open asymmetry in VLT-PM41
§8.1. The two files carry the same durability requirement: this crate's
records are how `vault-pm-application-storage-core` persists owner state
(`PreparedInit` / `Active` / `PendingPublication` / `PendingRotation`), which
sits under the exact same crash-safety journal as `vault-pm.toml`, and
recovery on both sides assumes a `put`/`compare_exchange`/`delete` that
returned `Ok` is durable rather than merely visible through the page cache.

The resolution, decided here rather than left open:

- **Unix (Linux, macOS): hard-fail.** A parent-directory `fsync` failure
  propagates as `StorageError::Unavailable` — the same class the tmp-file
  `fsync` in step 3 already uses. The record or unlink has already happened
  by this point (the rename/`remove_file` in step 4 succeeded), so the error
  means "durability is unconfirmed," not "nothing was written." Callers
  already have a mechanism built for exactly that ambiguity — the
  crash-safety journal replays or verifies the tail of a mutation rather than
  assuming success from a returned `Ok`.
- **Windows: stays a no-op**, but not because directory durability doesn't
  matter there — it does — but because `std::fs::File::open` cannot open a
  directory on Windows at all (`CreateFileW` needs `FILE_FLAG_BACKUP_SEMANTICS`,
  which safe `std::fs` never sets), and this crate is
  `#![forbid(unsafe_code)]`, so it has no escape hatch to the raw Win32 call
  the way `vault-pm-local-host`'s `windows.rs` does (which instead gets its
  durability from `MOVEFILE_WRITE_THROUGH` on the rename itself — a
  different primitive with no directory-handle equivalent). Hard-failing on
  Windows would mean every `put`/`delete` fails unconditionally on that
  platform, which is a regression, not a fix. NTFS also journals directory
  metadata changes (including renames and deletes) in its own `$LogFile`, so
  the userspace fsync this crate performs on Unix is compensating for a
  weaker guarantee that Windows's filesystem does not need compensated for
  in the same way.

`FsStorageBackendSummary::parent_directory_fsync_best_effort` reports this
split directly (`cfg!(windows)`) rather than the old unconditional `true`.

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
| `put`'s rename is durable in the live process but not on disk after a crash    | Unix: parent-directory `fsync` hard-fails to `Unavailable` rather than being discarded | `put_hard_fails_when_parent_directory_cannot_be_fsynced`            |
| `delete`'s unlink is durable in the live process but not on disk after a crash | Unix: parent-directory `fsync` after `remove_file`, hard-failing the same way          | `delete_fsyncs_the_parent_directory`, `delete_hard_fails_when_parent_directory_cannot_be_fsynced` |

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
