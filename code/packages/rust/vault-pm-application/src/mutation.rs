use crate::restore::remap_imported_item_state;
use crate::{
    encode_item_revision, encode_signed_audit_event, encode_signed_commit, seal_object,
    ActiveStateV1, ApplicationError, ApplicationRepository, ApplicationRepositoryError, CatalogV1,
    LocalSecretV1, LocalStateStore, LocalStateStoreError, LocalVaultStateV1, ObjectKind,
    ObjectRandomness, PublicationJournalV1, V1Keys,
};
use coding_adventures_ed25519::{generate_keypair, sign};
use coding_adventures_vault_attachments::BlobId;
use coding_adventures_vault_pm_audit::{AuditActionV1, AuditEventV1, AuditOutcomeV1};
use coding_adventures_vault_pm_domain::{
    AttachmentId, AttachmentManifestId, ItemCandidate, ItemDocument, ItemId, ItemState,
    OperationId, RevisionId, Tombstone,
};
use coding_adventures_vault_pm_format::{
    AnnouncementV1, CommitV1, ObjectFrameV1, ObjectId, Signature,
};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads, MAX_PUBLICATION_OBJECTS};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const ITEM_ID_BYTES: usize = 16;
const TRACE_ID_BYTES: usize = 32;
const OBJECT_RANDOM_BYTES: usize = 32 + 24 + 24;
const AUDIT_RANDOM_BYTES: usize = TRACE_ID_BYTES + OBJECT_RANDOM_BYTES;
/// Exact caller-filled CSPRNG bytes consumed by one audit-only access commit.
pub const AUDITED_ACCESS_RANDOM_BYTES: usize = TRACE_ID_BYTES + 2 * OBJECT_RANDOM_BYTES;
#[cfg(test)]
pub(crate) const AUDIT_ONLY_TEST_RANDOM_BYTES: usize = AUDITED_ACCESS_RANDOM_BYTES;

/// Exact caller-filled CSPRNG bytes consumed by one add-item mutation.
pub const ADD_ITEM_RANDOM_BYTES: usize =
    ITEM_ID_BYTES + 3 * OBJECT_RANDOM_BYTES + AUDIT_RANDOM_BYTES;
/// Exact caller-filled CSPRNG bytes consumed by one replace-item mutation.
pub const REPLACE_ITEM_RANDOM_BYTES: usize = 3 * OBJECT_RANDOM_BYTES + AUDIT_RANDOM_BYTES;
/// Exact caller-filled CSPRNG bytes consumed by one delete-item mutation.
pub const DELETE_ITEM_RANDOM_BYTES: usize = 3 * OBJECT_RANDOM_BYTES + AUDIT_RANDOM_BYTES;
/// Exact caller-filled CSPRNG bytes consumed by one restore-item mutation.
pub const RESTORE_ITEM_RANDOM_BYTES: usize = 3 * OBJECT_RANDOM_BYTES + AUDIT_RANDOM_BYTES;
/// Exact caller-filled CSPRNG bytes consumed by one conflict-resolution mutation.
pub const RESOLVE_ITEM_CONFLICT_RANDOM_BYTES: usize = 3 * OBJECT_RANDOM_BYTES + AUDIT_RANDOM_BYTES;

/// Owned wipe-on-drop entropy for one access trace, encrypted audit event, and
/// audit-only repository commit.
pub struct AuditedAccessRandomnessV1 {
    bytes: [u8; AUDITED_ACCESS_RANDOM_BYTES],
}

impl AuditedAccessRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; AUDITED_ACCESS_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for AuditedAccessRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for AuditedAccessRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for AuditedAccessRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuditedAccessRandomnessV1(<redacted>)")
    }
}

/// Return the exact caller-CSPRNG byte count required to import one opened
/// portable snapshot.
///
/// Import allocates one new 16-byte item identity per source item, one fresh
/// encrypted revision frame per retained candidate, fresh catalog and commit
/// frames, and a reserved trace plus audit-event frame. Snapshots too large for
/// one atomic repository publication are rejected before entropy is accepted.
pub fn portable_import_random_bytes(
    snapshot: &crate::OpenedPortableSnapshotV1,
) -> Result<usize, ApplicationError> {
    portable_import_random_bytes_for_counts(snapshot.item_count(), snapshot.candidate_count())
}

fn portable_import_random_bytes_for_counts(
    item_count: usize,
    candidate_count: usize,
) -> Result<usize, ApplicationError> {
    if candidate_count
        .checked_add(2)
        .is_none_or(|object_count| object_count > MAX_PUBLICATION_OBJECTS)
    {
        return Err(ApplicationError::BoundExceeded);
    }
    ITEM_ID_BYTES
        .checked_mul(item_count)
        .and_then(|item_bytes| {
            OBJECT_RANDOM_BYTES
                .checked_mul(candidate_count.checked_add(3)?)
                .and_then(|object_bytes| item_bytes.checked_add(object_bytes))
                .and_then(|bytes| bytes.checked_add(TRACE_ID_BYTES))
        })
        .ok_or(ApplicationError::BoundExceeded)
}

/// Owned wipe-on-drop entropy for one atomic cross-vault portable import.
pub struct PortableImportRandomnessV1 {
    bytes: Vec<u8>,
    item_count: usize,
    candidate_count: usize,
}

impl PortableImportRandomnessV1 {
    /// Validate and take the exact host-CSPRNG bytes required by `snapshot`.
    pub fn new(
        mut bytes: Vec<u8>,
        snapshot: &crate::OpenedPortableSnapshotV1,
    ) -> Result<Self, ApplicationError> {
        let item_count = snapshot.item_count();
        let candidate_count = snapshot.candidate_count();
        let expected = match portable_import_random_bytes_for_counts(item_count, candidate_count) {
            Ok(expected) => expected,
            Err(error) => {
                bytes.zeroize();
                return Err(error);
            }
        };
        if bytes.len() != expected {
            bytes.zeroize();
            return Err(ApplicationError::InvalidInput);
        }
        Ok(Self {
            bytes,
            item_count,
            candidate_count,
        })
    }
}

impl Zeroize for PortableImportRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.item_count = 0;
        self.candidate_count = 0;
    }
}

impl Drop for PortableImportRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for PortableImportRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableImportRandomnessV1(<redacted>)")
    }
}

