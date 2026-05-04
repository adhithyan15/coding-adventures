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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
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
            leases: Mutex::new(HashMap::new()),
            lease_counter: AtomicU64::new(0),
        }
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
    if s.len() % 2 != 0 {
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
    let meta_len: u32 = meta_bytes.len().try_into().map_err(|_| StorageError::Backend {
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
        f.write_all(&meta_len.to_be_bytes()).map_err(io_to_storage)?;
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
    let meta_len =
        u32::from_be_bytes([all[5], all[6], all[7], all[8]]) as usize;
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
    let meta_str = std::str::from_utf8(&all[meta_start..meta_end])
        .map_err(|_| StorageError::Backend { message: "fs storage: meta not UTF-8".into() })?;
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

// ─────────────────────────────────────────────────────────────────────
// 5. StorageBackend impl
// ─────────────────────────────────────────────────────────────────────

impl StorageBackend for FsStorageBackend {
    fn initialize(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.root).map_err(io_to_storage)?;
        // Walk and remove stranded .tmp files. Also: scan all
        // record files to find the highest revision number so a
        // restart picks up where the previous process left off.
        let mut highest: u64 = 0;
        if let Ok(entries) = fs::read_dir(&self.root) {
            for e in entries.flatten() {
                let ns_path = e.path();
                if !ns_path.is_dir() {
                    continue;
                }
                if let Ok(inner) = fs::read_dir(&ns_path) {
                    for inner_e in inner.flatten() {
                        let p = inner_e.path();
                        if p.extension().and_then(|s| s.to_str()) == Some("tmp") {
                            let _ = fs::remove_file(&p);
                            continue;
                        }
                        // Try to parse the record to recover its revision.
                        if let Ok(Some((meta, _))) = read_record_full(&p) {
                            if let Some(n) = revision_to_u64(&meta.revision) {
                                if n > highest {
                                    highest = n;
                                }
                            }
                        }
                    }
                }
            }
        }
        self.revision_counter.store(highest, Ordering::SeqCst);
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| StorageError::Backend { message: "write lock poisoned".into() })?;

        let path = self.key_path(&input.namespace, &input.key);
        let tmp = self.key_tmp_path(&input.namespace, &input.key);

        // CAS check.
        let existing = read_record_full(&path)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| StorageError::Backend { message: "write lock poisoned".into() })?;
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
        Ok(StoragePage { records, next_cursor })
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        // Re-use `get` so the content hash is computed via the
        // upstream `StorageRecord::new` rather than duplicated here.
        match self.get(namespace, key)? {
            None => Ok(None),
            Some(rec) => Ok(Some(rec.stat())),
        }
    }

    fn acquire_lease(
        &self,
        name: &str,
        ttl_ms: u64,
    ) -> Result<Option<StorageLease>, StorageError> {
        let now = now_ms();
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| StorageError::Backend { message: "lease lock poisoned".into() })?;
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

    fn temp_root() -> PathBuf {
        let mut p = env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("storage-fs-test-{}-{}", std::process::id(), stamp));
        p
    }

    fn put_input(ns: &str, key: &str, body: &[u8]) -> StoragePutInput {
        StoragePutInput {
            namespace: ns.to_string(),
            key: key.to_string(),
            content_type: "vault/login/v1".to_string(),
            metadata: JsonValue::Object(Vec::new()),
            body: body.to_vec(),
            if_revision: None,
        }
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
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
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
        assert!(n2 > n1, "revision must advance across restart: {} <= {}", n2, n1);
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
