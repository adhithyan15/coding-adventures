// Builds a config by mutating a `Default::default()` base for readability;
// identical to a struct literal.
#![allow(clippy::field_reassign_with_default)]
//! # coding_adventures_storage_fs — STR-FILE
//!
//! ## What this crate does
//!
//! Filesystem-backed implementation of
//! [`storage_core::StorageBackend`]. Persists records to a
//! directory tree on disk with atomic write + rename + fsync for
//! crash safety. The backend is **opaque to record content** —
//! the Vault stack encrypts above it (VLT01 sealed-store), so
//! this layer only ever sees ciphertext + non-secret metadata.
//!
//! Use this when you want to hand a vault a path on disk
//! (`~/.vault/data` say) and have it Just Work.
//!
//! ## Layout on disk
//!
//! ```text
//!   <root>/
//!     <hex(namespace)>/
//!       <hex(key)>          ← single file per record (header + body)
//!       <hex(key)>.tmp      ← exists only mid-write; cleaned on init
//!     <hex(namespace_2)>/
//!       …
//! ```
//!
//! Each record file's binary format:
//!
//! ```text
//!   magic(4) "STRF" || version(1) = 1 ||
//!   meta_len(4 BE) || meta_json(N) || body(rest)
//! ```
//!
//! `meta_json` is a JSON object with the record's metadata
//! (revision, content_type, created_at, updated_at,
//! caller-supplied metadata). The body bytes follow immediately —
//! no length-of-body field because "all the rest" is the body.
//!
//! ## Atomicity & crash safety
//!
//! Writes use the standard "write to tmp, fsync, rename" pattern:
//!
//! 1. Open `<key>.tmp` for write+truncate.
//! 2. Write header + meta + body.
//! 3. `fsync` the tmp file.
//! 4. Atomic-rename `<key>.tmp` → `<key>` (POSIX `rename(2)` is
//!    atomic relative to readers).
//! 5. Best-effort `fsync` of the parent directory so the rename is
//!    durable; if that fails we don't error (some filesystems
//!    don't support directory fsync).
//!
//! On `initialize`, the backend walks `<root>` and removes any
//! stranded `.tmp` files — those are the result of crashes during
//! step 1–3 above and don't represent any committed state.
//!
//! ## What this crate does *not* do
//!
//! - **No encryption.** That's VLT01.
//! - **No replication / sync.** That's VLT10.
//! - **No durable leases.** Leases live in memory in the same
//!   shape as `InMemoryStorageBackend` — they are scoped to the
//!   current process. (Cross-process file-system leases would
//!   need POSIX `flock`/`lockf` which is platform-dependent;
//!   defer.)
//! - **No directory-level locking for concurrent writers.** Two
//!   `put`s of the same `(namespace, key)` from the same process
//!   are serialized via `Mutex` on the in-memory revision counter.
//!   Cross-process concurrency is not supported (a vault should
//!   be opened by one process at a time).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_serializer::serialize as json_serialize;
use coding_adventures_json_value::{parse as json_parse, JsonNumber, JsonValue};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};
use storage_core::{
    LeaseToken, Revision, StorageBackend, StorageError, StorageLease, StorageListOptions,
    StorageMetadata, StoragePage, StoragePutInput, StorageRecord, StorageStat, TimestampMs,
};

// ─────────────────────────────────────────────────────────────────────
// 1. Wire format
// ─────────────────────────────────────────────────────────────────────

const MAGIC: &[u8; 4] = b"STRF";
const VERSION: u8 = 1;
const HEADER_FIXED: usize = 4 + 1 + 4; // magic + version + meta_len

/// Payload-free description of the filesystem storage backend surface.
///
/// This is intended for D18A/D18D host/catalog diagnostics where the
/// caller needs to know what storage guarantees a backend provides
/// without logging the backend root path, namespaces, keys, metadata,
/// or record bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsStorageBackendSummary {
    /// Record magic bytes as an ASCII label.
    pub record_magic: &'static str,
    /// Version byte in each on-disk record.
    pub record_format_version: u8,
    /// Whether each logical record maps to one committed file.
    pub one_file_per_record: bool,
    /// Whether namespaces and keys are hex encoded for path safety.
    pub hex_encoded_names: bool,
    /// Whether writes use write-tmp, fsync, atomic rename.
    pub atomic_write_rename: bool,
    /// Whether `initialize` removes stranded `.tmp` files.
    ///
    /// Scope, since the recovery walk is now cached: the sweep happens on the
    /// FIRST successful `initialize()` of each `FsStorageBackend`, not on every
    /// call. A `.tmp` stranded after that point survives until a new backend is
    /// constructed over the root — which is the realistic recovery path anyway,
    /// since stranded tmp files come from a write interrupted by a crash.
    ///
    /// Nothing reads them in the meantime: `list()` skips the extension, and
    /// `get()`/`put()` address records by `hex_encode(key)`, which is `[0-9a-f]`
    /// only and so can never name a `.tmp` file. Growth is bounded at one per
    /// key, because the tmp path is deterministic and opened with
    /// `truncate(true)`.
    pub tmp_files_cleaned_on_initialize: bool,
    /// Whether the parent directory fsync is best-effort.
    pub parent_directory_fsync_best_effort: bool,
    /// Whether record bodies are opaque bytes to this backend.
    pub content_opaque_to_backend: bool,
    /// Whether leases survive process restart.
    pub leases_persisted: bool,
    /// Whether cross-process writer locking is provided.
    pub cross_process_locking: bool,
}

impl FsStorageBackendSummary {
    /// Summary for the STR-FILE surface implemented by this crate.
    pub const fn current() -> Self {
        Self {
            record_magic: "STRF",
            record_format_version: VERSION,
            one_file_per_record: true,
            hex_encoded_names: true,
            atomic_write_rename: true,
            tmp_files_cleaned_on_initialize: true,
            parent_directory_fsync_best_effort: true,
            content_opaque_to_backend: true,
            leases_persisted: false,
            cross_process_locking: false,
        }
    }
}

