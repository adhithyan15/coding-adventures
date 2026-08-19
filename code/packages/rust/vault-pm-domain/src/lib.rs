//! VLT-PM03 pure password-manager domain model.
//!
//! This crate owns product identifiers, validated item documents, mergeable
//! metadata, no-loss conflict decisions, and default redacted host views. It
//! deliberately owns no clock, entropy, storage, transport, or cryptography.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_vault_records::{
    AnyRecord, VaultRecordKind, API_KEY_V1, CARD_V1, DATABASE_CREDENTIAL_V1, LOGIN_V1,
    SECURE_NOTE_V1, TOTP_SEED_V1,
};
use coding_adventures_zeroize::Zeroize;
use std::collections::{BTreeMap, BTreeSet};

/// Maximum bytes in a content-type identifier.
pub const MAX_CONTENT_TYPE_LEN: usize = 128;
/// Maximum present collection memberships per item.
pub const MAX_COLLECTIONS: usize = 64;
/// Maximum present tags per item.
pub const MAX_TAGS: usize = 64;
/// Maximum UTF-8 bytes in one tag.
pub const MAX_TAG_LEN: usize = 128;
/// Maximum present attachment memberships per item.
pub const MAX_ATTACHMENTS: usize = 64;
/// Maximum direct causal parents on one candidate.
pub const MAX_CAUSAL_PARENTS: usize = 16;
/// Maximum distinct values retained by one observed-remove set.
pub const MAX_OBSERVED_VALUES: usize = 256;
/// Maximum add-operation IDs retained by one observed-remove set.
pub const MAX_OBSERVED_ADD_OPERATIONS: usize = 1024;
/// Maximum removal tombstones retained by one observed-remove set.
pub const MAX_OBSERVED_TOMBSTONES: usize = 1024;

/// Closed, payload-free domain failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainError {
    /// An identifier was not canonical or had the wrong width.
    InvalidIdentifier,
    /// A content type was empty, oversized, or used forbidden bytes.
    InvalidContentType,
    /// An update, deletion, or resolution timestamp was inconsistent.
    InvalidTimestamp,
    /// A tag was empty, oversized, or contained a control character.
    InvalidTag,
    /// A bounded collection exceeded its V1 limit.
    BoundExceeded,
    /// Observed-set state reused an operation ID or referenced an unknown add.
    InvalidObservedSet,
    /// An item's declared schema did not match its VLT02 record.
    SchemaMismatch,
    /// Candidate item identities or immutable schemas disagreed.
    IdentityMismatch,
    /// Candidate relation or conflict resolution data was inconsistent.
    InvalidConflict,
    /// Attachment membership and attachment manifest references disagreed.
    ///
    /// VLT-PM47 §4.7. The two are one fact stored in two places, so every
    /// state in which they differ is a state with no meaning: a retained
    /// `AttachmentId` with no manifest is an attachment whose bytes cannot be
    /// found, and a manifest reference with no retained id is a pointer to
    /// bytes nothing claims. Merging two documents that name different
    /// manifests for one immutable attachment id is the same fault.
    AttachmentManifestMismatch,
}

impl core::fmt::Display for DomainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidIdentifier => "vault-pm-domain: invalid identifier",
            Self::InvalidContentType => "vault-pm-domain: invalid content type",
            Self::InvalidTimestamp => "vault-pm-domain: invalid timestamp",
            Self::InvalidTag => "vault-pm-domain: invalid tag",
            Self::BoundExceeded => "vault-pm-domain: bound exceeded",
            Self::InvalidObservedSet => "vault-pm-domain: invalid observed set",
            Self::SchemaMismatch => "vault-pm-domain: record schema mismatch",
            Self::IdentityMismatch => "vault-pm-domain: immutable identity mismatch",
            Self::InvalidConflict => "vault-pm-domain: invalid conflict operation",
            Self::AttachmentManifestMismatch => {
                "vault-pm-domain: attachment manifest reference mismatch"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for DomainError {}

const BASE32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn encode_base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity((bytes.len() * 8).div_ceil(5));
    let mut buffer = 0u16;
    let mut bits = 0usize;
    for &byte in bytes {
        buffer = ((buffer & ((1u16 << bits) - 1)) << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn decode_base32<const N: usize>(encoded: &str) -> Result<[u8; N], DomainError> {
    if encoded.len() != (N * 8).div_ceil(5) || !encoded.is_ascii() {
        return Err(DomainError::InvalidIdentifier);
    }
    let mut out = [0u8; N];
    let mut written = 0usize;
    let mut buffer = 0u16;
    let mut bits = 0usize;
    for byte in encoded.bytes() {
        let value = BASE32
            .iter()
            .position(|candidate| *candidate == byte)
            .ok_or(DomainError::InvalidIdentifier)? as u16;
        buffer = ((buffer & ((1u16 << bits) - 1)) << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out[written] = (buffer >> bits) as u8;
            written += 1;
        }
    }
    let residual_mask = (1u16 << bits) - 1;
    if written != N || buffer & residual_mask != 0 {
        return Err(DomainError::InvalidIdentifier);
    }
    Ok(out)
}

macro_rules! product_id {
    ($name:ident, $size:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $size]);

        impl $name {
            /// Construct an ID from its exact bytes.
            pub const fn new(bytes: [u8; $size]) -> Self {
                Self(bytes)
            }

            /// Borrow the exact bytes.
            pub const fn as_bytes(&self) -> &[u8; $size] {
                &self.0
            }

            /// Return the canonical user-boundary representation.
            pub fn to_user_string(self) -> String {
                encode_base32(&self.0)
            }

            /// Parse a strict canonical user-boundary representation.
            pub fn from_user_string(encoded: &str) -> Result<Self, DomainError> {
                decode_base32(encoded).map(Self)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, concat!(stringify!($name), "(<redacted>)"))
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(concat!("<redacted ", stringify!($name), ">"))
            }
        }
    };
}

product_id!(ItemId, 16, "Random 128-bit stable item identifier.");
product_id!(CollectionId, 16, "Random 128-bit collection identifier.");
product_id!(
    AttachmentId,
    16,
    "Random 128-bit immutable attachment identifier."
);
product_id!(
    AttachmentManifestId,
    32,
    "Opaque 256-bit identifier of one attachment's manifest object.\n\nVLT-PM47 §4.3. This is the manifest object's repository `ObjectId` — the\nSHA-256 of its sealed frame — carried at domain level so an item document\ncan say *where* each of its attachments is without the domain package\ndepending on the repository format."
);
product_id!(ConflictId, 16, "Random 128-bit stable conflict identifier.");
product_id!(RevisionId, 32, "Opaque 256-bit item revision identifier.");
product_id!(
    OperationId,
    32,
    "Globally unique 256-bit metadata operation identifier."
);

/// Validated VLT02 content-type identifier.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentType(String);

impl ContentType {
    /// Validate and construct a content type.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CONTENT_TYPE_LEN
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'.' | b'/' | b'+' | b'_' | b':')
            })
        {
            return Err(DomainError::InvalidContentType);
        }
        Ok(Self(value))
    }

    /// Explicitly borrow the content type for application behavior or rendering.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Debug for ContentType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ContentType(<redacted>)")
    }
}

impl core::fmt::Display for ContentType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted content type>")
    }
}

/// Observed-remove set whose additions are named by globally unique operations.
#[derive(Clone, PartialEq, Eq)]
pub struct ObservedSet<T: Ord> {
    adds: BTreeMap<T, BTreeSet<OperationId>>,
    removals: BTreeMap<T, BTreeSet<OperationId>>,
}

impl<T: Ord> Default for ObservedSet<T> {
    fn default() -> Self {
        Self {
            adds: BTreeMap::new(),
            removals: BTreeMap::new(),
        }
    }
}

impl<T: Ord + Clone> ObservedSet<T> {
    /// Construct an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe an idempotent add operation.
    pub fn add(&mut self, value: T, operation: OperationId) -> Result<bool, DomainError> {
        if self
            .adds
            .get(&value)
            .is_some_and(|operations| operations.contains(&operation))
        {
            return Ok(false);
        }
        if self
            .adds
            .iter()
            .any(|(candidate, operations)| candidate != &value && operations.contains(&operation))
        {
            return Err(DomainError::InvalidObservedSet);
        }
        if self.add_operation_count() == MAX_OBSERVED_ADD_OPERATIONS
            || (!self.adds.contains_key(&value)
                && self.retained_value_count() == MAX_OBSERVED_VALUES)
        {
            return Err(DomainError::BoundExceeded);
        }
        Ok(self.adds.entry(value).or_default().insert(operation))
    }

    /// Remove all add operations for `value` observed by this replica.
    pub fn remove(&mut self, value: &T) -> bool {
        let Some(observed) = self.adds.get(value) else {
            return false;
        };
        self.removals
            .entry(value.clone())
            .or_default()
            .extend(observed.iter().copied());
        true
    }

    /// Record one exact removal tombstone while reconstructing bounded state.
    ///
    /// The referenced add must already be present for the same value. This
    /// lets a persistent decoder rebuild mixed removed/re-added state without
    /// accepting dangling or cross-value operation IDs.
    pub fn observe_removal(
        &mut self,
        value: &T,
        operation: OperationId,
    ) -> Result<bool, DomainError> {
        if !self
            .adds
            .get(value)
            .is_some_and(|operations| operations.contains(&operation))
        {
            return Err(DomainError::InvalidObservedSet);
        }
        if self
            .removals
            .get(value)
            .is_some_and(|operations| operations.contains(&operation))
        {
            return Ok(false);
        }
        if self.tombstone_count() == MAX_OBSERVED_TOMBSTONES {
            return Err(DomainError::BoundExceeded);
        }
        Ok(self
            .removals
            .entry(value.clone())
            .or_default()
            .insert(operation))
    }

