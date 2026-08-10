use crate::initialize::{unlock_active_material, UnlockedActiveMaterial};
use crate::mutation::{
    add_item, delete_item, replace_item, restore_item, AddItemRandomnessV1, DeleteItemRandomnessV1,
    ReplaceItemRandomnessV1, RestoreItemRandomnessV1,
};
use crate::search::SearchProjectionV1;
use crate::{
    open_object, ActiveStateV1, ApplicationError, ApplicationRepository,
    ApplicationRepositoryError, ApplicationRepositoryFactory, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, CatalogV1, LocalSecretV1, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, ObjectKind, V1Keys,
};
use coding_adventures_vault_pm_domain::{
    CollectionId, ItemCandidate, ItemDocument, ItemId, ItemState, RedactedItemView, RevisionId,
};
use coding_adventures_vault_pm_format::{DeviceId, ObjectId, VaultId};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

use crate::codec::{MAX_CANDIDATES_PER_ITEM, MAX_CATALOG_ENTRIES};

/// Default maximum number of historical revisions returned for one item.
pub const DEFAULT_ITEM_HISTORY_LIMIT: usize = 100;
/// Hard maximum number of historical revisions returned for one item.
pub const MAX_ITEM_HISTORY_LIMIT: usize = 4_096;

/// One secret-free historical item revision projection.
#[derive(Clone, PartialEq, Eq)]
pub struct ItemHistoryViewV1 {
    revision_id: RevisionId,
    redacted_item: Option<RedactedItemView>,
    causal_parent_count: usize,
    advisory_time_ms: u64,
}

impl ItemHistoryViewV1 {
    fn from_candidate(candidate: &ItemCandidate) -> Result<Self, ApplicationError> {
        let (redacted_item, advisory_time_ms) = match candidate.state() {
            ItemState::Live(document) => (
                Some(
                    RedactedItemView::from_document(document)
                        .map_err(|_| ApplicationError::InternalInvariant)?,
                ),
                document.updated_at_ms(),
            ),
            ItemState::Tombstone(tombstone) => (None, tombstone.deleted_at_ms),
        };
        Ok(Self {
            revision_id: candidate.revision_id(),
            redacted_item,
            causal_parent_count: candidate.causal_parents().len(),
            advisory_time_ms,
        })
    }

    /// Return the exact encrypted revision object identity.
    pub const fn revision_id(&self) -> RevisionId {
        self.revision_id
    }

    /// Borrow safe live metadata, or `None` when this revision is a tombstone.
    pub const fn redacted_item(&self) -> Option<&RedactedItemView> {
        self.redacted_item.as_ref()
    }

    /// Return whether this historical revision is a deletion marker.
    pub const fn is_deleted(&self) -> bool {
        self.redacted_item.is_none()
    }

    /// Return the number of direct causal parents named by this revision.
    pub const fn causal_parent_count(&self) -> usize {
        self.causal_parent_count
    }

    /// Return the document-update or tombstone-deletion advisory time.
    pub const fn advisory_time_ms(&self) -> u64 {
        self.advisory_time_ms
    }
}