/// Return a payload-free description of the filesystem storage backend surface.
pub const fn fs_storage_backend_summary() -> FsStorageBackendSummary {
    FsStorageBackendSummary::current()
}

// ─────────────────────────────────────────────────────────────────────
// 2. Backend struct
// ─────────────────────────────────────────────────────────────────────

/// Filesystem-backed `StorageBackend`. Wrap a directory and you've
/// got a persistent vault store with crash-safe writes.
pub struct FsStorageBackend {
    root: PathBuf,
    /// Mutex serialises all writes within this process; needed so
    /// the revision counter advances monotonically.
    write_lock: Mutex<()>,
    revision_counter: AtomicU64,
    /// Whether the one-time recovery scan in `initialize()` has completed.
    /// Per backend instance, so a genuine restart (a NEW `FsStorageBackend`
    /// over the same root) scans again.
    ///
    /// That re-scan recovers a floor, but NOT necessarily the true one, and the
    /// difference is a live defect rather than a subtlety. `highest` is derived
    /// from surviving records, so deleting the record that holds the high-water
    /// mark lowers it; a fresh instance over that root then re-issues revisions
    /// the previous instance already handed out, and a stale `if_revision`
    /// compare-and-swap guard passes where it must fail. Caching closes this
    /// within one instance -- which is what #12139 asked for -- and closes it
    /// not at all across instances, including two live backends over one root.
    ///
    /// The real fix is to stop deriving the high-water mark from live records:
    /// persist it beside them, or fold a per-boot epoch into the revision. That
    /// is tracked separately; do not read this flag as a guarantee of global
    /// revision uniqueness, because it is not one.
    scanned: AtomicBool,
    /// In-memory leases. Same shape as `InMemoryStorageBackend`'s.
    leases: Mutex<HashMap<String, StorageLease>>,
    lease_counter: AtomicU64,
}

impl FsStorageBackend {
    /// Build a backend rooted at `root`. The directory is created
    /// on first `initialize()` if it doesn't exist.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            write_lock: Mutex::new(()),
            revision_counter: AtomicU64::new(0),
            scanned: AtomicBool::new(false),
            leases: Mutex::new(HashMap::new()),
            lease_counter: AtomicU64::new(0),
        }
    }

    /// Describe the storage guarantees without exposing this backend's root.
    pub fn surface_summary(&self) -> FsStorageBackendSummary {
        fs_storage_backend_summary()
    }

    fn next_revision(&self) -> Result<Revision, StorageError> {
        let n = self.revision_counter.fetch_add(1, Ordering::SeqCst) + 1;
        Revision::new(format!("rev-{:020}", n))
    }

    fn ns_dir(&self, namespace: &str) -> PathBuf {
        self.root.join(hex_encode(namespace.as_bytes()))
    }

    fn key_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.ns_dir(namespace).join(hex_encode(key.as_bytes()))
    }

    fn key_tmp_path(&self, namespace: &str, key: &str) -> PathBuf {
        let mut p = self.key_path(namespace, key);
        p.set_extension("tmp");
        p
    }
}

