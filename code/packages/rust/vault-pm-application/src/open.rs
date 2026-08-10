use crate::initialize::{unlock_active_material, UnlockedActiveMaterial};
use crate::search::SearchProjectionV1;
use crate::{
    open_object, ActiveStateV1, ApplicationError, ApplicationRepository,
    ApplicationRepositoryError, ApplicationRepositoryFactory, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, CatalogV1, LocalSecretV1, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, ObjectKind, V1Keys,
};
use coding_adventures_vault_pm_domain::{
    CollectionId, ItemCandidate, ItemId, ItemState, RedactedItemView, RevisionId,
};
use coding_adventures_vault_pm_format::{DeviceId, ObjectId, VaultId};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

use crate::codec::{MAX_CANDIDATES_PER_ITEM, MAX_CATALOG_ENTRIES};

struct CurrentCatalogV1 {
    items: BTreeMap<ItemId, Vec<ItemCandidate>>,
    candidate_count: usize,
}

impl CurrentCatalogV1 {
    fn item_count(&self) -> usize {
        self.items.len()
    }

    const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    fn conflicted_item_count(&self) -> usize {
        self.items
            .values()
            .filter(|candidates| candidates.len() > 1)
            .count()
    }
}

/// One authenticated active-vault session with live keys and a verified
/// repository view.
///
/// Dropping the session wipes its application keys and owner/device secrets.
/// The repository owns a separate wipe-on-drop verifier key set.
pub struct UnlockedVaultV1 {
    active: ActiveStateV1,
    report: OpenReport,
    current_catalog: CurrentCatalogV1,
    search: SearchProjectionV1,
    _keys: V1Keys,
    _local_secret: LocalSecretV1,
    _repository: Box<dyn ApplicationRepository>,
}

impl UnlockedVaultV1 {
    /// Return the authenticated vault identity.
    pub const fn vault_id(&self) -> VaultId {
        self.active.vault_id()
    }

    /// Return the authenticated local device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.active.device_id()
    }

    /// Borrow the durable local head pins used to anchor this open.
    pub const fn local_pins(&self) -> &PinnedHeads {
        self.active.pinned_heads()
    }

    /// Borrow the complete payload-free verified repository report.
    pub const fn open_report(&self) -> &OpenReport {
        &self.report
    }

    /// Return the number of distinct current item identities without exposing
    /// any identity or item metadata.
    pub fn item_count(&self) -> usize {
        self.current_catalog.item_count()
    }

    /// Return the number of retained current revision candidates across all
    /// items. A value larger than [`Self::item_count`] indicates conflicts.
    pub const fn candidate_count(&self) -> usize {
        self.current_catalog.candidate_count()
    }

    /// Return how many current items retain more than one revision candidate.
    pub fn conflicted_item_count(&self) -> usize {
        self.current_catalog.conflicted_item_count()
    }

    /// Return the ordinary redacted view for one unambiguous live item.
    ///
    /// A missing item and a current tombstone both return `None`. Multiple
    /// retained candidates fail closed with [`ApplicationError::ConflictRequired`]
    /// rather than selecting a winner or exposing only part of the conflict.
    pub fn get_item(&self, item_id: ItemId) -> Result<Option<RedactedItemView>, ApplicationError> {
        let Some(candidates) = self.current_catalog.items.get(&item_id) else {
            return Ok(None);
        };
        project_current_item(candidates)
    }

    /// Return every unambiguous live item as an ordinary redacted view.
    ///
    /// Views are ordered by exact item-ID bytes. A current conflict aborts the
    /// complete read with [`ApplicationError::ConflictRequired`]; no partial
    /// list is returned and every retained candidate remains in the session.
    pub fn list_items(&self) -> Result<Vec<RedactedItemView>, ApplicationError> {
        let mut views = Vec::with_capacity(self.current_catalog.items.len());
        for candidates in self.current_catalog.items.values() {
            if let Some(view) = project_current_item(candidates)? {
                views.push(view);
            }
        }
        Ok(views)
    }

    /// Search unambiguous live items using only approved redacted metadata.
    ///
    /// The owned query is wiped on every return path. It must contain 1–256
    /// UTF-8 bytes and no control characters. Results optionally require
    /// membership in one explicit collection and are ordered by normalized
    /// display title, schema, then exact item-ID bytes. Any current conflict
    /// aborts the complete search without returning partial results.
    pub fn search_items(
        &self,
        query: Zeroizing<String>,
        collection: Option<CollectionId>,
        limit: usize,
    ) -> Result<Vec<RedactedItemView>, ApplicationError> {
        if self.current_catalog.conflicted_item_count() != 0 {
            return Err(ApplicationError::ConflictRequired);
        }
        self.search
            .search(query, collection, limit, &self.current_catalog.items)
    }

    /// Return how many unambiguous live items are held in the wipe-on-lock
    /// search projection without exposing item identities or indexed text.
    pub fn search_item_count(&self) -> usize {
        self.search.len()
    }
}

