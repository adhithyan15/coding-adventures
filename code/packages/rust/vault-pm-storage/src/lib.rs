//! # `vault-pm-storage`
//!
//! VLT-PM02's storage layer is intentionally boring: opaque bucket names,
//! opaque object names, and immutable byte strings. “Boring” is a security
//! feature here. A Google Drive adapter must not need to know whether bytes are
//! a login item, a commit, or random padding.
//!
//! The crate contains four things:
//!
//! 1. [`VaultObjectStore`], the provider-neutral contract;
//! 2. [`InMemoryObjectStore`], the deterministic executable model;
//! 3. [`FaultInjectingObjectStore`], which makes hostile storage behavior
//!    repeatable in repository tests; and
//! 4. [`ReplicaSetObjectStore`], VLT-PM00 §11.5's mirror decorator: it
//!    publishes every immutable write to one primary and zero or more
//!    best-effort mirror replicas without letting a slow or unreachable
//!    mirror block the primary commit (§19.2).
//!
//! The implementation has no I/O authority. Filesystem and cloud adapters live
//! in separate packages and run [`run_conformance_suite`] against the same
//! public behavior.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Mutex;

use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_format::ObjectId as FormatObjectId;

/// V1's absolute body bound, aligned with VLT-PM01 object frames.
pub const MAX_OBJECT_BYTES: usize = 64 * 1024 * 1024;
/// Largest page a caller may request.
pub const MAX_LIST_LIMIT: usize = 10_000;
/// Largest backend-owned continuation token accepted at the boundary.
pub const MAX_CURSOR_BYTES: usize = 256;
/// Largest provider revision token accepted at the boundary.
pub const MAX_REVISION_BYTES: usize = 256;
/// Upper bound for one optional change-feed page.
pub const MAX_CHANGE_EVENTS: usize = 1_000;
/// Language-neutral operation vector consumed by every adapter implementation.
pub const CONFORMANCE_FIXTURE_V1: &str =
    include_str!("../../../../specs/fixtures/vault-pm-storage-v1.json");

macro_rules! private_fixed_bytes {
    ($name:ident, $size:expr, $label:literal) => {
        #[doc = $label]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $size]);

        impl $name {
            /// Construct the opaque value from exact bytes.
            pub const fn new(bytes: [u8; $size]) -> Self {
                Self(bytes)
            }

            /// Borrow bytes for a provider adapter's lossless name encoding.
            pub const fn as_bytes(&self) -> &[u8; $size] {
                &self.0
            }

            /// Consume the wrapper and return its exact bytes.
            pub const fn into_bytes(self) -> [u8; $size] {
                self.0
            }
        }

        impl Debug for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!($label, "(<redacted>)"))
            }
        }
    };
}

private_fixed_bytes!(VaultLocator, 32, "VaultLocator");
private_fixed_bytes!(BucketId, 32, "BucketId");
private_fixed_bytes!(ObjectId, 32, "ObjectId");

impl From<FormatObjectId> for ObjectId {
    fn from(value: FormatObjectId) -> Self {
        Self::new(value.into_bytes())
    }
}

impl From<ObjectId> for FormatObjectId {
    fn from(value: ObjectId) -> Self {
        Self::new(value.into_bytes())
    }
}

/// A bounded opaque body. Debug output reports length, never content.
#[derive(Clone, PartialEq, Eq)]
pub struct ObjectBytes(Vec<u8>);

impl ObjectBytes {
    /// Validate and own an object body.
    pub fn new(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.len() > MAX_OBJECT_BYTES {
            return Err(StoreError::InvalidInput(InputViolation::ObjectTooLarge));
        }
        Ok(Self(bytes))
    }

    /// Borrow exact bytes for transport or verification.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Return the body length.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Report whether the body is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume the wrapper.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    fn corrupted_clone(&self) -> Self {
        let mut bytes = self.0.clone();
        if let Some(first) = bytes.first_mut() {
            *first ^= 0x80;
        } else {
            bytes.push(0x80);
        }
        Self(bytes)
    }
}

impl Debug for ObjectBytes {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectBytes")
            .field("len", &self.len())
            .field("bytes", &"<redacted>")
            .finish()
    }
}

macro_rules! private_token {
    ($name:ident, $label:literal) => {
        #[doc = $label]
        #[derive(Clone, PartialEq, Eq)]
        pub struct $name(Vec<u8>);

        impl $name {
            /// Restore an opaque token previously returned by a backend.
            pub fn new(bytes: Vec<u8>) -> Result<Self, StoreError> {
                if bytes.is_empty() {
                    return Err(StoreError::InvalidInput(InputViolation::CursorMalformed));
                }
                if bytes.len() > MAX_CURSOR_BYTES {
                    return Err(StoreError::InvalidInput(InputViolation::CursorTooLarge));
                }
                Ok(Self(bytes))
            }

            /// Borrow exact bytes for persistence between calls.
            pub fn as_bytes(&self) -> &[u8] {
                &self.0
            }

            fn trusted(bytes: Vec<u8>) -> Self {
                debug_assert!(!bytes.is_empty() && bytes.len() <= MAX_CURSOR_BYTES);
                Self(bytes)
            }
        }

        impl Debug for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct($label)
                    .field("len", &self.0.len())
                    .field("bytes", &"<redacted>")
                    .finish()
            }
        }
    };
}

private_token!(ListCursor, "ListCursor");
private_token!(ChangeCursor, "ChangeCursor");

/// Opaque, bounded provider revision. Its value is redacted from Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderRevision(String);

impl ProviderRevision {
    /// Validate a non-empty, single-line provider revision.
    pub fn new(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_REVISION_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(StoreError::InvalidInput(InputViolation::RevisionMalformed));
        }
        Ok(Self(value))
    }

    /// Borrow the exact revision for a conditional provider request.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for ProviderRevision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderRevision(<redacted>)")
    }
}

/// Body-free advisory metadata for one committed object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectStat {
    pub body_len: u64,
    pub revision: Option<ProviderRevision>,
    pub server_checksum: Option<[u8; 32]>,
}

/// One logical object in an ordered listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectEntry {
    pub object: ObjectId,
    pub stat: ObjectStat,
}

/// One page of unique, ascending logical objects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectPage {
    pub entries: Vec<ObjectEntry>,
    pub next_cursor: Option<ListCursor>,
}

/// Result of an immutable put.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PutImmutableOutcome {
    Created,
    AlreadyPresent,
}

/// Result of an optional physical delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    Missing,
}

/// Change-feed event kind. Events are discovery hints, never authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Put,
    Delete,
}

/// One backend-local monotonically sequenced change hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    pub sequence: u64,
    pub bucket: BucketId,
    pub object: ObjectId,
    pub kind: ChangeKind,
}

/// One bounded page from an optional change feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangePage {
    pub events: Vec<ChangeEvent>,
    /// Watermark to persist even when `events` is empty.
    pub cursor: ChangeCursor,
}

/// Optimization capabilities. None weakens repository verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub strong_read_after_write: bool,
    pub strong_list_after_write: bool,
    pub conditional_create: bool,
    pub conditional_replace: bool,
    pub change_feed: bool,
    pub push_notifications: bool,
    pub resumable_upload: bool,
    pub range_read: bool,
    pub server_checksum: bool,
    pub physical_delete: bool,
    pub shareable_container: bool,
    pub max_object_size: Option<u64>,
    pub preferred_pack_size: u64,
}