// ─────────────────────────────────────────────────────────────────────
// 3. Hex encoding (used for filename safety)
// ─────────────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Result<Vec<u8>, StorageError> {
    if !s.len().is_multiple_of(2) {
        return Err(StorageError::Backend {
            message: "fs storage: malformed hex filename".into(),
        });
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let pair = std::str::from_utf8(chunk).map_err(|_| StorageError::Backend {
            message: "fs storage: non-utf8 hex filename".into(),
        })?;
        let n = u8::from_str_radix(pair, 16).map_err(|_| StorageError::Backend {
            message: "fs storage: non-hex character in filename".into(),
        })?;
        out.push(n);
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────
// 4. Header + metadata serialisation
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct StoredMeta {
    revision: Revision,
    content_type: String,
    metadata: StorageMetadata,
    created_at: TimestampMs,
    updated_at: TimestampMs,
}

fn now_ms() -> TimestampMs {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_millis() as TimestampMs
}

fn meta_to_json(meta: &StoredMeta) -> JsonValue {
    JsonValue::Object(vec![
        (
            "revision".to_string(),
            JsonValue::String(meta.revision.as_str().to_string()),
        ),
        (
            "content_type".to_string(),
            JsonValue::String(meta.content_type.clone()),
        ),
        (
            "created_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(meta.created_at as i64)),
        ),
        (
            "updated_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(meta.updated_at as i64)),
        ),
        ("metadata".to_string(), meta.metadata.clone()),
    ])
}

fn meta_from_json(v: &JsonValue) -> Result<StoredMeta, StorageError> {
    let entries = match v {
        JsonValue::Object(e) => e,
        _ => {
            return Err(StorageError::Backend {
                message: "fs storage: meta JSON not an object".into(),
            });
        }
    };
    let mut revision: Option<Revision> = None;
    let mut content_type: Option<String> = None;
    let mut created_at: Option<TimestampMs> = None;
    let mut updated_at: Option<TimestampMs> = None;
    let mut metadata: Option<StorageMetadata> = None;
    for (k, v) in entries {
        match k.as_str() {
            "revision" => {
                if let JsonValue::String(s) = v {
                    revision = Some(Revision::new(s.clone())?);
                }
            }
            "content_type" => {
                if let JsonValue::String(s) = v {
                    content_type = Some(s.clone());
                }
            }
            "created_at" => {
                if let JsonValue::Number(JsonNumber::Integer(n)) = v {
                    created_at = Some(*n as TimestampMs);
                }
            }
            "updated_at" => {
                if let JsonValue::Number(JsonNumber::Integer(n)) = v {
                    updated_at = Some(*n as TimestampMs);
                }
            }
            "metadata" => {
                metadata = Some(v.clone());
            }
            _ => {}
        }
    }
    Ok(StoredMeta {
        revision: revision.ok_or_else(|| StorageError::Backend {
            message: "fs storage: meta missing revision".into(),
        })?,
        content_type: content_type.ok_or_else(|| StorageError::Backend {
            message: "fs storage: meta missing content_type".into(),
        })?,
        created_at: created_at.ok_or_else(|| StorageError::Backend {
            message: "fs storage: meta missing created_at".into(),
        })?,
        updated_at: updated_at.ok_or_else(|| StorageError::Backend {
            message: "fs storage: meta missing updated_at".into(),
        })?,
        // Default to an empty JSON object — the storage-core
        // validator requires `metadata` to be an object, not Null.
        metadata: metadata.unwrap_or(JsonValue::Object(Vec::new())),
    })
}

fn write_record_atomic(
    tmp: &Path,
    final_path: &Path,
    meta: &StoredMeta,
    body: &[u8],
) -> Result<(), StorageError> {
    let meta_json = meta_to_json(meta);
    let meta_str = json_serialize(&meta_json).map_err(|e| StorageError::Backend {
        message: format!("fs storage: serialize meta: {}", e),
    })?;
    let meta_bytes = meta_str.into_bytes();
    let meta_len: u32 = meta_bytes
        .len()
        .try_into()
        .map_err(|_| StorageError::Backend {
            message: "fs storage: metadata too large for 4-byte length".into(),
        })?;

    // 1. Ensure parent dir exists.
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).map_err(io_to_storage)?;
    }

    // 2. Write tmp.
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(tmp)
            .map_err(io_to_storage)?;
        f.write_all(MAGIC).map_err(io_to_storage)?;
        f.write_all(&[VERSION]).map_err(io_to_storage)?;
        f.write_all(&meta_len.to_be_bytes())
            .map_err(io_to_storage)?;
        f.write_all(&meta_bytes).map_err(io_to_storage)?;
        f.write_all(body).map_err(io_to_storage)?;
        f.sync_all().map_err(io_to_storage)?;
    } // f is closed before rename — important on Windows.

    // 3. Atomic rename.
    fs::rename(tmp, final_path).map_err(io_to_storage)?;

    // 4. Best-effort fsync of parent dir for true durability.
    if let Some(parent) = final_path.parent() {
        if let Ok(d) = File::open(parent) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

fn read_record_full(path: &Path) -> Result<Option<(StoredMeta, Vec<u8>)>, StorageError> {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_to_storage(e)),
    };
    let mut all = Vec::new();
    f.read_to_end(&mut all).map_err(io_to_storage)?;
    if all.len() < HEADER_FIXED {
        return Err(StorageError::Backend {
            message: "fs storage: record file shorter than header".into(),
        });
    }
    if &all[..4] != MAGIC {
        return Err(StorageError::Backend {
            message: "fs storage: bad magic in record file".into(),
        });
    }
    if all[4] != VERSION {
        return Err(StorageError::Backend {
            message: "fs storage: unsupported record-file version".into(),
        });
    }
    let meta_len = u32::from_be_bytes([all[5], all[6], all[7], all[8]]) as usize;
    let meta_start = HEADER_FIXED;
    let meta_end = meta_start
        .checked_add(meta_len)
        .ok_or_else(|| StorageError::Backend {
            message: "fs storage: meta_len overflow".into(),
        })?;
    if meta_end > all.len() {
        return Err(StorageError::Backend {
            message: "fs storage: meta extends past EOF".into(),
        });
    }
    let meta_str =
        std::str::from_utf8(&all[meta_start..meta_end]).map_err(|_| StorageError::Backend {
            message: "fs storage: meta not UTF-8".into(),
        })?;
    let meta_json = json_parse(meta_str).map_err(|e| StorageError::Backend {
        message: format!("fs storage: parse meta JSON: {}", e),
    })?;
    let meta = meta_from_json(&meta_json)?;
    let body = all[meta_end..].to_vec();
    Ok(Some((meta, body)))
}

fn io_to_storage(e: io::Error) -> StorageError {
    StorageError::Unavailable {
        message: format!("fs storage io error: {}", e),
    }
}

/// Read a directory, returning every entry's path, and treat any failure as a
/// hard error rather than as an empty directory.
///
/// The distinction matters because the caller caches its result. `read_dir`
/// failing with EACCES, EMFILE or EIO is "I could not tell what is here", and
/// silently converting that into "nothing is here" is how a recovery scan
/// concludes that the highest revision on disk is zero.
///
/// A missing directory IS genuinely empty, so `NotFound` alone maps to no
/// entries: namespaces are created lazily, so a root with no namespace
/// subdirectory yet is an ordinary cold start rather than a fault.
/// Returns the iterator rather than a collected `Vec` so the walk stays O(1) in
/// memory. Record counts are not bounded anywhere -- `validate_path_like` caps
/// the character set and shape of a key, never how many there are -- so
/// materialising every path would allocate in proportion to the store, on the
/// daemon's reconcile path.
fn read_dir_total(path: &Path) -> Result<Option<fs::ReadDir>, StorageError> {
    match fs::read_dir(path) {
        Ok(entries) => Ok(Some(entries)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_to_storage(e)),
    }
}

/// `Path::is_dir()` answers `false` for *any* stat failure, so on its own it
/// converts EACCES, EIO, ELOOP or ESTALE into "not a directory" and skips a
/// namespace that may hold the highest revision on disk. That is the same
/// swallow `read_dir_total` exists to prevent, and it is worth its own helper
/// precisely because the standard-library method makes it invisible.
///
/// `NotFound` is the one honest `false`: a namespace unlinked mid-walk really
/// has no records left to read.
///
/// `fs::metadata` rather than `DirEntry::file_type()`, deliberately -- metadata
/// follows symlinks and `file_type` does not, and following them preserves the
/// behaviour `is_dir()` had here.
fn metadata_total(path: &Path) -> Result<Option<fs::Metadata>, StorageError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_to_storage(e)),
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. StorageBackend impl
// ─────────────────────────────────────────────────────────────────────

