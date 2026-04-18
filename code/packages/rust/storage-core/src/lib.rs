//! # storage-core
//!
//! `storage-core` defines the portable storage contract for Chief of Staff
//! stores.
//!
//! The existing CAS packages in this repository already established an
//! important pattern:
//!
//! ```text
//! repository-owned trait  -->  swappable persistence backend
//! ```
//!
//! `storage-core` keeps that pattern, but widens it from raw hash-addressed
//! blobs into named records with metadata, revisions, listings, and leases.
//! That is the level needed by `ContextStore`, `ArtifactStore`, `SkillStore`,
//! and `MemoryStore`.
//!
//! ## Layering
//!
//! ```text
//! ContextStore / ArtifactStore / SkillStore / MemoryStore
//!                         |
//!                         v
//!                   StorageBackend
//!                         |
//!          +--------------+--------------+
//!          |                             |
//!   local-folder backend           SQLite backend
//! ```
//!
//! ## What "portable" means here
//!
//! Callers can rely on:
//!
//! - point reads by `(namespace, key)`
//! - compare-and-swap writes via `if_revision`
//! - stable prefix listing
//! - JSON metadata plus opaque byte bodies
//! - advisory leases for background maintenance
//!
//! Callers cannot rely on:
//!
//! - backend-native SQL queries
//! - transactions spanning arbitrary records
//! - filesystem path tricks
//! - full-text or vector search

use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use coding_adventures_json_value::JsonValue;
use coding_adventures_sha256::sha256;

/// Milliseconds since the Unix epoch in UTC.
pub type TimestampMs = u64;

/// Return the current UTC timestamp in milliseconds.
pub fn now_utc_ms() -> TimestampMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the unix epoch")
        .as_millis() as TimestampMs
}

/// JSON metadata stored alongside a record body.
pub type StorageMetadata = JsonValue;

/// Opaque backend revision used for compare-and-swap writes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(String);

impl Revision {
    /// Create a new revision token after validating that it is non-empty and
    /// single-line.
    pub fn new(value: impl Into<String>) -> Result<Self, StorageError> {
        let value = value.into();
        validate_single_line_token("revision", &value)?;
        Ok(Self(value))
    }

    /// Borrow the underlying revision string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Revision {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque advisory lease token returned by the backend.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeaseToken(String);

impl LeaseToken {
    /// Create a new lease token after validating that it is non-empty and
    /// single-line.
    pub fn new(value: impl Into<String>) -> Result<Self, StorageError> {
        let value = value.into();
        validate_single_line_token("lease_token", &value)?;
        Ok(Self(value))
    }

    /// Borrow the underlying token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for LeaseToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Input used for one `put` operation.
#[derive(Debug, Clone, PartialEq)]
pub struct StoragePutInput {
    pub namespace: String,
    pub key: String,
    pub content_type: String,
    pub metadata: StorageMetadata,
    pub body: Vec<u8>,
    pub if_revision: Option<Revision>,
}

impl StoragePutInput {
    /// Construct and validate a put input.
    pub fn new(
        namespace: impl Into<String>,
        key: impl Into<String>,
        content_type: impl Into<String>,
        metadata: StorageMetadata,
        body: Vec<u8>,
    ) -> Result<Self, StorageError> {
        let input = Self {
            namespace: namespace.into(),
            key: key.into(),
            content_type: content_type.into(),
            metadata,
            body,
            if_revision: None,
        };
        input.validate()?;
        Ok(input)
    }

    /// Attach a compare-and-swap revision requirement.
    pub fn with_if_revision(mut self, revision: Option<Revision>) -> Self {
        self.if_revision = revision;
        self
    }

    /// Validate the input according to the repository-owned storage rules.
    pub fn validate(&self) -> Result<(), StorageError> {
        validate_namespace(&self.namespace)?;
        validate_record_key(&self.key)?;
        validate_content_type(&self.content_type)?;
        validate_metadata_object(&self.metadata)?;
        if let Some(revision) = &self.if_revision {
            validate_single_line_token("if_revision", revision.as_str())?;
        }
        Ok(())
    }

