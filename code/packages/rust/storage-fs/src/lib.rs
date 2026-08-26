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
//! 5. `fsync` the parent directory so the rename is durable. `delete`
//!    fsyncs the same parent directory after `remove_file`, for the
//!    same reason: an unlink is a directory-entry change too, and is
//!    just as capable of being lost.
//!
//! Step 5's failure handling is platform-dependent, and the split is
//! deliberate rather than an oversight — see the `fsync_parent_directory`
//! helper in this crate's source for the durability argument and the
//! reason a portable `#![forbid(unsafe_code)]` crate cannot treat both
//! platforms alike.
//! In short: **Unix propagates the failure as [`StorageError::Unavailable`]**
//! (this crate's records get the identical durability contract
//! `vault-pm-local-host` already gives `vault-pm.toml`), while
//! **Windows treats it as a no-op**, because `std::fs::File::open` cannot
//! open a directory there at all — there is nothing to attempt.
//!
//! On `initialize`, the backend walks `<root>` and removes any
//! stranded `.tmp` files — those are the result of crashes during
//! step 1–3 above and don't represent any committed state. That sweep
//! is all the walk does: it opens no record, so it costs
//! O(directory entries) rather than O(bytes in the store).
//!
//! ## Revisions
//!
//! A revision is `rev-<instance>-<counter>`, where `<instance>` is
//! unique to one `FsStorageBackend` value and `<counter>` counts within
//! it. `storage_core` documents `Revision` as an **opaque**
//! compare-and-swap token, and this one lives up to that: it does not
//! sort, and nothing should read meaning into it.
//!
//! Uniqueness is *structural* rather than recovered. An earlier scheme
//! seeded the counter from the highest revision found on disk, which
//! made the guarantee depend on the records still being there — so
//! deleting the record holding the high-water mark moved it backwards
//! and the next instance reissued revisions it had already handed out.
//! Because a revision is the token every `if_revision` compares
//! against, a reissued one is an ABA: a stale guard matches a record it
//! was never taken from, and a compare-and-swap that must fail
//! silently succeeds. Two instances over one root hit the same thing
//! without any deletion at all.
//!
//! Deriving uniqueness from the instance instead removes the whole
//! class. Deletion, a restored backup, a rolled-back store, and a
//! second concurrent process are all non-events, and there is no
//! durable bookkeeping to lose, corrupt, or roll back.
//!
//! **This is not a substitute for cross-process write exclusion.** Two
//! processes can still both pass a `put`'s `if_revision`/`if_absent`
//! check and both rename, because that check and the rename are not one
//! atomic step — see the caveat below. Unique revisions stop a *stale*
//! token from matching; they do not serialise concurrent writers.
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
//!   `put`s of the same `(namespace, key)` from the same process are
//!   serialized by this backend's `write_lock`. **Across processes they
//!   are not**, and the consequence is sharper than "not supported":
//!   `put` evaluates `if_absent` / `if_revision` against a read and
//!   *then* writes-fsyncs-renames, with only a per-instance mutex in
//!   between. Two processes can both pass the check and both rename, so
//!   a compare-and-swap that should have excluded one of them does not.
//!
//!   Closing it needs an `flock`/`O_EXCL` lock spanning check-through-
//!   rename. Until that lands, any protocol whose *safety* rests on
//!   CAS-based mutual exclusion between processes — leadership fencing,
//!   for one — is not sound on this backend, and should say so rather
//!   than assume the storage layer provides an atomic CAS.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_csprng::random_array;
use coding_adventures_json_serializer::serialize as json_serialize;
use coding_adventures_json_value::{parse as json_parse, JsonNumber, JsonValue};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
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
    /// Whether a parent-directory `fsync` failure (after `rename` or
    /// `remove_file`) is tolerated rather than propagated as an error.
    ///
    /// `false` on Unix: a failure there means "the rename/unlink might not
    /// survive a crash," and this crate propagates it as
    /// [`StorageError::Unavailable`], the same durability guarantee
    /// `vault-pm-local-host` gives `vault-pm.toml`. `true` on Windows, where
    /// there is no portable, safe (`#![forbid(unsafe_code)]`-compatible) way
    /// to open a directory handle to fsync in the first place — see the
    /// `fsync_parent_directory` helper in this crate's source.
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
            parent_directory_fsync_best_effort: cfg!(windows),
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