    /// Return whether at least one add remains unremoved.
    pub fn contains(&self, value: &T) -> bool {
        self.adds.get(value).is_some_and(|adds| {
            let removals = self.removals.get(value);
            adds.iter()
                .any(|operation| removals.is_none_or(|set| !set.contains(operation)))
        })
    }

    /// Return the number of currently present values.
    pub fn len(&self) -> usize {
        self.adds
            .keys()
            .filter(|value| self.contains(value))
            .count()
    }

    /// Return whether no value is currently present.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Borrow currently present values in deterministic order.
    pub fn values(&self) -> Vec<&T> {
        self.adds
            .keys()
            .filter(|value| self.contains(value))
            .collect()
    }

    /// Iterate every retained value, including values hidden by removals.
    ///
    /// Persistent codecs must use this together with
    /// [`Self::retained_add_operations`] and
    /// [`Self::retained_removal_operations`] instead of serializing only
    /// [`Self::values`]. Dropping removed observations can resurrect a value
    /// after a later merge.
    pub fn retained_values(&self) -> impl ExactSizeIterator<Item = &T> {
        self.adds.keys()
    }

    /// Iterate the retained add operations for one exact retained value.
    ///
    /// The iterator is deterministically ordered. An unknown value produces
    /// an empty iterator.
    pub fn retained_add_operations<'a>(
        &'a self,
        value: &T,
    ) -> impl Iterator<Item = OperationId> + 'a {
        self.adds
            .get(value)
            .into_iter()
            .flat_map(|operations| operations.iter().copied())
    }

    /// Iterate the retained removal tombstones for one exact retained value.
    ///
    /// The iterator is deterministically ordered. An unknown or never-removed
    /// value produces an empty iterator.
    pub fn retained_removal_operations<'a>(
        &'a self,
        value: &T,
    ) -> impl Iterator<Item = OperationId> + 'a {
        self.removals
            .get(value)
            .into_iter()
            .flat_map(|operations| operations.iter().copied())
    }

    /// Return the number of distinct values retained on wire, including absent values.
    pub fn retained_value_count(&self) -> usize {
        self.adds.len()
    }

    /// Return the number of retained add-operation IDs.
    pub fn add_operation_count(&self) -> usize {
        self.adds.values().map(BTreeSet::len).sum()
    }

    /// Return the number of retained removal tombstones.
    pub fn tombstone_count(&self) -> usize {
        self.removals.values().map(BTreeSet::len).sum()
    }

    /// Merge by unioning add observations and removal tombstones.
    pub fn merge(&self, other: &Self) -> Result<Self, DomainError> {
        let mut merged = self.clone();
        for (value, operations) in &other.adds {
            for operation in operations {
                merged.add(value.clone(), *operation)?;
            }
        }
        for (value, operations) in &other.removals {
            for operation in operations {
                merged.observe_removal(value, *operation)?;
            }
        }
        Ok(merged)
    }

    /// Compact removed add/tombstone pairs proven causally stable by the repository.
    ///
    /// The predicate may return `true` only when every retained head has
    /// observed the removal and no authorized publisher can later reintroduce
    /// the pre-removal add. Without that external proof, callers must retain
    /// the pair. The return value is the number of compacted operation IDs.
    pub fn compact_stable_removals(
        &mut self,
        mut is_causally_stable: impl FnMut(OperationId) -> bool,
    ) -> usize {
        let mut compacted = 0;
        let removals = &mut self.removals;
        self.adds.retain(|value, adds| {
            let Some(tombstones) = removals.get_mut(value) else {
                return true;
            };
            adds.retain(|operation| {
                if tombstones.contains(operation) && is_causally_stable(*operation) {
                    tombstones.remove(operation);
                    compacted += 1;
                    false
                } else {
                    true
                }
            });
            !adds.is_empty()
        });
        self.removals.retain(|_, tombstones| !tombstones.is_empty());
        compacted
    }
}

impl ObservedSet<String> {
    fn validate_tags(&self) -> Result<(), DomainError> {
        if self.len() > MAX_TAGS {
            return Err(DomainError::BoundExceeded);
        }
        if self.adds.keys().any(|tag| {
            tag.is_empty() || tag.len() > MAX_TAG_LEN || tag.chars().any(char::is_control)
        }) {
            return Err(DomainError::InvalidTag);
        }
        Ok(())
    }

    fn zeroize_values(&mut self) {
        for (mut value, _) in core::mem::take(&mut self.adds) {
            value.zeroize();
        }
        for (mut value, _) in core::mem::take(&mut self.removals) {
            value.zeroize();
        }
    }
}

impl<T: Ord + Clone> core::fmt::Debug for ObservedSet<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ObservedSet")
            .field("present_count", &self.len())
            .field("retained_value_count", &self.retained_value_count())
            .field("add_operation_count", &self.add_operation_count())
            .field("tombstone_count", &self.tombstone_count())
            .finish()
    }
}

/// Deterministic last-writer-wins register.
#[derive(Clone, PartialEq, Eq)]
pub struct LwwRegister<T> {
    value: T,
    updated_at_ms: u64,
    operation: OperationId,
}

impl<T> LwwRegister<T> {
    /// Construct a register from a caller timestamp and unique operation ID.
    pub const fn new(value: T, updated_at_ms: u64, operation: OperationId) -> Self {
        Self {
            value,
            updated_at_ms,
            operation,
        }
    }

    /// Borrow the selected value.
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Return the selected caller timestamp.
    pub const fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// Return the selected operation ID.
    pub const fn operation(&self) -> OperationId {
        self.operation
    }
}

impl<T: Clone> LwwRegister<T> {
    /// Deterministically select the greater timestamp and operation pair.
    pub fn merge(&self, other: &Self) -> Self {
        if (other.updated_at_ms, other.operation) > (self.updated_at_ms, self.operation) {
            other.clone()
        } else {
            self.clone()
        }
    }
}

impl<T> core::fmt::Debug for LwwRegister<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LwwRegister")
            .field("value", &"<redacted>")
            .field("updated_at_ms", &self.updated_at_ms)
            .field("operation", &"<redacted>")
            .finish()
    }
}

/// Validated password-manager item document.
#[derive(Clone, PartialEq, Eq)]
pub struct ItemDocument {
    id: ItemId,
    schema: ContentType,
    created_at_ms: u64,
    updated_at_ms: u64,
    favorite: LwwRegister<bool>,
    collection_ids: ObservedSet<CollectionId>,
    tags: ObservedSet<String>,
    payload: AnyRecord,
    attachments: ObservedSet<AttachmentId>,
    attachment_manifests: BTreeMap<AttachmentId, AttachmentManifestId>,
}

impl ItemDocument {
    /// Construct and validate a complete document.
    ///
    /// `attachment_manifests` must have exactly the key set of
    /// `attachments.retained_values()` — including values currently hidden by
    /// a removal tombstone, because a later merge can bring one back and a
    /// resurrected attachment whose manifest reference was dropped would name
    /// bytes nobody can find. See [`Self::validate`] and VLT-PM47 §4.7.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ItemId,
        schema: ContentType,
        created_at_ms: u64,
        updated_at_ms: u64,
        favorite: LwwRegister<bool>,
        collection_ids: ObservedSet<CollectionId>,
        tags: ObservedSet<String>,
        payload: AnyRecord,
        attachments: ObservedSet<AttachmentId>,
        attachment_manifests: BTreeMap<AttachmentId, AttachmentManifestId>,
    ) -> Result<Self, DomainError> {
        let document = Self {
            id,
            schema,
            created_at_ms,
            updated_at_ms,
            favorite,
            collection_ids,
            tags,
            payload,
            attachments,
            attachment_manifests,
        };
        document.validate()?;
        Ok(document)
    }

    /// Validate document bounds and immutable schema agreement.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.updated_at_ms < self.created_at_ms
            || self.favorite.updated_at_ms() < self.created_at_ms
            || self.favorite.updated_at_ms() > self.updated_at_ms
        {
            return Err(DomainError::InvalidTimestamp);
        }
        if self.collection_ids.len() > MAX_COLLECTIONS || self.attachments.len() > MAX_ATTACHMENTS {
            return Err(DomainError::BoundExceeded);
        }
        // Attachment membership and manifest references are one fact recorded
        // twice, so the only legal relation between them is equality. A
        // one-directional check would admit the half-existing attachment this
        // pair exists to make impossible, in whichever direction it omitted.
        if self.attachment_manifests.len() != self.attachments.retained_value_count()
            || !self
                .attachments
                .retained_values()
                .all(|id| self.attachment_manifests.contains_key(id))
        {
            return Err(DomainError::AttachmentManifestMismatch);
        }
        self.tags.validate_tags()?;
        if record_content_type(&self.payload) != self.schema.as_str() {
            return Err(DomainError::SchemaMismatch);
        }
        Ok(())
    }

    /// Return the stable item ID.
    pub const fn id(&self) -> ItemId {
        self.id
    }

    /// Borrow the validated record content type.
    pub const fn schema(&self) -> &ContentType {
        &self.schema
    }

    /// Return the creation timestamp.
    pub const fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// Return the latest document timestamp.
    pub const fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// Borrow the favorite register.
    pub const fn favorite(&self) -> &LwwRegister<bool> {
        &self.favorite
    }

    /// Borrow collection membership.
    pub const fn collection_ids(&self) -> &ObservedSet<CollectionId> {
        &self.collection_ids
    }

    /// Borrow tag membership.
    pub const fn tags(&self) -> &ObservedSet<String> {
        &self.tags
    }

    /// Explicitly borrow the secret-bearing VLT02 payload.
    pub const fn payload(&self) -> &AnyRecord {
        &self.payload
    }

    /// Borrow attachment membership.
    pub const fn attachments(&self) -> &ObservedSet<AttachmentId> {
        &self.attachments
    }

    /// Borrow the manifest object reference for every retained attachment.
    pub const fn attachment_manifests(&self) -> &BTreeMap<AttachmentId, AttachmentManifestId> {
        &self.attachment_manifests
    }
}