impl BackendCapabilities {
    /// Capabilities of the full in-memory executable model.
    pub const fn in_memory() -> Self {
        Self {
            strong_read_after_write: true,
            strong_list_after_write: true,
            conditional_create: true,
            conditional_replace: false,
            change_feed: true,
            push_notifications: false,
            resumable_upload: false,
            range_read: false,
            server_checksum: true,
            physical_delete: true,
            shareable_container: false,
            max_object_size: Some(MAX_OBJECT_BYTES as u64),
            preferred_pack_size: 0,
        }
    }

    /// The weakest conforming baseline: immutable bytes plus complete listing.
    pub const fn baseline() -> Self {
        Self {
            strong_read_after_write: false,
            strong_list_after_write: false,
            conditional_create: false,
            conditional_replace: false,
            change_feed: false,
            push_notifications: false,
            resumable_upload: false,
            range_read: false,
            server_checksum: false,
            physical_delete: false,
            shareable_container: false,
            max_object_size: None,
            preferred_pack_size: 0,
        }
    }
}

/// Machine-comparable invalid-input reasons without attacker-controlled text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputViolation {
    ObjectTooLarge,
    ListLimit,
    CursorTooLarge,
    CursorMalformed,
    CursorScope,
    RevisionMalformed,
    FaultOperationMismatch,
}

/// Closed storage error taxonomy. Display and Debug contain no input bytes.
#[derive(Clone, PartialEq, Eq)]
pub enum StoreError {
    InvalidInput(InputViolation),
    NotInitialized,
    Authorization,
    Quota,
    RateLimited { retry_after_ms: Option<u64> },
    Network,
    Corruption,
    Conflict,
    Unsupported,
    Provider,
}

impl Debug for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reason) => {
                formatter.debug_tuple("InvalidInput").field(reason).finish()
            }
            Self::RateLimited { retry_after_ms } => formatter
                .debug_struct("RateLimited")
                .field("retry_after_ms", retry_after_ms)
                .finish(),
            other => formatter.write_str(other.label()),
        }
    }
}

impl StoreError {
    fn label(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "InvalidInput",
            Self::NotInitialized => "NotInitialized",
            Self::Authorization => "Authorization",
            Self::Quota => "Quota",
            Self::RateLimited { .. } => "RateLimited",
            Self::Network => "Network",
            Self::Corruption => "Corruption",
            Self::Conflict => "Conflict",
            Self::Unsupported => "Unsupported",
            Self::Provider => "Provider",
        }
    }
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("vault-pm-storage: ")?;
        formatter.write_str(match self {
            Self::InvalidInput(_) => "invalid input",
            Self::NotInitialized => "store is not initialized",
            Self::Authorization => "authorization failed",
            Self::Quota => "provider quota exhausted",
            Self::RateLimited { .. } => "provider rate limited the operation",
            Self::Network => "network operation failed",
            Self::Corruption => "immutable storage corruption detected",
            Self::Conflict => "storage identity conflict",
            Self::Unsupported => "operation is unsupported",
            Self::Provider => "provider operation failed",
        })
    }
}

impl std::error::Error for StoreError {}