impl StorageBackend for FsStorageBackend {
    fn initialize(&self) -> Result<(), StorageError> {
        // The root must exist after every `initialize()`, and creating it is a
        // single cheap syscall, so that part stays unconditional.
        fs::create_dir_all(&self.root).map_err(io_to_storage)?;

        // The recovery scan below is the expensive part: it reads the body of
        // every record in every namespace. `ServiceRegistry::load` and `::list`
        // both call `initialize()`, so on the shipped 5s health-check interval
        // this walk ran several times per reconcile tick -- over a state
        // directory shared by the registry, the audit log, channel state and
        // smart-home state. It therefore grew with data an agent can influence,
        // and every consumer paid for all of it. It only needs to happen once
        // per backend instance.
        //
        // Fast path: a plain `Acquire` load, no lock, for every call after the
        // first.
        if self.scanned.load(Ordering::Acquire) {
            return Ok(());
        }

        // The slow path holds `write_lock` -- the same lock `put` holds while
        // it allocates a revision with `fetch_add`. That is the point, not a
        // detail. The scan ends in a `store` on `revision_counter`, and doing
        // that store unlocked let a concurrent `initialize` reset the counter
        // to a value a writer had already allocated past. The same revision
        // would then be issued twice, and a stale `if_revision` compare-and-swap
        // guard would pass where it must fail -- a silently lost CAS, in the
        // guards the registry's whole concurrency story rests on.
        //
        // Caching also removes a second, subtler version of the same hazard:
        // re-scanning a root whose records have since been DELETED lowers
        // `highest`, so the counter walks backwards over revisions it has
        // already handed out. Scanning once can only ever move it forward.
        // The guarded data is `()` -- the mutex orders writers, it does not
        // protect a value that a panic could leave half-updated. So recover from
        // poisoning rather than failing: `ServiceRegistry::{load,list,register}`
        // all begin with `initialize()?`, and turning a poisoned write lock into
        // a hard error there would take pure READS down with the writes.
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        // Re-check under the lock: another thread may have finished the scan
        // between our fast-path load and our acquiring the mutex.
        if self.scanned.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Walk and remove stranded .tmp files. Also: scan all
        // record files to find the highest revision number so a
        // restart picks up where the previous process left off.
        //
        // This walk must be TOTAL. Every failure here used to be swallowed by an
        // `if let Ok(..)`, which was survivable only because the scan re-ran on
        // the next call: a transient EACCES/EMFILE/EIO produced `highest = 0`
        // and the following tick corrected it. Now that the result is cached for
        // the life of the backend, a swallowed error would freeze the counter at
        // a bogus floor permanently, and every subsequent revision would be one
        // already issued. An unreadable directory is "I could not tell", which
        // must never be recorded as "there was nothing there".
        let mut highest: u64 = 0;
        if let Some(namespaces) = read_dir_total(&self.root)? {
            for entry in namespaces {
                let ns_path = entry.map_err(io_to_storage)?.path();
                if !metadata_total(&ns_path)?.is_some_and(|m| m.is_dir()) {
                    continue;
                }
                let Some(records) = read_dir_total(&ns_path)? else {
                    continue;
                };
                for record in records {
                    let p = record.map_err(io_to_storage)?.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("tmp") {
                        let _ = fs::remove_file(&p);
                        continue;
                    }

                    // Only regular files are records. A namespace directory can
                    // legitimately contain other things -- vault-pm nests a
                    // directory per named target under its root -- and opening
                    // one of those as a record yields EISDIR, an I/O error that
                    // the arm below would propagate. Discriminate by TYPE here
                    // so that the error arm keeps its meaning: it should fire
                    // for a file we could not read, never for something that was
                    // never a record to begin with.
                    if !metadata_total(&p)?.is_some_and(|m| m.is_file()) {
                        continue;
                    }

                    // The asymmetry here is deliberate, but it is a split
                    // between ERROR KINDS, not between records and directories.
                    //
                    // "This file is not a record" is a local fact: we still read
                    // every other record, so the floor is sound, and refusing to
                    // start over one corrupt file is the failure mode #12137 is
                    // about. Tolerate it.
                    //
                    // "I could not read this file" is not local at all. It means
                    // the same thing a failed `read_dir` means, and it fails in
                    // correlated ways -- under fd exhaustion EVERY record read
                    // fails while the directory reads still succeed, so the
                    // floor silently collapses to zero and, being cached, every
                    // revision the instance issues afterwards is one already
                    // handed out. Propagate it.
                    //
                    // `read_record_full` already draws exactly this line:
                    // `io_to_storage` yields `Unavailable`, while format and
                    // parse failures yield `Backend`.
                    match read_record_full(&p) {
                        Ok(Some((meta, _))) => {
                            if let Some(n) = revision_to_u64(&meta.revision) {
                                if n > highest {
                                    highest = n;
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(StorageError::Backend { .. }) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        // `fetch_max`, not `store`. The counter must be monotone by
        // construction: `put()` does not require `initialize()` and does not set
        // `scanned`, so a first `initialize()` can legitimately run AFTER
        // revisions have been issued, and a plain `store` would drop the counter
        // back below them. Taking the maximum makes lowering it impossible
        // regardless of call order -- and would, on its own, have prevented the
        // defect this change is about.
        self.revision_counter.fetch_max(highest, Ordering::SeqCst);

        // `Release`, paired with the `Acquire` on the fast path, so the counter
        // update above is visible to any thread that skips the scan. Set last,
        // and only once the walk has completed without error, so a failed
        // `initialize` is retried rather than remembered as done.
        self.scanned.store(true, Ordering::Release);
        Ok(())
    }

    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
        let path = self.key_path(namespace, key);
        match read_record_full(&path)? {
            None => Ok(None),
            Some((meta, body)) => Ok(Some(StorageRecord::new(
                namespace.to_string(),
                key.to_string(),
                meta.revision,
                meta.content_type,
                meta.metadata,
                body,
                meta.created_at,
                meta.updated_at,
            )?)),
        }
    }

    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
        input.validate()?;
        let _guard = self.write_lock.lock().map_err(|_| StorageError::Backend {
            message: "write lock poisoned".into(),
        })?;

        let path = self.key_path(&input.namespace, &input.key);
        let tmp = self.key_tmp_path(&input.namespace, &input.key);

        // CAS check.
        let existing = read_record_full(&path)?;
        if input.if_absent {
            if let Some((meta, _)) = &existing {
                return Err(StorageError::Conflict {
                    namespace: input.namespace.clone(),
                    key: input.key.clone(),
                    expected_revision: None,
                    actual_revision: Some(meta.revision.as_str().to_string()),
                });
            }
        }
        match (&input.if_revision, &existing) {
            (Some(expected), Some((meta, _))) if meta.revision != *expected => {
                return Err(StorageError::Conflict {
                    namespace: input.namespace.clone(),
                    key: input.key.clone(),
                    expected_revision: Some(expected.as_str().to_string()),
                    actual_revision: Some(meta.revision.as_str().to_string()),
                });
            }
            (Some(expected), None) => {
                return Err(StorageError::Conflict {
                    namespace: input.namespace.clone(),
                    key: input.key.clone(),
                    expected_revision: Some(expected.as_str().to_string()),
                    actual_revision: None,
                });
            }
            _ => {}
        }

        let now = now_ms();
        let created_at = match &existing {
            Some((m, _)) => m.created_at,
            None => now,
        };
        let revision = self.next_revision()?;
        let meta = StoredMeta {
            revision: revision.clone(),
            content_type: input.content_type.clone(),
            metadata: input.metadata.clone(),
            created_at,
            updated_at: now,
        };
        write_record_atomic(&tmp, &path, &meta, &input.body)?;

        StorageRecord::new(
            input.namespace,
            input.key,
            revision,
            input.content_type,
            input.metadata,
            input.body,
            created_at,
            now,
        )
    }

    fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<&Revision>,
    ) -> Result<(), StorageError> {
        let _guard = self.write_lock.lock().map_err(|_| StorageError::Backend {
            message: "write lock poisoned".into(),
        })?;
        let path = self.key_path(namespace, key);
        let existing = read_record_full(&path)?;
        match (if_revision, &existing) {
            (Some(expected), Some((meta, _))) if meta.revision != *expected => {
                return Err(StorageError::Conflict {
                    namespace: namespace.to_string(),
                    key: key.to_string(),
                    expected_revision: Some(expected.as_str().to_string()),
                    actual_revision: Some(meta.revision.as_str().to_string()),
                });
            }
            _ => {}
        }
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_to_storage(e)),
        }
    }

    fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StoragePage, StorageError> {
        options.validate()?;
        let ns_path = self.ns_dir(namespace);
        let mut keys: Vec<String> = Vec::new();
        match fs::read_dir(&ns_path) {
            Ok(it) => {
                for e in it.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("tmp") {
                        continue;
                    }
                    let stem = match p.file_name().and_then(|s| s.to_str()) {
                        Some(s) => s,
                        None => continue,
                    };
                    let bytes = match hex_decode(stem) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    let key = match String::from_utf8(bytes) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    keys.push(key);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Namespace not yet created → empty list.
                return Ok(StoragePage::empty());
            }
            Err(e) => return Err(io_to_storage(e)),
        }
        keys.sort();

        // Apply prefix filter.
        if let Some(pfx) = options.prefix.as_deref() {
            keys.retain(|k| k.starts_with(pfx));
        }

        // Apply cursor: skip entries <= cursor.
        if let Some(c) = options.cursor.as_deref() {
            keys.retain(|k| k.as_str() > c);
        }

        // Page-size truncation.
        let mut next_cursor: Option<String> = None;
        if let Some(n) = options.page_size {
            if keys.len() > n {
                keys.truncate(n);
                next_cursor = keys.last().cloned();
            }
        }

        // Materialise records.
        let mut records = Vec::with_capacity(keys.len());
        for k in &keys {
            if let Some(rec) = self.get(namespace, k)? {
                records.push(rec);
            }
        }
        Ok(StoragePage {
            records,
            next_cursor,
        })
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        // Re-use `get` so the content hash is computed via the
        // upstream `StorageRecord::new` rather than duplicated here.
        match self.get(namespace, key)? {
            None => Ok(None),
            Some(rec) => Ok(Some(rec.stat())),
        }
    }

    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError> {
        let now = now_ms();
        let mut leases = self.leases.lock().map_err(|_| StorageError::Backend {
            message: "lease lock poisoned".into(),
        })?;
        if let Some(lease) = leases.get(name) {
            if lease.expires_at > now {
                return Ok(None);
            }
        }
        let n = self.lease_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let token = LeaseToken::new(format!("lease-{:020}", n))?;
        let expires_at: TimestampMs = now + ttl_ms;
        let lease = StorageLease::new(name.to_string(), token, now, expires_at)?;
        leases.insert(name.to_string(), lease.clone());
        Ok(Some(lease))
    }
}

// Parse "rev-NNNN…" → u64.
fn revision_to_u64(r: &Revision) -> Option<u64> {
    let s = r.as_str();
    let stripped = s.strip_prefix("rev-")?;
    stripped.parse::<u64>().ok()
}

// ─────────────────────────────────────────────────────────────────────
// 6. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use storage_core::conformance;

    static TEMP_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let mut p = env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = TEMP_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        p.push(format!(
            "storage-fs-test-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        p
    }

    fn put_input(ns: &str, key: &str, body: &[u8]) -> StoragePutInput {
        StoragePutInput {
            namespace: ns.to_string(),
            key: key.to_string(),
            content_type: "vault/login/v1".to_string(),
            metadata: JsonValue::Object(Vec::new()),
            body: body.to_vec(),
            if_absent: false,
            if_revision: None,
        }
    }

    fn with_backend<T>(test: impl FnOnce(&FsStorageBackend) -> T) -> T {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        let output = test(&be);
        let _ = fs::remove_dir_all(&root);
        output
    }

    #[test]
    fn surface_summary_reports_storage_contract_without_root_path() {
        let root = temp_root().join("secret-vault-root");
        let be = FsStorageBackend::new(&root);
        let summary = be.surface_summary();

        assert_eq!(summary, fs_storage_backend_summary());
        assert_eq!(summary.record_magic, "STRF");
        assert_eq!(summary.record_format_version, VERSION);
        assert!(summary.one_file_per_record);
        assert!(summary.hex_encoded_names);
        assert!(summary.atomic_write_rename);
        assert!(summary.tmp_files_cleaned_on_initialize);
        assert!(summary.parent_directory_fsync_best_effort);
        assert!(summary.content_opaque_to_backend);
        assert!(!summary.leases_persisted);
        assert!(!summary.cross_process_locking);
        assert!(!format!("{summary:?}").contains("secret-vault-root"));
    }

    // --- Shared storage-core conformance ---

    #[test]
    fn conformance_initialize_twice_is_safe() {
        with_backend(|be| conformance::initialize_twice_is_safe(be).unwrap());
    }

    #[test]
    fn conformance_put_then_get_round_trips() {
        with_backend(|be| conformance::put_then_get_round_trips(be).unwrap());
    }

    #[test]
    fn conformance_stale_revision_is_rejected() {
        with_backend(|be| conformance::stale_revision_is_rejected(be).unwrap());
    }

    #[test]
    fn conformance_create_if_absent_rejects_existing() {
        with_backend(|be| conformance::create_if_absent_rejects_existing(be).unwrap());
    }

    #[test]
    fn conformance_concurrent_create_if_absent_has_one_winner() {
        let root = temp_root();
        let backend = std::sync::Arc::new(FsStorageBackend::new(&root));
        conformance::concurrent_create_if_absent_has_one_winner(backend).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn conformance_multiple_write_conditions_are_rejected() {
        with_backend(|be| conformance::multiple_write_conditions_are_rejected(be).unwrap());
    }

    #[test]
    fn conformance_delete_is_idempotent() {
        with_backend(|be| conformance::delete_is_idempotent(be).unwrap());
    }

    #[test]
    fn conformance_prefix_listing_is_stable() {
        with_backend(|be| conformance::prefix_listing_is_stable(be).unwrap());
    }

    #[test]
    fn conformance_advisory_lease_expires() {
        with_backend(|be| conformance::advisory_lease_expires(be).unwrap());
    }

    // --- Round-trip ---

    #[test]
    fn put_get_roundtrip() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();

        let rec = be.put(put_input("ns1", "k1", b"hello")).unwrap();
        let got = be.get("ns1", "k1").unwrap().expect("present");
        assert_eq!(got.body, b"hello");
        assert_eq!(got.namespace, "ns1");
        assert_eq!(got.key, "k1");
        assert_eq!(got.content_type, "vault/login/v1");
        assert_eq!(got.revision, rec.revision);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn get_missing_returns_none() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        assert!(be.get("ns1", "missing").unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn put_overwrite_advances_revision() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let r1 = be.put(put_input("ns", "k", b"v1")).unwrap();
        let r2 = be.put(put_input("ns", "k", b"v2")).unwrap();
        assert_ne!(r1.revision, r2.revision);
        assert_eq!(be.get("ns", "k").unwrap().unwrap().body, b"v2");
        let _ = fs::remove_dir_all(&root);
    }

    // --- CAS ---

    #[test]
    fn put_with_correct_if_revision_succeeds() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let r1 = be.put(put_input("ns", "k", b"v1")).unwrap();
        let mut second = put_input("ns", "k", b"v2");
        second.if_revision = Some(r1.revision.clone());
        be.put(second).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn put_with_wrong_if_revision_conflicts() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v1")).unwrap();
        let mut second = put_input("ns", "k", b"v2");
        second.if_revision = Some(Revision::new("rev-99999999999999999999".to_string()).unwrap());
        match be.put(second) {
            Err(StorageError::Conflict { .. }) => {}
            other => panic!(
                "expected Conflict, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn put_with_if_revision_against_missing_record_conflicts() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let mut input = put_input("ns", "k", b"v");
        input.if_revision = Some(Revision::new("rev-00000000000000000001".to_string()).unwrap());
        match be.put(input) {
            Err(StorageError::Conflict { .. }) => {}
            _ => panic!("expected Conflict against missing record"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    // --- Delete ---

    #[test]
    fn delete_removes_record() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        be.delete("ns", "k", None).unwrap();
        assert!(be.get("ns", "k").unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_missing_succeeds() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.delete("ns", "missing", None).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_with_wrong_if_revision_conflicts() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        let bogus = Revision::new("rev-77777777777777777777".to_string()).unwrap();
        match be.delete("ns", "k", Some(&bogus)) {
            Err(StorageError::Conflict { .. }) => {}
            _ => panic!("expected Conflict"),
        }
        // Original record still present.
        assert!(be.get("ns", "k").unwrap().is_some());
        let _ = fs::remove_dir_all(&root);
    }

    // --- List ---

    #[test]
    fn list_returns_keys_sorted() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "z", b"1")).unwrap();
        be.put(put_input("ns", "a", b"2")).unwrap();
        be.put(put_input("ns", "m", b"3")).unwrap();
        let page = be.list("ns", StorageListOptions::default()).unwrap();
        let keys: Vec<&str> = page.records.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "m", "z"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn list_with_prefix_filter() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "alpha", b"1")).unwrap();
        be.put(put_input("ns", "alphabet", b"2")).unwrap();
        be.put(put_input("ns", "beta", b"3")).unwrap();
        let mut opts = StorageListOptions::default();
        opts.prefix = Some("alpha".to_string());
        let page = be.list("ns", opts).unwrap();
        let keys: Vec<&str> = page.records.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["alpha", "alphabet"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn list_unknown_namespace_is_empty() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let page = be.list("unknown", StorageListOptions::default()).unwrap();
        assert!(page.records.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    // --- stat ---

    #[test]
    fn stat_returns_metadata_without_body() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"hello world")).unwrap();
        let st = be.stat("ns", "k").unwrap().unwrap();
        assert_eq!(st.body_len, b"hello world".len());
        assert_eq!(st.content_type, "vault/login/v1");
        let _ = fs::remove_dir_all(&root);
    }

    // --- Crash safety: stranded .tmp removed on initialize ---

    #[test]
    fn initialize_removes_stranded_tmp_files() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        // Manually fabricate a stranded .tmp file.
        let stranded = be.key_tmp_path("ns", "ghost");
        fs::create_dir_all(stranded.parent().unwrap()).unwrap();
        fs::write(&stranded, b"partial garbage that should be removed").unwrap();
        assert!(stranded.exists());
        // Recreate the backend (simulating restart) and initialize.
        let be2 = FsStorageBackend::new(&root);
        be2.initialize().unwrap();
        assert!(!stranded.exists(), ".tmp file should be removed on init");
        // Real record survives.
        assert_eq!(be2.get("ns", "k").unwrap().unwrap().body, b"v");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reinitialize_does_not_reissue_a_revision() {
        // The regression test for the unlocked-counter defect, made
        // deterministic. The concurrency window (a `put` in flight while
        // `initialize` stores a lower `highest`) is hard to hit on demand, but
        // DELETION reaches the identical end state with no threads at all:
        // remove the record holding the highest revision, and a re-scan lowers
        // the counter below revisions that were already issued.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();

        let r1 = be.put(put_input("ns", "k1", b"v1")).unwrap();
        let r2 = be.put(put_input("ns", "k2", b"v2")).unwrap();
        // Delete the holder of the high-water mark. Disk now tops out at r1.
        be.delete("ns", "k2", None).unwrap();

        // A second `initialize()` on the SAME backend. Before the fix this
        // re-scanned, found only r1, and stored that -- walking the counter
        // backwards over r2, which had already been handed out.
        be.initialize().unwrap();
        let r3 = be.put(put_input("ns", "k3", b"v3")).unwrap();

        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        let n3 = revision_to_u64(&r3.revision).unwrap();
        assert!(n2 > n1, "revisions must be monotonic: {} <= {}", n2, n1);
        assert!(
            n3 > n2,
            "re-initialize reissued revision {}, already held by k2; a stale \
             if_revision compare-and-swap guard would now pass",
            n3
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn initialize_reports_an_unreadable_root_instead_of_caching_a_bogus_floor() {
        // Before the walk was made total, an unreadable directory was swallowed
        // by `if let Ok(..)`, yielding `highest = 0`. That was survivable only
        // while the scan re-ran every call; cached, it would freeze the counter
        // at zero for the life of the backend and reissue every revision.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let seed = FsStorageBackend::new(&root);
        seed.initialize().unwrap();
        let r1 = seed.put(put_input("ns", "k", b"v")).unwrap();
        drop(seed);

        // Make the namespace directory unreadable, then scan it fresh.
        let ns_dir = root.join(hex_encode(b"ns"));
        let original = fs::metadata(&ns_dir).unwrap().permissions();
        fs::set_permissions(&ns_dir, fs::Permissions::from_mode(0o000)).unwrap();

        // A root-owned runner ignores the mode bits entirely, so confirm the
        // directory really did become unreadable before asserting on it.
        // Otherwise this silently inverts into a false failure under a
        // privileged CI container.
        let permissions_bite = fs::read_dir(&ns_dir).is_err();

        let be = FsStorageBackend::new(&root);
        let outcome = be.initialize();

        // Restore before asserting, so a failure cannot leave an
        // undeletable directory behind.
        fs::set_permissions(&ns_dir, original).unwrap();

        if !permissions_bite {
            // Running with privileges that bypass the mode bits (root in a
            // container). There is no unreadable directory to observe, so
            // there is nothing here to assert.
            let _ = fs::remove_dir_all(&root);
            return;
        }

        assert!(
            outcome.is_err(),
            "an unreadable namespace must be reported, not recorded as empty"
        );

        // And having failed, the backend must not consider itself scanned: a
        // retry once permissions are back has to recover the real floor.
        be.initialize().unwrap();
        let r2 = be.put(put_input("ns", "k2", b"v2")).unwrap();
        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        assert!(
            n2 > n1,
            "retry after a failed initialize must recover the floor: {} <= {}",
            n2,
            n1
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initialize_tolerates_an_unparseable_record() {
        // The counterweight to the two tests below. Propagating I/O failures
        // must NOT turn into refusing to start over a corrupt file: one
        // undecodable record stopping the daemon is #12137, and this crate must
        // not reintroduce it while closing the I/O hole.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let r1 = be.put(put_input("ns", "k", b"v")).unwrap();
        drop(be);

        // Garbage and an empty file, beside a good record, both readable.
        let ns_dir = root.join(hex_encode(b"ns"));
        fs::write(ns_dir.join(hex_encode(b"junk")), b"not a record at all").unwrap();
        fs::write(ns_dir.join(hex_encode(b"empty")), b"").unwrap();

        let be2 = FsStorageBackend::new(&root);
        be2.initialize()
            .expect("an unparseable record must not stop initialize (#12137)");

        // The good record was still seen, so the floor is intact.
        let r2 = be2.put(put_input("ns", "k2", b"v2")).unwrap();
        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        assert!(n2 > n1, "floor lost across a corrupt sibling: {} <= {}", n2, n1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initialize_ignores_non_record_entries_inside_a_namespace() {
        // Regression test for a real break, caught by running the downstream
        // consumers. `vault-pm` nests a directory per named target under its
        // storage root, so a namespace directory legitimately contains
        // subdirectories. Opening one as a record yields EISDIR -- an I/O error
        // -- and propagating I/O errors (correctly, for unreadable FILES) turned
        // every vault-pm CLI command into "storage unavailable".
        //
        // The error arm must fire for a record we could not read, never for
        // something that was never a record.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        let r1 = be.put(put_input("ns", "k", b"v")).unwrap();
        drop(be);

        fs::create_dir_all(root.join(hex_encode(b"ns")).join("nested-target")).unwrap();

        let be2 = FsStorageBackend::new(&root);
        be2.initialize()
            .expect("a subdirectory inside a namespace must not fail initialize");

        // The real record was still read, so the floor survived.
        let r2 = be2.put(put_input("ns", "k2", b"v2")).unwrap();
        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        assert!(n2 > n1, "floor lost beside a nested dir: {} <= {}", n2, n1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn initialize_reports_an_unreadable_record_instead_of_lowering_the_floor() {
        // An I/O failure reading a record is "I could not tell", exactly as a
        // failed read_dir is. It matters because these fail in CORRELATED ways:
        // under fd exhaustion every record read fails while the directory reads
        // succeed, so a tolerant walk collapses the floor to zero and caches it.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let seed = FsStorageBackend::new(&root);
        seed.initialize().unwrap();
        seed.put(put_input("ns", "k", b"v")).unwrap();
        drop(seed);

        let record = root.join(hex_encode(b"ns")).join(hex_encode(b"k"));
        let original = fs::metadata(&record).unwrap().permissions();
        fs::set_permissions(&record, fs::Permissions::from_mode(0o000)).unwrap();
        let permissions_bite = File::open(&record).is_err();

        let outcome = FsStorageBackend::new(&root).initialize();
        fs::set_permissions(&record, original).unwrap();

        if permissions_bite {
            assert!(
                outcome.is_err(),
                "an unreadable record must be reported, not silently skipped"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn initialize_reports_an_unstattable_namespace() {
        // `Path::is_dir()` answers false for ANY stat failure. A root that is
        // listable but not traversable (0o400) therefore made every namespace
        // look like "not a directory", skipping the whole store while returning
        // Ok -- the same swallow, one line below the guard against it.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let seed = FsStorageBackend::new(&root);
        seed.initialize().unwrap();
        seed.put(put_input("ns", "k", b"v")).unwrap();
        drop(seed);

        let original = fs::metadata(&root).unwrap().permissions();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o400)).unwrap();
        let ns_dir = root.join(hex_encode(b"ns"));
        // Listable, but stat on the child fails.
        let permissions_bite = fs::read_dir(&root).is_ok() && fs::metadata(&ns_dir).is_err();

        let outcome = FsStorageBackend::new(&root).initialize();
        fs::set_permissions(&root, original).unwrap();

        if permissions_bite {
            assert!(
                outcome.is_err(),
                "a namespace that cannot be stat-ed must be reported, not skipped"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initialize_never_lowers_a_counter_put_has_already_advanced() {
        // `put()` does not require `initialize()`, so the first scan can run
        // after revisions have been issued. `fetch_max` makes that harmless;
        // a plain `store` would drop the counter back below them.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        let r1 = be.put(put_input("ns", "k1", b"v1")).unwrap();
        be.delete("ns", "k1", None).unwrap();

        // First initialize, now, with nothing on disk to recover from.
        be.initialize().unwrap();
        let r2 = be.put(put_input("ns", "k1", b"v2")).unwrap();

        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        assert!(
            n2 > n1,
            "a late first initialize lowered the counter: {} <= {}",
            n2,
            n1
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initialize_is_idempotent_and_still_creates_the_root() {
        // The scan is cached, but the directory postcondition is not: callers
        // may rely on `initialize()` re-creating a root that has vanished.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();

        fs::remove_dir_all(&root).unwrap();
        be.initialize().unwrap();
        assert!(root.is_dir(), "initialize must re-create a missing root");

        // And the backend is still usable afterwards.
        be.put(put_input("ns", "k2", b"v2")).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restart_picks_up_revision_counter() {
        let root = temp_root();
        let be1 = FsStorageBackend::new(&root);
        be1.initialize().unwrap();
        let r1 = be1.put(put_input("ns", "k", b"v1")).unwrap();
        // Drop be1, build a new one, re-initialize.
        drop(be1);
        let be2 = FsStorageBackend::new(&root);
        be2.initialize().unwrap();
        let r2 = be2.put(put_input("ns", "k", b"v2")).unwrap();
        // Revisions are monotonic across restart.
        let n1 = revision_to_u64(&r1.revision).unwrap();
        let n2 = revision_to_u64(&r2.revision).unwrap();
        assert!(
            n2 > n1,
            "revision must advance across restart: {} <= {}",
            n2,
            n1
        );
        let _ = fs::remove_dir_all(&root);
    }

    // --- Lease ---

    #[test]
    fn acquire_lease_first_time_succeeds() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        assert!(be.acquire_lease("flush", 60_000).unwrap().is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn acquire_lease_held_returns_none() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.acquire_lease("flush", 60_000).unwrap();
        assert!(be.acquire_lease("flush", 60_000).unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    // --- Tamper detection on the file format ---

    #[test]
    fn corrupted_magic_returns_backend_error() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        // Corrupt the on-disk file.
        let path = be.key_path("ns", "k");
        let mut buf = fs::read(&path).unwrap();
        buf[0] = b'X';
        fs::write(&path, &buf).unwrap();
        match be.get("ns", "k") {
            Err(StorageError::Backend { .. }) => {}
            _ => panic!("expected Backend error on corrupted magic"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn truncated_file_returns_backend_error() {
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"some body")).unwrap();
        let path = be.key_path("ns", "k");
        let buf = fs::read(&path).unwrap();
        fs::write(&path, &buf[..3]).unwrap();
        match be.get("ns", "k") {
            Err(StorageError::Backend { .. }) => {}
            _ => panic!("expected Backend error on truncated file"),
        }
        let _ = fs::remove_dir_all(&root);
    }
}