/// Return the exact caller-CSPRNG byte count required to attach a file of
/// `plaintext_len` bytes.
///
/// The layout, in the order it is consumed, is VLT-PM47 §6.4's:
///
/// ```text
/// attachment / blob id      16
/// blob DEK                  32
/// per chunk object          80 x chunk_count
/// manifest object           80
/// revision, catalog, commit 80 x 3
/// trace id                  32
/// audit-event object        80
/// ```
///
/// One `AttachmentId` value serves as both the attachment identity and the
/// VLT14 blob id, so there is one 16-byte draw rather than two (§4.3).
///
/// This is the only place the arithmetic appears. Sizes that would not fit one
/// atomic publication are refused here, before any entropy is accepted — which
/// is also why the attach ceremony never needs a resumable protocol.
pub fn attachment_random_bytes(plaintext_len: u64) -> Result<usize, ApplicationError> {
    if plaintext_len == 0 {
        return Err(ApplicationError::InvalidInput);
    }
    if plaintext_len > crate::MAX_ATTACHMENT_BYTES as u64 {
        return Err(ApplicationError::BoundExceeded);
    }
    let chunk_count = crate::codec::expected_chunk_count(plaintext_len);
    // Revision, catalog, manifest, every chunk, and the audit event all land
    // in one commit, so the publication bound is a bound on the file.
    if chunk_count
        .checked_add(4)
        .is_none_or(|object_count| object_count > MAX_PUBLICATION_OBJECTS)
    {
        return Err(ApplicationError::BoundExceeded);
    }
    OBJECT_RANDOM_BYTES
        .checked_mul(
            chunk_count
                .checked_add(1)
                .ok_or(ApplicationError::BoundExceeded)?,
        )
        .and_then(|chunk_bytes| chunk_bytes.checked_add(ATTACHMENT_IDENTITY_BYTES))
        .and_then(|bytes| bytes.checked_add(REPLACE_ITEM_RANDOM_BYTES))
        .ok_or(ApplicationError::BoundExceeded)
}

const ATTACHMENT_IDENTITY_BYTES: usize = 16 + 32;

/// Owned wipe-on-drop entropy for one complete attachment write.
///
/// Variable-length for the same reason [`PortableImportRandomnessV1`] is: the
/// requirement depends on the input. It is validated against
/// [`attachment_random_bytes`] at construction, so a short or long block is an
/// `InvalidInput` before the vault is touched rather than a partition that
/// silently reads the wrong offsets.
pub struct AttachmentRandomnessV1 {
    bytes: Vec<u8>,
    chunk_count: usize,
}

impl AttachmentRandomnessV1 {
    /// Validate and take the exact host-CSPRNG bytes a `plaintext_len`-byte
    /// attachment requires.
    pub fn new(mut bytes: Vec<u8>, plaintext_len: u64) -> Result<Self, ApplicationError> {
        let expected = match attachment_random_bytes(plaintext_len) {
            Ok(expected) => expected,
            Err(error) => {
                bytes.zeroize();
                return Err(error);
            }
        };
        if bytes.len() != expected {
            bytes.zeroize();
            return Err(ApplicationError::InvalidInput);
        }
        Ok(Self {
            bytes,
            chunk_count: crate::codec::expected_chunk_count(plaintext_len),
        })
    }
}

impl Zeroize for AttachmentRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.chunk_count = 0;
    }
}

impl Drop for AttachmentRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for AttachmentRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AttachmentRandomnessV1(<redacted>)")
    }
}

/// Owned wipe-on-drop entropy for one item ID, three mutation frames, one
/// operation trace, and one encrypted audit-event frame.
pub struct AddItemRandomnessV1 {
    bytes: [u8; ADD_ITEM_RANDOM_BYTES],
}

impl AddItemRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; ADD_ITEM_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }

    /// Return the item ID derived from the dedicated first 16 random bytes.
    pub fn item_id(&self) -> ItemId {
        ItemId::new(
            self.bytes[..ITEM_ID_BYTES]
                .try_into()
                .expect("the item-ID partition length is constant"),
        )
    }
}

impl Zeroize for AddItemRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for AddItemRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for AddItemRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AddItemRandomnessV1(<redacted>)")
    }
}

/// Owned wipe-on-drop entropy for replacement, trace, and audit-event frames.
pub struct ReplaceItemRandomnessV1 {
    bytes: [u8; REPLACE_ITEM_RANDOM_BYTES],
}

impl ReplaceItemRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; REPLACE_ITEM_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for ReplaceItemRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for ReplaceItemRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for ReplaceItemRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReplaceItemRandomnessV1(<redacted>)")
    }
}

/// Owned wipe-on-drop entropy for deletion, trace, and audit-event frames.
pub struct DeleteItemRandomnessV1 {
    bytes: [u8; DELETE_ITEM_RANDOM_BYTES],
}

impl DeleteItemRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; DELETE_ITEM_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for DeleteItemRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for DeleteItemRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for DeleteItemRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeleteItemRandomnessV1(<redacted>)")
    }
}

/// Owned wipe-on-drop entropy for restoration, trace, and audit-event frames.
pub struct RestoreItemRandomnessV1 {
    bytes: [u8; RESTORE_ITEM_RANDOM_BYTES],
}

impl RestoreItemRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; RESTORE_ITEM_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for RestoreItemRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for RestoreItemRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for RestoreItemRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("RestoreItemRandomnessV1(<redacted>)")
    }
}

/// Owned wipe-on-drop entropy for conflict, trace, and audit-event frames.
pub struct ResolveItemConflictRandomnessV1 {
    bytes: [u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES],
}