fn project_current_item(
    candidates: &[ItemCandidate],
) -> Result<Option<RedactedItemView>, ApplicationError> {
    let [candidate] = candidates else {
        return Err(ApplicationError::ConflictRequired);
    };
    match candidate.state() {
        ItemState::Live(document) => RedactedItemView::from_document(document)
            .map(Some)
            .map_err(|_| ApplicationError::InternalInvariant),
        ItemState::Tombstone(_) => Ok(None),
    }
}

impl Debug for UnlockedVaultV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnlockedVaultV1")
            .field("local_pin_count", &self.active.pinned_heads().len())
            .field("verified_head_count", &self.report.heads().len())
            .field("item_count", &self.current_catalog.item_count())
            .field("candidate_count", &self.current_catalog.candidate_count())
            .field(
                "conflicted_item_count",
                &self.current_catalog.conflicted_item_count(),
            )
            .field("search_item_count", &self.search.len())
            .finish_non_exhaustive()
    }
}

/// Authenticated-reopen one stable `Active` vault from injected byte stores.
///
/// This slice deliberately accepts only `Active`; callers must complete a
/// `PreparedInit` or recover a `PendingPublication` before invoking it. The
/// latest bootstrap must exactly match the locally pinned signed generation,
/// the passphrase must authenticate its root wrap, all private seeds must
/// reproduce pinned public identities, and the repository must open relative
/// to non-empty local pins.
pub fn open_active_vault(
    passphrase: Zeroizing<Vec<u8>>,
    locator: BootstrapLocator,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    repository_factory: &dyn ApplicationRepositoryFactory,
) -> Result<UnlockedVaultV1, ApplicationError> {
    let exact_state = local_state_store
        .load(locator)
        .map_err(map_local_state_store)?
        .ok_or(ApplicationError::NotInitialized)?;
    let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_state)? else {
        return Err(ApplicationError::InvalidInput);
    };
    if active.bootstrap_locator() != locator {
        return Err(ApplicationError::IntegrityFailure);
    }

    let exact_bootstrap = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    let material = unlock_active_material(passphrase, &active, &exact_bootstrap)?;
    let repository = repository_factory
        .connect(material.repository_address, Box::new(material.verifier))
        .map_err(map_repository)?;
    repository.initialize().map_err(map_repository)?;
    let report = repository
        .open(active.pinned_heads())
        .map_err(map_repository)?;
    if report.fresh_device_unanchored() || report.heads().is_empty() {
        return Err(ApplicationError::IntegrityFailure);
    }
    let current_catalog = materialize_current_catalog(
        &material.keys,
        repository.as_ref(),
        &report,
        active.vault_id(),
    )?;
    let search = SearchProjectionV1::build(&current_catalog.items)?;

    Ok(UnlockedVaultV1 {
        active,
        report,
        current_catalog,
        search,
        _keys: material.keys,
        _local_secret: material.local_secret,
        _repository: repository,
    })
}