/// Provider-neutral immutable object storage.
pub trait VaultObjectStore: Send + Sync {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError>;
    fn capabilities(&self) -> BackendCapabilities;
    fn get(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectBytes>, StoreError>;
    fn stat(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectStat>, StoreError>;
    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError>;
    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError>;
    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError>;
    fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError>;
}

#[derive(Clone)]
struct StoredObject {
    bytes: ObjectBytes,
    revision: u64,
}

#[derive(Default)]
struct MemoryState {
    locator: Option<VaultLocator>,
    revision: u64,
    objects: BTreeMap<(BucketId, ObjectId), StoredObject>,
    changes: Vec<ChangeEvent>,
}

/// Thread-safe deterministic reference backend.
pub struct InMemoryObjectStore {
    capabilities: BackendCapabilities,
    state: Mutex<MemoryState>,
}

impl Default for InMemoryObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryObjectStore {
    /// Construct the fully featured in-memory model.
    pub fn new() -> Self {
        Self::with_capabilities(BackendCapabilities::in_memory())
    }

    /// Construct a model with optional features enabled exactly as reported.
    pub fn with_capabilities(capabilities: BackendCapabilities) -> Self {
        Self {
            capabilities,
            state: Mutex::new(MemoryState::default()),
        }
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, MemoryState>, StoreError> {
        self.state.lock().map_err(|_| StoreError::Provider)
    }

    fn require_initialized(state: &MemoryState) -> Result<(), StoreError> {
        if state.locator.is_none() {
            Err(StoreError::NotInitialized)
        } else {
            Ok(())
        }
    }

    fn stat_for(&self, object: &StoredObject) -> Result<ObjectStat, StoreError> {
        Ok(ObjectStat {
            body_len: u64::try_from(object.bytes.len())
                .map_err(|_| StoreError::InvalidInput(InputViolation::ObjectTooLarge))?,
            revision: Some(ProviderRevision::new(object.revision.to_string())?),
            server_checksum: self
                .capabilities
                .server_checksum
                .then(|| sha256(object.bytes.as_slice())),
        })
    }

    fn next_revision(state: &mut MemoryState) -> Result<u64, StoreError> {
        state.revision = state.revision.checked_add(1).ok_or(StoreError::Provider)?;
        Ok(state.revision)
    }

    fn record_change(
        state: &mut MemoryState,
        sequence: u64,
        bucket: BucketId,
        object: ObjectId,
        kind: ChangeKind,
    ) {
        state.changes.push(ChangeEvent {
            sequence,
            bucket,
            object,
            kind,
        });
    }
}

impl VaultObjectStore for InMemoryObjectStore {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        let mut state = self.lock_state()?;
        match state.locator {
            None => {
                state.locator = Some(*locator);
                Ok(())
            }
            Some(existing) if existing == *locator => Ok(()),
            Some(_) => Err(StoreError::Conflict),
        }
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.capabilities.clone()
    }

    fn get(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectBytes>, StoreError> {
        let state = self.lock_state()?;
        Self::require_initialized(&state)?;
        Ok(state
            .objects
            .get(&(*bucket, *object))
            .map(|entry| entry.bytes.clone()))
    }

    fn stat(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectStat>, StoreError> {
        let state = self.lock_state()?;
        Self::require_initialized(&state)?;
        state
            .objects
            .get(&(*bucket, *object))
            .map(|entry| self.stat_for(entry))
            .transpose()
    }

    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        let mut state = self.lock_state()?;
        Self::require_initialized(&state)?;
        if let Some(max) = self.capabilities.max_object_size {
            if u64::try_from(bytes.len())
                .map_err(|_| StoreError::InvalidInput(InputViolation::ObjectTooLarge))?
                > max
            {
                return Err(StoreError::InvalidInput(InputViolation::ObjectTooLarge));
            }
        }
        let key = (*bucket, *object);
        if let Some(existing) = state.objects.get(&key) {
            return if existing.bytes == *bytes {
                Ok(PutImmutableOutcome::AlreadyPresent)
            } else {
                Err(StoreError::Corruption)
            };
        }

        let revision = Self::next_revision(&mut state)?;
        state.objects.insert(
            key,
            StoredObject {
                bytes: bytes.clone(),
                revision,
            },
        );
        Self::record_change(&mut state, revision, *bucket, *object, ChangeKind::Put);
        Ok(PutImmutableOutcome::Created)
    }

    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError> {
        let state = self.lock_state()?;
        Self::require_initialized(&state)?;
        if !(1..=MAX_LIST_LIMIT).contains(&limit) {
            return Err(StoreError::InvalidInput(InputViolation::ListLimit));
        }
        let after = cursor
            .map(|value| decode_list_cursor(value, bucket))
            .transpose()?;

        let mut candidates = state
            .objects
            .iter()
            .filter(|((entry_bucket, object), _)| {
                entry_bucket == bucket && after.is_none_or(|position| *object > position)
            })
            .take(limit + 1)
            .map(|((_, object), stored)| {
                Ok(ObjectEntry {
                    object: *object,
                    stat: self.stat_for(stored)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let has_more = candidates.len() > limit;
        if has_more {
            candidates.truncate(limit);
        }
        let next_cursor = if has_more {
            candidates
                .last()
                .map(|entry| encode_list_cursor(bucket, &entry.object))
        } else {
            None
        };
        Ok(ObjectPage {
            entries: candidates,
            next_cursor,
        })
    }

    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError> {
        let mut state = self.lock_state()?;
        Self::require_initialized(&state)?;
        if !self.capabilities.physical_delete {
            return Err(StoreError::Unsupported);
        }
        if state.objects.remove(&(*bucket, *object)).is_none() {
            return Ok(DeleteOutcome::Missing);
        }
        let revision = Self::next_revision(&mut state)?;
        Self::record_change(&mut state, revision, *bucket, *object, ChangeKind::Delete);
        Ok(DeleteOutcome::Deleted)
    }

    fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
        let state = self.lock_state()?;
        Self::require_initialized(&state)?;
        if !self.capabilities.change_feed {
            return Err(StoreError::Unsupported);
        }
        let locator = state.locator.ok_or(StoreError::NotInitialized)?;
        let after = cursor
            .map(|value| decode_change_cursor(value, &locator))
            .transpose()?
            .unwrap_or(0);
        if after > state.revision {
            return Err(StoreError::InvalidInput(InputViolation::CursorMalformed));
        }
        let events = state
            .changes
            .iter()
            .filter(|event| event.sequence > after)
            .take(MAX_CHANGE_EVENTS)
            .cloned()
            .collect::<Vec<_>>();
        let watermark = events.last().map_or(after, |event| event.sequence);
        if events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ChangePage {
                events,
                cursor: encode_change_cursor(&locator, watermark),
            }))
        }
    }
}

const LIST_CURSOR_VERSION: u8 = 1;
const LIST_CURSOR_LEN: usize = 1 + 32 + 32;

fn encode_list_cursor(bucket: &BucketId, object: &ObjectId) -> ListCursor {
    let mut bytes = Vec::with_capacity(LIST_CURSOR_LEN);
    bytes.push(LIST_CURSOR_VERSION);
    bytes.extend_from_slice(bucket.as_bytes());
    bytes.extend_from_slice(object.as_bytes());
    ListCursor::trusted(bytes)
}

fn decode_list_cursor(cursor: &ListCursor, bucket: &BucketId) -> Result<ObjectId, StoreError> {
    let bytes = cursor.as_bytes();
    if bytes.len() != LIST_CURSOR_LEN || bytes[0] != LIST_CURSOR_VERSION {
        return Err(StoreError::InvalidInput(InputViolation::CursorMalformed));
    }
    if &bytes[1..33] != bucket.as_bytes() {
        return Err(StoreError::InvalidInput(InputViolation::CursorScope));
    }
    let mut object = [0_u8; 32];
    object.copy_from_slice(&bytes[33..]);
    Ok(ObjectId::new(object))
}

const CHANGE_CURSOR_VERSION: u8 = 1;
const CHANGE_CURSOR_LEN: usize = 1 + 32 + 8;

fn encode_change_cursor(locator: &VaultLocator, sequence: u64) -> ChangeCursor {
    let mut bytes = Vec::with_capacity(CHANGE_CURSOR_LEN);
    bytes.push(CHANGE_CURSOR_VERSION);
    bytes.extend_from_slice(locator.as_bytes());
    bytes.extend_from_slice(&sequence.to_be_bytes());
    ChangeCursor::trusted(bytes)
}

fn decode_change_cursor(cursor: &ChangeCursor, locator: &VaultLocator) -> Result<u64, StoreError> {
    let bytes = cursor.as_bytes();
    if bytes.len() != CHANGE_CURSOR_LEN || bytes[0] != CHANGE_CURSOR_VERSION {
        return Err(StoreError::InvalidInput(InputViolation::CursorMalformed));
    }
    if &bytes[1..33] != locator.as_bytes() {
        return Err(StoreError::InvalidInput(InputViolation::CursorScope));
    }
    let mut sequence = [0_u8; 8];
    sequence.copy_from_slice(&bytes[33..]);
    Ok(u64::from_be_bytes(sequence))
}

/// Operations addressable by deterministic faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreOperation {
    Initialize,
    Get,
    Stat,
    PutImmutable,
    List,
    DeleteUnreferenced,
    Changes,
}

/// One-shot fault effect. Input bytes cannot be embedded in an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaultEffect {
    Return(StoreError),
    CorruptGet,
    OmitLastListEntry,
    DuplicateFirstListEntry,
    CommitPutThenNetwork,
}

/// One operation-scoped fault action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultAction {
    pub operation: StoreOperation,
    pub effect: FaultEffect,
}

/// Transparent-by-default wrapper with deterministic one-shot faults.
pub struct FaultInjectingObjectStore<S> {
    inner: S,
    faults: Mutex<VecDeque<FaultAction>>,
}

impl<S> FaultInjectingObjectStore<S> {
    /// Wrap a store with an initially empty fault queue.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            faults: Mutex::new(VecDeque::new()),
        }
    }

    /// Borrow the wrapped store for assertions or configuration.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Consume the wrapper and return the store.
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Enqueue one validated operation-specific action.
    pub fn enqueue(&self, action: FaultAction) -> Result<(), StoreError> {
        if !effect_matches(action.operation, &action.effect) {
            return Err(StoreError::InvalidInput(
                InputViolation::FaultOperationMismatch,
            ));
        }
        self.faults
            .lock()
            .map_err(|_| StoreError::Provider)?
            .push_back(action);
        Ok(())
    }

    /// Return the number of actions not yet consumed.
    pub fn pending_faults(&self) -> Result<usize, StoreError> {
        Ok(self.faults.lock().map_err(|_| StoreError::Provider)?.len())
    }

    fn take_fault(&self, operation: StoreOperation) -> Result<Option<FaultEffect>, StoreError> {
        let mut faults = self.faults.lock().map_err(|_| StoreError::Provider)?;
        let Some(position) = faults
            .iter()
            .position(|action| action.operation == operation)
        else {
            return Ok(None);
        };
        Ok(faults.remove(position).map(|action| action.effect))
    }
}

fn effect_matches(operation: StoreOperation, effect: &FaultEffect) -> bool {
    match effect {
        FaultEffect::Return(_) => true,
        FaultEffect::CorruptGet => operation == StoreOperation::Get,
        FaultEffect::OmitLastListEntry | FaultEffect::DuplicateFirstListEntry => {
            operation == StoreOperation::List
        }
        FaultEffect::CommitPutThenNetwork => operation == StoreOperation::PutImmutable,
    }
}

fn immediate_error(effect: Option<FaultEffect>) -> Result<Option<FaultEffect>, StoreError> {
    match effect {
        Some(FaultEffect::Return(error)) => Err(error),
        other => Ok(other),
    }
}