/// Union two documents' attachment manifest references.
///
/// An `AttachmentId` is a random 128-bit value drawn once, and the manifest it
/// names is an immutable content address, so two replicas that both know an id
/// necessarily know the same manifest. Two that disagree have not produced a
/// conflict a person could resolve — one of them is wrong — so this is a fault
/// and not a merge outcome.
fn merge_attachment_manifests(
    left: &BTreeMap<AttachmentId, AttachmentManifestId>,
    right: &BTreeMap<AttachmentId, AttachmentManifestId>,
) -> Result<BTreeMap<AttachmentId, AttachmentManifestId>, DomainError> {
    let mut merged = left.clone();
    for (id, manifest) in right {
        match merged.insert(*id, *manifest) {
            Some(existing) if existing != *manifest => {
                return Err(DomainError::AttachmentManifestMismatch)
            }
            _ => {}
        }
    }
    Ok(merged)
}

impl core::fmt::Debug for ItemDocument {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ItemDocument")
            .field("id", &"<redacted>")
            .field("schema", &"<redacted>")
            .field("created_at_ms", &self.created_at_ms)
            .field("updated_at_ms", &self.updated_at_ms)
            .field("favorite", &self.favorite)
            .field("collection_count", &self.collection_ids.len())
            .field("tag_count", &self.tags.len())
            .field("payload", &"<redacted>")
            .field("attachment_count", &self.attachments.len())
            .finish()
    }
}

impl Zeroize for ItemDocument {
    fn zeroize(&mut self) {
        self.payload.zeroize();
        self.tags.zeroize_values();
    }
}

impl Drop for ItemDocument {
    fn drop(&mut self) {
        self.zeroize();
    }
}

fn record_content_type(record: &AnyRecord) -> &str {
    match record {
        AnyRecord::Login(_) => LOGIN_V1,
        AnyRecord::SecureNote(_) => SECURE_NOTE_V1,
        AnyRecord::Card(_) => CARD_V1,
        AnyRecord::TotpSeed(_) => TOTP_SEED_V1,
        AnyRecord::ApiKey(_) => API_KEY_V1,
        AnyRecord::DatabaseCredential(_) => DATABASE_CREDENTIAL_V1,
        AnyRecord::Opaque { content_type, .. } => content_type,
        AnyRecord::Quarantined { content_type, .. } => content_type,
    }
}

/// Deleted item state retained as a causal tombstone.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tombstone {
    /// Stable deleted item identity.
    pub item_id: ItemId,
    /// Caller-provided deletion timestamp.
    pub deleted_at_ms: u64,
}

impl core::fmt::Debug for Tombstone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tombstone")
            .field("item_id", &"<redacted>")
            .field("deleted_at_ms", &self.deleted_at_ms)
            .finish()
    }
}

/// Live or deleted state of one item revision candidate.
#[derive(Clone, PartialEq, Eq)]
pub enum ItemState {
    /// Complete live document.
    Live(Box<ItemDocument>),
    /// Retained deletion marker.
    Tombstone(Tombstone),
}

impl ItemState {
    /// Return the stable item identity in either state.
    pub const fn item_id(&self) -> ItemId {
        match self {
            Self::Live(document) => document.id(),
            Self::Tombstone(tombstone) => tombstone.item_id,
        }
    }
}

impl core::fmt::Debug for ItemState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Live(_) => f.write_str("Live(<redacted>)"),
            Self::Tombstone(tombstone) => tombstone.fmt(f),
        }
    }
}

/// One repository revision candidate supplied to domain merge.
#[derive(Clone, PartialEq, Eq)]
pub struct ItemCandidate {
    revision_id: RevisionId,
    causal_parents: BTreeSet<RevisionId>,
    state: ItemState,
}

impl ItemCandidate {
    /// Construct a bounded candidate.
    pub fn new(
        revision_id: RevisionId,
        causal_parents: impl IntoIterator<Item = RevisionId>,
        state: ItemState,
    ) -> Result<Self, DomainError> {
        let causal_parents: BTreeSet<_> = causal_parents.into_iter().collect();
        if causal_parents.len() > MAX_CAUSAL_PARENTS {
            return Err(DomainError::BoundExceeded);
        }
        if causal_parents.contains(&revision_id) {
            return Err(DomainError::InvalidConflict);
        }
        Ok(Self {
            revision_id,
            causal_parents,
            state,
        })
    }

    /// Return this candidate's revision ID.
    pub const fn revision_id(&self) -> RevisionId {
        self.revision_id
    }

    /// Borrow direct causal parents in deterministic order.
    pub const fn causal_parents(&self) -> &BTreeSet<RevisionId> {
        &self.causal_parents
    }

    /// Borrow the live or deleted state.
    pub const fn state(&self) -> &ItemState {
        &self.state
    }

    /// Return the stable item ID.
    pub const fn item_id(&self) -> ItemId {
        self.state.item_id()
    }
}

impl core::fmt::Debug for ItemCandidate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ItemCandidate")
            .field("revision_id", &"<redacted>")
            .field("causal_parent_count", &self.causal_parents.len())
            .field("state", &self.state)
            .finish()
    }
}

/// Repository-proven relationship between two candidate revisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalRelation {
    /// Both arguments identify the same revision.
    Same,
    /// The left revision causally descends from the right.
    LeftDescends,
    /// The right revision causally descends from the left.
    RightDescends,
    /// Neither revision causally descends from the other.
    Concurrent,
}

/// Resolution state retained with both conflict candidates.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConflictState {
    /// No user-authored resolution revision exists.
    Unresolved,
    /// A later revision resolves the conflict while retaining both candidates.
    Resolved {
        /// New revision whose parents include both candidates.
        resolution_revision: RevisionId,
        /// Caller-provided resolution timestamp.
        resolved_at_ms: u64,
    },
}

impl core::fmt::Debug for ConflictState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unresolved => f.write_str("Unresolved"),
            Self::Resolved { resolved_at_ms, .. } => f
                .debug_struct("Resolved")
                .field("resolution_revision", &"<redacted>")
                .field("resolved_at_ms", resolved_at_ms)
                .finish(),
        }
    }
}

/// Stable no-loss item conflict containing both complete candidates.
#[derive(Clone, PartialEq, Eq)]
pub struct ItemConflict {
    id: ConflictId,
    item_id: ItemId,
    left: ItemCandidate,
    right: ItemCandidate,
    discovered_at_ms: u64,
    state: ConflictState,
}

impl ItemConflict {
    fn new(
        id: ConflictId,
        left: ItemCandidate,
        right: ItemCandidate,
        discovered_at_ms: u64,
    ) -> Result<Self, DomainError> {
        if left.item_id() != right.item_id() || left.revision_id() == right.revision_id() {
            return Err(DomainError::InvalidConflict);
        }
        Ok(Self {
            id,
            item_id: left.item_id(),
            left,
            right,
            discovered_at_ms,
            state: ConflictState::Unresolved,
        })
    }

    /// Return the stable conflict ID.
    pub const fn id(&self) -> ConflictId {
        self.id
    }

    /// Return the stable item ID.
    pub const fn item_id(&self) -> ItemId {
        self.item_id
    }

    /// Borrow the left candidate.
    pub const fn left(&self) -> &ItemCandidate {
        &self.left
    }

    /// Borrow the right candidate.
    pub const fn right(&self) -> &ItemCandidate {
        &self.right
    }

    /// Return when the conflict was discovered.
    pub const fn discovered_at_ms(&self) -> u64 {
        self.discovered_at_ms
    }

    /// Return the resolution state.
    pub const fn state(&self) -> ConflictState {
        self.state
    }

    /// Mark resolved without deleting either candidate.
    pub fn resolve(
        &mut self,
        resolution_revision: RevisionId,
        resolved_at_ms: u64,
    ) -> Result<(), DomainError> {
        if !matches!(self.state, ConflictState::Unresolved)
            || resolved_at_ms < self.discovered_at_ms
            || resolution_revision == self.left.revision_id()
            || resolution_revision == self.right.revision_id()
        {
            return Err(DomainError::InvalidConflict);
        }
        self.state = ConflictState::Resolved {
            resolution_revision,
            resolved_at_ms,
        };
        Ok(())
    }
}

impl core::fmt::Debug for ItemConflict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ItemConflict")
            .field("id", &"<redacted>")
            .field("item_id", &"<redacted>")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("discovered_at_ms", &self.discovered_at_ms)
            .field("state", &self.state)
            .finish()
    }
}

/// Pure outcome of merging two item candidates.
#[derive(Clone, PartialEq, Eq)]
pub enum MergeDecision {
    /// One existing revision is causally or deterministically selected.
    Selected(ItemCandidate),
    /// Concurrent metadata was merged into a caller-named revision.
    AutoMerged(ItemCandidate),
    /// Both complete candidates require user resolution.
    Conflict(Box<ItemConflict>),
}

impl core::fmt::Debug for MergeDecision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Selected(candidate) => f.debug_tuple("Selected").field(candidate).finish(),
            Self::AutoMerged(candidate) => f.debug_tuple("AutoMerged").field(candidate).finish(),
            Self::Conflict(conflict) => f.debug_tuple("Conflict").field(conflict).finish(),
        }
    }
}