    /// Compute the SHA-256 hash of the body bytes.
    pub fn content_hash(&self) -> [u8; 32] {
        sha256(&self.body)
    }
}

/// Fully materialized storage record returned by the backend.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageRecord {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub content_type: String,
    pub metadata: StorageMetadata,
    pub body: Vec<u8>,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
    pub content_hash: [u8; 32],
}

impl StorageRecord {
    /// Construct and validate a concrete record. The body hash is always derived
    /// from the body bytes, never trusted as caller input.
    pub fn new(
        namespace: impl Into<String>,
        key: impl Into<String>,
        revision: Revision,
        content_type: impl Into<String>,
        metadata: StorageMetadata,
        body: Vec<u8>,
        created_at: TimestampMs,
        updated_at: TimestampMs,
    ) -> Result<Self, StorageError> {
        let namespace = namespace.into();
        let key = key.into();
        let content_type = content_type.into();

        validate_namespace(&namespace)?;
        validate_record_key(&key)?;
        validate_content_type(&content_type)?;
        validate_metadata_object(&metadata)?;
        if updated_at < created_at {
            return Err(StorageError::Validation {
                field: "updated_at".to_string(),
                message: "must be greater than or equal to created_at".to_string(),
            });
        }

        Ok(Self {
            namespace,
            key,
            revision,
            content_type,
            metadata,
            content_hash: sha256(&body),
            body,
            created_at,
            updated_at,
        })
    }

    /// Return a lightweight stat view of the record.
    pub fn stat(&self) -> StorageStat {
        StorageStat {
            namespace: self.namespace.clone(),
            key: self.key.clone(),
            revision: self.revision.clone(),
            content_type: self.content_type.clone(),
            metadata: self.metadata.clone(),
            body_len: self.body.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            content_hash: self.content_hash,
        }
    }
}

/// Lightweight metadata returned by `stat`.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageStat {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub content_type: String,
    pub metadata: StorageMetadata,
    pub body_len: usize,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
    pub content_hash: [u8; 32],
}

/// Options used for prefix listing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageListOptions {
    pub prefix: Option<String>,
    pub recursive: bool,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

impl StorageListOptions {
    /// Validate listing options.
    pub fn validate(&self) -> Result<(), StorageError> {
        if let Some(prefix) = &self.prefix {
            validate_prefix(prefix)?;
        }
        if let Some(page_size) = self.page_size {
            if page_size == 0 {
                return Err(StorageError::Validation {
                    field: "page_size".to_string(),
                    message: "must be greater than zero when set".to_string(),
                });
            }
        }
        if let Some(cursor) = &self.cursor {
            validate_single_line_token("cursor", cursor)?;
        }
        Ok(())
    }
}

/// One page of list results. Records are expected to arrive in stable
/// lexicographic key order.
#[derive(Debug, Clone, PartialEq)]
pub struct StoragePage {
    pub records: Vec<StorageRecord>,
    pub next_cursor: Option<String>,
}

impl StoragePage {
    /// Construct an empty page.
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
            next_cursor: None,
        }
    }
}

/// An advisory lease for background operations such as index rebuilds or
/// compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLease {
    pub name: String,
    pub token: LeaseToken,
    pub issued_at: TimestampMs,
    pub expires_at: TimestampMs,
}

impl StorageLease {
    /// Construct a lease after validating the name and timestamp order.
    pub fn new(
        name: impl Into<String>,
        token: LeaseToken,
        issued_at: TimestampMs,
        expires_at: TimestampMs,
    ) -> Result<Self, StorageError> {
        let name = name.into();
        validate_lease_name(&name)?;
        if expires_at <= issued_at {
            return Err(StorageError::Validation {
                field: "expires_at".to_string(),
                message: "must be greater than issued_at".to_string(),
            });
        }
        Ok(Self {
            name,
            token,
            issued_at,
            expires_at,
        })
    }

    /// Return whether the lease is still active at the supplied timestamp.
    pub fn is_active_at(&self, now_ms: TimestampMs) -> bool {
        now_ms < self.expires_at
    }
}