impl<S: VaultObjectStore> VaultObjectStore for FaultInjectingObjectStore<S> {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        immediate_error(self.take_fault(StoreOperation::Initialize)?)?;
        self.inner.initialize(locator)
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.inner.capabilities()
    }

    fn get(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectBytes>, StoreError> {
        let effect = immediate_error(self.take_fault(StoreOperation::Get)?)?;
        let result = self.inner.get(bucket, object)?;
        match effect {
            Some(FaultEffect::CorruptGet) => Ok(result.map(|bytes| bytes.corrupted_clone())),
            _ => Ok(result),
        }
    }

    fn stat(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectStat>, StoreError> {
        immediate_error(self.take_fault(StoreOperation::Stat)?)?;
        self.inner.stat(bucket, object)
    }

    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        let effect = immediate_error(self.take_fault(StoreOperation::PutImmutable)?)?;
        if effect == Some(FaultEffect::CommitPutThenNetwork) {
            self.inner.put_immutable(bucket, object, bytes)?;
            return Err(StoreError::Network);
        }
        self.inner.put_immutable(bucket, object, bytes)
    }

    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError> {
        let effect = immediate_error(self.take_fault(StoreOperation::List)?)?;
        let mut page = self.inner.list(bucket, cursor, limit)?;
        match effect {
            Some(FaultEffect::OmitLastListEntry) => {
                page.entries.pop();
            }
            Some(FaultEffect::DuplicateFirstListEntry) => {
                if let Some(first) = page.entries.first().cloned() {
                    page.entries.insert(0, first);
                }
            }
            _ => {}
        }
        Ok(page)
    }

    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError> {
        immediate_error(self.take_fault(StoreOperation::DeleteUnreferenced)?)?;
        self.inner.delete_unreferenced(bucket, object)
    }

    fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
        immediate_error(self.take_fault(StoreOperation::Changes)?)?;
        self.inner.changes(cursor)
    }
}

/// Per-mirror outcome counters kept by [`ReplicaSetObjectStore`].
///
/// A mirror's own error type is retained rather than collapsed to a bool so a
/// caller (`storage check`, VLT-PM00 §23 item 14) can distinguish "briefly
/// rate-limited" from "storage was reconfigured out from under us" without
/// the decorator inventing a second error taxonomy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicaHealth {
    /// Total mirror operations attempted since construction.
    pub attempted: u64,
    /// Of those, the number that returned success.
    pub succeeded: u64,
    /// The most recent failure, or `None` if the last attempt succeeded or
    /// none has been made yet.
    pub last_error: Option<StoreError>,
}

impl ReplicaHealth {
    /// Whether the most recent attempt against this mirror failed.
    pub const fn is_degraded(&self) -> bool {
        self.last_error.is_some()
    }

    fn record(&mut self, result: &Result<(), StoreError>) {
        self.attempted = self.attempted.saturating_add(1);
        match result {
            Ok(()) => {
                self.succeeded = self.succeeded.saturating_add(1);
                self.last_error = None;
            }
            Err(error) => self.last_error = Some(error.clone()),
        }
    }
}

/// VLT-PM00 §11.5's `ReplicaSetObjectStore` decorator.
///
/// Wraps one authoritative primary store and zero or more mirror stores that
/// receive the same immutable bytes. Two invariants this type exists to
/// enforce, both straight from §19.2:
///
/// - **a local commit succeeds independently of remote availability** — every
///   mutating call is answered from the primary alone; mirror writes happen
///   *after* the primary call returns, and a mirror failure is recorded, not
///   propagated; and
/// - **replicas receive identical ciphertext objects** — mirrors are only
///   ever given the exact bytes the primary just accepted, never a
///   caller-chosen alternative, so a mirror cannot silently drift from the
///   authoritative copy this decorator itself controls.
///
/// With zero configured mirrors this type is a transparent pass-through to
/// the primary — `ReplicaSetObjectStore::single` is the ordinary
/// no-replication construction used everywhere this crate's callers do not
/// need §19.2 behavior.
///
/// **Deferred** (VLT-PM00 §23 item 14): the explicit `sync --wait` ceremony
/// with a configurable `one`/`all`/quorum durability target, and treating a
/// change feed rather than write-time propagation as the source of replica
/// truth. What ships here is the write-time propagation and health
/// accounting those richer features would be built on top of.
pub struct ReplicaSetObjectStore<P, M = P> {
    primary: P,
    mirrors: Vec<M>,
    health: Mutex<Vec<ReplicaHealth>>,
}

impl<P, M> ReplicaSetObjectStore<P, M> {
    /// Wrap a primary with no mirrors — behaviorally identical to using `P`
    /// directly.
    pub fn single(primary: P) -> Self {
        Self::new(primary, Vec::new())
    }

    /// Wrap a primary with an ordered, fixed set of mirrors.
    pub fn new(primary: P, mirrors: Vec<M>) -> Self {
        let health = mirrors.iter().map(|_| ReplicaHealth::default()).collect();
        Self {
            primary,
            mirrors,
            health: Mutex::new(health),
        }
    }

    /// Borrow the authoritative primary store.
    pub const fn primary(&self) -> &P {
        &self.primary
    }

    /// Borrow the configured mirrors in publication order.
    pub fn mirrors(&self) -> &[M] {
        &self.mirrors
    }

    /// Return the number of configured mirrors.
    pub fn mirror_count(&self) -> usize {
        self.mirrors.len()
    }

    /// Snapshot each mirror's health in the same order as [`Self::mirrors`].
    ///
    /// `storage check` (VLT-PM00 §23 item 14) reports a replica as degraded
    /// exactly when `is_degraded()` is true here — a real, observed failure,
    /// never a guess based on elapsed time.
    pub fn replica_health(&self) -> Vec<ReplicaHealth> {
        self.health
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn record(&self, index: usize, result: &Result<(), StoreError>) {
        if let Ok(mut health) = self.health.lock() {
            if let Some(entry) = health.get_mut(index) {
                entry.record(result);
            }
        }
    }
}

impl<P, M> Debug for ReplicaSetObjectStore<P, M> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplicaSetObjectStore")
            .field("mirror_count", &self.mirrors.len())
            .finish()
    }
}

impl<P: VaultObjectStore, M: VaultObjectStore> VaultObjectStore for ReplicaSetObjectStore<P, M> {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        self.primary.initialize(locator)?;
        for (index, mirror) in self.mirrors.iter().enumerate() {
            let result = mirror.initialize(locator);
            self.record(index, &result);
        }
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.primary.capabilities()
    }

    fn get(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectBytes>, StoreError> {
        match self.primary.get(bucket, object) {
            Ok(Some(bytes)) => Ok(Some(bytes)),
            Ok(None) => Ok(self.read_fallback(bucket, object)),
            Err(primary_error) => match self.read_fallback(bucket, object) {
                Some(bytes) => Ok(Some(bytes)),
                None => Err(primary_error),
            },
        }
    }

    fn stat(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectStat>, StoreError> {
        self.primary.stat(bucket, object)
    }

    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        let outcome = self.primary.put_immutable(bucket, object, bytes)?;
        for (index, mirror) in self.mirrors.iter().enumerate() {
            let result = mirror.put_immutable(bucket, object, bytes).map(|_| ());
            self.record(index, &result);
        }
        Ok(outcome)
    }

    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError> {
        self.primary.list(bucket, cursor, limit)
    }

    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError> {
        // Deliberately primary-only. Propagating physical delete to mirrors
        // is deferred alongside VLT-PM00 §19.4's replica-aware GC: a mirror
        // that independently loses a still-referenced object before every
        // device has observed the pruning checkpoint would violate that
        // section's own retention rule, and getting that ordering right
        // needs the GC planner, not this decorator, to own it.
        self.primary.delete_unreferenced(bucket, object)
    }

    fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
        self.primary.changes(cursor)
    }
}