/// Merge two repository-related candidates without storage or clock effects.
pub fn merge_candidates(
    left: &ItemCandidate,
    right: &ItemCandidate,
    relation: CausalRelation,
    merge_revision: RevisionId,
    conflict_id: ConflictId,
    now_ms: u64,
) -> Result<MergeDecision, DomainError> {
    if left.item_id() != right.item_id() {
        return Err(DomainError::IdentityMismatch);
    }
    if let (ItemState::Live(a), ItemState::Live(b)) = (left.state(), right.state()) {
        if a.schema() != b.schema() {
            return Err(DomainError::IdentityMismatch);
        }
    }
    match relation {
        CausalRelation::Same => {
            if left.revision_id() != right.revision_id() || left != right {
                return Err(DomainError::InvalidConflict);
            }
            Ok(MergeDecision::Selected(left.clone()))
        }
        CausalRelation::LeftDescends => Ok(MergeDecision::Selected(left.clone())),
        CausalRelation::RightDescends => Ok(MergeDecision::Selected(right.clone())),
        CausalRelation::Concurrent => {
            merge_concurrent(left, right, merge_revision, conflict_id, now_ms)
        }
    }
}

fn merge_concurrent(
    left: &ItemCandidate,
    right: &ItemCandidate,
    merge_revision: RevisionId,
    conflict_id: ConflictId,
    now_ms: u64,
) -> Result<MergeDecision, DomainError> {
    match (left.state(), right.state()) {
        (ItemState::Live(a), ItemState::Live(b)) if a.payload() == b.payload() => {
            let document = ItemDocument::new(
                a.id(),
                a.schema().clone(),
                a.created_at_ms().min(b.created_at_ms()),
                a.updated_at_ms().max(b.updated_at_ms()),
                a.favorite().merge(b.favorite()),
                a.collection_ids().merge(b.collection_ids())?,
                a.tags().merge(b.tags())?,
                a.payload().clone(),
                a.attachments().merge(b.attachments())?,
                merge_attachment_manifests(a.attachment_manifests(), b.attachment_manifests())?,
            )?;
            let candidate = ItemCandidate::new(
                merge_revision,
                [left.revision_id(), right.revision_id()],
                ItemState::Live(Box::new(document)),
            )?;
            Ok(MergeDecision::AutoMerged(candidate))
        }
        (ItemState::Tombstone(a), ItemState::Tombstone(b)) => {
            let choose_right =
                (b.deleted_at_ms, right.revision_id()) > (a.deleted_at_ms, left.revision_id());
            Ok(MergeDecision::Selected(if choose_right {
                right.clone()
            } else {
                left.clone()
            }))
        }
        _ => Ok(MergeDecision::Conflict(Box::new(ItemConflict::new(
            conflict_id,
            left.clone(),
            right.clone(),
            now_ms,
        )?))),
    }
}

/// Marker placed where a plaintext secret was intentionally omitted.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct RedactedSecret;

impl core::fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl core::fmt::Display for RedactedSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Secret-free, typed projection of a VLT02 record for normal host rendering.
#[derive(Clone, PartialEq, Eq)]
pub enum RedactedRecordView {
    /// Login metadata with password and notes omitted.
    Login {
        /// Display title.
        title: String,
        /// Username shown after unlock.
        username: String,
        /// Associated URLs shown after unlock.
        urls: Vec<String>,
        /// Password omission marker.
        password: RedactedSecret,
        /// Whether optional notes exist.
        has_notes: bool,
    },
    /// Secure-note title with body omitted.
    SecureNote {
        /// Display title.
        title: String,
        /// Body omission marker.
        body: RedactedSecret,
    },
    /// Payment-card metadata with PAN and CVV omitted.
    Card {
        /// Display title.
        title: String,
        /// Cardholder name.
        holder: String,
        /// Last four PAN characters, when available.
        last_four: String,
        /// Expiry month.
        expiry_month: u8,
        /// Expiry year.
        expiry_year: u16,
        /// PAN omission marker.
        number: RedactedSecret,
        /// CVV omission marker.
        cvv: RedactedSecret,
        /// Whether billing postal metadata exists.
        has_billing_zip: bool,
    },
    /// TOTP configuration metadata with seed omitted.
    TotpSeed {
        /// Display label.
        label: String,
        /// Optional issuer.
        issuer: Option<String>,
        /// HMAC algorithm.
        algorithm: String,
        /// Output digit count.
        digits: u8,
        /// Time step in seconds.
        period: u32,
        /// Seed omission marker.
        secret: RedactedSecret,
    },
    /// API-key metadata with token omitted.
    ApiKey {
        /// Display label.
        label: String,
        /// Service name.
        service: String,
        /// Granted scopes.
        scopes: Vec<String>,
        /// Optional expiry.
        expires_at: Option<u64>,
        /// Token omission marker.
        token: RedactedSecret,
    },
    /// Database connection metadata with password and lease ID omitted.
    DatabaseCredential {
        /// Display label.
        label: String,
        /// Database engine.
        engine: String,
        /// Database host.
        host: String,
        /// Database port.
        port: u16,
        /// Optional database/catalog name.
        database: Option<String>,
        /// Username shown after unlock.
        username: String,
        /// Optional expiry.
        expires_at: Option<u64>,
        /// Whether a lease ID exists.
        has_lease_id: bool,
        /// Password omission marker.
        password: RedactedSecret,
    },
    /// Unknown record metadata with all payload bytes omitted.
    Opaque {
        /// Validated unknown content type.
        content_type: ContentType,
        /// Canonical payload byte count.
        payload_bytes: usize,
        /// Payload omission marker.
        payload: RedactedSecret,
    },
    /// A record whose declared content type this crate recognises, but
    /// whose payload does not decode as that type's schema — a peer
    /// authored (by bug or by malice) a first-party-tagged record this
    /// client cannot materialise. See `AnyRecord::Quarantined`.
    Quarantined {
        /// The declared (but unreadable) content type.
        content_type: ContentType,
        /// Canonical payload byte count.
        payload_bytes: usize,
        /// Static, non-sensitive description of what failed to decode,
        /// e.g. `"Login.password missing"`. Safe to surface as-is: this
        /// is always one of a fixed set of literals this crate's own
        /// typed decoders define, never input-derived text.
        reason: &'static str,
        /// Payload omission marker.
        payload: RedactedSecret,
    },
}

impl RedactedRecordView {
    /// Build a view without cloning plaintext secret fields.
    pub fn from_record(record: &AnyRecord) -> Result<Self, DomainError> {
        Ok(match record {
            AnyRecord::Login(value) => Self::Login {
                title: value.title.clone(),
                username: value.username.clone(),
                urls: value.urls.clone(),
                password: RedactedSecret,
                has_notes: value.notes.is_some(),
            },
            AnyRecord::SecureNote(value) => Self::SecureNote {
                title: value.title.clone(),
                body: RedactedSecret,
            },
            AnyRecord::Card(value) => Self::Card {
                title: value.title.clone(),
                holder: value.holder.clone(),
                last_four: value
                    .number
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect(),
                expiry_month: value.expiry_month,
                expiry_year: value.expiry_year,
                number: RedactedSecret,
                cvv: RedactedSecret,
                has_billing_zip: value.billing_zip.is_some(),
            },
            AnyRecord::TotpSeed(value) => Self::TotpSeed {
                label: value.label.clone(),
                issuer: value.issuer.clone(),
                algorithm: value.algorithm.clone(),
                digits: value.digits,
                period: value.period,
                secret: RedactedSecret,
            },
            AnyRecord::ApiKey(value) => Self::ApiKey {
                label: value.label.clone(),
                service: value.service.clone(),
                scopes: value.scopes.clone(),
                expires_at: value.expires_at,
                token: RedactedSecret,
            },
            AnyRecord::DatabaseCredential(value) => Self::DatabaseCredential {
                label: value.label.clone(),
                engine: value.engine.clone(),
                host: value.host.clone(),
                port: value.port,
                database: value.database.clone(),
                username: value.username.clone(),
                expires_at: value.expires_at,
                has_lease_id: value.lease_id.is_some(),
                password: RedactedSecret,
            },
            AnyRecord::Opaque {
                content_type,
                payload_bytes,
            } => Self::Opaque {
                content_type: ContentType::new(content_type.clone())?,
                payload_bytes: payload_bytes.len(),
                payload: RedactedSecret,
            },
            AnyRecord::Quarantined {
                content_type,
                payload_bytes,
                reason,
            } => Self::Quarantined {
                content_type: ContentType::new(content_type.clone())?,
                payload_bytes: payload_bytes.len(),
                reason,
                payload: RedactedSecret,
            },
        })
    }

    /// Return the value-redacted VLT02 kind.
    pub const fn kind(&self) -> VaultRecordKind {
        match self {
            Self::Login { .. } => VaultRecordKind::Login,
            Self::SecureNote { .. } => VaultRecordKind::SecureNote,
            Self::Card { .. } => VaultRecordKind::Card,
            Self::TotpSeed { .. } => VaultRecordKind::TotpSeed,
            Self::ApiKey { .. } => VaultRecordKind::ApiKey,
            Self::DatabaseCredential { .. } => VaultRecordKind::DatabaseCredential,
            Self::Opaque { .. } => VaultRecordKind::Opaque,
            Self::Quarantined { .. } => VaultRecordKind::Quarantined,
        }
    }
}