/// Repository-owned storage errors. Backends should translate their own failure
/// modes into one of these variants instead of leaking backend-specific errors
/// upward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    NotFound {
        namespace: String,
        key: String,
    },
    Conflict {
        namespace: String,
        key: String,
        expected_revision: Option<String>,
        actual_revision: Option<String>,
    },
    Unavailable {
        message: String,
    },
    LeaseDenied {
        name: String,
    },
    Validation {
        field: String,
        message: String,
    },
    Backend {
        message: String,
    },
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound { namespace, key } => {
                write!(f, "storage record not found: {namespace}/{key}")
            }
            StorageError::Conflict {
                namespace,
                key,
                expected_revision,
                actual_revision,
            } => write!(
                f,
                "storage conflict for {namespace}/{key}: expected {:?}, actual {:?}",
                expected_revision, actual_revision
            ),
            StorageError::Unavailable { message } => {
                write!(f, "storage unavailable: {message}")
            }
            StorageError::LeaseDenied { name } => {
                write!(f, "storage lease denied: {name}")
            }
            StorageError::Validation { field, message } => {
                write!(f, "storage validation failed for {field}: {message}")
            }
            StorageError::Backend { message } => {
                write!(f, "storage backend error: {message}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// A backend capable of storing named records for the Chief of Staff stores.
///
/// The trait is intentionally synchronous and small. Async runtimes can wrap it
/// higher in the stack, but the semantic contract remains repository-owned.
pub trait StorageBackend: Send + Sync {
    /// Initialize any backend-owned on-disk or in-memory structures.
    ///
    /// Must be safe to call more than once.
    fn initialize(&self) -> Result<(), StorageError>;

    /// Fetch the full record for `(namespace, key)`.
    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError>;

    /// Store a record body plus metadata, optionally guarded by `if_revision`.
    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError>;

    /// Delete a record. Deleting a missing record must succeed.
    fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<&Revision>,
    ) -> Result<(), StorageError>;

    /// List records within one namespace using prefix filtering and stable
    /// ordering by key.
    fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StoragePage, StorageError>;

    /// Fetch metadata for `(namespace, key)` without loading the full body.
    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError>;

    /// Attempt to acquire an advisory lease. Returns `Ok(None)` when a still-
    /// active lease already exists.
    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError>;
}

/// Pure in-memory backend implementing the repository-owned storage contract.
///
/// The higher-level Chief of Staff stores need one backend that is:
///
/// - always available in tests
/// - deterministic for examples
/// - free from OS-specific filesystem or database setup
///
/// `InMemoryStorageBackend` fills that role. It keeps all records and leases in
/// memory and generates revision/lease tokens from monotonic counters.
#[derive(Debug, Default)]
pub struct InMemoryStorageBackend {
    records: Mutex<BTreeMap<(String, String), StorageRecord>>,
    revision_counter: AtomicU64,
    lease_counter: AtomicU64,
    leases: Mutex<HashMap<String, StorageLease>>,
}

impl InMemoryStorageBackend {
    /// Construct an empty backend.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_revision(&self) -> Revision {
        Revision::new(format!(
            "r{}",
            self.revision_counter.fetch_add(1, Ordering::SeqCst) + 1
        ))
        .expect("generated revisions should be valid")
    }

    fn next_lease_token(&self) -> LeaseToken {
        LeaseToken::new(format!(
            "lease-{}",
            self.lease_counter.fetch_add(1, Ordering::SeqCst) + 1
        ))
        .expect("generated lease tokens should be valid")
    }
}