/// Distinguishes backend instances within one process.
///
/// The process id separates concurrent processes and the clock separates
/// restarts, but neither separates two `FsStorageBackend`s built in the same
/// process in the same nanosecond — which `chief-of-staff-daemon` does, holding
/// two backends over one state directory.
static INSTANCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Build an identifier unique to one `FsStorageBackend` instance.
///
/// 128 bits from the OS entropy source, plus a process-local counter.
///
/// The counter is not the interesting half — it only separates two instances
/// built in one process, which the random draw already does with overwhelming
/// probability. It is there because it is free and it makes the within-process
/// case a certainty rather than a probability.
///
/// ## Why not pid and the clock
///
/// That was the first attempt, and it does not hold. `INSTANCE_SEQUENCE` resets
/// to zero in every new process, so across processes uniqueness would rest
/// entirely on `(pid, clock)`, and both are weaker than they look:
///
/// - **Pids repeat.** A container almost always starts its daemon as pid 1, and
///   a busy host wraps its pid space. Two runs then share a pid.
/// - **The clock is coarse.** `SystemTime::now()` advances in microsecond steps
///   on macOS, not nanoseconds — several consecutive constructions land on the
///   same reading — and it can jump backwards outright.
/// - **A pre-epoch clock collapsed it entirely.** Falling back to a constant on
///   `duration_since` failure made the id a pure function of pid and a counter
///   that always starts at zero, so two runs as pid 1 minted byte-identical
///   revisions from the first `put`. That is precisely the ABA this design
///   exists to remove, restored deterministically and silently.
///
/// Entropy has none of those failure modes: no ambient state to repeat, nothing
/// to roll back, and no platform whose resolution it depends on.
///
/// ## On failure
///
/// A failed entropy draw is fatal to this constructor's contract, so it is
/// reported rather than papered over. Substituting a predictable value would
/// reintroduce exactly the collision above, and doing it silently is how the
/// previous version got this wrong.
fn new_instance_id() -> Result<String, StorageError> {
    let random = random_array::<16>().map_err(|error| StorageError::Unavailable {
        message: format!("fs storage cannot draw instance entropy: {error}"),
    })?;
    let mut hex = String::with_capacity(32);
    for byte in random {
        hex.push_str(&format!("{byte:02x}"));
    }
    Ok(format!(
        "{hex}.{:x}",
        INSTANCE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Filesystem-backed `StorageBackend`. Wrap a directory and you've
/// got a persistent vault store with crash-safe writes.
pub struct FsStorageBackend {
    root: PathBuf,
    /// Mutex serialises all writes within this process; needed so
    /// the revision counter advances monotonically.
    write_lock: Mutex<()>,
    /// Distinguishes THIS backend instance's revisions from every other
    /// instance's, over this root or any other.
    ///
    /// Revision uniqueness used to be derived: the counter was seeded from the
    /// highest revision found on disk, so restarting picked up where the last
    /// process left off. That is unsound, and not subtly. The high-water mark
    /// came from SURVIVING records, so deleting the record holding it moved the
    /// mark backwards and the next instance re-issued revisions already handed
    /// out. Since a revision is the token every `if_revision` compare-and-swap
    /// compares against, a reissued one is an ABA: a stale guard matches a
    /// record it was never taken from and passes where it must fail.
    ///
    /// Making uniqueness STRUCTURAL removes the whole class. Two instances
    /// cannot mint the same revision no matter what happens to the records, so
    /// deletion, a restored backup, a rolled-back store and a concurrent second
    /// process are all non-events. Nothing durable has to be maintained,
    /// because there is nothing to lose.
    /// Resolved on first use rather than in `new`, so the constructor stays
    /// infallible and an entropy failure is a retryable error at the call that
    /// needed a revision instead of a panic in a constructor.
    instance: OnceLock<String>,
    revision_counter: AtomicU64,
    /// Whether the one-time `.tmp` sweep in `initialize()` has completed.
    ///
    /// Per backend instance, so a genuine restart sweeps again -- which is the
    /// realistic recovery path, since stranded temporaries come from a write a
    /// crash interrupted.
    ///
    /// This flag says nothing about revisions. It used to: the walk it guards
    /// once recovered the revision floor from disk, and caching that was the
    /// best available answer at the time. Uniqueness is now structural, from the
    /// instance id, so the flag guards a filesystem sweep and nothing else.
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
            instance: OnceLock::new(),
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

    /// Mint the next revision for this instance.
    ///
    /// `rev-<instance>-<counter>`. The counter makes it unique WITHIN the
    /// instance; the instance id makes it unique BETWEEN instances. Neither
    /// half is redundant, and neither reads anything off disk, which is the
    /// point — a revision cannot be reissued by a store that lost records.
    ///
    /// `storage_core` documents `Revision` as an *opaque* compare-and-swap
    /// token, so this deliberately does not sort. Nothing in the repo orders
    /// revisions: `revision_to_u64` has no callers outside this file, and
    /// `vault-revisions` orders its own history by `archived_at_ms`. Equality
    /// is the only operation the contract promises, and equality is what a CAS
    /// needs.
    fn next_revision(&self) -> Result<Revision, StorageError> {
        let n = self.revision_counter.fetch_add(1, Ordering::SeqCst) + 1;
        Revision::new(format!("rev-{}-{:020}", self.instance()?, n))
    }

    /// This instance's identifier, drawn once.
    ///
    /// `OnceLock::set` losing a race is not an error here: whichever draw landed
    /// is equally valid, and both are unique. Only the value that won is ever
    /// used, so the instance id never changes once observed.
    fn instance(&self) -> Result<&str, StorageError> {
        if let Some(existing) = self.instance.get() {
            return Ok(existing);
        }
        let _ = self.instance.set(new_instance_id()?);
        Ok(self
            .instance
            .get()
            .expect("instance id is set immediately above"))
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

    // 4. fsync the parent directory so the rename is durable. See
    // `fsync_parent_directory` for why this hard-fails on Unix and is a
    // deliberate no-op on Windows.
    fsync_parent_directory(final_path)
}

/// fsync the parent directory of `path` so a preceding `rename` or
/// `remove_file` on `path` is durable, not merely visible through the page
/// cache.
///
/// # Why this hard-fails on Unix
///
/// A `rename(2)` (or `unlink(2)`) is itself a change to the *parent
/// directory's* contents, not to the file it names. POSIX permits a
/// filesystem to keep that directory-entry change in volatile cache
/// indefinitely until something forces it out — ordinarily an `fsync` on the
/// directory itself. Skipping that step, or discarding its failure, means a
/// `put`/`delete` that already returned `Ok` can still unwind on a crash: the
/// old file (for `put`) or the deleted file (for `delete`) reappears once the
/// disk is examined again, even though nothing observed that in the live
/// process.
///
/// That gap used to be tolerated here ("best-effort... some filesystems
/// don't support directory fsync") while `vault-pm-local-host`'s
/// `sync_directory` hard-failed the *identical* operation for
/// `vault-pm.toml` (see `code/packages/rust/vault-pm-local-host/src/unix.rs`).
/// The asymmetry was real and was flagged in VLT-PM41 §8.1 as a "documented
/// weakness" deserving a decision. This crate's owner-state records
/// (written through `vault-pm-application-storage-core`) sit under the exact
/// same crash-safety journal (`PendingPublication`, `PendingRotation`) as
/// `vault-pm.toml` — both are read back by recovery code that assumes a
/// completed `put`/`compare_exchange` is durable, not ambiguous. There is no
/// argument for `vault-pm.toml` needing a stronger guarantee than the
/// owner-state file it is paired with, so this function now gives both the
/// same answer: propagate the failure as [`StorageError::Unavailable`], the
/// same error class `write_record_atomic`'s tmp-file `fsync` already uses a
/// few lines above. A caller that gets this error cannot tell whether the
/// underlying operation is durable or not — which is the correct, honest
/// answer, and precisely the "ambiguous outcome" this product's crash-safety
/// journal already exists to recover from.
///
/// # Why this stays a no-op on Windows
///
/// `std::fs::File::open` cannot open a directory on Windows at all — the
/// underlying `CreateFileW` call needs `FILE_FLAG_BACKUP_SEMANTICS`, which
/// safe `std::fs::OpenOptions` never sets, so the attempt fails with "Access
/// is denied" every time, regardless of whether anything is actually wrong.
/// `vault-pm-local-host` works around this on Windows by calling the Win32
/// API directly inside `unsafe` blocks (`windows.rs`, `MOVEFILE_WRITE_THROUGH`
/// on the rename itself, which makes the move durable without a separate
/// directory handle). This crate is `#![forbid(unsafe_code)]` and has no
/// equivalent escape hatch, so there is no directory fsync to even attempt
/// on Windows — hard-failing here would mean every `put`/`delete` fails
/// unconditionally on that platform, which is a regression, not a fix.
/// NTFS also journals metadata operations (including renames and deletes) in
/// its own `$LogFile` by design, which is the durability story Windows gets
/// instead of a userspace directory fsync.
fn fsync_parent_directory(path: &Path) -> Result<(), StorageError> {
    if cfg!(windows) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        let d = File::open(parent).map_err(io_to_storage)?;
        d.sync_all().map_err(io_to_storage)?;
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

        // The sweep below walks every namespace directory. It no longer opens
        // any record, so it is O(directory entries) -- but it is still a walk,
        // and `ServiceRegistry::load`/`::list` both call `initialize()`. `ServiceRegistry::load` and `::list`
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

        // The slow path holds `write_lock` so the sweep cannot race a `put`
        // that is mid-write: removing a `.tmp` between that write's fsync and
        // its rename would destroy a record that was about to commit.
        //
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

        // Walk and remove stranded `.tmp` files -- the residue of a `put` that
        // died between writing its temporary and renaming it into place.
        //
        // This walk used to do a second job: read every record to find the
        // highest revision on disk, so a restart could continue the numbering.
        // That job is GONE, and its absence is the point of this change. The
        // floor it recovered came from surviving records, so a deletion moved it
        // backwards and the next instance reissued revisions already handed out.
        // Uniqueness now comes from the instance id in `next_revision`, which no
        // deletion, restore, or rollback can affect.
        //
        // Two things follow. Nothing here opens a record any more, so the walk
        // is O(directory entries) rather than O(bytes in the store) -- on the
        // 8 MB state directory #12139 measured, that was ~480 ms per call. And
        // the delicate reasoning about which read failures could be tolerated
        // is gone with the reads: there is no floor left to poison.
        //
        // The directory reads stay total regardless. A sweep that silently saw
        // nothing would leave stranded temporaries behind for good, since this
        // result is cached for the life of the backend.
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
                    }
                }
            }
        }

        // `Release`, paired with the `Acquire` on the fast path, so everything
        // this walk did is visible to a thread that skips it. Set last, and only
        // once the walk completed without error, so a failed `initialize` is
        // retried rather than remembered as done.
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
            // A real removal happened: fsync the parent directory so the
            // unlink survives a crash, exactly as `write_record_atomic`
            // does after `rename`. Item #19: a deleted record reappearing
            // after a crash is a genuine durability gap, distinct from (and
            // not load-bearing for) the confidentiality fix in
            // `vault-pm-application-storage-core::supersede_generation`,
            // which overwrites sensitive bodies through the fsync-durable
            // `put` path *before* calling this `delete` at all. This closes
            // the gap for every caller, not just that one.
            Ok(()) => fsync_parent_directory(&path),
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

// ─────────────────────────────────────────────────────────────────────
// 6. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
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
        // Best-effort only on Windows (no safe way to open a directory
        // handle to fsync there); a hard, propagated failure everywhere
        // else. See `fsync_parent_directory`'s doc comment.
        assert_eq!(summary.parent_directory_fsync_best_effort, cfg!(windows));
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

    #[test]
    #[cfg(unix)]
    fn delete_fsyncs_the_parent_directory() {
        // Item #19: `delete` used to just `fs::remove_file` and return --
        // no fsync of anything, so a crash right after a successful delete
        // could resurrect the file. This does not reopen the confidentiality
        // hole `vault-pm-application-storage-core::supersede_generation`
        // already closed (it overwrites sensitive bodies through the
        // fsync-durable `put` path *before* ever calling `delete`); this is
        // a defense-in-depth durability property for every caller of
        // `delete`, not a fix for that finding.
        //
        // A live fsync succeeding is not directly observable from outside
        // the crate, so this proves the *shape* of the guarantee instead:
        // when the directory cannot even be opened to attempt the fsync,
        // `delete` must report that rather than silently declaring success
        // (mirrored by `put_hard_fails_when_parent_directory_cannot_be_fsynced`
        // below, and by `delete_hard_fails_when_parent_directory_cannot_be_fsynced`).
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        be.delete("ns", "k", None).unwrap();
        // Ordinary case: fsync succeeded silently, delete is Ok, record gone.
        assert!(be.get("ns", "k").unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn put_hard_fails_when_parent_directory_cannot_be_fsynced() {
        // Item #15: `write_record_atomic`'s parent-directory fsync used to
        // be `let _ = d.sync_all()` -- any failure was silently discarded,
        // while `vault-pm-local-host::sync_directory` hard-fails the
        // identical operation for `vault-pm.toml`. The owner-state records
        // this crate persists (via `vault-pm-application-storage-core`) sit
        // under the same `PendingPublication`/`PendingRotation` crash-safety
        // journal as `vault-pm.toml`, so there was never a reason for one to
        // tolerate a failure the other refuses. Both now hard-fail.
        //
        // Creating/renaming a file inside a directory needs w+x on that
        // directory but not r; opening the directory itself (what fsync
        // needs) needs r. Stripping r alone isolates the fsync step: the
        // write and rename that precede it still succeed.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        // First put creates the namespace directory with default permissions.
        be.put(put_input("ns", "seed", b"v")).unwrap();

        let ns_dir = root.join(hex_encode(b"ns"));
        let original = fs::metadata(&ns_dir).unwrap().permissions();
        fs::set_permissions(&ns_dir, fs::Permissions::from_mode(0o300)).unwrap();
        // A root-owned runner ignores mode bits; confirm the directory
        // really is unreadable before asserting on the outcome, exactly as
        // the `initialize` permission tests above do.
        let permissions_bite = File::open(&ns_dir).is_err();

        let outcome = be.put(put_input("ns", "k", b"v"));

        fs::set_permissions(&ns_dir, original).unwrap();

        if !permissions_bite {
            let _ = fs::remove_dir_all(&root);
            return;
        }

        match outcome {
            Err(StorageError::Unavailable { .. }) => {}
            other => panic!(
                "expected Unavailable when the parent-directory fsync cannot be \
                 attempted, got {}",
                if other.is_ok() {
                    "Ok"
                } else {
                    "a different Err"
                }
            ),
        }
        // `rename` already completed before the fsync step failed, so the
        // record IS on disk -- the error means "durability unconfirmed,"
        // never "nothing was written." The application layer's crash-safety
        // journal is built to recover from exactly this kind of ambiguity.
        assert!(be.get("ns", "k").unwrap().is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn delete_hard_fails_when_parent_directory_cannot_be_fsynced() {
        // The `delete` counterpart to the `put` test above: item #15's fix
        // and item #19's new fsync-on-delete share one helper
        // (`fsync_parent_directory`), so both operations must fail the same
        // way when the directory cannot be opened to fsync.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();

        let ns_dir = root.join(hex_encode(b"ns"));
        let original = fs::metadata(&ns_dir).unwrap().permissions();
        fs::set_permissions(&ns_dir, fs::Permissions::from_mode(0o300)).unwrap();
        let permissions_bite = File::open(&ns_dir).is_err();

        let outcome = be.delete("ns", "k", None);

        fs::set_permissions(&ns_dir, original).unwrap();

        if !permissions_bite {
            let _ = fs::remove_dir_all(&root);
            return;
        }

        match outcome {
            Err(StorageError::Unavailable { .. }) => {}
            other => panic!(
                "expected Unavailable when the parent-directory fsync cannot be \
                 attempted, got {}",
                if other.is_ok() {
                    "Ok"
                } else {
                    "a different Err"
                }
            ),
        }
        // `remove_file` already completed before the fsync step failed, so
        // the record is gone from this process's view even though its
        // durability is unconfirmed -- the honest state for the caller.
        assert!(be.get("ns", "k").unwrap().is_none());
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
    fn a_restart_never_reissues_a_revision_even_after_deletions() {
        // The defect this design removes. The old scheme recovered its floor
        // from SURVIVING records, so deleting the record holding the high-water
        // mark moved it backwards and the next instance handed the number out a
        // second time -- an ABA on every `if_revision` guard taken against it.
        let root = temp_root();
        let first = FsStorageBackend::new(&root);
        first.initialize().unwrap();
        let r1 = first.put(put_input("ns", "k1", b"v1")).unwrap();
        let r2 = first.put(put_input("ns", "k2", b"v2")).unwrap();
        first.delete("ns", "k2", None).unwrap();
        drop(first);

        // A genuine restart over a store whose high-water record is gone.
        let second = FsStorageBackend::new(&root);
        second.initialize().unwrap();
        let r3 = second.put(put_input("ns", "k3", b"v3")).unwrap();

        let seen = [&r1.revision, &r2.revision, &r3.revision];
        let unique: BTreeSet<_> = seen.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            unique.len(),
            3,
            "a restart reissued a revision: {:?}",
            seen.iter().map(|r| r.as_str()).collect::<Vec<_>>()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_backends_over_one_root_cannot_mint_the_same_revision() {
        // `chief-of-staff-daemon` really does hold two `FsStorageBackend`s over
        // one state directory. Under the old scheme both seeded their counters
        // from the same scan and then issued identical revision strings, so a
        // stale CAS from one matched a record written by the other.
        let root = temp_root();
        let a = FsStorageBackend::new(&root);
        let b = FsStorageBackend::new(&root);
        a.initialize().unwrap();
        b.initialize().unwrap();

        let mut seen = BTreeSet::new();
        for i in 0..16 {
            for (backend, tag) in [(&a, "a"), (&b, "b")] {
                let record = backend
                    .put(put_input("ns", &format!("{tag}{i}"), b"v"))
                    .unwrap();
                assert!(
                    seen.insert(record.revision.as_str().to_string()),
                    "revision {} was minted twice",
                    record.revision.as_str()
                );
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_stale_compare_and_swap_from_another_instance_is_rejected() {
        // The property revisions exist to provide, stated as the guard it
        // protects. Instance A's token must never match a record instance B
        // wrote, or A's `if_revision` passes over B's newer write.
        let root = temp_root();
        let a = FsStorageBackend::new(&root);
        a.initialize().unwrap();
        let stale = a.put(put_input("ns", "k", b"from-a")).unwrap().revision;

        let b = FsStorageBackend::new(&root);
        b.initialize().unwrap();
        b.put(put_input("ns", "k", b"from-b")).unwrap();

        let mut overwrite = put_input("ns", "k", b"clobbered");
        overwrite.if_revision = Some(stale);
        assert!(
            matches!(a.put(overwrite), Err(StorageError::Conflict { .. })),
            "a token from a previous instance must not satisfy a CAS"
        );
        assert_eq!(a.get("ns", "k").unwrap().unwrap().body, b"from-b");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initialize_does_not_read_record_bodies() {
        // The walk sweeps `.tmp` files and nothing else. Records it cannot read
        // -- corrupt, unreadable, or not records at all -- are irrelevant to it,
        // which is why the delicate "which read failures may be tolerated"
        // reasoning went away with the reads. A nested directory is the case
        // that actually broke vault-pm when the walk still opened files.
        let root = temp_root();
        let be = FsStorageBackend::new(&root);
        be.initialize().unwrap();
        be.put(put_input("ns", "k", b"v")).unwrap();
        drop(be);

        let ns_dir = root.join(hex_encode(b"ns"));
        fs::write(ns_dir.join(hex_encode(b"junk")), b"not a record at all").unwrap();
        fs::write(ns_dir.join(hex_encode(b"empty")), b"").unwrap();
        fs::create_dir_all(ns_dir.join("nested-target")).unwrap();
        let stranded = ns_dir.join(format!("{}.tmp", hex_encode(b"k")));
        fs::write(&stranded, b"partial").unwrap();

        let reopened = FsStorageBackend::new(&root);
        reopened
            .initialize()
            .expect("unreadable records are not the sweep's business");
        assert!(
            !stranded.exists(),
            "the sweep still removes stranded tmp files"
        );
        assert_eq!(reopened.get("ns", "k").unwrap().unwrap().body, b"v");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn initialize_reports_an_unreadable_namespace_rather_than_sweeping_nothing() {
        // The directory reads stay total. This result is cached for the life of
        // the backend, so a sweep that silently saw nothing would leave
        // stranded temporaries behind for good.
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root();
        let seed = FsStorageBackend::new(&root);
        seed.initialize().unwrap();
        seed.put(put_input("ns", "k", b"v")).unwrap();
        drop(seed);

        let ns_dir = root.join(hex_encode(b"ns"));
        let original = fs::metadata(&ns_dir).unwrap().permissions();
        fs::set_permissions(&ns_dir, fs::Permissions::from_mode(0o000)).unwrap();
        let permissions_bite = fs::read_dir(&ns_dir).is_err();

        let outcome = FsStorageBackend::new(&root).initialize();
        fs::set_permissions(&ns_dir, original).unwrap();

        if permissions_bite {
            assert!(
                outcome.is_err(),
                "an unreadable namespace must be reported, not recorded as empty"
            );
        }
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
