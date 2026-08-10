use crate::{
    encode_item_revision, encode_signed_commit, seal_object, ActiveStateV1, ApplicationError,
    ApplicationRepository, ApplicationRepositoryError, CatalogV1, LocalSecretV1, LocalStateStore,
    LocalStateStoreError, LocalVaultStateV1, ObjectKind, ObjectRandomness, PublicationJournalV1,
    V1Keys,
};
use coding_adventures_ed25519::{generate_keypair, sign};
use coding_adventures_vault_pm_domain::{
    ItemCandidate, ItemDocument, ItemId, ItemState, RevisionId,
};
use coding_adventures_vault_pm_format::{AnnouncementV1, CommitV1, Signature};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const ITEM_ID_BYTES: usize = 16;
const OBJECT_RANDOM_BYTES: usize = 32 + 24 + 24;

/// Exact caller-filled CSPRNG bytes consumed by one add-item mutation.
pub const ADD_ITEM_RANDOM_BYTES: usize = ITEM_ID_BYTES + 3 * OBJECT_RANDOM_BYTES;

/// Owned wipe-on-drop entropy for one item ID and three encrypted frames.
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

    let publication = prepare_add_publication(
        active,
        current_items,
        keys,
        local_secret,
        document,
        wall_time_ms,
        randomness,
    )?;
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

fn prepare_add_publication(
    active: &ActiveStateV1,
    current_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    keys: &V1Keys,
    local_secret: &LocalSecretV1,
    document: ItemDocument,
    wall_time_ms: u64,
    randomness: AddItemRandomnessV1,
) -> Result<PublicationJournalV1, ApplicationError> {
    let device_counter = active
        .last_device_counter()
        .checked_add(1)
        .ok_or(ApplicationError::BoundExceeded)?;
    let mut offset = ITEM_ID_BYTES;
    let revision_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    let catalog_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    let commit_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    debug_assert_eq!(offset, ADD_ITEM_RANDOM_BYTES);

    let item_id = document.id();
    let item_state = ItemState::Live(Box::new(document));
    let revision_plaintext = Zeroizing::new(encode_item_revision(&BTreeSet::new(), &item_state)?);
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
    let catalog_plaintext = Zeroizing::new(CatalogV1::new(catalog_entries)?.encode()?);
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
    let mut added_objects = vec![revision_object_id, catalog_id];
    added_objects.sort_unstable();
    added_objects.dedup();
    if added_objects.len() != 2 {
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

    PublicationJournalV1::new(
        vec![revision_frame, catalog_frame],
        commit_frame,
        announcement,
        active.pinned_heads().clone(),
        expected_heads,
        device_counter,
        catalog_id,
    )
}

fn take_object_randomness(
    bytes: &[u8; ADD_ITEM_RANDOM_BYTES],
    offset: &mut usize,
) -> ObjectRandomness {
    ObjectRandomness::new(
        take(bytes, offset),
        take(bytes, offset),
        take(bytes, offset),
    )
}

fn take<const N: usize>(bytes: &[u8; ADD_ITEM_RANDOM_BYTES], offset: &mut usize) -> [u8; N] {
    let end = *offset + N;
    let value = bytes[*offset..end]
        .try_into()
        .expect("add-item partition lengths are constant");
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
    fn add_item_randomness_redacts_and_zeroizes() {
        let mut randomness = AddItemRandomnessV1::new([0x5a; ADD_ITEM_RANDOM_BYTES]);

        assert_eq!(format!("{randomness:?}"), "AddItemRandomnessV1(<redacted>)");
        assert_eq!(randomness.item_id(), ItemId::new([0x5a; ITEM_ID_BYTES]));

        randomness.zeroize();
        assert_eq!(randomness.item_id(), ItemId::new([0; ITEM_ID_BYTES]));
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