impl StorageBackend for InMemoryStorageBackend {
    fn initialize(&self) -> Result<(), StorageError> {
        Ok(())
    }

    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
        validate_namespace(namespace)?;
        validate_record_key(key)?;
        Ok(self
            .records
            .lock()
            .expect("records mutex poisoned")
            .get(&(namespace.to_string(), key.to_string()))
            .cloned())
    }

    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
        input.validate()?;
        let mut records = self.records.lock().expect("records mutex poisoned");
        let map_key = (input.namespace.clone(), input.key.clone());
        let existing = records.get(&map_key).cloned();

        if let Some(expected) = &input.if_revision {
            match &existing {
                Some(record) if &record.revision == expected => {}
                Some(record) => {
                    return Err(StorageError::Conflict {
                        namespace: input.namespace,
                        key: input.key,
                        expected_revision: Some(expected.to_string()),
                        actual_revision: Some(record.revision.to_string()),
                    })
                }
                None => {
                    return Err(StorageError::Conflict {
                        namespace: input.namespace,
                        key: input.key,
                        expected_revision: Some(expected.to_string()),
                        actual_revision: None,
                    })
                }
            }
        }

        let created_at = existing
            .as_ref()
            .map(|record| record.created_at)
            .unwrap_or_else(now_utc_ms);
        let updated_at = now_utc_ms();
        let revision = self.next_revision();
        let record = StorageRecord::new(
            input.namespace,
            input.key,
            revision,
            input.content_type,
            input.metadata,
            input.body,
            created_at,
            updated_at,
        )?;
        records.insert(map_key, record.clone());
        Ok(record)
    }

    fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<&Revision>,
    ) -> Result<(), StorageError> {
        validate_namespace(namespace)?;
        validate_record_key(key)?;

        let mut records = self.records.lock().expect("records mutex poisoned");
        let map_key = (namespace.to_string(), key.to_string());
        if let Some(record) = records.get(&map_key) {
            if let Some(expected) = if_revision {
                if &record.revision != expected {
                    return Err(StorageError::Conflict {
                        namespace: namespace.to_string(),
                        key: key.to_string(),
                        expected_revision: Some(expected.to_string()),
                        actual_revision: Some(record.revision.to_string()),
                    });
                }
            }
        }
        records.remove(&map_key);
        Ok(())
    }

    fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StoragePage, StorageError> {
        validate_namespace(namespace)?;
        options.validate()?;

        let cursor = options.cursor.clone();
        let page_size = options.page_size.unwrap_or(usize::MAX);
        let prefix = options.prefix.unwrap_or_default();

        let mut filtered = self
            .records
            .lock()
            .expect("records mutex poisoned")
            .values()
            .filter(|record| {
                record.namespace == namespace
                    && record.key.starts_with(&prefix)
                    && cursor
                        .as_ref()
                        .map(|cursor| &record.key > cursor)
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();

        filtered.sort_by(|left, right| left.key.cmp(&right.key));

        let next_cursor = if filtered.len() > page_size {
            Some(filtered[page_size - 1].key.clone())
        } else {
            None
        };
        filtered.truncate(page_size);

        Ok(StoragePage {
            records: filtered,
            next_cursor,
        })
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        Ok(self.get(namespace, key)?.map(|record| record.stat()))
    }

    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError> {
        validate_lease_name(name)?;
        if ttl_ms == 0 {
            return Err(StorageError::Validation {
                field: "ttl_ms".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }

        let now = now_utc_ms();
        let mut leases = self.leases.lock().expect("leases mutex poisoned");
        if let Some(existing) = leases.get(name) {
            if existing.is_active_at(now) {
                return Ok(None);
            }
        }

        let lease =
            StorageLease::new(name.to_string(), self.next_lease_token(), now, now + ttl_ms)?;
        leases.insert(name.to_string(), lease.clone());
        Ok(Some(lease))
    }
}

/// Shared backend conformance helpers. Concrete backends can call these inside
/// their own test suites to assert the repository-owned semantics.
pub mod conformance {
    use super::*;
    use coding_adventures_json_value::JsonNumber;

    /// `initialize()` must be idempotent.
    pub fn initialize_twice_is_safe<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        backend.initialize()?;
        Ok(())
    }

    /// `put()` followed by `get()` must round-trip metadata and bytes.
    pub fn put_then_get_round_trips<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        let expected_metadata = JsonValue::Object(vec![
            ("kind".to_string(), JsonValue::String("demo".to_string())),
            (
                "count".to_string(),
                JsonValue::Number(JsonNumber::Integer(2)),
            ),
        ]);
        let record = backend.put(StoragePutInput::new(
            "context",
            "entries/a.json",
            "application/json",
            expected_metadata.clone(),
            br#"{"a":1}"#.to_vec(),
        )?)?;

        let fetched = backend
            .get("context", "entries/a.json")?
            .expect("put record must exist");

        assert_eq!(fetched.metadata, expected_metadata);
        assert_eq!(fetched.body, br#"{"a":1}"#);
        assert_eq!(fetched.revision, record.revision);
        Ok(())
    }

    /// A stale compare-and-swap revision must be rejected.
    pub fn stale_revision_is_rejected<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        let first = backend.put(StoragePutInput::new(
            "artifacts",
            "plans/demo.txt",
            "text/plain",
            JsonValue::Object(vec![]),
            b"v1".to_vec(),
        )?)?;

        let second = backend.put(
            StoragePutInput::new(
                "artifacts",
                "plans/demo.txt",
                "text/plain",
                JsonValue::Object(vec![]),
                b"v2".to_vec(),
            )?
            .with_if_revision(Some(first.revision.clone())),
        )?;

        let stale_attempt = backend.put(
            StoragePutInput::new(
                "artifacts",
                "plans/demo.txt",
                "text/plain",
                JsonValue::Object(vec![]),
                b"v3".to_vec(),
            )?
            .with_if_revision(Some(first.revision)),
        );

        match stale_attempt {
            Err(StorageError::Conflict { .. }) => {}
            other => panic!("expected conflict on stale write, got {:?}", other),
        }

        let latest = backend
            .get("artifacts", "plans/demo.txt")?
            .expect("record should still exist");
        assert_eq!(latest.revision, second.revision);
        assert_eq!(latest.body, b"v2");
        Ok(())
    }

    /// Deleting a missing record must succeed without error.
    pub fn delete_is_idempotent<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        backend.delete("skills", "versions/missing.json", None)?;
        backend.delete("skills", "versions/missing.json", None)?;
        Ok(())
    }

    /// Prefix listing must be stable and lexicographically ordered by key.
    pub fn prefix_listing_is_stable<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        backend.put(StoragePutInput::new(
            "memory",
            "records/beta.json",
            "application/json",
            JsonValue::Object(vec![]),
            b"beta".to_vec(),
        )?)?;
        backend.put(StoragePutInput::new(
            "memory",
            "records/alpha.json",
            "application/json",
            JsonValue::Object(vec![]),
            b"alpha".to_vec(),
        )?)?;
        backend.put(StoragePutInput::new(
            "memory",
            "records/gamma.json",
            "application/json",
            JsonValue::Object(vec![]),
            b"gamma".to_vec(),
        )?)?;

        let page = backend.list(
            "memory",
            StorageListOptions {
                prefix: Some("records/".to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        let keys: Vec<String> = page
            .records
            .iter()
            .map(|record| record.key.clone())
            .collect();
        assert_eq!(
            keys,
            vec![
                "records/alpha.json".to_string(),
                "records/beta.json".to_string(),
                "records/gamma.json".to_string(),
            ]
        );
        Ok(())
    }

    /// Advisory leases should eventually expire.
    pub fn advisory_lease_expires<B: StorageBackend>(backend: &B) -> Result<(), StorageError> {
        backend.initialize()?;
        let lease = backend
            .acquire_lease("context-compaction", 25)?
            .expect("first lease acquisition should succeed");

        assert!(lease.is_active_at(lease.issued_at));
        assert!(
            backend.acquire_lease("context-compaction", 25)?.is_none(),
            "lease should still be held before expiry"
        );

        sleep(Duration::from_millis(40));
        assert!(
            backend.acquire_lease("context-compaction", 25)?.is_some(),
            "lease should be acquirable again after expiry"
        );
        Ok(())
    }
}

fn validate_namespace(value: &str) -> Result<(), StorageError> {
    validate_path_like("namespace", value)
}

fn validate_record_key(value: &str) -> Result<(), StorageError> {
    validate_path_like("key", value)
}

fn validate_prefix(value: &str) -> Result<(), StorageError> {
    validate_non_empty("prefix", value)?;
    if value.contains('\n') || value.contains('\r') {
        return Err(StorageError::Validation {
            field: "prefix".to_string(),
            message: "must not contain carriage returns or newlines".to_string(),
        });
    }
    if value.starts_with('/') {
        return Err(StorageError::Validation {
            field: "prefix".to_string(),
            message: "must not start with '/'".to_string(),
        });
    }
    Ok(())
}

fn validate_path_like(field: &str, value: &str) -> Result<(), StorageError> {
    validate_non_empty(field, value)?;
    if value.starts_with('/') || value.ends_with('/') {
        return Err(StorageError::Validation {
            field: field.to_string(),
            message: "must not start or end with '/'".to_string(),
        });
    }
    if value.contains("//") {
        return Err(StorageError::Validation {
            field: field.to_string(),
            message: "must not contain empty path segments".to_string(),
        });
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(StorageError::Validation {
            field: field.to_string(),
            message: "must not contain carriage returns or newlines".to_string(),
        });
    }
    for segment in value.split('/') {
        if segment.is_empty() {
            return Err(StorageError::Validation {
                field: field.to_string(),
                message: "must not contain empty path segments".to_string(),
            });
        }
        if segment
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')))
        {
            return Err(StorageError::Validation {
                field: field.to_string(),
                message:
                    "segments must use only ASCII letters, digits, dots, underscores, or hyphens"
                        .to_string(),
            });
        }
    }
    Ok(())
}

fn validate_content_type(value: &str) -> Result<(), StorageError> {
    validate_non_empty("content_type", value)?;
    if value.contains('\n') || value.contains('\r') {
        return Err(StorageError::Validation {
            field: "content_type".to_string(),
            message: "must not contain carriage returns or newlines".to_string(),
        });
    }
    if !value.contains('/') {
        return Err(StorageError::Validation {
            field: "content_type".to_string(),
            message: "must contain a '/' separator like a MIME type".to_string(),
        });
    }
    Ok(())
}

fn validate_metadata_object(metadata: &StorageMetadata) -> Result<(), StorageError> {
    if !matches!(metadata, JsonValue::Object(_)) {
        return Err(StorageError::Validation {
            field: "metadata".to_string(),
            message: "must be a JSON object".to_string(),
        });
    }
    Ok(())
}

fn validate_lease_name(value: &str) -> Result<(), StorageError> {
    validate_path_like("lease_name", value)
}

fn validate_single_line_token(field: &str, value: &str) -> Result<(), StorageError> {
    validate_non_empty(field, value)?;
    if value.contains('\n') || value.contains('\r') {
        return Err(StorageError::Validation {
            field: field.to_string(),
            message: "must not contain carriage returns or newlines".to_string(),
        });
    }
    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::Validation {
            field: field.to_string(),
            message: "must not be empty".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> StorageMetadata {
        JsonValue::Object(vec![(
            "source".to_string(),
            JsonValue::String("test".to_string()),
        )])
    }

    #[test]
    fn storage_put_input_rejects_non_object_metadata() {
        let error = StoragePutInput::new(
            "context",
            "entries/demo.json",
            "application/json",
            JsonValue::String("nope".to_string()),
            Vec::new(),
        )
        .expect_err("metadata must be a JSON object");

        assert!(matches!(error, StorageError::Validation { .. }));
    }

    #[test]
    fn storage_put_input_computes_hash_from_body() {
        let input = StoragePutInput::new(
            "artifacts",
            "plans/demo.txt",
            "text/plain",
            metadata(),
            b"hello".to_vec(),
        )
        .expect("input should be valid");

        assert_eq!(input.content_hash(), sha256(b"hello"));
    }

    #[test]
    fn storage_record_stat_matches_record_fields() {
        let record = StorageRecord::new(
            "skills",
            "manifests/demo.json",
            Revision::new("r1").unwrap(),
            "application/json",
            metadata(),
            b"{}".to_vec(),
            10,
            12,
        )
        .expect("record should be valid");

        let stat = record.stat();
        assert_eq!(stat.namespace, "skills");
        assert_eq!(stat.key, "manifests/demo.json");
        assert_eq!(stat.body_len, 2);
        assert_eq!(stat.content_hash, sha256(b"{}"));
    }

    #[test]
    fn invalid_path_segments_are_rejected() {
        let error = StoragePutInput::new(
            "context",
            "entries/bad segment.json",
            "application/json",
            metadata(),
            Vec::new(),
        )
        .expect_err("spaces are not valid in key segments");

        assert!(matches!(error, StorageError::Validation { field, .. } if field == "key"));
    }

    #[test]
    fn lease_reports_active_window() {
        let lease = StorageLease::new(
            "memory-rebuild",
            LeaseToken::new("lease-1").unwrap(),
            100,
            150,
        )
        .expect("lease should be valid");

        assert!(lease.is_active_at(100));
        assert!(lease.is_active_at(149));
        assert!(!lease.is_active_at(150));
    }

    #[test]
    fn conformance_initialize_twice_is_safe() {
        let backend = InMemoryStorageBackend::default();
        conformance::initialize_twice_is_safe(&backend).unwrap();
    }

    #[test]
    fn conformance_put_then_get_round_trips() {
        let backend = InMemoryStorageBackend::default();
        conformance::put_then_get_round_trips(&backend).unwrap();
    }

    #[test]
    fn conformance_stale_revision_is_rejected() {
        let backend = InMemoryStorageBackend::default();
        conformance::stale_revision_is_rejected(&backend).unwrap();
    }

    #[test]
    fn conformance_delete_is_idempotent() {
        let backend = InMemoryStorageBackend::default();
        conformance::delete_is_idempotent(&backend).unwrap();
    }

    #[test]
    fn conformance_prefix_listing_is_stable() {
        let backend = InMemoryStorageBackend::default();
        conformance::prefix_listing_is_stable(&backend).unwrap();
    }

    #[test]
    fn conformance_advisory_lease_expires() {
        let backend = InMemoryStorageBackend::default();
        conformance::advisory_lease_expires(&backend).unwrap();
    }

    #[test]
    fn list_paginates_with_cursor() {
        let backend = InMemoryStorageBackend::default();
        backend.initialize().unwrap();
        for key in ["a.json", "b.json", "c.json"] {
            backend
                .put(
                    StoragePutInput::new(
                        "context",
                        format!("entries/{key}"),
                        "application/json",
                        metadata(),
                        key.as_bytes().to_vec(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let first = backend
            .list(
                "context",
                StorageListOptions {
                    prefix: Some("entries/".to_string()),
                    recursive: true,
                    page_size: Some(2),
                    cursor: None,
                },
            )
            .unwrap();

        assert_eq!(first.records.len(), 2);
        assert_eq!(first.next_cursor.as_deref(), Some("entries/b.json"));

        let second = backend
            .list(
                "context",
                StorageListOptions {
                    prefix: Some("entries/".to_string()),
                    recursive: true,
                    page_size: Some(2),
                    cursor: first.next_cursor,
                },
            )
            .unwrap();

        assert_eq!(second.records.len(), 1);
        assert_eq!(second.records[0].key, "entries/c.json");
        assert_eq!(second.next_cursor, None);
    }

    #[test]
    fn revision_and_lease_tokens_expose_strings_and_reject_newlines() {
        let revision = Revision::new("r42").unwrap();
        let lease = LeaseToken::new("lease-42").unwrap();

        assert_eq!(revision.as_str(), "r42");
        assert_eq!(lease.as_str(), "lease-42");
        assert_eq!(revision.to_string(), "r42");
        assert_eq!(lease.to_string(), "lease-42");

        assert!(Revision::new("bad\nrevision").is_err());
        assert!(LeaseToken::new("bad\nlease").is_err());
    }

    #[test]
    fn storage_record_and_list_options_validate_edge_cases() {
        let error = StorageRecord::new(
            "context",
            "entries/demo.json",
            Revision::new("r1").unwrap(),
            "application/json",
            metadata(),
            b"{}".to_vec(),
            20,
            10,
        )
        .unwrap_err();
        assert!(matches!(error, StorageError::Validation { .. }));

        let options = StorageListOptions {
            prefix: Some("entries/".to_string()),
            recursive: true,
            page_size: Some(0),
            cursor: Some("cursor".to_string()),
        };
        assert!(options.validate().is_err());
    }

    #[test]
    fn in_memory_backend_reports_conflicts_and_lease_validation_errors() {
        let backend = InMemoryStorageBackend::default();
        backend.initialize().unwrap();

        let record = backend
            .put(
                StoragePutInput::new(
                    "context",
                    "entries/demo.json",
                    "application/json",
                    metadata(),
                    b"{}".to_vec(),
                )
                .unwrap(),
            )
            .unwrap();

        let error = backend
            .delete(
                "context",
                "entries/demo.json",
                Some(&Revision::new("wrong").unwrap()),
            )
            .unwrap_err();
        assert!(matches!(error, StorageError::Conflict { .. }));

        let error = backend.acquire_lease("lease", 0).unwrap_err();
        assert!(matches!(error, StorageError::Validation { .. }));

        let stat = backend
            .stat("context", "entries/demo.json")
            .unwrap()
            .unwrap();
        assert_eq!(stat.revision, record.revision);
    }
}