fn materialize_current_catalog(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
) -> Result<CurrentCatalogV1, ApplicationError> {
    let mut materialized = BTreeMap::<ItemId, BTreeMap<RevisionId, ItemCandidate>>::new();
    let mut seen_catalogs = BTreeSet::new();

    for head_id in report.heads().iter().copied() {
        let commit = repository.read_commit(head_id).map_err(map_repository)?;
        if commit.id() != head_id || commit.vault_id() != vault_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let catalog_id = commit.catalog_root();
        if !seen_catalogs.insert(catalog_id) {
            continue;
        }
        let catalog_object = repository.read_object(catalog_id).map_err(map_repository)?;
        if catalog_object.id() != catalog_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let catalog_plaintext = open_object(keys, ObjectKind::Catalog, catalog_object.frame())?;
        let catalog = CatalogV1::decode(&catalog_plaintext)?;

        for (item_id, revision_ids) in catalog.entries() {
            if !materialized.contains_key(item_id) && materialized.len() == MAX_CATALOG_ENTRIES {
                return Err(ApplicationError::IntegrityFailure);
            }
            let candidates = materialized.entry(*item_id).or_default();
            for revision_id in revision_ids {
                if candidates.contains_key(revision_id) {
                    continue;
                }
                if candidates.len() == MAX_CANDIDATES_PER_ITEM {
                    return Err(ApplicationError::IntegrityFailure);
                }
                let candidate = read_candidate(keys, repository, *revision_id)?;
                if candidate.item_id() != *item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
                for parent_id in candidate.causal_parents() {
                    let parent = read_candidate(keys, repository, *parent_id)?;
                    if parent.item_id() != *item_id {
                        return Err(ApplicationError::IntegrityFailure);
                    }
                }
                candidates.insert(*revision_id, candidate);
            }
        }
    }

    let candidate_count = materialized.values().map(BTreeMap::len).sum();
    Ok(CurrentCatalogV1 {
        items: materialized
            .into_iter()
            .map(|(item_id, candidates)| (item_id, candidates.into_values().collect()))
            .collect(),
        candidate_count,
    })
}

fn read_candidate(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    revision_id: RevisionId,
) -> Result<ItemCandidate, ApplicationError> {
    let object_id = ObjectId::new(*revision_id.as_bytes());
    let revision_object = repository.read_object(object_id).map_err(map_repository)?;
    if revision_object.id() != object_id {
        return Err(ApplicationError::IntegrityFailure);
    }
    let revision_plaintext = open_object(keys, ObjectKind::ItemRevision, revision_object.frame())?;
    crate::decode_item_revision(revision_id, &revision_plaintext)
}