impl Debug for ItemHistoryViewV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ItemHistoryViewV1")
            .field("revision_id", &"<redacted>")
            .field("is_deleted", &self.is_deleted())
            .field("causal_parent_count", &self.causal_parent_count)
            .field("advisory_time_ms", &self.advisory_time_ms)
            .finish_non_exhaustive()
    }
}

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

    /// Return bounded secret-free history for one item across every current
    /// repository head.
    ///
    /// Traversal is newest ancestry depth first. Commits at the same depth and
    /// revisions in the same catalog are ordered by exact object ID. Revisions
    /// reached through more than one head are returned once. `limit` must be
    /// between 1 and [`MAX_ITEM_HISTORY_LIMIT`], inclusive.
    pub fn item_history(
        &self,
        item_id: ItemId,
        limit: usize,
    ) -> Result<Vec<ItemHistoryViewV1>, ApplicationError> {
        materialize_item_history_candidates(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            item_id,
            limit,
        )?
        .iter()
        .map(ItemHistoryViewV1::from_candidate)
        .collect()
    }

    /// Add one new item through the exact crash-resumable publication state
    /// machine and return the resulting durable active owner state.
    ///
    /// The session is consumed so a successful caller cannot keep using stale
    /// pins, catalog contents, or search state. The document and randomness
    /// are owned and wiped on every return path. Hosts must reopen a new
    /// session after success or recover the durable pending journal after an
    /// interrupted provider/local effect.
    pub fn add_item(
        self,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: AddItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        add_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Replace the sole expected current live revision and return the resulting
    /// durable active owner state.
    ///
    /// The replacement preserves item identity, content schema, and creation
    /// time. Its new revision directly names `expected_revision` as its causal
    /// parent. A missing item returns `NotFound`; a stale, tombstoned, or
    /// conflicted current candidate returns `ConflictRequired`. The session and
    /// all owned mutation inputs are consumed on every return path.
    pub fn replace_item(
        self,
        expected_revision: RevisionId,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: ReplaceItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        replace_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            expected_revision,
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Delete the sole expected current live revision by publishing a causal
    /// tombstone and return the resulting durable active owner state.
    ///
    /// A revision absent from the current catalog returns `NotFound`; a
    /// conflicted or already-tombstoned target returns `ConflictRequired`.
    /// Advisory deletion and commit times are supplied separately and do not
    /// establish causality. The session and randomness are consumed on every
    /// return path.
    pub fn delete_item(
        self,
        expected_revision: RevisionId,
        deleted_at_ms: u64,
        wall_time_ms: u64,
        randomness: DeleteItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        delete_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            expected_revision,
            deleted_at_ms,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Restore one reachable historical live revision as a new current live
    /// revision and return the resulting durable active owner state.
    ///
    /// The selected revision must be reachable within the hard history bound,
    /// belong to an item with exactly one current candidate, and differ from
    /// that current revision. Tombstones cannot be restored. The new revision
    /// copies the selected live document and names only the selected revision
    /// as its direct causal parent; repository heads are never rewound.
    pub fn restore_item(
        self,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        randomness: RestoreItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let selected = find_reachable_historical_candidate(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            selected_revision,
        )?;
        restore_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            selected,
            wall_time_ms,
            randomness,
            local_state_store,
        )
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

fn materialize_item_history_candidates(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
    item_id: ItemId,
    limit: usize,
) -> Result<Vec<ItemCandidate>, ApplicationError> {
    if limit == 0 {
        return Err(ApplicationError::InvalidInput);
    }
    if limit > MAX_ITEM_HISTORY_LIMIT {
        return Err(ApplicationError::BoundExceeded);
    }

    let mut histories = Vec::with_capacity(report.heads().len());
    for head_id in report.heads().iter().copied() {
        let history = repository.history(head_id, limit).map_err(map_repository)?;
        if history.first().map(|commit| commit.id()) != Some(head_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        histories.push(history);
    }

    let mut seen_commits = BTreeSet::new();
    let mut seen_catalogs = BTreeSet::new();
    let mut seen_revisions = BTreeSet::new();
    let mut candidates = Vec::new();

    for depth in 0..limit {
        let mut commits = histories
            .iter()
            .filter_map(|history| history.get(depth))
            .filter(|commit| !seen_commits.contains(&commit.id()))
            .collect::<Vec<_>>();
        commits.sort_unstable_by_key(|commit| commit.id());

        for commit in commits {
            if !seen_commits.insert(commit.id()) {
                continue;
            }
            if commit.vault_id() != vault_id {
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
            let Some(revision_ids) = catalog.entries().get(&item_id) else {
                continue;
            };

            for revision_id in revision_ids {
                if seen_revisions.contains(revision_id) {
                    continue;
                }
                let candidate = read_candidate(keys, repository, *revision_id)?;
                if candidate.item_id() != item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
                for parent_id in candidate.causal_parents() {
                    let parent = read_candidate(keys, repository, *parent_id)?;
                    if parent.item_id() != item_id {
                        return Err(ApplicationError::IntegrityFailure);
                    }
                }
                seen_revisions.insert(*revision_id);
                candidates.push(candidate);
                if candidates.len() == limit {
                    return Ok(candidates);
                }
            }
        }
    }

    Ok(candidates)
}

fn find_reachable_historical_candidate(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
    selected_revision: RevisionId,
) -> Result<ItemCandidate, ApplicationError> {
    let mut histories = Vec::with_capacity(report.heads().len());
    for head_id in report.heads().iter().copied() {
        let history = repository
            .history(head_id, MAX_ITEM_HISTORY_LIMIT)
            .map_err(map_repository)?;
        if history.first().map(|commit| commit.id()) != Some(head_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        histories.push(history);
    }

    let mut seen_commits = BTreeSet::new();
    let mut seen_catalogs = BTreeSet::new();
    for depth in 0..MAX_ITEM_HISTORY_LIMIT {
        let mut commits = histories
            .iter()
            .filter_map(|history| history.get(depth))
            .filter(|commit| !seen_commits.contains(&commit.id()))
            .collect::<Vec<_>>();
        commits.sort_unstable_by_key(|commit| commit.id());

        for commit in commits {
            if !seen_commits.insert(commit.id()) {
                continue;
            }
            if commit.vault_id() != vault_id {
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
            let Some((item_id, _)) = catalog
                .entries()
                .iter()
                .find(|(_, revisions)| revisions.binary_search(&selected_revision).is_ok())
            else {
                continue;
            };

            let candidate = read_candidate(keys, repository, selected_revision)?;
            if candidate.item_id() != *item_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            for parent_id in candidate.causal_parents() {
                let parent = read_candidate(keys, repository, *parent_id)?;
                if parent.item_id() != *item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
            }
            return Ok(candidate);
        }
    }

    Err(ApplicationError::NotFound)
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
        V1ApplicationRepositoryFactory, V1Keys, ADD_ITEM_RANDOM_BYTES, DELETE_ITEM_RANDOM_BYTES,
        GENERATION_ZERO_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES, RESTORE_ITEM_RANDOM_BYTES,
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
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    #[derive(Default)]
    struct MemoryLocalStateStore(
        Mutex<Option<Vec<u8>>>,
        AtomicBool,
        Mutex<Option<(usize, LocalStateStoreError)>>,
        AtomicUsize,
    );

    impl MemoryLocalStateStore {
        fn with_state(state: Vec<u8>) -> Self {
            Self(
                Mutex::new(Some(state)),
                AtomicBool::new(false),
                Mutex::new(None),
                AtomicUsize::new(0),
            )
        }

        fn concurrent_winner_on_next_compare(&self) {
            self.1.store(true, Ordering::SeqCst);
        }

        fn fail_next_compare(&self, error: LocalStateStoreError) {
            self.fail_compare_after(0, error);
        }

        fn fail_compare_after(&self, successful_calls: usize, error: LocalStateStoreError) {
            let target = self
                .3
                .load(Ordering::SeqCst)
                .checked_add(successful_calls + 1)
                .unwrap();
            *self.2.lock().unwrap() = Some((target, error));
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
            let call = self.3.fetch_add(1, Ordering::SeqCst) + 1;
            let mut state = self.0.lock().unwrap();
            if state.as_deref() != expected {
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            let scheduled_error = *self.2.lock().unwrap();
            if let Some((target, error)) = scheduled_error {
                if target == call {
                    self.2.lock().unwrap().take();
                    return Err(error);
                }
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

    fn add_item_randomness(seed: u8) -> AddItemRandomnessV1 {
        let mut bytes = [0; ADD_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(seed);
        }
        AddItemRandomnessV1::new(bytes)
    }

    fn replace_item_randomness(seed: u8) -> ReplaceItemRandomnessV1 {
        let mut bytes = [0; REPLACE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(seed);
        }
        ReplaceItemRandomnessV1::new(bytes)
    }

    fn delete_item_randomness(seed: u8) -> DeleteItemRandomnessV1 {
        let mut bytes = [0; DELETE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(31).wrapping_add(seed);
        }
        DeleteItemRandomnessV1::new(bytes)
    }

    fn restore_item_randomness(seed: u8) -> RestoreItemRandomnessV1 {
        let mut bytes = [0; RESTORE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(37).wrapping_add(seed);
        }
        RestoreItemRandomnessV1::new(bytes)
    }

    fn new_login_document(item_id: ItemId, title: &str, password: &str) -> ItemDocument {
        ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            300,
            300,
            LwwRegister::new(false, 300, OperationId::new([0x71; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: title.to_owned(),
                username: "new-user@example.test".to_owned(),
                password: password.to_owned(),
                urls: vec!["https://new.example.test".to_owned()],
                notes: None,
            }),
            ObservedSet::new(),
        )
        .unwrap()
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
            let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
            let exact_state = local.0.lock().unwrap().clone().unwrap();
            assert_eq!(
                session.replace_item(
                    expected_revision,
                    new_login_document(item_id, "Cannot replace", "secret"),
                    299,
                    replace_item_randomness(0x31),
                    &local,
                ),
                Err(ApplicationError::ConflictRequired)
            );
            assert_eq!(
                local.0.lock().unwrap().as_deref(),
                Some(exact_state.as_slice())
            );
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
    fn add_item_publishes_parentless_revision_and_requires_reopen() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        let randomness = add_item_randomness(0x41);
        let item_id = randomness.item_id();
        local.concurrent_winner_on_next_compare();
        let active = session
            .add_item(
                new_login_document(item_id, "New portal", "new-password-secret"),
                301,
                randomness,
                &local,
            )
            .unwrap();

        assert_eq!(active.last_device_counter(), 2);
        assert_ne!(active.pinned_heads(), &prior_heads);
        assert_eq!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(active.clone())
        );

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 1);
        assert_eq!(reopened.search_item_count(), 1);
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("new item must have exactly one current revision")
        };
        assert!(candidate.causal_parents().is_empty());
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(commit.device_counter(), 2);
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.catalog_root(), active.catalog_root());
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 301);
    }

    #[test]
    fn add_item_rejects_mismatched_or_existing_random_identity_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let mismatched = add_item_randomness(0x51);
        assert_eq!(
            session.add_item(
                new_login_document(ItemId::new([0x99; 16]), "Wrong", "secret"),
                301,
                mismatched,
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0x61);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "First", "secret"),
                302,
                randomness,
                &local,
            )
            .unwrap();
        let exact_after_first = local.0.lock().unwrap().clone().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let mut duplicate_bytes = [0; ADD_ITEM_RANDOM_BYTES];
        duplicate_bytes[..16].copy_from_slice(item_id.as_bytes());
        assert_eq!(
            session.add_item(
                new_login_document(item_id, "Duplicate", "secret"),
                303,
                AddItemRandomnessV1::new(duplicate_bytes),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_after_first.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let second_randomness = add_item_randomness(0x71);
        let second_item_id = second_randomness.item_id();
        session
            .add_item(
                new_login_document(second_item_id, "Second", "secret"),
                304,
                second_randomness,
                &local,
            )
            .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 2);
        assert!(reopened.get_item(item_id).unwrap().is_some());
        assert!(reopened.get_item(second_item_id).unwrap().is_some());
    }

    #[test]
    fn add_item_retains_exact_pending_journal_across_ambiguous_provider_failure() {
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
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0x81);
        let item_id = randomness.item_id();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert_eq!(
            session.add_item(
                new_login_document(item_id, "Crash safe", "secret"),
                304,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&exact_pending).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));

        let active = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.last_device_counter(), 2);
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn add_item_persists_before_publish_and_recovers_after_final_cas_failure() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let randomness = add_item_randomness(0x91);
        let item_id = randomness.item_id();
        local.fail_next_compare(LocalStateStoreError::Unavailable);
        assert_eq!(
            session.add_item(
                new_login_document(item_id, "No early publish", "secret"),
                305,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.open_report().commit_count(), 1);

        let randomness = add_item_randomness(0xa1);
        let item_id = randomness.item_id();
        local.fail_compare_after(1, LocalStateStoreError::Unavailable);
        assert_eq!(
            reopened.add_item(
                new_login_document(item_id, "Recover final state", "secret"),
                306,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&exact_pending).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        assert_eq!(reopened.open_report().commit_count(), 2);
    }

    #[test]
    fn replace_item_advances_one_expected_live_revision_and_preserves_others() {
        let (locator, local, bootstrap, factory) = initialized();
        let first_randomness = add_item_randomness(0xb1);
        let first_item_id = first_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(first_item_id, "Before", "old-secret"),
            401,
            first_randomness,
            &local,
        )
        .unwrap();
        let second_randomness = add_item_randomness(0xc1);
        let second_item_id = second_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(second_item_id, "Untouched", "other-secret"),
            402,
            second_randomness,
            &local,
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
        let prior_heads = session.local_pins().clone();
        let expected_revision = session.current_catalog.items[&first_item_id][0].revision_id();
        let active = session
            .replace_item(
                expected_revision,
                new_login_document(first_item_id, "After", "new-secret"),
                403,
                replace_item_randomness(0xd1),
                &local,
            )
            .unwrap();

        assert_eq!(active.last_device_counter(), 4);
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 2);
        assert!(reopened.get_item(second_item_id).unwrap().is_some());
        let [candidate] = reopened.current_catalog.items[&first_item_id].as_slice() else {
            panic!("replacement must become the sole current candidate")
        };
        assert_ne!(candidate.revision_id(), expected_revision);
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([expected_revision])
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("replacement must remain live")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("replacement schema must remain login")
        };
        assert_eq!(login.title, "After");
        assert_eq!(login.password, "new-secret");
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 403);
    }

    #[test]
    fn replace_item_rejects_missing_stale_and_immutable_identity_changes_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let missing_randomness = add_item_randomness(0xe1);
        let item_id = missing_randomness.item_id();
        let exact_empty = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .replace_item(
                RevisionId::new([0x91; 32]),
                new_login_document(item_id, "Missing", "secret"),
                404,
                replace_item_randomness(0xf1),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Original", "secret"),
            405,
            missing_randomness,
            &local,
        )
        .unwrap();
        let exact_item = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .replace_item(
                RevisionId::new([0x92; 32]),
                new_login_document(item_id, "Stale", "secret"),
                406,
                replace_item_randomness(0x01),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_item.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
        let changed_creation = ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            299,
            300,
            LwwRegister::new(false, 300, OperationId::new([0x72; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: "Changed creation".to_owned(),
                username: "new-user@example.test".to_owned(),
                password: "secret".to_owned(),
                urls: vec!["https://new.example.test".to_owned()],
                notes: None,
            }),
            ObservedSet::new(),
        )
        .unwrap();
        assert_eq!(
            session.replace_item(
                expected_revision,
                changed_creation,
                407,
                replace_item_randomness(0x11),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_item.as_slice())
        );
    }

    #[test]
    fn delete_item_publishes_one_parent_tombstone_and_rejects_repeat_delete() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x21);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Delete me", "secret"),
            450,
            add_randomness,
            &local,
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
        let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
        let prior_heads = session.local_pins().clone();
        let active = session
            .delete_item(
                expected_revision,
                451,
                452,
                delete_item_randomness(0x31),
                &local,
            )
            .unwrap();
        assert_eq!(active.last_device_counter(), 3);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 1);
        assert_eq!(reopened.search_item_count(), 0);
        assert_eq!(reopened.get_item(item_id).unwrap(), None);
        assert!(reopened.list_items().unwrap().is_empty());
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("deletion must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([expected_revision])
        );
        let ItemState::Tombstone(tombstone) = candidate.state() else {
            panic!("deletion must materialize a tombstone")
        };
        assert_eq!(tombstone.item_id, item_id);
        assert_eq!(tombstone.deleted_at_ms, 451);
        let tombstone_revision = candidate.revision_id();
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 452);
        let exact_deleted = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            reopened.delete_item(
                tombstone_revision,
                453,
                454,
                delete_item_randomness(0x41),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
        );
    }

    #[test]
    fn delete_item_rejects_missing_revision_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .delete_item(
                RevisionId::new([0x51; 32]),
                455,
                456,
                delete_item_randomness(0x51),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
    }

    #[test]
    fn item_history_materializes_live_and_deleted_revisions_in_ancestry_order() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x61);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "History one", "first-secret"),
            500,
            add_randomness,
            &local,
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
        let first_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                first_revision,
                new_login_document(item_id, "History two", "second-secret"),
                501,
                replace_item_randomness(0x71),
                &local,
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
        let second_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .delete_item(
                second_revision,
                502,
                503,
                delete_item_randomness(0x81),
                &local,
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
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let history = session
            .item_history(item_id, DEFAULT_ITEM_HISTORY_LIMIT)
            .unwrap();

        assert_eq!(history.len(), 3);
        assert_eq!(history[0].revision_id(), tombstone_revision);
        assert!(history[0].is_deleted());
        assert_eq!(history[0].causal_parent_count(), 1);
        assert_eq!(history[0].advisory_time_ms(), 502);
        assert_eq!(history[1].revision_id(), second_revision);
        assert_eq!(history[1].causal_parent_count(), 1);
        assert_eq!(history[2].revision_id(), first_revision);
        assert_eq!(history[2].causal_parent_count(), 0);
        let Some(RedactedItemView {
            record: RedactedRecordView::Login { title, .. },
            ..
        }) = history[1].redacted_item()
        else {
            panic!("historical live revision must retain safe login metadata")
        };
        assert_eq!(title, "History two");
        assert!(!format!("{:?}", history[1]).contains("History two"));

        let limited = session.item_history(item_id, 2).unwrap();
        assert_eq!(
            limited
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            vec![tombstone_revision, second_revision]
        );
        assert!(session
            .item_history(ItemId::new([0xff; 16]), 100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn item_history_rejects_invalid_bounds_without_disclosing_identity() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let item_id = ItemId::new([0x91; 16]);

        assert_eq!(
            session.item_history(item_id, 0),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.item_history(item_id, MAX_ITEM_HISTORY_LIMIT + 1),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn restore_item_copies_one_reachable_live_revision_without_rewinding_heads() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xa1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Restore me", "original-secret"),
            600,
            add_randomness,
            &local,
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
        let original_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                original_revision,
                new_login_document(item_id, "Changed", "changed-secret"),
                601,
                replace_item_randomness(0xb1),
                &local,
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
        let changed_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .delete_item(
                changed_revision,
                602,
                603,
                delete_item_randomness(0xc1),
                &local,
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
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let prior_heads = session.local_pins().clone();
        let active = session
            .restore_item(
                original_revision,
                604,
                restore_item_randomness(0xd1),
                &local,
            )
            .unwrap();
        assert_eq!(active.last_device_counter(), 5);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("restoration must become the sole current candidate")
        };
        let restored_revision = candidate.revision_id();
        assert_ne!(restored_revision, original_revision);
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([original_revision])
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("restoration must create a live revision")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("restoration must preserve the selected schema")
        };
        assert_eq!(login.title, "Restore me");
        assert_eq!(login.password, "original-secret");
        let history = reopened.item_history(item_id, 100).unwrap();
        assert_eq!(
            history
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            vec![
                restored_revision,
                tombstone_revision,
                changed_revision,
                original_revision
            ]
        );
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 604);
    }

    #[test]
    fn restore_item_rejects_missing_current_and_tombstone_selections_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xe1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Restore guards", "secret"),
            610,
            add_randomness,
            &local,
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
        let live_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_live = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.restore_item(live_revision, 611, restore_item_randomness(0xf1), &local,),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_live.as_slice())
        );

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .delete_item(
            live_revision,
            612,
            613,
            delete_item_randomness(0x01),
            &local,
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
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_deleted = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.restore_item(
                tombstone_revision,
                614,
                restore_item_randomness(0x11),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
        );

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .restore_item(
                RevisionId::new([0x7f; 32]),
                615,
                restore_item_randomness(0x21),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
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