impl core::fmt::Debug for RedactedRecordView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Login {
                urls, has_notes, ..
            } => f
                .debug_struct("Login")
                .field("display_values", &"<redacted>")
                .field("url_count", &urls.len())
                .field("has_notes", has_notes)
                .finish(),
            Self::SecureNote { .. } => f.write_str("SecureNote(<redacted>)"),
            Self::Card { .. } => f.write_str("Card(<redacted>)"),
            Self::TotpSeed { .. } => f.write_str("TotpSeed(<redacted>)"),
            Self::ApiKey { scopes, .. } => f
                .debug_struct("ApiKey")
                .field("display_values", &"<redacted>")
                .field("scope_count", &scopes.len())
                .finish(),
            Self::DatabaseCredential { .. } => f.write_str("DatabaseCredential(<redacted>)"),
            Self::Opaque { payload_bytes, .. } => f
                .debug_struct("Opaque")
                .field("content_type", &"<redacted>")
                .field("payload_bytes", payload_bytes)
                .finish(),
            Self::Quarantined {
                payload_bytes,
                reason,
                ..
            } => f
                .debug_struct("Quarantined")
                .field("content_type", &"<redacted>")
                .field("payload_bytes", payload_bytes)
                .field("reason", reason)
                .finish(),
        }
    }
}

impl Zeroize for RedactedRecordView {
    fn zeroize(&mut self) {
        match self {
            Self::Login {
                title,
                username,
                urls,
                ..
            } => {
                title.zeroize();
                username.zeroize();
                for url in urls.iter_mut() {
                    url.zeroize();
                }
                *urls = Vec::new();
            }
            Self::SecureNote { title, .. } => title.zeroize(),
            Self::Card {
                title,
                holder,
                last_four,
                ..
            } => {
                title.zeroize();
                holder.zeroize();
                last_four.zeroize();
            }
            Self::TotpSeed {
                label,
                issuer,
                algorithm,
                ..
            } => {
                label.zeroize();
                issuer.zeroize();
                algorithm.zeroize();
            }
            Self::ApiKey {
                label,
                service,
                scopes,
                ..
            } => {
                label.zeroize();
                service.zeroize();
                for scope in scopes.iter_mut() {
                    scope.zeroize();
                }
                *scopes = Vec::new();
            }
            Self::DatabaseCredential {
                label,
                engine,
                host,
                database,
                username,
                ..
            } => {
                label.zeroize();
                engine.zeroize();
                host.zeroize();
                database.zeroize();
                username.zeroize();
            }
            Self::Opaque { content_type, .. } => content_type.0.zeroize(),
            Self::Quarantined { content_type, .. } => content_type.0.zeroize(),
        }
    }
}

impl Drop for RedactedRecordView {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Default secret-free item projection for an unlocked host.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedItemView {
    /// Stable item identity, rendered only through its explicit method.
    pub item_id: ItemId,
    /// Validated content type.
    pub schema: ContentType,
    /// Record-specific redacted view.
    pub record: RedactedRecordView,
    /// Selected favorite value.
    pub favorite: bool,
    /// Present collection count.
    pub collection_count: usize,
    /// Present tag count.
    pub tag_count: usize,
    /// Present attachment count.
    pub attachment_count: usize,
    /// Latest item update time.
    pub updated_at_ms: u64,
}

impl RedactedItemView {
    /// Project a validated item without copying its plaintext secret fields.
    pub fn from_document(document: &ItemDocument) -> Result<Self, DomainError> {
        Ok(Self {
            item_id: document.id(),
            schema: document.schema().clone(),
            record: RedactedRecordView::from_record(document.payload())?,
            favorite: *document.favorite().value(),
            collection_count: document.collection_ids().len(),
            tag_count: document.tags().len(),
            attachment_count: document.attachments().len(),
            updated_at_ms: document.updated_at_ms(),
        })
    }
}

impl core::fmt::Debug for RedactedItemView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RedactedItemView")
            .field("item_id", &"<redacted>")
            .field("schema", &"<redacted>")
            .field("record", &self.record)
            .field("favorite", &self.favorite)
            .field("collection_count", &self.collection_count)
            .field("tag_count", &self.tag_count)
            .field("attachment_count", &self.attachment_count)
            .field("updated_at_ms", &self.updated_at_ms)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_records::{
        ApiKey, Card, DatabaseCredential, Login, SecureNote, TotpSeed,
    };

    fn operation(byte: u8) -> OperationId {
        OperationId::new([byte; 32])
    }

    fn indexed_operation(index: usize) -> OperationId {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        OperationId::new(bytes)
    }

    fn login(password: &str) -> AnyRecord {
        AnyRecord::Login(Login {
            title: "Example".into(),
            username: "ada@example.test".into(),
            password: password.into(),
            urls: vec!["https://example.test".into()],
            notes: Some("fixture notes".into()),
        })
    }

    fn document(id: u8, revision: u8, password: &str) -> ItemCandidate {
        let mut tags = ObservedSet::new();
        tags.add("work".to_string(), operation(revision)).unwrap();
        let document = ItemDocument::new(
            ItemId::new([id; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            10,
            20,
            LwwRegister::new(false, 20, operation(revision)),
            ObservedSet::new(),
            tags,
            login(password),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        ItemCandidate::new(
            RevisionId::new([revision; 32]),
            [],
            ItemState::Live(Box::new(document)),
        )
        .unwrap()
    }

    #[test]
    fn identifier_round_trips_are_canonical_and_redacted() {
        let item = ItemId::new([0xabu8; 16]);
        let rendered = item.to_user_string();
        assert_eq!(rendered.len(), 26);
        assert_eq!(ItemId::from_user_string(&rendered), Ok(item));
        assert_eq!(format!("{item}"), "<redacted ItemId>");
        assert_eq!(format!("{item:?}"), "ItemId(<redacted>)");
        assert_eq!(
            ItemId::from_user_string(&rendered.to_ascii_lowercase()),
            Err(DomainError::InvalidIdentifier)
        );
        let mut noncanonical = rendered;
        noncanonical.replace_range(25..26, "Z");
        assert_eq!(
            ItemId::from_user_string(&noncanonical),
            Err(DomainError::InvalidIdentifier)
        );
    }

    #[test]
    fn all_identifier_widths_round_trip() {
        macro_rules! check {
            ($type:ident, $size:expr, $byte:expr) => {{
                let id = $type::new([$byte; $size]);
                assert_eq!($type::from_user_string(&id.to_user_string()), Ok(id));
            }};
        }
        check!(CollectionId, 16, 1);
        check!(AttachmentId, 16, 2);
        check!(AttachmentManifestId, 32, 6);
        check!(ConflictId, 16, 3);
        check!(RevisionId, 32, 4);
        check!(OperationId, 32, 5);
    }

    #[test]
    fn content_types_are_bounded_and_redacted() {
        let value = ContentType::new("vault/login/v1").unwrap();
        assert_eq!(value.as_str(), "vault/login/v1");
        assert!(!format!("{value:?}").contains("login"));
        assert_eq!(
            ContentType::new("Vault/Login/V1"),
            Err(DomainError::InvalidContentType)
        );
        assert_eq!(
            ContentType::new("bad type"),
            Err(DomainError::InvalidContentType)
        );
        assert_eq!(
            ContentType::new("x".repeat(129)),
            Err(DomainError::InvalidContentType)
        );
    }

    #[test]
    fn observed_set_supports_remove_and_readd() {
        let mut set = ObservedSet::new();
        set.add("a".to_string(), operation(1)).unwrap();
        assert!(set.contains(&"a".to_string()));
        assert!(set.remove(&"a".to_string()));
        assert!(!set.contains(&"a".to_string()));
        set.add("a".to_string(), operation(2)).unwrap();
        assert!(set.contains(&"a".to_string()));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn observed_set_merge_obeys_crdt_laws() {
        let mut a = ObservedSet::new();
        a.add("a".to_string(), operation(1)).unwrap();
        let mut b = ObservedSet::new();
        b.add("b".to_string(), operation(2)).unwrap();
        let mut c = ObservedSet::new();
        c.add("c".to_string(), operation(3)).unwrap();
        assert_eq!(a.merge(&a).unwrap(), a);
        assert_eq!(a.merge(&b).unwrap(), b.merge(&a).unwrap());
        assert_eq!(
            a.merge(&b).unwrap().merge(&c).unwrap(),
            a.merge(&b.merge(&c).unwrap()).unwrap()
        );
    }

    #[test]
    fn observed_remove_wins_for_observed_add_after_merge() {
        let mut source = ObservedSet::new();
        source.add("a".to_string(), operation(1)).unwrap();
        let mut removed = source.clone();
        removed.remove(&"a".to_string());
        assert!(!source.merge(&removed).unwrap().contains(&"a".to_string()));
    }

    #[test]
    fn observed_set_enforces_retained_wire_bounds_and_operation_uniqueness() {
        let mut values = ObservedSet::new();
        for index in 0..MAX_OBSERVED_VALUES {
            assert_eq!(
                values.add(format!("value-{index}"), indexed_operation(index)),
                Ok(true)
            );
        }
        assert_eq!(values.retained_value_count(), MAX_OBSERVED_VALUES);
        assert_eq!(
            values.add(
                "one-value-too-many".to_string(),
                indexed_operation(MAX_OBSERVED_VALUES)
            ),
            Err(DomainError::BoundExceeded)
        );

        let mut operations = ObservedSet::new();
        for index in 0..MAX_OBSERVED_ADD_OPERATIONS {
            assert_eq!(
                operations.add("value".to_string(), indexed_operation(index)),
                Ok(true)
            );
        }
        assert_eq!(
            operations.add(
                "value".to_string(),
                indexed_operation(MAX_OBSERVED_ADD_OPERATIONS)
            ),
            Err(DomainError::BoundExceeded)
        );
        assert_eq!(
            operations.add_operation_count(),
            MAX_OBSERVED_ADD_OPERATIONS
        );
        assert!(operations.remove(&"value".to_string()));
        assert_eq!(operations.tombstone_count(), MAX_OBSERVED_TOMBSTONES);

        let mut collision = ObservedSet::new();
        assert_eq!(collision.add("a".to_string(), operation(1)), Ok(true));
        assert_eq!(collision.add("a".to_string(), operation(1)), Ok(false));
        assert_eq!(
            collision.add("b".to_string(), operation(1)),
            Err(DomainError::InvalidObservedSet)
        );
    }

    #[test]
    fn exact_removal_reconstruction_rejects_dangling_operations() {
        let mut set = ObservedSet::new();
        set.add("a".to_string(), operation(1)).unwrap();
        set.add("a".to_string(), operation(2)).unwrap();
        assert_eq!(
            set.observe_removal(&"a".to_string(), operation(1)),
            Ok(true)
        );
        assert_eq!(
            set.observe_removal(&"a".to_string(), operation(1)),
            Ok(false)
        );
        assert!(set.contains(&"a".to_string()));
        assert_eq!(set.tombstone_count(), 1);
        assert_eq!(
            set.observe_removal(&"a".to_string(), operation(3)),
            Err(DomainError::InvalidObservedSet)
        );
        assert_eq!(
            set.observe_removal(&"missing".to_string(), operation(1)),
            Err(DomainError::InvalidObservedSet)
        );
    }

    #[test]
    fn retained_observations_round_trip_without_resurrection() {
        let mut source = ObservedSet::new();
        source.add("a".to_string(), operation(1)).unwrap();
        source.add("a".to_string(), operation(2)).unwrap();
        source.add("b".to_string(), operation(3)).unwrap();
        source.remove(&"a".to_string());
        source.add("a".to_string(), operation(4)).unwrap();
        source.remove(&"b".to_string());

        let mut reconstructed = ObservedSet::new();
        for value in source.retained_values() {
            for operation in source.retained_add_operations(value) {
                reconstructed.add(value.clone(), operation).unwrap();
            }
            for operation in source.retained_removal_operations(value) {
                reconstructed.observe_removal(value, operation).unwrap();
            }
        }

        assert_eq!(reconstructed, source);
        assert!(reconstructed.contains(&"a".to_string()));
        assert!(!reconstructed.contains(&"b".to_string()));
        assert!(source
            .retained_add_operations(&"missing".to_string())
            .next()
            .is_none());
        assert!(source
            .retained_removal_operations(&"missing".to_string())
            .next()
            .is_none());
    }

    #[test]
    fn observed_set_merge_rejects_combined_operation_amplification() {
        let mut left = ObservedSet::new();
        let mut right = ObservedSet::new();
        for index in 0..600 {
            left.add("value".to_string(), indexed_operation(index))
                .unwrap();
            right
                .add("value".to_string(), indexed_operation(index + 600))
                .unwrap();
        }
        assert_eq!(left.merge(&right), Err(DomainError::BoundExceeded));
    }

    #[test]
    fn compaction_requires_stability_and_preserves_readds() {
        let mut set = ObservedSet::new();
        set.add("a".to_string(), operation(1)).unwrap();
        set.remove(&"a".to_string());
        set.add("a".to_string(), operation(2)).unwrap();
        set.add("b".to_string(), operation(3)).unwrap();
        set.remove(&"b".to_string());

        assert_eq!(
            set.compact_stable_removals(|candidate| candidate == operation(1)),
            1
        );
        assert!(set.contains(&"a".to_string()));
        assert!(!set.contains(&"b".to_string()));
        assert_eq!(set.add_operation_count(), 2);
        assert_eq!(set.tombstone_count(), 1);
        assert_eq!(set.retained_value_count(), 2);

        assert_eq!(set.compact_stable_removals(|_| true), 1);
        assert_eq!(set.values(), vec![&"a".to_string()]);
        assert_eq!(set.retained_value_count(), 1);
        assert_eq!(set.add_operation_count(), 1);
        assert_eq!(set.tombstone_count(), 0);
    }

    #[test]
    fn lww_uses_operation_as_clock_tie_breaker() {
        let left = LwwRegister::new(false, 10, operation(1));
        let right = LwwRegister::new(true, 10, operation(2));
        assert!(*left.merge(&right).value());
        assert_eq!(left.merge(&right), right.merge(&left));
    }

    #[test]
    fn document_rejects_schema_timestamp_and_tag_errors() {
        let wrong_schema = ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(SECURE_NOTE_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(1)),
            ObservedSet::new(),
            ObservedSet::new(),
            login("secret"),
            ObservedSet::new(),
            BTreeMap::new(),
        );
        assert_eq!(wrong_schema.unwrap_err(), DomainError::SchemaMismatch);

        let bad_timestamp = ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            2,
            1,
            LwwRegister::new(false, 1, operation(1)),
            ObservedSet::new(),
            ObservedSet::new(),
            login("secret"),
            ObservedSet::new(),
            BTreeMap::new(),
        );
        assert_eq!(bad_timestamp.unwrap_err(), DomainError::InvalidTimestamp);

        let mut tags = ObservedSet::new();
        tags.add("bad\ntag".to_string(), operation(2)).unwrap();
        let bad_tag = ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(1)),
            ObservedSet::new(),
            tags,
            login("secret"),
            ObservedSet::new(),
            BTreeMap::new(),
        );
        assert_eq!(bad_tag.unwrap_err(), DomainError::InvalidTag);
    }