impl<P: VaultObjectStore, M: VaultObjectStore> ReplicaSetObjectStore<P, M> {
    /// Try every mirror in order after the primary reported the object
    /// missing or unavailable. VLT-PM00 §19.2: "read fallback verifies all
    /// bytes" — the bytes returned here still pass through the same
    /// content-addressed and AEAD verification every other object does one
    /// layer up, so no extra verification duty belongs in a store that is
    /// deliberately blind to what it stores.
    fn read_fallback(&self, bucket: &BucketId, object: &ObjectId) -> Option<ObjectBytes> {
        for mirror in &self.mirrors {
            if let Ok(Some(bytes)) = mirror.get(bucket, object) {
                return Some(bytes);
            }
        }
        None
    }
}

/// Summary returned by a successful adapter conformance run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceReport {
    pub checks: usize,
    pub capabilities: BackendCapabilities,
}

/// A static step label plus the typed error that failed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceFailure {
    pub step: &'static str,
    pub error: Option<StoreError>,
}

impl Display for ConformanceFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "vault-pm-storage conformance failed at {}",
            self.step
        )
    }
}

impl std::error::Error for ConformanceFailure {}

fn failed(step: &'static str) -> ConformanceFailure {
    ConformanceFailure { step, error: None }
}

fn failed_with(step: &'static str, error: StoreError) -> ConformanceFailure {
    ConformanceFailure {
        step,
        error: Some(error),
    }
}

fn expect<T>(result: Result<T, StoreError>, step: &'static str) -> Result<T, ConformanceFailure> {
    result.map_err(|error| failed_with(step, error))
}