impl ResolveItemConflictRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for ResolveItemConflictRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for ResolveItemConflictRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for ResolveItemConflictRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResolveItemConflictRandomnessV1(<redacted>)")
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn import_opened_portable_snapshot(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    snapshot: crate::OpenedPortableSnapshotV1,
    wall_time_ms: u64,
    randomness: PortableImportRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    // Audit-only events are allowed before the first import so failed artifact
    // reads, credentials, and retries can remain traceable. Any prior logical
    // item mutation leaves a live or tombstone candidate and still fails here.
    if !current_items.is_empty() {
        return Err(ApplicationError::InvalidInput);
    }
    if randomness.item_count != snapshot.item_count()
        || randomness.candidate_count != snapshot.candidate_count()
        || randomness.bytes.len()
            != portable_import_random_bytes_for_counts(
                snapshot.item_count(),
                snapshot.candidate_count(),
            )?
    {
        return Err(ApplicationError::InvalidInput);
    }

    let (source_vault_id, source_items) = snapshot.into_import_parts();
    if source_vault_id == active.vault_id() {
        return Err(ApplicationError::InvalidInput);
    }

    let publication = prepare_portable_import_publication(
        active,
        keys,
        local_secret,
        source_items,
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

fn prepare_portable_import_publication(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    source_items: BTreeMap<ItemId, Vec<ItemCandidate>>,
    wall_time_ms: u64,
    randomness: &[u8],
) -> Result<PublicationJournalV1, ApplicationError> {
    let item_count = source_items.len();
    let candidate_count = source_items.values().map(Vec::len).sum();
    if randomness.len() != portable_import_random_bytes_for_counts(item_count, candidate_count)? {
        return Err(ApplicationError::InvalidInput);
    }

    let device_counter = active
        .last_device_counter()
        .checked_add(1)
        .ok_or(ApplicationError::BoundExceeded)?;
    let source_item_ids = source_items.keys().copied().collect::<BTreeSet<_>>();
    let source_revision_ids = source_items
        .values()
        .flatten()
        .map(ItemCandidate::revision_id)
        .collect::<BTreeSet<_>>();
    let mut offset = 0;
    let mut imported_item_ids = BTreeSet::new();
    let mut item_id_map = BTreeMap::new();
    for source_item_id in source_items.keys() {
        let imported_item_id = ItemId::new(take_slice(randomness, &mut offset));
        if source_item_ids.contains(&imported_item_id)
            || !imported_item_ids.insert(imported_item_id)
        {
            return Err(ApplicationError::InvalidInput);
        }
        item_id_map.insert(*source_item_id, imported_item_id);
    }

    let mut objects = Vec::with_capacity(candidate_count + 1);
    let mut catalog_entries = BTreeMap::new();
    let mut added_objects = Vec::with_capacity(candidate_count + 1);
    for (source_item_id, candidates) in source_items {
        let imported_item_id = item_id_map[&source_item_id];
        let mut imported_revision_ids = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if candidate.item_id() != source_item_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let imported_state = remap_imported_item_state(candidate.state(), imported_item_id)?;
            let revision_plaintext =
                Zeroizing::new(encode_item_revision(&BTreeSet::new(), &imported_state)?);
            let revision_frame = seal_object(
                keys,
                ObjectKind::ItemRevision,
                &revision_plaintext,
                &take_object_randomness_slice(randomness, &mut offset),
            )?;
            let revision_object_id = revision_frame
                .id()
                .map_err(|_| ApplicationError::InternalInvariant)?;
            let revision_id = RevisionId::new(*revision_object_id.as_bytes());
            if source_revision_ids.contains(&revision_id)
                || imported_revision_ids.contains(&revision_id)
                || added_objects.contains(&revision_object_id)
            {
                return Err(ApplicationError::InvalidInput);
            }
            imported_revision_ids.push(revision_id);
            added_objects.push(revision_object_id);
            objects.push(revision_frame);
        }
        imported_revision_ids.sort_unstable();
        catalog_entries.insert(imported_item_id, imported_revision_ids);
    }

    let catalog_plaintext = Zeroizing::new(CatalogV1::new(catalog_entries)?.encode()?);
    let catalog_frame = seal_object(
        keys,
        ObjectKind::Catalog,
        &catalog_plaintext,
        &take_object_randomness_slice(randomness, &mut offset),
    )?;
    let catalog_id = catalog_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    if added_objects.contains(&catalog_id) {
        return Err(ApplicationError::InvalidInput);
    }
    added_objects.push(catalog_id);
    objects.push(catalog_frame);

    let mut parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
    parents.sort_unstable();
    let commit_randomness = take_object_randomness_slice(randomness, &mut offset);
    let trace_id = OperationId::new(take_slice(randomness, &mut offset));
    let audit_randomness = take_object_randomness_slice(randomness, &mut offset);
    let audit_event = prepare_mutation_audit_event(
        active,
        keys,
        local_secret,
        device_counter,
        trace_id,
        AuditActionV1::PortableImport,
        None,
        None,
        None,
        parents.clone(),
        wall_time_ms,
        &audit_randomness,
    )?;
    if let Some((frame, id)) = audit_event.as_ref() {
        added_objects.push(*id);
        objects.push(frame.clone());
    }
    added_objects.sort_unstable();
    added_objects.dedup();
    if added_objects.len() != objects.len() {
        return Err(ApplicationError::InternalInvariant);
    }
    let (_, device_signing_secret) = generate_keypair(local_secret.device_signing_seed());
    let device_signing_secret = Zeroizing::new(device_signing_secret);
    let unsigned_commit = CommitV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        parents,
        catalog_root: catalog_id,
        added_objects,
        tombstone_root: None,
        wall_time_ms,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let commit_preimage = unsigned_commit
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let commit = unsigned_commit.with_signature(Signature::new(sign(
        &commit_preimage,
        &device_signing_secret,
    )));
    let commit_plaintext = Zeroizing::new(encode_signed_commit(&commit)?);
    let commit_frame = seal_object(
        keys,
        ObjectKind::Commit,
        &commit_plaintext,
        &commit_randomness,
    )?;
    debug_assert_eq!(offset, randomness.len());
    let commit_id = commit_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let unsigned_announcement = AnnouncementV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        commit_id,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let announcement_preimage = unsigned_announcement
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let announcement = unsigned_announcement
        .with_signature(Signature::new(sign(
            &announcement_preimage,
            &device_signing_secret,
        )))
        .encode()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let expected_heads =
        PinnedHeads::new([commit_id]).map_err(|_| ApplicationError::InternalInvariant)?;

    let publication = PublicationJournalV1::new(
        objects,
        commit_frame,
        announcement,
        active.pinned_heads().clone(),
        expected_heads,
        device_counter,
        catalog_id,
    )?;
    match audit_event.map(|(_, id)| id) {
        Some(head) => publication.with_audit_event_head(head),
        None => Ok(publication),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn add_item(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    document: ItemDocument,
    wall_time_ms: u64,
    randomness: AddItemRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    if document.id() != randomness.item_id() {
        return Err(ApplicationError::InvalidInput);
    }
    document
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
    if current_items.contains_key(&document.id()) {
        return Err(ApplicationError::InvalidInput);
    }

    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        document.id(),
        ItemState::Live(Box::new(document)),
        &BTreeSet::new(),
        AuditActionV1::ItemCreate,
        None,
        wall_time_ms,
        randomness.bytes[ITEM_ID_BYTES..]
            .try_into()
            .expect("the add-item object-randomness partition length is constant"),
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn replace_item(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    expected_revision: RevisionId,
    document: ItemDocument,
    wall_time_ms: u64,
    randomness: ReplaceItemRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    document
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
    let candidates = current_items
        .get(&document.id())
        .ok_or(ApplicationError::NotFound)?;
    let [candidate] = candidates.as_slice() else {
        return Err(ApplicationError::ConflictRequired);
    };
    let ItemState::Live(current_document) = candidate.state() else {
        return Err(ApplicationError::ConflictRequired);
    };
    if candidate.revision_id() != expected_revision {
        return Err(ApplicationError::ConflictRequired);
    }
    if current_document.schema() != document.schema()
        || current_document.created_at_ms() != document.created_at_ms()
    {
        return Err(ApplicationError::InvalidInput);
    }

    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        document.id(),
        ItemState::Live(Box::new(document)),
        &BTreeSet::from([expected_revision]),
        AuditActionV1::ItemUpdate,
        Some(expected_revision),
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

/// Attach one file to one live item, as a single ordinary mutation.
///
/// # Why there is no upload protocol here
///
/// Everything this function seals — every chunk, the manifest, the new
/// revision, the rebuilt catalog, the audit event, the commit, the
/// announcement — goes into one [`PublicationJournalV1`] and through the same
/// [`publish_mutation`] compare-exchange pair every other mutation uses. So
/// VLT-PM41's crash matrix and VLT-PM42's pending-publication recovery cover
/// this ceremony with no new argument: a crash before the first exchange
/// leaves the vault byte-identical, a crash after it leaves the exact frames
/// on disk for the next ordinary command to republish unchanged, and a crash
/// during publication leaves unreachable immutable objects of the kind
/// VLT-PM00 §10.4 already describes.
///
/// The alternative — writing chunks across several commits and tracking
/// progress — would have added a durable state the matrix does not cover, a
/// second recovery path, and a genuinely orphanable partial blob, to buy the
/// ability to attach a file larger than one publication can carry.
/// [`attachment_random_bytes`] sets the ceiling below that point on purpose.
///
/// The audit action is `ItemUpdate`, because attaching a file *is* an update
/// of the item document and VLT-PM15 §2's registry names user-visible
/// operations at item granularity rather than one code per field.
#[allow(clippy::too_many_arguments)]
pub(crate) fn attach_attachment(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    item_id: ItemId,
    name: String,
    plaintext: &[u8],
    wall_time_ms: u64,
    randomness: AttachmentRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<(ActiveStateV1, AttachmentId), ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    if randomness.chunk_count != crate::codec::expected_chunk_count(plaintext.len() as u64) {
        return Err(ApplicationError::InvalidInput);
    }
    let candidates = current_items
        .get(&item_id)
        .ok_or(ApplicationError::NotFound)?;
    let [candidate] = candidates.as_slice() else {
        return Err(ApplicationError::ConflictRequired);
    };
    let ItemState::Live(current) = candidate.state() else {
        return Err(ApplicationError::NotFound);
    };
    let expected_revision = candidate.revision_id();

    // The identity draw comes first so the two values that must never repeat
    // together — the blob id and its DEK — are the first thing spent, and so
    // the attachment id a person will see is fixed before anything is sealed.
    let mut offset = 0;
    let attachment_id = AttachmentId::new(take_slice::<16>(&randomness.bytes, &mut offset));
    let blob_dek = Zeroizing::new(take_slice::<{ crate::ATTACHMENT_DEK_BYTES }>(
        &randomness.bytes,
        &mut offset,
    ));
    let chunked = crate::attachment::chunk_attachment(
        plaintext,
        BlobId(*attachment_id.as_bytes()),
        blob_dek,
    )?;
    if chunked.chunks.len() != randomness.chunk_count {
        return Err(ApplicationError::InternalInvariant);
    }

    let mut content_objects = Vec::with_capacity(chunked.chunks.len() + 1);
    let mut chunk_ids = Vec::with_capacity(chunked.chunks.len());
    for chunk in &chunked.chunks {
        let plaintext = Zeroizing::new(crate::encode_attachment_chunk(chunk)?);
        let chunk_randomness = take_object_randomness_slice(&randomness.bytes, &mut offset);
        let frame = seal_object(
            keys,
            ObjectKind::AttachmentChunk,
            &plaintext,
            &chunk_randomness,
        )?;
        chunk_ids.push(
            frame
                .id()
                .map_err(|_| ApplicationError::InternalInvariant)?,
        );
        content_objects.push(frame);
    }

    let manifest = crate::AttachmentManifestV1::new(
        attachment_id,
        // The DEK is re-derived from the same partition rather than kept alive
        // across the chunking loop, so the only long-lived copy is the one
        // inside the manifest object about to be sealed.
        Zeroizing::new(
            randomness.bytes[16..16 + crate::ATTACHMENT_DEK_BYTES]
                .try_into()
                .map_err(|_| ApplicationError::InternalInvariant)?,
        ),
        name,
        chunked.total_plaintext_len,
        chunked.content_sha256,
        chunk_ids,
        wall_time_ms,
    )?;
    let manifest_plaintext = manifest.encode()?;
    let manifest_randomness = take_object_randomness_slice(&randomness.bytes, &mut offset);
    let manifest_frame = seal_object(
        keys,
        ObjectKind::AttachmentManifest,
        &manifest_plaintext,
        &manifest_randomness,
    )?;
    let manifest_id = manifest_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    content_objects.push(manifest_frame);

    // The add operation reuses the trace identity the audit event will carry,
    // which is drawn from the tail partition this function does not consume.
    let operation = OperationId::new(
        randomness.bytes[offset + 3 * OBJECT_RANDOM_BYTES..][..TRACE_ID_BYTES]
            .try_into()
            .map_err(|_| ApplicationError::InternalInvariant)?,
    );
    let mut attachments = current.attachments().clone();
    attachments
        .add(attachment_id, operation)
        .map_err(|_| ApplicationError::BoundExceeded)?;
    let mut manifests = current.attachment_manifests().clone();
    if manifests
        .insert(
            attachment_id,
            AttachmentManifestId::new(*manifest_id.as_bytes()),
        )
        .is_some()
    {
        // A fresh 128-bit identity that already exists is not a user error.
        return Err(ApplicationError::InternalInvariant);
    }
    let document = ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        wall_time_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        current.payload().clone(),
        attachments,
        manifests,
    )
    .map_err(|error| match error {
        coding_adventures_vault_pm_domain::DomainError::BoundExceeded => {
            ApplicationError::BoundExceeded
        }
        _ => ApplicationError::InvalidInput,
    })?;

    let publication = prepare_item_publication_with_objects(
        active,
        current_items,
        keys,
        local_secret,
        item_id,
        ItemState::Live(Box::new(document)),
        &BTreeSet::from([expected_revision]),
        AuditActionV1::ItemUpdate,
        Some(expected_revision),
        wall_time_ms,
        content_objects,
        &randomness.bytes[offset..],
    )?;
    let next = publish_mutation(active, repository, publication, local_state_store)?;
    Ok((next, attachment_id))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn delete_item(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    expected_revision: RevisionId,
    deleted_at_ms: u64,
    wall_time_ms: u64,
    randomness: DeleteItemRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    let (item_id, candidates) = current_items
        .iter()
        .find(|(_, candidates)| {
            candidates
                .iter()
                .any(|candidate| candidate.revision_id() == expected_revision)
        })
        .ok_or(ApplicationError::NotFound)?;
    let [candidate] = candidates.as_slice() else {
        return Err(ApplicationError::ConflictRequired);
    };
    if candidate.revision_id() != expected_revision
        || !matches!(candidate.state(), ItemState::Live(_))
    {
        return Err(ApplicationError::ConflictRequired);
    }

    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        *item_id,
        ItemState::Tombstone(Tombstone {
            item_id: *item_id,
            deleted_at_ms,
        }),
        &BTreeSet::from([expected_revision]),
        AuditActionV1::ItemDelete,
        Some(expected_revision),
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn restore_item(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    selected: ItemCandidate,
    wall_time_ms: u64,
    randomness: RestoreItemRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    let item_id = selected.item_id();
    let current = current_items
        .get(&item_id)
        .ok_or(ApplicationError::NotFound)?;
    let [current] = current.as_slice() else {
        return Err(ApplicationError::ConflictRequired);
    };
    if current.revision_id() == selected.revision_id() {
        return Err(ApplicationError::InvalidInput);
    }
    let ItemState::Live(document) = selected.state() else {
        return Err(ApplicationError::InvalidInput);
    };

    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        item_id,
        ItemState::Live(document.clone()),
        &BTreeSet::from([selected.revision_id()]),
        AuditActionV1::ItemRestore,
        Some(selected.revision_id()),
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_item_conflict(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    selected_revision: RevisionId,
    wall_time_ms: u64,
    randomness: ResolveItemConflictRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    let (item_id, candidates) = current_items
        .iter()
        .find(|(_, candidates)| {
            candidates
                .iter()
                .any(|candidate| candidate.revision_id() == selected_revision)
        })
        .ok_or(ApplicationError::NotFound)?;
    if candidates.len() < 2 {
        return Err(ApplicationError::ConflictRequired);
    }
    let selected = candidates
        .iter()
        .find(|candidate| candidate.revision_id() == selected_revision)
        .ok_or(ApplicationError::InternalInvariant)?;
    let causal_parents = candidates
        .iter()
        .map(ItemCandidate::revision_id)
        .collect::<BTreeSet<_>>();

    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        *item_id,
        selected.state().clone(),
        &causal_parents,
        AuditActionV1::ItemConflictResolve,
        Some(selected_revision),
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn merge_item_conflict(
    active: &ActiveStateV1,
    report: &OpenReport,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    document: ItemDocument,
    wall_time_ms: u64,
    randomness: ResolveItemConflictRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if report.heads() != active.pinned_heads() {
        return Err(ApplicationError::ConcurrentHost);
    }
    document
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
    let candidates = current_items
        .get(&document.id())
        .ok_or(ApplicationError::NotFound)?;
    if candidates.len() < 2 {
        return Err(ApplicationError::ConflictRequired);
    }

    let mut live_candidate_count = 0usize;
    for candidate in candidates {
        if let ItemState::Live(current) = candidate.state() {
            live_candidate_count += 1;
            if current.schema() != document.schema()
                || current.created_at_ms() != document.created_at_ms()
            {
                return Err(ApplicationError::InvalidInput);
            }
        }
    }
    if live_candidate_count == 0 {
        return Err(ApplicationError::InvalidInput);
    }

    let causal_parents = candidates
        .iter()
        .map(ItemCandidate::revision_id)
        .collect::<BTreeSet<_>>();
    let publication = prepare_item_publication(
        active,
        current_items,
        keys,
        local_secret,
        document.id(),
        ItemState::Live(Box::new(document)),
        &causal_parents,
        AuditActionV1::ItemConflictMerge,
        None,
        wall_time_ms,
        &randomness.bytes,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

fn publish_mutation(
    active: &ActiveStateV1,
    repository: &dyn ApplicationRepository,
    publication: PublicationJournalV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    let intended_active = active.after_publication(&publication)?;
    let exact_active = LocalVaultStateV1::Active(active.clone()).encode()?;
    let exact_pending =
        LocalVaultStateV1::pending_publication(active.clone(), publication.clone())?.encode()?;
    let exact_intended = LocalVaultStateV1::Active(intended_active.clone()).encode()?;
    let locator = active.bootstrap_locator();

    match local_state_store.compare_exchange(locator, Some(&exact_active), &exact_pending) {
        Ok(()) => {}
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_pending => {}
                Some(observed) if observed == exact_intended => return Ok(intended_active),
                _ => return Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => return Err(map_local_state_store(error)),
    }

    let receipt = repository
        .publish(publication.publication(), publication.base_heads())
        .map_err(map_repository)?;
    if receipt.heads() != publication.expected_heads() {
        return Err(ApplicationError::IntegrityFailure);
    }

    match local_state_store.compare_exchange(locator, Some(&exact_pending), &exact_intended) {
        Ok(()) => Ok(intended_active),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_intended => Ok(intended_active),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn activate_audit_epoch(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    wall_time_ms: u64,
    randomness: AuditedAccessRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if active.audit_event_head().is_some() {
        return Err(ApplicationError::InvalidInput);
    }
    publish_audit_only_event_inner(
        active,
        keys,
        local_secret,
        repository,
        AuditActionV1::AuditEpochStart,
        AuditOutcomeV1::Succeeded,
        None,
        None,
        wall_time_ms,
        None,
        None,
        &randomness.bytes,
        local_state_store,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn activate_audit_epoch_for_test(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    wall_time_ms: u64,
    event_basis_override: Option<Vec<ObjectId>>,
    event_signing_seed_override: Option<[u8; 32]>,
    randomness: [u8; AUDIT_ONLY_TEST_RANDOM_BYTES],
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    publish_audit_only_event_inner(
        active,
        keys,
        local_secret,
        repository,
        AuditActionV1::AuditEpochStart,
        AuditOutcomeV1::Succeeded,
        None,
        None,
        wall_time_ms,
        event_basis_override,
        event_signing_seed_override,
        &randomness,
        local_state_store,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_audited_access(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    action: AuditActionV1,
    outcome: AuditOutcomeV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    wall_time_ms: u64,
    randomness: AuditedAccessRandomnessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    if active.audit_event_head().is_none()
        || (action.is_item_mutation() && outcome == AuditOutcomeV1::Succeeded)
        || matches!(
            action,
            AuditActionV1::AuditEpochStart | AuditActionV1::VaultInitialize
        )
        || (action == AuditActionV1::PortableImport && outcome == AuditOutcomeV1::Succeeded)
    {
        return Err(ApplicationError::InvalidInput);
    }
    publish_audit_only_event_inner(
        active,
        keys,
        local_secret,
        repository,
        action,
        outcome,
        item_id,
        selected_revision,
        wall_time_ms,
        None,
        None,
        &randomness.bytes,
        local_state_store,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_audit_only_event_for_test(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    action: AuditActionV1,
    outcome: AuditOutcomeV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    wall_time_ms: u64,
    event_basis_override: Option<Vec<ObjectId>>,
    event_signing_seed_override: Option<[u8; 32]>,
    randomness: [u8; AUDIT_ONLY_TEST_RANDOM_BYTES],
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    publish_audit_only_event_inner(
        active,
        keys,
        local_secret,
        repository,
        action,
        outcome,
        item_id,
        selected_revision,
        wall_time_ms,
        event_basis_override,
        event_signing_seed_override,
        &randomness,
        local_state_store,
    )
}

#[allow(clippy::too_many_arguments)]
fn publish_audit_only_event_inner(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    repository: &dyn ApplicationRepository,
    action: AuditActionV1,
    outcome: AuditOutcomeV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    wall_time_ms: u64,
    event_basis_override: Option<Vec<ObjectId>>,
    event_signing_seed_override: Option<[u8; 32]>,
    randomness: &[u8; AUDITED_ACCESS_RANDOM_BYTES],
    local_state_store: &dyn LocalStateStore,
) -> Result<ActiveStateV1, ApplicationError> {
    let device_counter = active
        .last_device_counter()
        .checked_add(1)
        .ok_or(ApplicationError::BoundExceeded)?;
    let mut offset = 0;
    let trace_id = OperationId::new(take_slice(randomness, &mut offset));
    let audit_randomness = take_object_randomness_slice(randomness, &mut offset);
    let commit_randomness = take_object_randomness_slice(randomness, &mut offset);
    debug_assert_eq!(offset, AUDITED_ACCESS_RANDOM_BYTES);

    let mut parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
    parents.sort_unstable();
    let event_basis = event_basis_override.unwrap_or_else(|| parents.clone());
    let event = AuditEventV1::new(
        active.vault_id(),
        active.device_id(),
        device_counter,
        trace_id,
        action,
        outcome,
        item_id,
        selected_revision,
        None,
        active.audit_event_head(),
        event_basis,
        wall_time_ms,
    )
    .map_err(|_| ApplicationError::InternalInvariant)?
    .sign(
        event_signing_seed_override
            .as_ref()
            .unwrap_or(local_secret.device_signing_seed()),
    )
    .map_err(|_| ApplicationError::InternalInvariant)?;
    let event_plaintext = Zeroizing::new(encode_signed_audit_event(&event)?);
    let event_frame = seal_object(
        keys,
        ObjectKind::AuditEvent,
        &event_plaintext,
        &audit_randomness,
    )?;
    let event_id = event_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;

    let added_objects = vec![event_id];
    let (_, device_signing_secret) = generate_keypair(local_secret.device_signing_seed());
    let device_signing_secret = Zeroizing::new(device_signing_secret);
    let unsigned_commit = CommitV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        parents,
        catalog_root: active.catalog_root(),
        added_objects,
        tombstone_root: None,
        wall_time_ms,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let commit_preimage = unsigned_commit
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let commit = unsigned_commit.with_signature(Signature::new(sign(
        &commit_preimage,
        &device_signing_secret,
    )));
    let commit_plaintext = Zeroizing::new(encode_signed_commit(&commit)?);
    let commit_frame = seal_object(
        keys,
        ObjectKind::Commit,
        &commit_plaintext,
        &commit_randomness,
    )?;
    let commit_id = commit_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let unsigned_announcement = AnnouncementV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        commit_id,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let announcement_preimage = unsigned_announcement
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let announcement = unsigned_announcement
        .with_signature(Signature::new(sign(
            &announcement_preimage,
            &device_signing_secret,
        )))
        .encode()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let expected_heads =
        PinnedHeads::new([commit_id]).map_err(|_| ApplicationError::InternalInvariant)?;
    let publication = PublicationJournalV1::new_audit_only(
        vec![event_frame],
        commit_frame,
        announcement,
        active.pinned_heads().clone(),
        expected_heads,
        device_counter,
        active.catalog_root(),
        event_id,
    )?;
    publish_mutation(active, repository, publication, local_state_store)
}

#[allow(clippy::too_many_arguments)]
fn prepare_item_publication(
    active: &ActiveStateV1,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    item_id: ItemId,
    item_state: ItemState,
    causal_parents: &BTreeSet<RevisionId>,
    audit_action: AuditActionV1,
    selected_revision: Option<RevisionId>,
    wall_time_ms: u64,
    randomness: &[u8; REPLACE_ITEM_RANDOM_BYTES],
) -> Result<PublicationJournalV1, ApplicationError> {
    prepare_item_publication_with_objects(
        active,
        current_items,
        keys,
        local_secret,
        item_id,
        item_state,
        causal_parents,
        audit_action,
        selected_revision,
        wall_time_ms,
        Vec::new(),
        randomness,
    )
}

/// The one item-mutation publication builder, with room for content objects
/// that the new revision references but does not contain.
///
/// `content_objects` exists for VLT-PM47 attachments and is the whole reason
/// an attachment write needs no durable machinery of its own: the sealed chunk
/// and manifest frames are simply more objects in the *same* journal, the same
/// commit, and the same `publish_mutation` compare-exchange pair. They are
/// sealed by the caller, because only the caller knows how many there are.
///
/// `randomness` is a slice rather than a fixed array for the same reason: an
/// attachment's entropy requirement depends on its chunk count. Every caller's
/// partition is still exact, and the assertion at the end of the partition is
/// what says so.
#[allow(clippy::too_many_arguments)]
fn prepare_item_publication_with_objects(
    active: &ActiveStateV1,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    item_id: ItemId,
    item_state: ItemState,
    causal_parents: &BTreeSet<RevisionId>,
    audit_action: AuditActionV1,
    selected_revision: Option<RevisionId>,
    wall_time_ms: u64,
    content_objects: Vec<ObjectFrameV1>,
    randomness: &[u8],
) -> Result<PublicationJournalV1, ApplicationError> {
    let device_counter = active
        .last_device_counter()
        .checked_add(1)
        .ok_or(ApplicationError::BoundExceeded)?;
    if randomness.len() != REPLACE_ITEM_RANDOM_BYTES {
        return Err(ApplicationError::InternalInvariant);
    }
    let mut offset = 0;
    let revision_randomness = take_object_randomness_slice(randomness, &mut offset);
    let catalog_randomness = take_object_randomness_slice(randomness, &mut offset);
    let commit_randomness = take_object_randomness_slice(randomness, &mut offset);
    let trace_id = OperationId::new(take_slice(randomness, &mut offset));
    let audit_randomness = take_object_randomness_slice(randomness, &mut offset);
    debug_assert_eq!(offset, REPLACE_ITEM_RANDOM_BYTES);

    let mut content_object_ids = Vec::with_capacity(content_objects.len());
    for frame in &content_objects {
        content_object_ids.push(
            frame
                .id()
                .map_err(|_| ApplicationError::InternalInvariant)?,
        );
    }

    let revision_plaintext = Zeroizing::new(encode_item_revision(causal_parents, &item_state)?);
    let revision_frame = seal_object(
        keys,
        ObjectKind::ItemRevision,
        &revision_plaintext,
        &revision_randomness,
    )?;
    let revision_object_id = revision_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let revision_id = RevisionId::new(*revision_object_id.as_bytes());

    let previous_entry_count = current_items.len();
    let mut catalog_entries = BTreeMap::new();
    for (existing_item_id, candidates) in current_items {
        let mut revision_ids = candidates
            .iter()
            .map(ItemCandidate::revision_id)
            .collect::<Vec<_>>();
        revision_ids.sort_unstable();
        revision_ids.dedup();
        catalog_entries.insert(*existing_item_id, revision_ids);
    }
    catalog_entries.insert(item_id, vec![revision_id]);
    // `new_for_mutation`, not `CatalogV1::new`: this rebuild carries
    // forward every entry `current_items` already had and touches exactly
    // one, so it is only genuine growth when `item_id` was not already a
    // key -- and only genuine growth should be capped by the tight
    // admission ceiling. See VLT-PM05 §13.4 and that constructor's doc
    // comment: applying the tight bound unconditionally here would refuse
    // an ordinary edit or delete on a catalog that is already open but
    // sits above the admission ceiling, reopening the bug this fix exists
    // to close.
    let catalog_plaintext = Zeroizing::new(
        CatalogV1::new_for_mutation(catalog_entries, previous_entry_count)?.encode()?,
    );
    let catalog_frame = seal_object(
        keys,
        ObjectKind::Catalog,
        &catalog_plaintext,
        &catalog_randomness,
    )?;
    let catalog_id = catalog_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;

    let mut parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
    parents.sort_unstable();
    let audit_event = prepare_mutation_audit_event(
        active,
        keys,
        local_secret,
        device_counter,
        trace_id,
        audit_action,
        Some(item_id),
        selected_revision,
        Some(revision_id),
        parents.clone(),
        wall_time_ms,
        &audit_randomness,
    )?;
    let mut added_objects = vec![revision_object_id, catalog_id];
    added_objects.extend_from_slice(&content_object_ids);
    if let Some((_, audit_id)) = &audit_event {
        added_objects.push(*audit_id);
    }
    added_objects.sort_unstable();
    added_objects.dedup();
    // Deduplication must not have removed anything: every id here names a
    // distinct freshly sealed frame, so a collision would mean two frames were
    // sealed with the same randomness. That is a fault, not a saving.
    let expected_object_count = 2 + content_object_ids.len() + usize::from(audit_event.is_some());
    if added_objects.len() != expected_object_count {
        return Err(ApplicationError::InternalInvariant);
    }
    let (_, device_signing_secret) = generate_keypair(local_secret.device_signing_seed());
    let device_signing_secret = Zeroizing::new(device_signing_secret);
    let unsigned_commit = CommitV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        parents,
        catalog_root: catalog_id,
        added_objects,
        tombstone_root: None,
        wall_time_ms,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let commit_preimage = unsigned_commit
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let commit = unsigned_commit.with_signature(Signature::new(sign(
        &commit_preimage,
        &device_signing_secret,
    )));
    let commit_plaintext = Zeroizing::new(encode_signed_commit(&commit)?);
    let commit_frame = seal_object(
        keys,
        ObjectKind::Commit,
        &commit_plaintext,
        &commit_randomness,
    )?;
    let commit_id = commit_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let unsigned_announcement = AnnouncementV1 {
        vault_id: active.vault_id(),
        device_id: active.device_id(),
        device_counter,
        commit_id,
        device_certificate: active.device_certificate_id(),
        signature: Signature::new([0; 64]),
    };
    let announcement_preimage = unsigned_announcement
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let announcement = unsigned_announcement
        .with_signature(Signature::new(sign(
            &announcement_preimage,
            &device_signing_secret,
        )))
        .encode()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let expected_heads =
        PinnedHeads::new([commit_id]).map_err(|_| ApplicationError::InternalInvariant)?;

    let mut objects = vec![revision_frame, catalog_frame];
    objects.extend(content_objects);
    let audit_event_head = audit_event.map(|(frame, id)| {
        objects.push(frame);
        id
    });
    let publication = PublicationJournalV1::new(
        objects,
        commit_frame,
        announcement,
        active.pinned_heads().clone(),
        expected_heads,
        device_counter,
        catalog_id,
    )?;
    match audit_event_head {
        Some(head) => publication.with_audit_event_head(head),
        None => Ok(publication),
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_mutation_audit_event(
    active: &ActiveStateV1,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    device_counter: u64,
    trace_id: OperationId,
    action: AuditActionV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    result_revision: Option<RevisionId>,
    basis_heads: Vec<ObjectId>,
    timestamp_ms: u64,
    randomness: &ObjectRandomness,
) -> Result<Option<(ObjectFrameV1, ObjectId)>, ApplicationError> {
    let Some(previous_event) = active.audit_event_head() else {
        return Ok(None);
    };
    if !action.is_item_mutation() && action != AuditActionV1::PortableImport {
        return Err(ApplicationError::InternalInvariant);
    }
    let event = AuditEventV1::new(
        active.vault_id(),
        active.device_id(),
        device_counter,
        trace_id,
        action,
        AuditOutcomeV1::Succeeded,
        item_id,
        selected_revision,
        result_revision,
        Some(previous_event),
        basis_heads,
        timestamp_ms,
    )
    .map_err(|_| ApplicationError::InternalInvariant)?
    .sign(local_secret.device_signing_seed())
    .map_err(|_| ApplicationError::InternalInvariant)?;
    let plaintext = Zeroizing::new(encode_signed_audit_event(&event)?);
    let frame = seal_object(keys, ObjectKind::AuditEvent, &plaintext, randomness)?;
    let id = frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(Some((frame, id)))
}

fn take_object_randomness_slice(bytes: &[u8], offset: &mut usize) -> ObjectRandomness {
    ObjectRandomness::new(
        take_slice(bytes, offset),
        take_slice(bytes, offset),
        take_slice(bytes, offset),
    )
}

fn take_slice<const N: usize>(bytes: &[u8], offset: &mut usize) -> [u8; N] {
    let end = offset
        .checked_add(N)
        .expect("validated import randomness offset cannot overflow");
    let value = bytes[*offset..end]
        .try_into()
        .expect("validated import randomness partition must be exact");
    *offset = end;
    value
}

fn map_repository(error: ApplicationRepositoryError) -> ApplicationError {
    match error {
        ApplicationRepositoryError::NotInitialized => ApplicationError::NotInitialized,
        ApplicationRepositoryError::InvalidInput => ApplicationError::InvalidInput,
        ApplicationRepositoryError::BoundExceeded => ApplicationError::BoundExceeded,
        ApplicationRepositoryError::StorageUnavailable => ApplicationError::StorageUnavailable,
        ApplicationRepositoryError::IntegrityFailure => ApplicationError::IntegrityFailure,
    }
}

fn map_local_state_store(error: LocalStateStoreError) -> ApplicationError {
    match error {
        LocalStateStoreError::Unavailable => ApplicationError::StorageUnavailable,
        LocalStateStoreError::ConcurrentHost => ApplicationError::ConcurrentHost,
        LocalStateStoreError::Corruption => ApplicationError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audited_access_randomness_redacts_and_zeroizes() {
        let mut randomness = AuditedAccessRandomnessV1::new([0x96; AUDITED_ACCESS_RANDOM_BYTES]);

        assert_eq!(
            format!("{randomness:?}"),
            "AuditedAccessRandomnessV1(<redacted>)"
        );
        assert!(randomness.bytes.iter().all(|byte| *byte == 0x96));

        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn add_item_randomness_redacts_and_zeroizes() {
        let mut randomness = AddItemRandomnessV1::new([0x5a; ADD_ITEM_RANDOM_BYTES]);

        assert_eq!(format!("{randomness:?}"), "AddItemRandomnessV1(<redacted>)");
        assert_eq!(randomness.item_id(), ItemId::new([0x5a; ITEM_ID_BYTES]));

        randomness.zeroize();
        assert_eq!(randomness.item_id(), ItemId::new([0; ITEM_ID_BYTES]));
    }

    #[test]
    fn replace_item_randomness_redacts_and_zeroizes() {
        let mut randomness = ReplaceItemRandomnessV1::new([0xa5; REPLACE_ITEM_RANDOM_BYTES]);

        assert_eq!(
            format!("{randomness:?}"),
            "ReplaceItemRandomnessV1(<redacted>)"
        );
        assert!(randomness.bytes.iter().all(|byte| *byte == 0xa5));

        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn delete_item_randomness_redacts_and_zeroizes() {
        let mut randomness = DeleteItemRandomnessV1::new([0x3c; DELETE_ITEM_RANDOM_BYTES]);

        assert_eq!(
            format!("{randomness:?}"),
            "DeleteItemRandomnessV1(<redacted>)"
        );
        assert!(randomness.bytes.iter().all(|byte| *byte == 0x3c));

        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn restore_item_randomness_redacts_and_zeroizes() {
        let mut randomness = RestoreItemRandomnessV1::new([0xc3; RESTORE_ITEM_RANDOM_BYTES]);

        assert_eq!(
            format!("{randomness:?}"),
            "RestoreItemRandomnessV1(<redacted>)"
        );
        assert!(randomness.bytes.iter().all(|byte| *byte == 0xc3));

        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn conflict_resolution_randomness_redacts_and_zeroizes() {
        let mut randomness =
            ResolveItemConflictRandomnessV1::new([0x6d; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES]);

        assert_eq!(
            format!("{randomness:?}"),
            "ResolveItemConflictRandomnessV1(<redacted>)"
        );
        assert!(randomness.bytes.iter().all(|byte| *byte == 0x6d));

        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn portable_import_randomness_is_exact_bounded_redacted_and_zeroizing() {
        assert_eq!(portable_import_random_bytes_for_counts(2, 3), Ok(544));
        assert_eq!(
            portable_import_random_bytes_for_counts(1, MAX_PUBLICATION_OBJECTS),
            Err(ApplicationError::BoundExceeded)
        );
        assert_eq!(
            portable_import_random_bytes_for_counts(usize::MAX, 0),
            Err(ApplicationError::BoundExceeded)
        );

        let mut randomness = PortableImportRandomnessV1 {
            bytes: vec![0x87; 544],
            item_count: 2,
            candidate_count: 3,
        };
        assert_eq!(
            format!("{randomness:?}"),
            "PortableImportRandomnessV1(<redacted>)"
        );
        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
        assert_eq!(randomness.item_count, 0);
        assert_eq!(randomness.candidate_count, 0);
    }

    #[test]
    fn mutation_error_translation_is_closed() {
        assert_eq!(
            map_repository(ApplicationRepositoryError::NotInitialized),
            ApplicationError::NotInitialized
        );
        assert_eq!(
            map_repository(ApplicationRepositoryError::InvalidInput),
            ApplicationError::InvalidInput
        );
        assert_eq!(
            map_repository(ApplicationRepositoryError::BoundExceeded),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_repository(ApplicationRepositoryError::StorageUnavailable),
            ApplicationError::StorageUnavailable
        );
        assert_eq!(
            map_repository(ApplicationRepositoryError::IntegrityFailure),
            ApplicationError::IntegrityFailure
        );
        assert_eq!(
            map_local_state_store(LocalStateStoreError::Unavailable),
            ApplicationError::StorageUnavailable
        );
        assert_eq!(
            map_local_state_store(LocalStateStoreError::ConcurrentHost),
            ApplicationError::ConcurrentHost
        );
        assert_eq!(
            map_local_state_store(LocalStateStoreError::Corruption),
            ApplicationError::IntegrityFailure
        );
    }
}