    #[test]
    fn document_enforces_membership_bounds() {
        let mut tags = ObservedSet::new();
        for index in 0..=MAX_TAGS {
            tags.add(format!("tag-{index}"), OperationId::new([index as u8; 32]))
                .unwrap();
        }
        let result = ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(1)),
            ObservedSet::new(),
            tags,
            login("secret"),
            ObservedSet::new(),
            BTreeMap::new(),
        );
        assert_eq!(result.unwrap_err(), DomainError::BoundExceeded);
    }

    fn attachment_document(
        attachments: ObservedSet<AttachmentId>,
        manifests: BTreeMap<AttachmentId, AttachmentManifestId>,
    ) -> Result<ItemDocument, DomainError> {
        ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(1)),
            ObservedSet::new(),
            ObservedSet::new(),
            login("secret"),
            attachments,
            manifests,
        )
    }

    /// VLT-PM47 §4.7. Membership and manifest references are one fact stored
    /// twice, so both directions of disagreement have to be refused, and the
    /// key set has to be the *retained* one — a removed attachment that a
    /// later merge resurrects must still know where its bytes are.
    #[test]
    fn document_requires_a_manifest_for_exactly_every_retained_attachment() {
        let id = AttachmentId::new([9; 16]);
        let manifest = AttachmentManifestId::new([3; 32]);
        let mut attachments = ObservedSet::new();
        attachments.add(id, operation(2)).unwrap();

        let mut references = BTreeMap::new();
        references.insert(id, manifest);
        let document = attachment_document(attachments.clone(), references.clone()).unwrap();
        assert_eq!(document.attachment_manifests().get(&id), Some(&manifest));

        assert_eq!(
            attachment_document(attachments.clone(), BTreeMap::new()).unwrap_err(),
            DomainError::AttachmentManifestMismatch,
            "membership with no manifest names bytes nobody can find"
        );
        assert_eq!(
            attachment_document(ObservedSet::new(), references.clone()).unwrap_err(),
            DomainError::AttachmentManifestMismatch,
            "a manifest with no membership points at bytes nothing claims"
        );

        let mut wrong_key = BTreeMap::new();
        wrong_key.insert(AttachmentId::new([10; 16]), manifest);
        assert_eq!(
            attachment_document(attachments.clone(), wrong_key).unwrap_err(),
            DomainError::AttachmentManifestMismatch,
            "an equal count is not an equal key set"
        );

        // Removal hides the value from `values()` but retains it on the wire,
        // so the reference must survive the removal too.
        let mut removed = attachments.clone();
        removed.remove(&id);
        assert!(removed.values().is_empty());
        assert_eq!(
            attachment_document(removed.clone(), BTreeMap::new()).unwrap_err(),
            DomainError::AttachmentManifestMismatch
        );
        assert!(attachment_document(removed, references).is_ok());
    }

    /// Two replicas that both know an immutable attachment id necessarily know
    /// the same immutable manifest address, so a disagreement is a fault and
    /// not a conflict a person could be asked to resolve.
    #[test]
    fn merging_attachment_manifests_unions_and_refuses_divergence() {
        let shared = AttachmentId::new([1; 16]);
        let only_right = AttachmentId::new([2; 16]);
        let manifest = AttachmentManifestId::new([7; 32]);

        let mut left = BTreeMap::new();
        left.insert(shared, manifest);
        let mut right = BTreeMap::new();
        right.insert(shared, manifest);
        right.insert(only_right, AttachmentManifestId::new([8; 32]));

        let merged = merge_attachment_manifests(&left, &right).unwrap();
        assert_eq!(merged.len(), 2);
        assert_eq!(merge_attachment_manifests(&right, &left).unwrap(), merged);

        let mut divergent = BTreeMap::new();
        divergent.insert(shared, AttachmentManifestId::new([9; 32]));
        assert_eq!(
            merge_attachment_manifests(&left, &divergent).unwrap_err(),
            DomainError::AttachmentManifestMismatch
        );
        assert_eq!(
            DomainError::AttachmentManifestMismatch.to_string(),
            "vault-pm-domain: attachment manifest reference mismatch"
        );
    }

    #[test]
    fn document_debug_omits_identifiers_and_secrets() {
        let candidate = document(7, 9, "needle-password");
        let debug = format!("{candidate:?}");
        assert!(!debug.contains("needle-password"));
        assert!(!debug.contains(&candidate.item_id().to_user_string()));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn causal_selection_and_same_validation_are_deterministic() {
        let left = document(1, 1, "same");
        let right = document(1, 2, "same");
        assert!(matches!(
            merge_candidates(
                &left,
                &right,
                CausalRelation::RightDescends,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            )
            .unwrap(),
            MergeDecision::Selected(candidate) if candidate.revision_id() == right.revision_id()
        ));
        assert_eq!(
            merge_candidates(
                &left,
                &right,
                CausalRelation::Same,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            ),
            Err(DomainError::InvalidConflict)
        );
    }

    #[test]
    fn concurrent_equal_payloads_auto_merge_metadata() {
        let left = document(1, 1, "same");
        let mut right = document(1, 2, "same");
        if let ItemState::Live(value) = &mut right.state {
            value.tags.add("personal".into(), operation(8)).unwrap();
            value.favorite = LwwRegister::new(true, 21, operation(9));
            value.updated_at_ms = 21;
        }
        let decision = merge_candidates(
            &left,
            &right,
            CausalRelation::Concurrent,
            RevisionId::new([3; 32]),
            ConflictId::new([4; 16]),
            30,
        )
        .unwrap();
        let MergeDecision::AutoMerged(candidate) = decision else {
            panic!("expected automatic merge");
        };
        assert_eq!(candidate.causal_parents().len(), 2);
        let ItemState::Live(merged) = candidate.state() else {
            panic!("expected live merge");
        };
        assert_eq!(merged.tags().len(), 2);
        assert!(*merged.favorite().value());
    }

    #[test]
    fn concurrent_secret_edits_preserve_both_candidates() {
        let left = document(1, 1, "left-secret");
        let right = document(1, 2, "right-secret");
        let decision = merge_candidates(
            &left,
            &right,
            CausalRelation::Concurrent,
            RevisionId::new([3; 32]),
            ConflictId::new([4; 16]),
            30,
        )
        .unwrap();
        let MergeDecision::Conflict(conflict) = decision else {
            panic!("expected conflict");
        };
        assert_eq!(conflict.left().revision_id(), left.revision_id());
        assert_eq!(conflict.right().revision_id(), right.revision_id());
        let debug = format!("{conflict:?}");
        assert!(!debug.contains("left-secret"));
        assert!(!debug.contains("right-secret"));
    }

    #[test]
    fn delete_edit_race_is_a_retained_conflict() {
        let live = document(1, 1, "secret");
        let deleted = ItemCandidate::new(
            RevisionId::new([2; 32]),
            [],
            ItemState::Tombstone(Tombstone {
                item_id: ItemId::new([1; 16]),
                deleted_at_ms: 25,
            }),
        )
        .unwrap();
        assert!(matches!(
            merge_candidates(
                &live,
                &deleted,
                CausalRelation::Concurrent,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            )
            .unwrap(),
            MergeDecision::Conflict(_)
        ));
    }

    #[test]
    fn concurrent_tombstones_select_by_time_then_revision() {
        let make = |revision, deleted_at_ms| {
            ItemCandidate::new(
                RevisionId::new([revision; 32]),
                [],
                ItemState::Tombstone(Tombstone {
                    item_id: ItemId::new([1; 16]),
                    deleted_at_ms,
                }),
            )
            .unwrap()
        };
        let left = make(1, 20);
        let right = make(2, 20);
        assert!(matches!(
            merge_candidates(
                &left,
                &right,
                CausalRelation::Concurrent,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            )
            .unwrap(),
            MergeDecision::Selected(candidate) if candidate.revision_id() == right.revision_id()
        ));
    }

    #[test]
    fn conflict_resolution_retains_candidates_and_is_one_shot() {
        let left = document(1, 1, "left");
        let right = document(1, 2, "right");
        let MergeDecision::Conflict(mut conflict) = merge_candidates(
            &left,
            &right,
            CausalRelation::Concurrent,
            RevisionId::new([3; 32]),
            ConflictId::new([4; 16]),
            30,
        )
        .unwrap() else {
            panic!("expected conflict");
        };
        conflict.resolve(RevisionId::new([5; 32]), 31).unwrap();
        assert!(matches!(conflict.state(), ConflictState::Resolved { .. }));
        assert_eq!(conflict.left().revision_id(), left.revision_id());
        assert_eq!(conflict.right().revision_id(), right.revision_id());
        assert_eq!(
            conflict.resolve(RevisionId::new([6; 32]), 32),
            Err(DomainError::InvalidConflict)
        );
    }

    #[test]
    fn redacted_login_view_never_copies_secret_values() {
        let candidate = document(1, 1, "needle-password");
        let ItemState::Live(document) = candidate.state() else {
            panic!("expected live item");
        };
        let view = RedactedItemView::from_document(document).unwrap();
        let debug = format!("{view:?}");
        assert!(!debug.contains("needle-password"));
        assert!(!debug.contains("ada@example.test"));
        assert!(!debug.contains("https://example.test"));
        let RedactedRecordView::Login { password, .. } = &view.record else {
            panic!("expected login view");
        };
        assert_eq!(password.to_string(), "<redacted>");
    }

    #[test]
    fn redacted_secure_note_and_opaque_views_omit_payloads() {
        let note = AnyRecord::SecureNote(SecureNote {
            title: "title".into(),
            body: "needle-note-body".into(),
        });
        let note_view = RedactedRecordView::from_record(&note).unwrap();
        assert!(!format!("{note_view:?}").contains("needle-note-body"));

        let opaque = AnyRecord::Opaque {
            content_type: "example/future/v1".into(),
            payload_bytes: b"needle-opaque-secret".to_vec(),
        };
        let opaque_view = RedactedRecordView::from_record(&opaque).unwrap();
        let RedactedRecordView::Opaque { payload_bytes, .. } = &opaque_view else {
            panic!("expected opaque view");
        };
        assert_eq!(*payload_bytes, 20);

        let invalid_opaque = AnyRecord::Opaque {
            content_type: "INVALID TYPE".into(),
            payload_bytes: vec![1, 2, 3],
        };
        assert_eq!(
            RedactedRecordView::from_record(&invalid_opaque),
            Err(DomainError::InvalidContentType)
        );
    }

    #[test]
    fn every_redacted_record_variant_replaces_secret_fields() {
        let card = AnyRecord::Card(Card {
            title: "Personal".into(),
            holder: "Ada".into(),
            number: "4111111111111111".into(),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: "needle-cvv".into(),
            billing_zip: Some("12345".into()),
        });
        let card_view = RedactedRecordView::from_record(&card).unwrap();
        assert!(matches!(
            &card_view,
            RedactedRecordView::Card { last_four, number: RedactedSecret, cvv: RedactedSecret, .. }
                if last_four == "1111"
        ));
        assert!(!format!("{card_view:?}").contains("needle-cvv"));

        let totp = AnyRecord::TotpSeed(TotpSeed {
            label: "Example".into(),
            issuer: Some("Issuer".into()),
            secret: b"needle-totp-seed".to_vec(),
            algorithm: "SHA1".into(),
            digits: 6,
            period: 30,
        });
        assert!(matches!(
            RedactedRecordView::from_record(&totp).unwrap(),
            RedactedRecordView::TotpSeed {
                secret: RedactedSecret,
                ..
            }
        ));

        let api = AnyRecord::ApiKey(ApiKey {
            label: "API".into(),
            service: "example.test".into(),
            token: "needle-api-token".into(),
            scopes: vec!["read".into()],
            expires_at: Some(100),
        });
        assert!(matches!(
            RedactedRecordView::from_record(&api).unwrap(),
            RedactedRecordView::ApiKey {
                token: RedactedSecret,
                ..
            }
        ));

        let database = AnyRecord::DatabaseCredential(DatabaseCredential {
            label: "DB".into(),
            engine: "postgres".into(),
            host: "db.example.test".into(),
            port: 5432,
            database: Some("app".into()),
            username: "ada".into(),
            password: "needle-db-password".into(),
            lease_id: Some("needle-lease".into()),
            expires_at: Some(200),
        });
        let database_view = RedactedRecordView::from_record(&database).unwrap();
        assert!(matches!(
            &database_view,
            RedactedRecordView::DatabaseCredential {
                password: RedactedSecret,
                has_lease_id: true,
                ..
            }
        ));
        let debug = format!("{database_view:?}");
        assert!(!debug.contains("needle-db-password"));
        assert!(!debug.contains("needle-lease"));
    }

    #[test]
    fn public_accessors_and_formatters_remain_explicit_and_redacted() {
        assert_eq!(
            ItemId::from_user_string("short"),
            Err(DomainError::InvalidIdentifier)
        );
        assert_eq!(
            ItemId::from_user_string("0000000000000000000000000é"),
            Err(DomainError::InvalidIdentifier)
        );
        macro_rules! check_id {
            ($type:ident, $size:expr, $byte:expr) => {{
                let id = $type::new([$byte; $size]);
                assert_eq!(id.as_bytes(), &[$byte; $size]);
                assert!(format!("{id}").contains("redacted"));
                assert!(format!("{id:?}").contains("redacted"));
            }};
        }
        check_id!(ItemId, 16, 1);
        check_id!(CollectionId, 16, 2);
        check_id!(AttachmentId, 16, 3);
        check_id!(AttachmentManifestId, 32, 7);
        check_id!(ConflictId, 16, 4);
        check_id!(RevisionId, 32, 5);
        check_id!(OperationId, 32, 6);

        let content_type = ContentType::new(LOGIN_V1).unwrap();
        assert_eq!(content_type.to_string(), "<redacted content type>");

        let mut set = ObservedSet::new();
        assert!(set.is_empty());
        assert!(!set.remove(&"missing".to_string()));
        set.add("visible-only-explicitly".to_string(), operation(1))
            .unwrap();
        assert_eq!(set.values(), vec![&"visible-only-explicitly".to_string()]);
        let set_debug = format!("{set:?}");
        assert!(set_debug.contains("present_count"));
        assert!(!set_debug.contains("visible-only-explicitly"));

        let favorite = LwwRegister::new(true, 10, operation(2));
        assert_eq!(favorite.updated_at_ms(), 10);
        assert_eq!(favorite.operation(), operation(2));
        assert!(!format!("{favorite:?}").contains("true"));
        assert_eq!(format!("{:?}", RedactedSecret), "<redacted>");
    }

    #[test]
    fn every_vlt02_schema_validates_as_an_item_document() {
        let records = [
            (
                AnyRecord::SecureNote(SecureNote {
                    title: "note".into(),
                    body: "secret note".into(),
                }),
                SECURE_NOTE_V1,
            ),
            (
                AnyRecord::Card(Card {
                    title: "card".into(),
                    holder: "Ada".into(),
                    number: "4111111111111111".into(),
                    expiry_month: 1,
                    expiry_year: 2030,
                    cvv: "123".into(),
                    billing_zip: None,
                }),
                CARD_V1,
            ),
            (
                AnyRecord::TotpSeed(TotpSeed {
                    label: "totp".into(),
                    issuer: None,
                    secret: vec![1, 2, 3],
                    algorithm: "SHA1".into(),
                    digits: 6,
                    period: 30,
                }),
                TOTP_SEED_V1,
            ),
            (
                AnyRecord::ApiKey(ApiKey {
                    label: "api".into(),
                    service: "example.test".into(),
                    token: "token".into(),
                    scopes: vec![],
                    expires_at: None,
                }),
                API_KEY_V1,
            ),
            (
                AnyRecord::DatabaseCredential(DatabaseCredential {
                    label: "database".into(),
                    engine: "postgres".into(),
                    host: "localhost".into(),
                    port: 5432,
                    database: None,
                    username: "ada".into(),
                    password: "password".into(),
                    lease_id: None,
                    expires_at: None,
                }),
                DATABASE_CREDENTIAL_V1,
            ),
            (
                AnyRecord::Opaque {
                    content_type: "example/future/v1".into(),
                    payload_bytes: vec![7, 8, 9],
                },
                "example/future/v1",
            ),
        ];
        for (index, (record, schema)) in records.into_iter().enumerate() {
            let document = ItemDocument::new(
                ItemId::new([index as u8; 16]),
                ContentType::new(schema).unwrap(),
                1,
                1,
                LwwRegister::new(false, 1, operation(index as u8)),
                ObservedSet::new(),
                ObservedSet::new(),
                record,
                ObservedSet::new(),
                BTreeMap::new(),
            )
            .unwrap();
            assert_eq!(document.schema().as_str(), schema);
        }

        let mut tags = ObservedSet::new();
        tags.add("retired".into(), operation(20)).unwrap();
        tags.remove(&"retired".to_string());
        let document = ItemDocument::new(
            ItemId::new([8; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(21)),
            ObservedSet::new(),
            tags,
            login("needle"),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        let debug = format!("{document:?}");
        assert!(!debug.contains("needle"));
        assert!(!debug.contains(LOGIN_V1));
    }

    #[test]
    fn candidate_validation_and_merge_error_branches_are_closed() {
        let revision = RevisionId::new([1; 32]);
        assert_eq!(
            ItemCandidate::new(
                revision,
                [revision],
                ItemState::Tombstone(Tombstone {
                    item_id: ItemId::new([1; 16]),
                    deleted_at_ms: 1,
                }),
            ),
            Err(DomainError::InvalidConflict)
        );
        let too_many_parents =
            (0..=MAX_CAUSAL_PARENTS).map(|index| RevisionId::new([index as u8 + 2; 32]));
        assert_eq!(
            ItemCandidate::new(
                revision,
                too_many_parents,
                ItemState::Tombstone(Tombstone {
                    item_id: ItemId::new([1; 16]),
                    deleted_at_ms: 1,
                }),
            ),
            Err(DomainError::BoundExceeded)
        );

        let left = document(1, 1, "same");
        let other_item = document(2, 2, "same");
        assert_eq!(
            merge_candidates(
                &left,
                &other_item,
                CausalRelation::Concurrent,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            ),
            Err(DomainError::IdentityMismatch)
        );

        let note_document = ItemDocument::new(
            ItemId::new([1; 16]),
            ContentType::new(SECURE_NOTE_V1).unwrap(),
            1,
            1,
            LwwRegister::new(false, 1, operation(5)),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::SecureNote(SecureNote {
                title: "note".into(),
                body: "body".into(),
            }),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        let note = ItemCandidate::new(
            RevisionId::new([2; 32]),
            [],
            ItemState::Live(Box::new(note_document)),
        )
        .unwrap();
        assert_eq!(
            merge_candidates(
                &left,
                &note,
                CausalRelation::Concurrent,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            ),
            Err(DomainError::IdentityMismatch)
        );

        assert!(matches!(
            merge_candidates(
                &left,
                &left,
                CausalRelation::Same,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            )
            .unwrap(),
            MergeDecision::Selected(_)
        ));
        assert!(matches!(
            merge_candidates(
                &left,
                &note,
                CausalRelation::LeftDescends,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            ),
            Err(DomainError::IdentityMismatch)
        ));

        let same_revision_different_payload = document(1, 1, "different");
        assert_eq!(
            merge_candidates(
                &left,
                &same_revision_different_payload,
                CausalRelation::Concurrent,
                RevisionId::new([3; 32]),
                ConflictId::new([4; 16]),
                30,
            ),
            Err(DomainError::InvalidConflict)
        );
    }

    #[test]
    fn conflict_metadata_and_debug_paths_preserve_both_sides() {
        let left = document(1, 1, "left");
        let right = document(1, 2, "right");
        let MergeDecision::Conflict(mut conflict) = merge_candidates(
            &left,
            &right,
            CausalRelation::Concurrent,
            RevisionId::new([3; 32]),
            ConflictId::new([4; 16]),
            30,
        )
        .unwrap() else {
            panic!("expected conflict");
        };
        assert_eq!(conflict.id(), ConflictId::new([4; 16]));
        assert_eq!(conflict.item_id(), ItemId::new([1; 16]));
        assert_eq!(conflict.discovered_at_ms(), 30);
        assert_eq!(
            conflict.resolve(RevisionId::new([5; 32]), 29),
            Err(DomainError::InvalidConflict)
        );
        conflict.resolve(RevisionId::new([5; 32]), 31).unwrap();
        assert!(format!("{:?}", conflict.state()).contains("Resolved"));
        assert!(format!("{:?}", MergeDecision::Conflict(conflict)).contains("Conflict"));

        let make_tombstone = |revision, deleted_at_ms| {
            ItemCandidate::new(
                RevisionId::new([revision; 32]),
                [],
                ItemState::Tombstone(Tombstone {
                    item_id: ItemId::new([1; 16]),
                    deleted_at_ms,
                }),
            )
            .unwrap()
        };
        let newer = make_tombstone(6, 50);
        let older = make_tombstone(7, 40);
        assert!(format!("{:?}", newer.state()).contains("Tombstone"));
        assert!(matches!(
            merge_candidates(
                &newer,
                &older,
                CausalRelation::Concurrent,
                RevisionId::new([8; 32]),
                ConflictId::new([9; 16]),
                60,
            )
            .unwrap(),
            MergeDecision::Selected(candidate) if candidate.revision_id() == newer.revision_id()
        ));
    }

    #[test]
    fn redacted_view_kind_and_debug_cover_every_variant() {
        let records = [
            login("password"),
            AnyRecord::SecureNote(SecureNote {
                title: "note".into(),
                body: "body".into(),
            }),
            AnyRecord::Card(Card {
                title: "card".into(),
                holder: "Ada".into(),
                number: "4111111111111111".into(),
                expiry_month: 1,
                expiry_year: 2030,
                cvv: "123".into(),
                billing_zip: None,
            }),
            AnyRecord::TotpSeed(TotpSeed {
                label: "totp".into(),
                issuer: None,
                secret: vec![1],
                algorithm: "SHA1".into(),
                digits: 6,
                period: 30,
            }),
            AnyRecord::ApiKey(ApiKey {
                label: "api".into(),
                service: "service".into(),
                token: "token".into(),
                scopes: vec!["read".into()],
                expires_at: None,
            }),
            AnyRecord::DatabaseCredential(DatabaseCredential {
                label: "database".into(),
                engine: "postgres".into(),
                host: "localhost".into(),
                port: 5432,
                database: None,
                username: "ada".into(),
                password: "password".into(),
                lease_id: None,
                expires_at: None,
            }),
            AnyRecord::Opaque {
                content_type: "example/future/v1".into(),
                payload_bytes: vec![1, 2, 3],
            },
        ];
        let expected = [
            VaultRecordKind::Login,
            VaultRecordKind::SecureNote,
            VaultRecordKind::Card,
            VaultRecordKind::TotpSeed,
            VaultRecordKind::ApiKey,
            VaultRecordKind::DatabaseCredential,
            VaultRecordKind::Opaque,
        ];
        for (record, expected_kind) in records.iter().zip(expected) {
            let view = RedactedRecordView::from_record(record).unwrap();
            assert_eq!(view.kind(), expected_kind);
            assert!(
                format!("{view:?}").contains("redacted")
                    || expected_kind == VaultRecordKind::Opaque
            );
        }
    }

    #[test]
    fn errors_never_include_input_values() {
        for error in [
            DomainError::InvalidIdentifier,
            DomainError::InvalidContentType,
            DomainError::InvalidTimestamp,
            DomainError::InvalidTag,
            DomainError::BoundExceeded,
            DomainError::InvalidObservedSet,
            DomainError::SchemaMismatch,
            DomainError::IdentityMismatch,
            DomainError::InvalidConflict,
        ] {
            let display = error.to_string();
            assert!(display.starts_with("vault-pm-domain:"));
            assert!(!display.contains("needle"));
        }
    }
}