/// Replay one exact durable `PendingPublication` and atomically advance it to
/// `Active` only after the repository returns the journal's expected pins.
///
/// Provider ambiguity leaves the exact journal untouched for another retry.
/// A concurrent local writer is accepted only when it installed the identical
/// intended `Active` bytes; every other winner fails closed.
pub fn recover_pending_publication(
    passphrase: Zeroizing<Vec<u8>>,
    locator: BootstrapLocator,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    repository_factory: &dyn ApplicationRepositoryFactory,
) -> Result<ActiveStateV1, ApplicationError> {
    let exact_pending = local_state_store
        .load(locator)
        .map_err(map_local_state_store)?
        .ok_or(ApplicationError::NotInitialized)?;
    let LocalVaultStateV1::PendingPublication {
        active,
        publication,
    } = LocalVaultStateV1::decode(&exact_pending)?
    else {
        return Err(ApplicationError::InvalidInput);
    };
    if active.bootstrap_locator() != locator {
        return Err(ApplicationError::IntegrityFailure);
    }

    let exact_bootstrap = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    let UnlockedActiveMaterial {
        repository_address,
        keys: _keys,
        local_secret: _local_secret,
        verifier,
    } = unlock_active_material(passphrase, &active, &exact_bootstrap)?;
    let repository = repository_factory
        .connect(repository_address, Box::new(verifier))
        .map_err(map_repository)?;
    repository.initialize().map_err(map_repository)?;
    let receipt = repository
        .publish(publication.publication(), publication.base_heads())
        .map_err(map_repository)?;
    if receipt.heads() != publication.expected_heads() {
        return Err(ApplicationError::IntegrityFailure);
    }

    let intended_active = active.after_publication(&publication)?;
    let exact_active = LocalVaultStateV1::Active(intended_active.clone()).encode()?;
    match local_state_store.compare_exchange(locator, Some(&exact_pending), &exact_active) {
        Ok(()) => Ok(intended_active),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_active => Ok(intended_active),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

fn map_bootstrap_store(error: BootstrapStoreError) -> ApplicationError {
    match error {
        BootstrapStoreError::Unavailable => ApplicationError::StorageUnavailable,
        BootstrapStoreError::Conflict | BootstrapStoreError::Corruption => {
            ApplicationError::IntegrityFailure
        }
    }
}

fn map_local_state_store(error: LocalStateStoreError) -> ApplicationError {
    match error {
        LocalStateStoreError::Unavailable => ApplicationError::StorageUnavailable,
        LocalStateStoreError::ConcurrentHost => ApplicationError::ConcurrentHost,
        LocalStateStoreError::Corruption => ApplicationError::IntegrityFailure,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        complete_generation_zero, encode_item_revision, encode_signed_commit,
        prepare_generation_zero, seal_object, CatalogV1, GenerationZeroPolicyV1,
        GenerationZeroRandomness, ObjectKind, ObjectRandomness, PublicationJournalV1,
        V1ApplicationRepositoryFactory, V1Keys, GENERATION_ZERO_RANDOM_BYTES,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_domain::{
        ContentType, ItemDocument, ItemState, LwwRegister, ObservedSet, OperationId,
        RedactedRecordView, Tombstone,
    };
    use coding_adventures_vault_pm_format::{AnnouncementV1, BootstrapId, CommitV1, Signature};
    use coding_adventures_vault_pm_storage::{
        FaultAction, FaultEffect, FaultInjectingObjectStore, InMemoryObjectStore, StoreOperation,
    };
    use coding_adventures_vault_records::{AnyRecord, Login, LOGIN_V1};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    #[derive(Default)]
    struct MemoryLocalStateStore(
        Mutex<Option<Vec<u8>>>,
        AtomicBool,
        Mutex<Option<LocalStateStoreError>>,
    );

    impl MemoryLocalStateStore {
        fn with_state(state: Vec<u8>) -> Self {
            Self(
                Mutex::new(Some(state)),
                AtomicBool::new(false),
                Mutex::new(None),
            )
        }

        fn concurrent_winner_on_next_compare(&self) {
            self.1.store(true, Ordering::SeqCst);
        }

        fn fail_next_compare(&self, error: LocalStateStoreError) {
            *self.2.lock().unwrap() = Some(error);
        }
    }

    impl LocalStateStore for MemoryLocalStateStore {
        fn load(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn compare_exchange(
            &self,
            _locator: BootstrapLocator,
            expected: Option<&[u8]>,
            replacement: &[u8],
        ) -> Result<(), LocalStateStoreError> {
            let mut state = self.0.lock().unwrap();
            if state.as_deref() != expected {
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            if let Some(error) = self.2.lock().unwrap().take() {
                return Err(error);
            }
            if self.1.swap(false, Ordering::SeqCst) {
                *state = Some(replacement.to_vec());
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            *state = Some(replacement.to_vec());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemoryBootstrapStore(Mutex<Option<Vec<u8>>>);

    impl BootstrapStore for MemoryBootstrapStore {
        fn load_latest(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn put_generation(
            &self,
            _locator: BootstrapLocator,
            expected_previous: Option<BootstrapId>,
            exact_bootstrap: &[u8],
        ) -> Result<(), BootstrapStoreError> {
            if expected_previous.is_some() {
                return Err(BootstrapStoreError::Conflict);
            }
            let mut stored = self.0.lock().unwrap();
            match &*stored {
                Some(existing) if existing == exact_bootstrap => Ok(()),
                Some(_) => Err(BootstrapStoreError::Conflict),
                None => {
                    *stored = Some(exact_bootstrap.to_vec());
                    Ok(())
                }
            }
        }
    }

    fn generation_zero_bytes() -> [u8; GENERATION_ZERO_RANDOM_BYTES] {
        let mut bytes = [0; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(7);
        }
        bytes
    }

    fn randomness() -> GenerationZeroRandomness {
        GenerationZeroRandomness::new(generation_zero_bytes())
    }

    fn initialized() -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
    ) {
        let passphrase = Zeroizing::new(b"active passphrase".to_vec());
        let prepared = prepare_generation_zero(
            passphrase,
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(InMemoryObjectStore::new());
        let factory = V1ApplicationRepositoryFactory::from_shared(backend);
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        (locator, local, bootstrap, factory)
    }

    fn pending_publication(active: &ActiveStateV1) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &CatalogV1::empty().encode().unwrap(),
            &ObjectRandomness::new([0xd1; 32], [0xd2; 24], [0xd3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, Vec::new(), catalog_frame)
    }

    fn pending_tombstone_publication(
        active: &ActiveStateV1,
        catalog_item_id: ItemId,
        revision_item_id: ItemId,
        candidate_count: usize,
        causal_parent: Option<RevisionId>,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revision_ids = Vec::new();
        for index in 0..candidate_count {
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                causal_parent,
                ItemState::Tombstone(Tombstone {
                    item_id: revision_item_id,
                    deleted_at_ms: 100 + index as u64,
                }),
            )
            .unwrap();
            let base = 0x40u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revision_ids.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revision_ids.sort_unstable();
        let catalog =
            CatalogV1::new(BTreeMap::from([(catalog_item_id, revision_ids.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xa1; 32], [0xa2; 24], [0xa3; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revision_ids,
        )
    }

    fn pending_live_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
        title: &str,
        password: &str,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut collections = ObservedSet::new();
        collections
            .add(CollectionId::new([0x82; 16]), OperationId::new([0x83; 32]))
            .unwrap();
        let mut tags = ObservedSet::new();
        tags.add("Finance".to_owned(), OperationId::new([0x84; 32]))
            .unwrap();
        let document = ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            100,
            200,
            LwwRegister::new(false, 200, OperationId::new([0x81; 32])),
            collections,
            tags,
            AnyRecord::Login(Login {
                title: title.into(),
                username: "ada@example.test".into(),
                password: password.into(),
                urls: vec!["https://example.test".into()],
                notes: Some("private note".into()),
            }),
            ObservedSet::new(),
        )
        .unwrap();
        let candidate = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [],
            ItemState::Live(Box::new(document)),
        )
        .unwrap();
        let revision_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
            &ObjectRandomness::new([0x91; 32], [0x92; 24], [0x93; 24]),
        )
        .unwrap();
        let revision_id = RevisionId::new(*revision_frame.id().unwrap().as_bytes());
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, vec![revision_id])])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xa4; 32], [0xa5; 24], [0xa6; 24]),
        )
        .unwrap();
        publication_for_catalog(active, vec![revision_frame], catalog_frame)
    }

    fn pending_dangling_catalog(active: &ActiveStateV1) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog = CatalogV1::new(BTreeMap::from([(
            ItemId::new([0x31; 16]),
            vec![RevisionId::new([0x32; 32])],
        )]))
        .unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xb1; 32], [0xb2; 24], [0xb3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, Vec::new(), catalog_frame)
    }

    fn pending_child_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
        parent_item_id: ItemId,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let parent = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [],
            ItemState::Tombstone(Tombstone {
                item_id: parent_item_id,
                deleted_at_ms: 100,
            }),
        )
        .unwrap();
        let parent_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(parent.causal_parents(), parent.state()).unwrap(),
            &ObjectRandomness::new([0xc1; 32], [0xc2; 24], [0xc3; 24]),
        )
        .unwrap();
        let parent_id = RevisionId::new(*parent_frame.id().unwrap().as_bytes());
        let child = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [parent_id],
            ItemState::Tombstone(Tombstone {
                item_id,
                deleted_at_ms: 101,
            }),
        )
        .unwrap();
        let child_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(child.causal_parents(), child.state()).unwrap(),
            &ObjectRandomness::new([0xd1; 32], [0xd2; 24], [0xd3; 24]),
        )
        .unwrap();
        let child_id = RevisionId::new(*child_frame.id().unwrap().as_bytes());
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, vec![child_id])])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xe1; 32], [0xe2; 24], [0xe3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, vec![parent_frame, child_frame], catalog_frame)
    }

    fn publication_for_catalog(
        active: &ActiveStateV1,
        mut objects: Vec<coding_adventures_vault_pm_format::ObjectFrameV1>,
        catalog_frame: coding_adventures_vault_pm_format::ObjectFrameV1,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let signing_seed: [u8; 32] = fixture[168..200].try_into().unwrap();
        let (_, signing_secret) = generate_keypair(&signing_seed);
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog_id = catalog_frame.id().unwrap();
        objects.push(catalog_frame);
        let mut added_objects = objects
            .iter()
            .map(|frame| frame.id().unwrap())
            .collect::<Vec<_>>();
        added_objects.sort_unstable();
        let parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
        let commit = CommitV1 {
            vault_id: active.vault_id(),
            device_id: active.device_id(),
            device_counter: active.last_device_counter() + 1,
            parents,
            catalog_root: catalog_id,
            added_objects,
            tombstone_root: None,
            wall_time_ms: 20,
            device_certificate: active.device_certificate_id(),
            signature: Signature::new([0; 64]),
        };
        let commit_preimage = commit.signing_preimage().unwrap();
        let commit = commit.with_signature(Signature::new(sign(&commit_preimage, &signing_secret)));
        let commit_frame = seal_object(
            &keys,
            ObjectKind::Commit,
            &encode_signed_commit(&commit).unwrap(),
            &ObjectRandomness::new([0xe1; 32], [0xe2; 24], [0xe3; 24]),
        )
        .unwrap();
        let commit_id = commit_frame.id().unwrap();
        let announcement = AnnouncementV1 {
            vault_id: active.vault_id(),
            device_id: active.device_id(),
            device_counter: commit.device_counter,
            commit_id,
            device_certificate: active.device_certificate_id(),
            signature: Signature::new([0; 64]),
        };
        let announcement_preimage = announcement.signing_preimage().unwrap();
        let announcement = announcement.with_signature(Signature::new(sign(
            &announcement_preimage,
            &signing_secret,
        )));
        PublicationJournalV1::new(
            objects,
            commit_frame,
            announcement.encode().unwrap(),
            active.pinned_heads().clone(),
            PinnedHeads::new([commit_id]).unwrap(),
            commit.device_counter,
            catalog_id,
        )
        .unwrap()
    }

    fn install_pending(local: &MemoryLocalStateStore) -> PublicationJournalV1 {
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let publication = pending_publication(&active);
        let pending = LocalVaultStateV1::pending_publication(active, publication.clone()).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        publication
    }

    #[test]
    fn active_vault_reopens_from_only_durable_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(session.local_pins(), session.open_report().heads());
        assert_eq!(session.vault_id(), session.active.vault_id());
        assert_eq!(session.device_id(), session.active.device_id());
        assert_eq!(session.open_report().announcement_count(), 1);
        assert_eq!(session.open_report().commit_count(), 1);
        assert!(!session.open_report().fresh_device_unanchored());
        assert_eq!(session.item_count(), 0);
        assert_eq!(session.candidate_count(), 0);
        assert_eq!(session.conflicted_item_count(), 0);
        assert_eq!(session.search_item_count(), 0);
        assert_eq!(
            format!("{session:?}"),
            "UnlockedVaultV1 { local_pin_count: 1, verified_head_count: 1, item_count: 0, candidate_count: 0, conflicted_item_count: 0, search_item_count: 0, .. }"
        );
    }

    #[test]
    fn active_open_materializes_every_current_revision_candidate() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x21; 16]);
        let (publication, expected_revision_ids) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.item_count(), 1);
        assert_eq!(session.candidate_count(), 2);
        assert_eq!(session.conflicted_item_count(), 1);
        let candidates = session.current_catalog.items.get(&item_id).unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(ItemCandidate::revision_id)
                .collect::<Vec<_>>(),
            expected_revision_ids
        );
        assert!(format!("{session:?}").contains("item_count: 1"));
        assert!(!format!("{session:?}").contains(&item_id.to_user_string()));
    }

    #[test]
    fn current_item_reads_return_only_typed_redacted_views() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x27; 16]);
        let title = "ÉCLAIR Personal portal";
        let password = "never-log-this-password";
        let publication = pending_live_publication(&active, item_id, title, password);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let view = session.get_item(item_id).unwrap().unwrap();
        assert_eq!(view.item_id, item_id);
        assert_eq!(view.schema.as_str(), LOGIN_V1);
        match &view.record {
            RedactedRecordView::Login {
                title: view_title,
                username,
                urls,
                password: redacted_password,
                has_notes,
            } => {
                assert_eq!(view_title, title);
                assert_eq!(username, "ada@example.test");
                assert_eq!(urls, &["https://example.test"]);
                assert_eq!(redacted_password.to_string(), "<redacted>");
                assert!(*has_notes);
            }
            _ => panic!("fixture must project as a login"),
        }
        assert_eq!(session.list_items().unwrap(), vec![view.clone()]);
        assert_eq!(session.get_item(ItemId::new([0x28; 16])).unwrap(), None);
        assert_eq!(session.search_item_count(), 1);
        for query in [
            "E\u{301}CLAIR",
            "ada@EXAMPLE",
            "AMPLE.TEST",
            "finance",
            "portal ada@example",
            "a",
        ] {
            assert_eq!(
                session
                    .search_items(Zeroizing::new(query.to_owned()), None, 10)
                    .unwrap(),
                vec![view.clone()]
            );
        }
        assert!(session
            .search_items(Zeroizing::new("   ".to_owned()), None, 10)
            .unwrap()
            .is_empty());
        assert_eq!(
            session
                .search_items(
                    Zeroizing::new("portal".to_owned()),
                    Some(CollectionId::new([0x82; 16])),
                    10,
                )
                .unwrap(),
            vec![view.clone()]
        );
        assert!(session
            .search_items(
                Zeroizing::new("portal".to_owned()),
                Some(CollectionId::new([0x85; 16])),
                10,
            )
            .unwrap()
            .is_empty());
        for secret in [password, "private note"] {
            assert!(session
                .search_items(Zeroizing::new(secret.to_owned()), None, 10)
                .unwrap()
                .is_empty());
        }
        for invalid in ["", "line\nbreak", "\0"] {
            assert_eq!(
                session.search_items(Zeroizing::new(invalid.to_owned()), None, 10),
                Err(ApplicationError::InvalidInput)
            );
        }
        assert_eq!(
            session.search_items(Zeroizing::new("x".repeat(257)), None, 10),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.search_items(Zeroizing::new("portal".to_owned()), None, 0),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.search_items(Zeroizing::new("portal".to_owned()), None, 10_001),
            Err(ApplicationError::BoundExceeded)
        );
        let debug = format!("{view:?}");
        assert!(!debug.contains(title));
        assert!(!debug.contains(password));
        assert!(!debug.contains(&item_id.to_user_string()));
    }

    #[test]
    fn current_item_reads_hide_tombstones_and_fail_closed_on_conflicts() {
        for candidate_count in [1, 2] {
            let (locator, local, bootstrap, factory) = initialized();
            let exact_active = local.0.lock().unwrap().clone().unwrap();
            let LocalVaultStateV1::Active(active) =
                LocalVaultStateV1::decode(&exact_active).unwrap()
            else {
                panic!("fixture must be active")
            };
            let item_id = ItemId::new([0x29; 16]);
            let (publication, _) =
                pending_tombstone_publication(&active, item_id, item_id, candidate_count, None);
            let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
            *local.0.lock().unwrap() = Some(pending.encode().unwrap());
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            let session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

            if candidate_count == 1 {
                assert_eq!(session.get_item(item_id).unwrap(), None);
                assert!(session.list_items().unwrap().is_empty());
            } else {
                assert_eq!(
                    session.get_item(item_id),
                    Err(ApplicationError::ConflictRequired)
                );
                assert_eq!(
                    session.list_items(),
                    Err(ApplicationError::ConflictRequired)
                );
                assert_eq!(session.candidate_count(), 2);
                assert_eq!(
                    session.search_items(Zeroizing::new("anything".to_owned()), None, 10),
                    Err(ApplicationError::ConflictRequired)
                );
            }
        }
    }

    #[test]
    fn active_open_rejects_catalog_revision_item_mismatch() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let (publication, _) = pending_tombstone_publication(
            &active,
            ItemId::new([0x21; 16]),
            ItemId::new([0x22; 16]),
            1,
            None,
        );
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_rejects_dangling_catalog_revision() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let publication = pending_dangling_catalog(&active);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_rejects_dangling_causal_parent() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x41; 16]);
        let (publication, _) = pending_tombstone_publication(
            &active,
            item_id,
            item_id,
            1,
            Some(RevisionId::new([0x42; 32])),
        );
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_validates_existing_causal_parent_item_binding() {
        for matching_parent in [true, false] {
            let (locator, local, bootstrap, factory) = initialized();
            let exact_active = local.0.lock().unwrap().clone().unwrap();
            let LocalVaultStateV1::Active(active) =
                LocalVaultStateV1::decode(&exact_active).unwrap()
            else {
                panic!("fixture must be active")
            };
            let item_id = ItemId::new([0x51; 16]);
            let parent_item_id = if matching_parent {
                item_id
            } else {
                ItemId::new([0x52; 16])
            };
            let publication = pending_child_publication(&active, item_id, parent_item_id);
            let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
            *local.0.lock().unwrap() = Some(pending.encode().unwrap());
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

            let result = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            );
            if matching_parent {
                let session = result.unwrap();
                assert_eq!(session.item_count(), 1);
                assert_eq!(session.candidate_count(), 1);
            } else {
                assert_eq!(result.err(), Some(ApplicationError::IntegrityFailure));
            }
        }
    }

    #[test]
    fn active_open_closes_wrong_passphrase_and_bootstrap_rollback() {
        let (locator, local, bootstrap, factory) = initialized();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"wrong".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );

        bootstrap.0.lock().unwrap().as_mut().unwrap()[0] ^= 1;
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_requires_matching_locator_and_stable_state() {
        let (locator, local, bootstrap, factory) = initialized();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                BootstrapLocator::new([0x99; 32]),
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        *local.0.lock().unwrap() = None;
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::NotInitialized)
        );
    }

    #[test]
    fn active_open_rejects_unfinished_initialization() {
        let prepared = prepare_generation_zero(
            Zeroizing::new(b"active passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        let bootstrap = MemoryBootstrapStore::default();
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
    }

    #[test]
    fn pending_publication_replays_exactly_and_advances_active_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let publication = install_pending(&local);

        let active = recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert_eq!(active.last_device_counter(), 2);
        assert_eq!(active.catalog_root(), publication.catalog_root());
        assert!(matches!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(_)
        ));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.open_report().commit_count(), 2);
        assert_eq!(session.open_report().heads(), publication.expected_heads());
    }

    #[test]
    fn pending_recovery_retains_exact_journal_across_ambiguous_provider_failure() {
        let passphrase = b"active passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        let publication = install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );

        let active = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn pending_recovery_authenticates_before_any_repository_effect() {
        let (locator, local, bootstrap, factory) = initialized();
        install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(b"wrong".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );
    }

    #[test]
    fn pending_recovery_accepts_only_an_identical_concurrent_active_winner() {
        let (locator, local, bootstrap, factory) = initialized();
        let publication = install_pending(&local);
        local.concurrent_winner_on_next_compare();

        let active = recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert!(matches!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(_)
        ));
    }

    #[test]
    fn pending_recovery_retains_journal_when_final_local_commit_fails() {
        let (locator, local, bootstrap, factory) = initialized();
        install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        local.fail_next_compare(LocalStateStoreError::Unavailable);

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );

        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
    }
}