/// Exercise the public baseline contract against a newly constructed adapter.
///
/// Optional behavior is checked against the adapter's own capability report.
/// Provider packages call this function from their integration tests.
pub fn run_conformance_suite<S, F>(factory: F) -> Result<ConformanceReport, ConformanceFailure>
where
    S: VaultObjectStore,
    F: FnOnce() -> S,
{
    let store = factory();
    let locator = VaultLocator::new([0x11; 32]);
    let other_locator = VaultLocator::new([0x12; 32]);
    let bucket = BucketId::new([0x21; 32]);
    let other_bucket = BucketId::new([0x22; 32]);
    let absent = ObjectId::new([0x31; 32]);
    let first_body = ObjectBytes::new(b"vault-pm-storage-fixture".to_vec())
        .map_err(|error| failed_with("fixture body", error))?;
    let conflicting_body = ObjectBytes::new(b"different".to_vec())
        .map_err(|error| failed_with("conflict body", error))?;
    let mut checks = 0;

    if store.get(&bucket, &absent) != Err(StoreError::NotInitialized)
        || store.stat(&bucket, &absent) != Err(StoreError::NotInitialized)
        || store.put_immutable(&bucket, &absent, &first_body) != Err(StoreError::NotInitialized)
        || store.list(&bucket, None, 0) != Err(StoreError::NotInitialized)
        || store.delete_unreferenced(&bucket, &absent) != Err(StoreError::NotInitialized)
        || store.changes(None) != Err(StoreError::NotInitialized)
    {
        return Err(failed("pre-initialization operations"));
    }
    checks += 6;
    expect(store.initialize(&locator), "initialize")?;
    expect(store.initialize(&locator), "idempotent initialize")?;
    if store.initialize(&other_locator) != Err(StoreError::Conflict) {
        return Err(failed("locator binding"));
    }
    checks += 3;

    if expect(store.get(&bucket, &absent), "absent get")?.is_some()
        || expect(store.stat(&bucket, &absent), "absent stat")?.is_some()
    {
        return Err(failed("absent point operations"));
    }
    checks += 2;

    if expect(
        store.put_immutable(&bucket, &absent, &first_body),
        "first immutable put",
    )? != PutImmutableOutcome::Created
    {
        return Err(failed("created outcome"));
    }
    if expect(
        store.put_immutable(&bucket, &absent, &first_body),
        "idempotent immutable put",
    )? != PutImmutableOutcome::AlreadyPresent
    {
        return Err(failed("already-present outcome"));
    }
    if store.put_immutable(&bucket, &absent, &conflicting_body) != Err(StoreError::Corruption) {
        return Err(failed("immutable conflict"));
    }
    checks += 3;

    if expect(store.get(&bucket, &absent), "exact get")? != Some(first_body.clone()) {
        return Err(failed("exact body"));
    }
    let stat = expect(store.stat(&bucket, &absent), "exact stat")?
        .ok_or_else(|| failed("present stat"))?;
    if stat.body_len != first_body.len() as u64 {
        return Err(failed("stat length"));
    }
    checks += 2;

    let mut expected = BTreeSet::from([absent]);
    for byte in [0x01_u8, 0x20, 0x40, 0x80, 0xf0] {
        let object = ObjectId::new([byte; 32]);
        let body = ObjectBytes::new(vec![byte]).map_err(|error| failed_with("page body", error))?;
        expect(
            store.put_immutable(&bucket, &object, &body),
            "page fixture put",
        )?;
        expected.insert(object);
    }

    let first_page = expect(store.list(&bucket, None, 2), "first list page")?;
    let cross_bucket_cursor = first_page
        .next_cursor
        .clone()
        .ok_or_else(|| failed("first continuation"))?;
    if store.list(&other_bucket, Some(&cross_bucket_cursor), 2)
        != Err(StoreError::InvalidInput(InputViolation::CursorScope))
    {
        return Err(failed("cursor bucket scope"));
    }
    if store.list(&bucket, None, 0) != Err(StoreError::InvalidInput(InputViolation::ListLimit))
        || store.list(&bucket, None, MAX_LIST_LIMIT + 1)
            != Err(StoreError::InvalidInput(InputViolation::ListLimit))
    {
        return Err(failed("list bounds"));
    }
    checks += 3;

    let mut observed = Vec::new();
    let mut cursor = None;
    loop {
        let page = expect(store.list(&bucket, cursor.as_ref(), 2), "paginated list")?;
        observed.extend(page.entries.into_iter().map(|entry| entry.object));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    let observed_set = observed.iter().copied().collect::<BTreeSet<_>>();
    if observed.windows(2).any(|pair| pair[0] >= pair[1])
        || observed_set.len() != observed.len()
        || observed_set != expected
    {
        return Err(failed("ordered exhaustive pagination"));
    }
    checks += 1;

    let capabilities = store.capabilities();
    let delete_result = store.delete_unreferenced(&bucket, &absent);
    if capabilities.physical_delete {
        if expect(delete_result, "supported delete")? != DeleteOutcome::Deleted
            || expect(
                store.delete_unreferenced(&bucket, &absent),
                "idempotent delete",
            )? != DeleteOutcome::Missing
        {
            return Err(failed("delete outcomes"));
        }
    } else if delete_result != Err(StoreError::Unsupported) {
        return Err(failed("unsupported delete"));
    }
    checks += 1;

    if capabilities.change_feed {
        let changes = expect(store.changes(None), "change feed")?;
        let page = changes.ok_or_else(|| failed("advertised change feed"))?;
        if page
            .events
            .windows(2)
            .any(|pair| pair[0].sequence >= pair[1].sequence)
        {
            return Err(failed("change ordering"));
        }
        expect(store.changes(Some(&page.cursor)), "change resume")?;
    } else if store.changes(None) != Err(StoreError::Unsupported) {
        return Err(failed("unsupported change feed"));
    }
    checks += 2;

    let debug = format!("{locator:?} {bucket:?} {absent:?} {first_body:?}");
    if debug.contains("17, 17")
        || debug.contains("33, 33")
        || debug.contains("49, 49")
        || debug.contains("vault-pm-storage-fixture")
    {
        return Err(failed("redacted debug"));
    }
    checks += 1;

    Ok(ConformanceReport {
        checks,
        capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialized() -> InMemoryObjectStore {
        let store = InMemoryObjectStore::new();
        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        store
    }

    #[test]
    fn in_memory_model_passes_shared_conformance() {
        let report = run_conformance_suite(InMemoryObjectStore::new).unwrap();
        assert_eq!(report.checks, 24);
        assert!(report.capabilities.change_feed);
    }

    #[test]
    fn shared_fixture_is_embedded_and_versioned() {
        assert!(CONFORMANCE_FIXTURE_V1.contains("VLT-PM-STORAGE-CONFORMANCE"));
        assert!(CONFORMANCE_FIXTURE_V1.contains("\"version\": 1"));
        assert!(CONFORMANCE_FIXTURE_V1.contains("CommitPutThenNetwork"));
        assert!(CONFORMANCE_FIXTURE_V1.len() < 16 * 1024);
    }

    #[test]
    fn baseline_optional_capabilities_pass_shared_conformance() {
        let report = run_conformance_suite(|| {
            InMemoryObjectStore::with_capabilities(BackendCapabilities::baseline())
        })
        .unwrap();
        assert!(!report.capabilities.change_feed);
        assert!(!report.capabilities.physical_delete);
    }

    #[test]
    fn format_object_ids_round_trip_without_exposing_bytes() {
        let format = FormatObjectId::new([0xa5; 32]);
        let storage = ObjectId::from(format);
        assert_eq!(FormatObjectId::from(storage), format);
        assert_eq!(format!("{storage:?}"), "ObjectId(<redacted>)");
    }

    #[test]
    fn value_bounds_and_debug_redaction_are_enforced() {
        assert_eq!(
            ListCursor::new(vec![]),
            Err(StoreError::InvalidInput(InputViolation::CursorMalformed))
        );
        assert_eq!(
            ChangeCursor::new(vec![0; MAX_CURSOR_BYTES + 1]),
            Err(StoreError::InvalidInput(InputViolation::CursorTooLarge))
        );
        assert_eq!(
            ProviderRevision::new("line\nbreak"),
            Err(StoreError::InvalidInput(InputViolation::RevisionMalformed))
        );
        let secret = ObjectBytes::new(b"do-not-print-me".to_vec()).unwrap();
        assert!(!format!("{secret:?}").contains("do-not-print-me"));
        assert_eq!(
            StoreError::Corruption.to_string(),
            "vault-pm-storage: immutable storage corruption detected"
        );
    }

    #[test]
    fn public_value_helpers_are_lossless_and_redacted() {
        let locator = VaultLocator::new([7; 32]);
        assert_eq!(*locator.as_bytes(), [7; 32]);
        assert_eq!(locator.into_bytes(), [7; 32]);
        let bucket = BucketId::new([8; 32]);
        assert_eq!(*bucket.as_bytes(), [8; 32]);
        assert_eq!(bucket.into_bytes(), [8; 32]);
        let object = ObjectId::new([9; 32]);
        assert_eq!(*object.as_bytes(), [9; 32]);
        assert_eq!(object.into_bytes(), [9; 32]);

        let empty = ObjectBytes::new(Vec::new()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.into_vec(), Vec::<u8>::new());

        let list = ListCursor::new(vec![1, 2, 3]).unwrap();
        assert_eq!(list.as_bytes(), &[1, 2, 3]);
        assert!(!format!("{list:?}").contains("1, 2, 3"));
        let change = ChangeCursor::new(vec![4, 5, 6]).unwrap();
        assert_eq!(change.as_bytes(), &[4, 5, 6]);
        assert!(!format!("{change:?}").contains("4, 5, 6"));

        let revision = ProviderRevision::new("provider-etag").unwrap();
        assert_eq!(revision.as_str(), "provider-etag");
        assert_eq!(format!("{revision:?}"), "ProviderRevision(<redacted>)");
        assert_eq!(
            ProviderRevision::new(""),
            Err(StoreError::InvalidInput(InputViolation::RevisionMalformed))
        );
        assert_eq!(
            ProviderRevision::new("x".repeat(MAX_REVISION_BYTES + 1)),
            Err(StoreError::InvalidInput(InputViolation::RevisionMalformed))
        );

        assert_eq!(
            InMemoryObjectStore::default().capabilities(),
            BackendCapabilities::in_memory()
        );
    }

    #[test]
    fn every_error_variant_has_stable_redacted_formatting() {
        let errors = [
            StoreError::InvalidInput(InputViolation::ListLimit),
            StoreError::NotInitialized,
            StoreError::Authorization,
            StoreError::Quota,
            StoreError::RateLimited {
                retry_after_ms: Some(250),
            },
            StoreError::Network,
            StoreError::Corruption,
            StoreError::Conflict,
            StoreError::Unsupported,
            StoreError::Provider,
        ];
        for error in errors {
            let debug = format!("{error:?}");
            let display = error.to_string();
            assert!(!debug.is_empty());
            assert!(display.starts_with("vault-pm-storage: "));
        }
        assert_eq!(
            format!(
                "{:?}",
                StoreError::RateLimited {
                    retry_after_ms: None
                }
            ),
            "RateLimited { retry_after_ms: None }"
        );
    }

    #[test]
    fn provider_size_cap_is_enforced_without_mutation() {
        let mut capabilities = BackendCapabilities::in_memory();
        capabilities.max_object_size = Some(2);
        let store = InMemoryObjectStore::with_capabilities(capabilities);
        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        let result = store.put_immutable(
            &BucketId::new([2; 32]),
            &ObjectId::new([3; 32]),
            &ObjectBytes::new(vec![1, 2, 3]).unwrap(),
        );
        assert_eq!(
            result,
            Err(StoreError::InvalidInput(InputViolation::ObjectTooLarge))
        );
        assert!(store
            .list(&BucketId::new([2; 32]), None, 10)
            .unwrap()
            .entries
            .is_empty());
    }

    #[test]
    fn cursor_encoding_rejects_tampering_and_future_change_positions() {
        let store = initialized();
        let bucket = BucketId::new([2; 32]);
        for byte in 1..=2 {
            store
                .put_immutable(
                    &bucket,
                    &ObjectId::new([byte; 32]),
                    &ObjectBytes::new(vec![byte]).unwrap(),
                )
                .unwrap();
        }
        let cursor = store.list(&bucket, None, 1).unwrap().next_cursor.unwrap();
        let mut malformed = cursor.as_bytes().to_vec();
        malformed[0] = 99;
        let malformed = ListCursor::new(malformed).unwrap();
        assert_eq!(
            store.list(&bucket, Some(&malformed), 1),
            Err(StoreError::InvalidInput(InputViolation::CursorMalformed))
        );

        let future = encode_change_cursor(&VaultLocator::new([1; 32]), 99);
        assert_eq!(
            store.changes(Some(&future)),
            Err(StoreError::InvalidInput(InputViolation::CursorMalformed))
        );
        let malformed_change = ChangeCursor::new(vec![99]).unwrap();
        assert_eq!(
            store.changes(Some(&malformed_change)),
            Err(StoreError::InvalidInput(InputViolation::CursorMalformed))
        );

        let other_store_cursor = encode_change_cursor(&VaultLocator::new([9; 32]), 0);
        assert_eq!(
            store.changes(Some(&other_store_cursor)),
            Err(StoreError::InvalidInput(InputViolation::CursorScope))
        );
    }

    #[test]
    fn deterministic_generated_set_lists_in_sorted_pages() {
        let store = initialized();
        let bucket = BucketId::new([8; 32]);
        let mut state = 0x9e37_79b9_u32;
        let mut expected = BTreeSet::new();
        for _ in 0..257 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let mut id = [0_u8; 32];
            for chunk in id.chunks_exact_mut(4) {
                chunk.copy_from_slice(&state.to_be_bytes());
                state = state.rotate_left(7).wrapping_add(0x7f4a_7c15);
            }
            let object = ObjectId::new(id);
            let body = ObjectBytes::new(state.to_be_bytes().to_vec()).unwrap();
            let expected_outcome = if expected.insert(object) {
                PutImmutableOutcome::Created
            } else {
                PutImmutableOutcome::AlreadyPresent
            };
            assert_eq!(
                store.put_immutable(&bucket, &object, &body).unwrap(),
                expected_outcome
            );
        }

        for limit in [1, 2, 17, MAX_LIST_LIMIT] {
            let mut cursor = None;
            let mut actual = Vec::new();
            loop {
                let page = store.list(&bucket, cursor.as_ref(), limit).unwrap();
                actual.extend(page.entries.into_iter().map(|entry| entry.object));
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            assert_eq!(actual, expected.iter().copied().collect::<Vec<_>>());
        }
    }

    #[test]
    fn queued_return_fault_is_one_shot_and_operation_scoped() {
        let faulty = FaultInjectingObjectStore::new(InMemoryObjectStore::new());
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::Return(StoreError::Authorization),
            })
            .unwrap();
        faulty.initialize(&VaultLocator::new([1; 32])).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        assert_eq!(faulty.get(&bucket, &object), Err(StoreError::Authorization));
        assert_eq!(faulty.get(&bucket, &object), Ok(None));
        assert_eq!(faulty.pending_faults().unwrap(), 0);
    }

    #[test]
    fn invalid_fault_pair_is_rejected() {
        let faulty = FaultInjectingObjectStore::new(InMemoryObjectStore::new());
        assert_eq!(
            faulty.enqueue(FaultAction {
                operation: StoreOperation::Stat,
                effect: FaultEffect::CorruptGet,
            }),
            Err(StoreError::InvalidInput(
                InputViolation::FaultOperationMismatch
            ))
        );
    }

    #[test]
    fn corrupt_get_does_not_mutate_inner_store() {
        let faulty = FaultInjectingObjectStore::new(initialized());
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"ciphertext".to_vec()).unwrap();
        faulty.put_immutable(&bucket, &object, &body).unwrap();
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::CorruptGet,
            })
            .unwrap();
        assert_ne!(faulty.get(&bucket, &object).unwrap(), Some(body.clone()));
        assert_eq!(faulty.get(&bucket, &object).unwrap(), Some(body));
    }

    #[test]
    fn ambiguous_committed_put_converges_on_retry() {
        let faulty = FaultInjectingObjectStore::new(initialized());
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"ciphertext".to_vec()).unwrap();
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();
        assert_eq!(
            faulty.put_immutable(&bucket, &object, &body),
            Err(StoreError::Network)
        );
        assert_eq!(
            faulty.put_immutable(&bucket, &object, &body).unwrap(),
            PutImmutableOutcome::AlreadyPresent
        );
    }

    #[test]
    fn stale_and_duplicate_list_faults_are_explicitly_adversarial() {
        let faulty = FaultInjectingObjectStore::new(initialized());
        let bucket = BucketId::new([2; 32]);
        for byte in 1..=2 {
            faulty
                .put_immutable(
                    &bucket,
                    &ObjectId::new([byte; 32]),
                    &ObjectBytes::new(vec![byte]).unwrap(),
                )
                .unwrap();
        }
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::List,
                effect: FaultEffect::OmitLastListEntry,
            })
            .unwrap();
        assert_eq!(faulty.list(&bucket, None, 10).unwrap().entries.len(), 1);
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::List,
                effect: FaultEffect::DuplicateFirstListEntry,
            })
            .unwrap();
        let duplicate = faulty.list(&bucket, None, 10).unwrap();
        assert_eq!(duplicate.entries.len(), 3);
        assert_eq!(duplicate.entries[0].object, duplicate.entries[1].object);
        assert_eq!(faulty.list(&bucket, None, 10).unwrap().entries.len(), 2);
    }

    #[test]
    fn fault_wrapper_delegates_every_unfaulted_operation() {
        let faulty = FaultInjectingObjectStore::new(InMemoryObjectStore::new());
        assert_eq!(faulty.capabilities(), BackendCapabilities::in_memory());
        let locator = VaultLocator::new([1; 32]);
        faulty.initialize(&locator).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(vec![4]).unwrap();
        faulty.put_immutable(&bucket, &object, &body).unwrap();
        assert!(faulty.stat(&bucket, &object).unwrap().is_some());
        assert!(faulty.changes(None).unwrap().is_some());
        assert_eq!(
            faulty.delete_unreferenced(&bucket, &object).unwrap(),
            DeleteOutcome::Deleted
        );
        assert_eq!(faulty.inner().get(&bucket, &object).unwrap(), None);
        let inner = faulty.into_inner();
        assert_eq!(inner.capabilities(), BackendCapabilities::in_memory());
    }

    #[test]
    fn empty_body_corruption_and_empty_list_duplication_are_deterministic() {
        let faulty = FaultInjectingObjectStore::new(initialized());
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let empty = ObjectBytes::new(Vec::new()).unwrap();
        faulty.put_immutable(&bucket, &object, &empty).unwrap();
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::CorruptGet,
            })
            .unwrap();
        assert_eq!(
            faulty.get(&bucket, &object).unwrap().unwrap().into_vec(),
            vec![0x80]
        );

        let empty_bucket = BucketId::new([9; 32]);
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::List,
                effect: FaultEffect::DuplicateFirstListEntry,
            })
            .unwrap();
        assert!(faulty
            .list(&empty_bucket, None, 1)
            .unwrap()
            .entries
            .is_empty());
    }

    #[test]
    fn conformance_failures_are_static_and_redacted() {
        let plain = failed("example step");
        assert_eq!(plain.error, None);
        assert_eq!(
            plain.to_string(),
            "vault-pm-storage conformance failed at example step"
        );
        let typed = failed_with("provider step", StoreError::Provider);
        assert_eq!(typed.error, Some(StoreError::Provider));
        assert!(!typed.to_string().contains("Provider"));
    }

    #[test]
    fn single_mirror_free_replica_set_is_a_transparent_passthrough() {
        let report = run_conformance_suite(|| {
            ReplicaSetObjectStore::<InMemoryObjectStore>::single(InMemoryObjectStore::new())
        })
        .unwrap();
        assert_eq!(report.checks, 24);
        assert!(report.capabilities.change_feed);
    }

    #[test]
    fn mirrored_replica_set_passes_shared_conformance_reading_through_primary() {
        let report = run_conformance_suite(|| {
            ReplicaSetObjectStore::new(
                InMemoryObjectStore::new(),
                vec![InMemoryObjectStore::new(), InMemoryObjectStore::new()],
            )
        })
        .unwrap();
        assert_eq!(report.checks, 24);
    }

    #[test]
    fn put_propagates_identical_bytes_to_every_mirror() {
        let replicas = ReplicaSetObjectStore::new(
            InMemoryObjectStore::new(),
            vec![InMemoryObjectStore::new(), InMemoryObjectStore::new()],
        );
        let locator = VaultLocator::new([1; 32]);
        replicas.initialize(&locator).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"ciphertext".to_vec()).unwrap();
        assert_eq!(
            replicas.put_immutable(&bucket, &object, &body).unwrap(),
            PutImmutableOutcome::Created
        );
        for mirror in replicas.mirrors() {
            assert_eq!(mirror.get(&bucket, &object).unwrap(), Some(body.clone()));
        }
        for health in replicas.replica_health() {
            assert_eq!(health.attempted, 2); // one initialize, one put
            assert_eq!(health.succeeded, 2);
            assert!(!health.is_degraded());
        }
    }

    #[test]
    fn primary_commit_succeeds_and_records_degraded_health_when_a_mirror_is_unreachable() {
        let broken_mirror = FaultInjectingObjectStore::new(InMemoryObjectStore::new());
        broken_mirror
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::Return(StoreError::Network),
            })
            .unwrap();
        let replicas = ReplicaSetObjectStore::new(InMemoryObjectStore::new(), vec![broken_mirror]);
        let locator = VaultLocator::new([1; 32]);
        replicas.initialize(&locator).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"ciphertext".to_vec()).unwrap();

        // The primary commit succeeds even though the mirror is about to
        // fail -- VLT-PM00 §19.2's "a local commit succeeds independently of
        // remote availability".
        assert_eq!(
            replicas.put_immutable(&bucket, &object, &body).unwrap(),
            PutImmutableOutcome::Created
        );
        let health = replicas.replica_health();
        assert_eq!(health.len(), 1);
        assert!(health[0].is_degraded());
        assert_eq!(health[0].last_error, Some(StoreError::Network));
        assert_eq!(health[0].attempted, 2);
        assert_eq!(health[0].succeeded, 1); // the earlier initialize

        // A later successful mirror write clears the degraded flag.
        let second_object = ObjectId::new([4; 32]);
        replicas
            .put_immutable(&bucket, &second_object, &body)
            .unwrap();
        let recovered = replicas.replica_health();
        assert!(!recovered[0].is_degraded());
        assert_eq!(recovered[0].attempted, 3);
        assert_eq!(recovered[0].succeeded, 2);
    }

    #[test]
    fn get_falls_back_to_a_mirror_when_the_primary_is_missing_or_unavailable() {
        let primary = InMemoryObjectStore::new();
        let mirror = InMemoryObjectStore::new();
        let locator = VaultLocator::new([1; 32]);
        primary.initialize(&locator).unwrap();
        mirror.initialize(&locator).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"mirror-only".to_vec()).unwrap();
        // The object exists only on the mirror -- e.g. a write that reached
        // the mirror before a primary that later lost the object entirely.
        mirror.put_immutable(&bucket, &object, &body).unwrap();

        let faulting_primary = FaultInjectingObjectStore::new(primary);
        faulting_primary.initialize(&locator).unwrap();
        let replicas = ReplicaSetObjectStore::new(faulting_primary, vec![mirror]);

        // Ordinary miss: primary answers `None`, fallback finds the mirror copy.
        assert_eq!(replicas.get(&bucket, &object).unwrap(), Some(body.clone()));

        // Primary itself errors (e.g. a removable drive briefly unplugged):
        // fallback still serves the mirror copy rather than surfacing the
        // primary's error when a good answer exists.
        replicas
            .primary()
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::Return(StoreError::Network),
            })
            .unwrap();
        assert_eq!(replicas.get(&bucket, &object).unwrap(), Some(body));

        // Neither store has this one: the primary's real error surfaces.
        let absent = ObjectId::new([9; 32]);
        replicas
            .primary()
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::Return(StoreError::Network),
            })
            .unwrap();
        assert_eq!(replicas.get(&bucket, &absent), Err(StoreError::Network));
    }

    #[test]
    fn delete_and_changes_and_stat_and_capabilities_delegate_to_primary_only() {
        let primary = InMemoryObjectStore::new();
        let mirror = InMemoryObjectStore::new();
        let locator = VaultLocator::new([1; 32]);
        let replicas = ReplicaSetObjectStore::new(primary, vec![mirror]);
        replicas.initialize(&locator).unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(vec![9]).unwrap();
        replicas.put_immutable(&bucket, &object, &body).unwrap();

        assert_eq!(replicas.capabilities(), BackendCapabilities::in_memory());
        assert!(replicas.stat(&bucket, &object).unwrap().is_some());
        assert!(replicas.changes(None).unwrap().is_some());
        assert_eq!(
            replicas.delete_unreferenced(&bucket, &object).unwrap(),
            DeleteOutcome::Deleted
        );
        // The delete never reached the mirror -- deliberately deferred to a
        // future replica-aware GC planner (VLT-PM00 §19.4).
        assert_eq!(
            replicas.mirrors()[0].get(&bucket, &object).unwrap(),
            Some(body)
        );
    }

    #[test]
    fn health_snapshot_is_empty_for_a_mirror_free_replica_set_and_debug_is_closed() {
        let replicas =
            ReplicaSetObjectStore::<InMemoryObjectStore>::single(InMemoryObjectStore::new());
        assert!(replicas.replica_health().is_empty());
        assert_eq!(replicas.mirror_count(), 0);
        let debug = format!("{replicas:?}");
        assert!(debug.contains("mirror_count: 0"));
    }

    #[test]
    fn primary_failure_still_fails_the_call_even_with_healthy_mirrors() {
        let faulting_primary = FaultInjectingObjectStore::new(InMemoryObjectStore::new());
        let replicas =
            ReplicaSetObjectStore::new(faulting_primary, vec![InMemoryObjectStore::new()]);
        replicas.initialize(&VaultLocator::new([1; 32])).unwrap();
        replicas
            .primary()
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::Return(StoreError::Quota),
            })
            .unwrap();
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(vec![1]).unwrap();
        assert_eq!(
            replicas.put_immutable(&bucket, &object, &body),
            Err(StoreError::Quota)
        );
        // The primary's own rejection means the mirror is never attempted.
        assert_eq!(replicas.replica_health()[0].attempted, 1); // only initialize
    }

    #[test]
    fn change_feed_resumes_after_watermark_and_records_deletes() {
        let store = initialized();
        let bucket = BucketId::new([2; 32]);
        let first = ObjectId::new([3; 32]);
        let second = ObjectId::new([4; 32]);
        let body = ObjectBytes::new(vec![5]).unwrap();
        store.put_immutable(&bucket, &first, &body).unwrap();
        let page = store.changes(None).unwrap().unwrap();
        assert_eq!(page.events.len(), 1);
        store.put_immutable(&bucket, &second, &body).unwrap();
        store.delete_unreferenced(&bucket, &first).unwrap();
        let resumed = store.changes(Some(&page.cursor)).unwrap().unwrap();
        assert_eq!(resumed.events.len(), 2);
        assert_eq!(resumed.events[0].kind, ChangeKind::Put);
        assert_eq!(resumed.events[1].kind, ChangeKind::Delete);
        assert!(resumed.events[0].sequence < resumed.events[1].sequence);
    }
}
