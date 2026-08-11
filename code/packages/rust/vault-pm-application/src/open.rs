use crate::initialize::{unlock_active_material, UnlockedActiveMaterial};
use crate::mutation::{
    add_item, delete_item, import_opened_portable_snapshot, merge_item_conflict, replace_item,
    resolve_item_conflict, restore_item, AddItemRandomnessV1, DeleteItemRandomnessV1,
    PortableImportRandomnessV1, ReplaceItemRandomnessV1, ResolveItemConflictRandomnessV1,
    RestoreItemRandomnessV1,
};
use crate::search::SearchProjectionV1;
use crate::{
    open_object, ActiveStateV1, ApplicationError, ApplicationRepository,
    ApplicationRepositoryError, ApplicationRepositoryFactory, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, CatalogV1, LocalSecretV1, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, ObjectKind, RevealedSecretV1, SecretDisclosureIntentV1, SecretFieldV1,
    V1Keys,
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
    pub(crate) const fn active_state(&self) -> &ActiveStateV1 {
        &self.active
    }

    pub(crate) const fn bootstrap_locator(&self) -> BootstrapLocator {
        self.active.bootstrap_locator()
    }

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

    /// Re-verify the complete reachable vault and return aggregate counts.
    ///
    /// This repeats repository discovery relative to durable local pins,
    /// checks the local writer counter/catalog anchor, walks complete verified
    /// ancestry from every head, and decrypts every distinct catalog and
    /// catalog-referenced revision. It returns no identities or item metadata.
    pub fn audit_verify(&self) -> Result<crate::AuditVerificationV1, ApplicationError> {
        crate::audit::audit_verify(&self.active, &self._keys, self._repository.as_ref())
    }

    /// Build one canonical authenticated encrypted snapshot for host persistence.
    ///
    /// The passphrase must be collected separately from the live vault
    /// passphrase. The host supplies fresh salt/nonce randomness and chooses the
    /// destination only after this method returns. Every current live,
    /// tombstone, and conflicted candidate is included; local private state,
    /// provider credentials, pins, and search projections are excluded.
    pub fn export_portable_with_passphrase(
        &self,
        exact_bootstrap: &[u8],
        passphrase: Zeroizing<Vec<u8>>,
        policy: crate::PortableExportPolicyV1,
        randomness: crate::PortableExportRandomnessV1,
    ) -> Result<crate::PortableExportArtifactV1, ApplicationError> {
        crate::export::export_portable_with_passphrase(
            &self.current_catalog.items,
            &self.active,
            exact_bootstrap,
            passphrase,
            policy,
            randomness,
        )
    }

    /// Consume an authenticated portable snapshot into this untouched target
    /// vault and return the resulting durable active owner state.
    ///
    /// The target must still be the empty generation-zero vault. Every source
    /// item and retained live, tombstone, or conflicted candidate receives a
    /// new target item/revision/object identity and is encrypted by the target
    /// vault's independent keys. Source causal-parent identities are not
    /// copied. The complete import is one crash-resumable publication, and the
    /// session, opaque snapshot, and owned randomness are consumed on every
    /// return path.
    pub fn import_opened_portable_snapshot(
        self,
        snapshot: crate::OpenedPortableSnapshotV1,
        wall_time_ms: u64,
        randomness: PortableImportRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        import_opened_portable_snapshot(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            snapshot,
            wall_time_ms,
            randomness,
            local_state_store,
        )
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

    /// Return the exact sole current live revision for optimistic mutation.
    ///
    /// A missing item and a current tombstone both return `None`. Multiple
    /// retained candidates fail closed with [`ApplicationError::ConflictRequired`].
    /// The revision identity is an application capability for a later
    /// compare-and-swap mutation and is not an ordinary display value.
    pub fn current_item_revision(
        &self,
        item_id: ItemId,
    ) -> Result<Option<RevisionId>, ApplicationError> {
        let Some(candidates) = self.current_catalog.items.get(&item_id) else {
            return Ok(None);
        };
        let [candidate] = candidates.as_slice() else {
            return Err(ApplicationError::ConflictRequired);
        };
        Ok(matches!(candidate.state(), ItemState::Live(_)).then_some(candidate.revision_id()))
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

    /// Return every retained current candidate for one conflicted item as a
    /// deterministic secret-free view.
    ///
    /// Candidates are ordered by exact revision ID. A missing item returns
    /// `NotFound`; an item with fewer than two current candidates returns
    /// `ConflictRequired`. No candidate is selected or discarded.
    pub fn conflict_candidates(
        &self,
        item_id: ItemId,
    ) -> Result<Vec<ItemHistoryViewV1>, ApplicationError> {
        let candidates = self
            .current_catalog
            .items
            .get(&item_id)
            .ok_or(ApplicationError::NotFound)?;
        if candidates.len() < 2 {
            return Err(ApplicationError::ConflictRequired);
        }
        candidates
            .iter()
            .map(ItemHistoryViewV1::from_candidate)
            .collect()
    }

    /// Explicitly reveal one reachable live revision inside an owned
    /// wipe-on-drop wrapper.
    ///
    /// The exact revision must appear in a catalog reachable within the hard
    /// history bound from a current head. Tombstones return `InvalidInput` and
    /// unreachable revisions return `NotFound`. The wrapper deliberately has
    /// no `Debug`, `Display`, or `Clone` implementation.
    pub fn reveal_item_revision(
        &self,
        selected_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let selected = find_reachable_historical_candidate(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            selected_revision,
        )?;
        let ItemState::Live(document) = selected.state() else {
            return Err(ApplicationError::InvalidInput);
        };
        Ok(Zeroizing::new(document.as_ref().clone()))
    }

    /// Select and authorize disclosure of one secret-bearing field from one
    /// exact reachable live revision.
    ///
    /// Policy is checked before repository traversal. The returned value is
    /// owned, non-printable, non-cloneable, and wipe-on-drop. The host remains
    /// responsible for its controlling-TTY facts, warning output, and secure
    /// clipboard ownership/clear behavior.
    pub fn reveal_item_revision_field(
        &self,
        selected_revision: RevisionId,
        field: SecretFieldV1,
        intent: SecretDisclosureIntentV1,
    ) -> Result<RevealedSecretV1, ApplicationError> {
        intent.authorize()?;
        let document = self.reveal_item_revision(selected_revision)?;
        crate::disclosure::select_secret(document.payload(), field)
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

    /// Resolve one current conflict by choosing an existing authenticated
    /// candidate and publishing it as a new current revision.
    ///
    /// The selected revision must be one of at least two current candidates.
    /// The resolution revision copies its complete live document or tombstone
    /// and names every retained current candidate as a direct causal parent.
    /// This consumes the session and never deletes the losing immutable bytes.
    pub fn resolve_item_conflict(
        self,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        resolve_item_conflict(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Resolve one current conflict with a complete caller-authored document.
    ///
    /// The document must name an item with at least two current candidates and
    /// preserve the schema and creation time of every retained live candidate.
    /// At least one live candidate is required. The new revision names the
    /// complete current conflict set as direct causal parents, consumes the
    /// session and owned secret-bearing document, and never deletes immutable
    /// candidate bytes.
    pub fn merge_item_conflict(
        self,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        merge_item_conflict(
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

pub(crate) fn read_candidate(
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
    use crate::mutation::{
        activate_audit_epoch_for_test, publish_audit_only_event_for_test,
        AUDIT_ONLY_TEST_RANDOM_BYTES,
    };
    use crate::{
        complete_generation_zero, decode_signed_audit_event, encode_item_revision,
        encode_signed_commit, prepare_generation_zero, seal_object, CatalogV1,
        GenerationZeroPolicyV1, GenerationZeroRandomness, ObjectKind, ObjectRandomness,
        PublicationJournalV1, V1ApplicationRepositoryFactory, V1Keys, ADD_ITEM_RANDOM_BYTES,
        DELETE_ITEM_RANDOM_BYTES, GENERATION_ZERO_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES,
        RESOLVE_ITEM_CONFLICT_RANDOM_BYTES, RESTORE_ITEM_RANDOM_BYTES,
    };
    use coding_adventures_canonical_cbor::{
        decode as decode_cbor, encode as encode_cbor, CborValue,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_audit::{AuditActionV1, AuditOutcomeV1};
    use coding_adventures_vault_pm_domain::{
        ContentType, ItemDocument, ItemState, LwwRegister, ObservedSet, OperationId,
        RedactedRecordView, Tombstone,
    };
    use coding_adventures_vault_pm_format::{AnnouncementV1, BootstrapId, CommitV1, Signature};
    use coding_adventures_vault_pm_storage::{
        FaultAction, FaultEffect, FaultInjectingObjectStore, InMemoryObjectStore, StoreOperation,
    };
    use coding_adventures_vault_records::{AnyRecord, Login, LOGIN_V1};
    use coding_adventures_zeroize::Zeroize;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    struct FailingAuditRepository(ApplicationRepositoryError);

    impl ApplicationRepository for FailingAuditRepository {
        fn initialize(&self) -> Result<(), ApplicationRepositoryError> {
            Err(self.0)
        }

        fn open(&self, _pins: &PinnedHeads) -> Result<OpenReport, ApplicationRepositoryError> {
            Err(self.0)
        }

        fn publish(
            &self,
            _publication: coding_adventures_vault_pm_repository::Publication,
            _current_heads: &PinnedHeads,
        ) -> Result<
            coding_adventures_vault_pm_repository::PublicationReceipt,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }

        fn read_object(
            &self,
            _id: ObjectId,
        ) -> Result<coding_adventures_vault_pm_repository::VerifiedObject, ApplicationRepositoryError>
        {
            Err(self.0)
        }

        fn read_commit(
            &self,
            _id: ObjectId,
        ) -> Result<coding_adventures_vault_pm_repository::CommitSummary, ApplicationRepositoryError>
        {
            Err(self.0)
        }

        fn history(
            &self,
            _start: ObjectId,
            _limit: usize,
        ) -> Result<
            Vec<coding_adventures_vault_pm_repository::CommitSummary>,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }

        fn complete_history(
            &self,
            _start: ObjectId,
        ) -> Result<
            Vec<coding_adventures_vault_pm_repository::CommitSummary>,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }
    }

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

    fn replace_top_level_version(encoded: &[u8], version: u64) -> Vec<u8> {
        let CborValue::Map(mut fields) = decode_cbor(encoded).unwrap() else {
            panic!("fixture must be a CBOR map")
        };
        let mut replaced = false;
        for (key, value) in &mut fields {
            if key == &CborValue::Unsigned(1) {
                *value = CborValue::Unsigned(version);
                replaced = true;
                break;
            }
        }
        assert!(replaced);
        encode_cbor(&CborValue::Map(fields))
    }

    fn take_cbor_field(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> CborValue {
        let index = fields
            .iter()
            .position(|(candidate, _)| candidate == &CborValue::Unsigned(key))
            .unwrap();
        fields.remove(index).1
    }

    fn replace_cbor_field(fields: &mut [(CborValue, CborValue)], key: u64, replacement: CborValue) {
        let (_, value) = fields
            .iter_mut()
            .find(|(candidate, _)| candidate == &CborValue::Unsigned(key))
            .unwrap();
        *value = replacement;
    }

    fn refresh_portable_snapshot_hash(fields: &mut [(CborValue, CborValue)]) {
        let CborValue::Bytes(mut bootstrap) = fields
            .iter()
            .find(|(key, _)| key == &CborValue::Unsigned(2))
            .map(|(_, value)| value.clone())
            .unwrap()
        else {
            panic!()
        };
        let entries = fields
            .iter()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| encode_cbor(value))
            .unwrap();
        let hash = crate::export::snapshot_hash(&bootstrap, &entries).unwrap();
        replace_cbor_field(fields, 5, CborValue::Bytes(hash.to_vec()));
        bootstrap.zeroize();
    }

    fn authenticate_portable_snapshot(
        fields: Vec<(CborValue, CborValue)>,
        passphrase: &[u8],
        randomness: u8,
    ) -> Vec<u8> {
        let mut plaintext = encode_cbor(&CborValue::Map(fields));
        let artifact = crate::export::encrypt_portable_for_test(
            &plaintext,
            Zeroizing::new(passphrase.to_vec()),
            crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            crate::PortableExportRandomnessV1::new(
                [randomness; crate::PORTABLE_EXPORT_RANDOM_BYTES],
            ),
        );
        plaintext.zeroize();
        artifact.into_bytes()
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

    fn resolve_item_conflict_randomness(seed: u8) -> ResolveItemConflictRandomnessV1 {
        let mut bytes = [0; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(41).wrapping_add(seed);
        }
        ResolveItemConflictRandomnessV1::new(bytes)
    }

    fn new_login_document(item_id: ItemId, title: &str, password: &str) -> ItemDocument {
        login_document_with_times(item_id, title, password, 300, 300)
    }

    fn login_document_with_times(
        item_id: ItemId,
        title: &str,
        password: &str,
        created_at_ms: u64,
        updated_at_ms: u64,
    ) -> ItemDocument {
        ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            created_at_ms,
            updated_at_ms,
            LwwRegister::new(false, updated_at_ms, OperationId::new([0x71; 32])),
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
        initialized_with(
            b"active passphrase",
            GenerationZeroRandomness::new(generation_zero_bytes()),
        )
    }

    fn initialized_with(
        passphrase: &[u8],
        randomness: GenerationZeroRandomness,
    ) -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
    ) {
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness,
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

    fn pending_live_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<(RevisionId, String)>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (title, password)) in
            [("Keep left", "left-secret"), ("Keep right", "right-secret")]
                .into_iter()
                .enumerate()
        {
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(new_login_document(item_id, title, password))),
            )
            .unwrap();
            let base = 0xb0u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            let revision_id = RevisionId::new(*frame.id().unwrap().as_bytes());
            revisions.push((revision_id, title.to_owned()));
            objects.push(frame);
        }
        revisions.sort_unstable_by_key(|(revision_id, _)| *revision_id);
        let catalog = CatalogV1::new(BTreeMap::from([(
            item_id,
            revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect(),
        )]))
        .unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xc0; 32], [0xc1; 24], [0xc2; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
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
    fn audit_verify_reopens_complete_ancestry_and_reports_only_counts() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let initial = session.audit_verify().unwrap();
        assert!(initial.integrity_verified());
        assert_eq!(initial.announcement_count(), 1);
        assert_eq!(initial.commit_count(), 1);
        assert_eq!(initial.catalog_count(), 1);
        assert_eq!(initial.revision_count(), 0);
        assert_eq!(initial.item_count(), 0);
        assert_eq!(initial.audit_event_count(), 0);

        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            699,
            None,
            None,
            [0xa7; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let epoch = session.audit_verify().unwrap();
        assert_eq!(epoch.commit_count(), 2);
        assert_eq!(epoch.catalog_count(), 1);
        assert_eq!(epoch.audit_event_count(), 1);

        publish_audit_only_event_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            AuditActionV1::ItemList,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            699,
            None,
            None,
            [0xaa; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let accessed = session.audit_verify().unwrap();
        assert_eq!(accessed.commit_count(), 3);
        assert_eq!(accessed.catalog_count(), 1);
        assert_eq!(accessed.audit_event_count(), 2);

        let randomness = add_item_randomness(0xa8);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "Audited login", "audit-secret"),
                700,
                randomness,
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
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.announcement_count(), 4);
        assert_eq!(report.commit_count(), 4);
        assert_eq!(report.catalog_count(), 2);
        assert_eq!(report.revision_count(), 1);
        assert_eq!(report.item_count(), 1);
        assert_eq!(report.audit_event_count(), 3);
        assert_eq!(
            format!("{report:?}"),
            "AuditVerificationV1 { integrity_verified: true, announcement_count: 4, commit_count: 4, catalog_count: 2, revision_count: 1, item_count: 1, audit_event_count: 3 }"
        );
        assert!(!format!("{report:?}").contains("Audited login"));
        assert!(!format!("{report:?}").contains("audit-secret"));

        let expected_revision = reopened.current_catalog.items[&item_id][0].revision_id();
        reopened
            .replace_item(
                expected_revision,
                login_document_with_times(
                    item_id,
                    "Audited login updated",
                    "audit-secret-updated",
                    300,
                    701,
                ),
                702,
                replace_item_randomness(0xa9),
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
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.announcement_count(), 5);
        assert_eq!(report.commit_count(), 5);
        assert_eq!(report.catalog_count(), 3);
        assert_eq!(report.revision_count(), 2);
        assert_eq!(report.item_count(), 1);
        assert_eq!(report.audit_event_count(), 4);
    }

    #[test]
    fn audit_only_publication_replays_after_ambiguous_provider_failure() {
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
        let catalog_root = session.active.catalog_root();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert_eq!(
            activate_audit_epoch_for_test(
                &session.active,
                &session._keys,
                &session._local_secret,
                session._repository.as_ref(),
                703,
                None,
                None,
                [0xab; AUDIT_ONLY_TEST_RANDOM_BYTES],
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::PendingPublication {
            active,
            publication,
        } = LocalVaultStateV1::decode(&exact_pending).unwrap()
        else {
            panic!("audit-only failure must retain the exact pending journal")
        };
        assert_eq!(active.catalog_root(), catalog_root);
        assert_eq!(publication.catalog_root(), catalog_root);
        assert_eq!(publication.objects().len(), 1);
        assert_eq!(
            publication.audit_event_head(),
            publication.objects()[0].id().ok()
        );
        drop(session);

        let recovered = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(recovered.catalog_root(), catalog_root);
        assert_eq!(recovered.last_device_counter(), 2);
        assert!(recovered.audit_event_head().is_some());
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 2);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 1);
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn audit_verify_rejects_wrong_event_basis_and_signer() {
        for (basis_override, signing_seed_override, randomness) in [
            (
                Some(vec![ObjectId::new([0xfe; 32])]),
                None,
                [0xb1; AUDIT_ONLY_TEST_RANDOM_BYTES],
            ),
            (None, Some([0xfd; 32]), [0xb2; AUDIT_ONLY_TEST_RANDOM_BYTES]),
        ] {
            let (locator, local, bootstrap, factory) = initialized();
            let session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            activate_audit_epoch_for_test(
                &session.active,
                &session._keys,
                &session._local_secret,
                session._repository.as_ref(),
                800,
                basis_override,
                signing_seed_override,
                randomness,
                &local,
            )
            .unwrap();
            drop(session);
            let reopened = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(
                reopened.audit_verify(),
                Err(ApplicationError::IntegrityFailure)
            );
        }
    }

    #[test]
    fn audit_verify_rejects_a_skipped_durable_event_head() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let epoch = activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            810,
            None,
            None,
            [0xb3; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        let epoch_head = epoch.audit_event_head().unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0xb4);
        let item_id = randomness.item_id();
        let latest = session
            .add_item(
                new_login_document(item_id, "Linked audit", "linked-secret"),
                811,
                randomness,
                &local,
            )
            .unwrap();
        let exact_latest = LocalVaultStateV1::Active(latest.clone()).encode().unwrap();
        let skipped = latest.with_audit_event_head(epoch_head).unwrap();
        let exact_skipped = LocalVaultStateV1::Active(skipped).encode().unwrap();
        local
            .compare_exchange(locator, Some(&exact_latest), &exact_skipped)
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
            reopened.audit_verify(),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_export_encrypts_every_current_candidate_under_a_separate_passphrase() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0xaa);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "Portable portal", "portable-export-secret"),
                800,
                randomness,
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
        let exact_bootstrap = bootstrap.0.lock().unwrap().clone().unwrap();
        let policy = crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        let export_randomness = [0x5b; crate::PORTABLE_EXPORT_RANDOM_BYTES];
        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                crate::PortableExportRandomnessV1::new(export_randomness),
            )
            .unwrap();
        let repeated = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                crate::PortableExportRandomnessV1::new(export_randomness),
            )
            .unwrap();
        assert_eq!(artifact.as_bytes(), repeated.as_bytes());
        assert_eq!(
            coding_adventures_sha256::sha256(artifact.as_bytes()),
            [
                0xb8, 0xb7, 0x05, 0x87, 0xea, 0x11, 0x3b, 0x11, 0xea, 0x23, 0xa7, 0x7a, 0x9b, 0x36,
                0xe6, 0xc4, 0x0e, 0x51, 0x04, 0x86, 0x99, 0x60, 0xb9, 0x38, 0x98, 0x93, 0xaf, 0x5e,
                0x96, 0x53, 0xa1, 0x3a,
            ]
        );
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 1);
        assert_eq!(opened.candidate_count(), 1);
        assert_eq!(
            format!("{opened:?}"),
            "OpenedPortableSnapshotV1(<redacted>)"
        );
        for plaintext in [b"Portable portal".as_slice(), b"portable-export-secret"] {
            assert!(!artifact
                .as_bytes()
                .windows(plaintext.len())
                .any(|window| window == plaintext));
        }
        assert!(crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"wrong export passphrase".to_vec()),
        )
        .is_none());
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(b"wrong export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        let mut tampered = artifact.as_bytes().to_vec();
        let last = tampered.last_mut().unwrap();
        *last ^= 1;
        assert!(crate::export::decrypt_portable_for_test(
            &tampered,
            Zeroizing::new(b"separate export passphrase".to_vec()),
        )
        .is_none());
        assert_eq!(
            crate::open_portable_with_passphrase(
                &tampered,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );

        let plaintext = crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
        )
        .unwrap();
        let CborValue::Map(mut snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!("portable snapshot must be a canonical map")
        };
        assert_eq!(take_cbor_field(&mut snapshot, 1), CborValue::Unsigned(1));
        let CborValue::Bytes(exported_bootstrap) = take_cbor_field(&mut snapshot, 2) else {
            panic!()
        };
        assert_eq!(exported_bootstrap, exact_bootstrap);
        let mut entries = take_cbor_field(&mut snapshot, 3);
        let encoded_entries = encode_cbor(&entries);
        let CborValue::Unsigned(candidate_count) = take_cbor_field(&mut snapshot, 4) else {
            panic!()
        };
        assert_eq!(candidate_count, 1);
        let CborValue::Bytes(exported_hash) = take_cbor_field(&mut snapshot, 5) else {
            panic!()
        };
        assert_eq!(
            exported_hash,
            crate::export::snapshot_hash(&exact_bootstrap, &encoded_entries)
                .unwrap()
                .to_vec()
        );
        assert!(snapshot.is_empty());

        let CborValue::Array(exported_candidates) = &mut entries else {
            panic!()
        };
        assert_eq!(exported_candidates.len(), 1);
        let CborValue::Map(mut entry) = exported_candidates.remove(0) else {
            panic!()
        };
        let CborValue::Bytes(exported_item_id) = take_cbor_field(&mut entry, 1) else {
            panic!()
        };
        assert_eq!(exported_item_id, item_id.as_bytes());
        let CborValue::Bytes(exported_revision_id) = take_cbor_field(&mut entry, 2) else {
            panic!()
        };
        let revision_id = RevisionId::new(exported_revision_id.try_into().unwrap());
        let CborValue::Bytes(mut encoded_revision) = take_cbor_field(&mut entry, 3) else {
            panic!()
        };
        assert!(entry.is_empty());
        let candidate = crate::decode_item_revision(revision_id, &encoded_revision).unwrap();
        let ItemState::Live(document) = candidate.state() else {
            panic!()
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!()
        };
        assert_eq!(login.title, "Portable portal");
        assert_eq!(login.password, "portable-export-secret");
        encoded_revision.zeroize();
        crate::export::zeroize_cbor(&mut entries);

        let CborValue::Map(mut invalid_count) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        replace_cbor_field(&mut invalid_count, 4, CborValue::Unsigned(2));
        let invalid_count =
            authenticate_portable_snapshot(invalid_count, b"separate export passphrase", 0x5c);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_count,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut invalid_hash) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        replace_cbor_field(&mut invalid_hash, 5, CborValue::Bytes(vec![0; 32]));
        let invalid_hash =
            authenticate_portable_snapshot(invalid_hash, b"separate export passphrase", 0x5d);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_hash,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut invalid_bootstrap) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Bytes(bootstrap_bytes) = invalid_bootstrap
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(2))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        *bootstrap_bytes.last_mut().unwrap() ^= 1;
        refresh_portable_snapshot_hash(&mut invalid_bootstrap);
        let invalid_bootstrap =
            authenticate_portable_snapshot(invalid_bootstrap, b"separate export passphrase", 0x5e);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut mismatched_item) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Array(candidate_entries) = mismatched_item
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        let CborValue::Map(candidate_fields) = &mut candidate_entries[0] else {
            panic!()
        };
        replace_cbor_field(candidate_fields, 1, CborValue::Bytes(vec![0xfe; 16]));
        refresh_portable_snapshot_hash(&mut mismatched_item);
        let mismatched_item =
            authenticate_portable_snapshot(mismatched_item, b"separate export passphrase", 0x5f);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &mismatched_item,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            format!("{artifact:?}"),
            "PortableExportArtifactV1(<encrypted>)"
        );
    }

    #[test]
    fn portable_export_rejects_credential_and_bootstrap_misuse() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_bootstrap = bootstrap.0.lock().unwrap().clone().unwrap();
        let policy = crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        let randomness =
            || crate::PortableExportRandomnessV1::new([0x6c; crate::PORTABLE_EXPORT_RANDOM_BYTES]);
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &exact_bootstrap,
                    Zeroizing::new(Vec::new()),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &exact_bootstrap,
                    Zeroizing::new(vec![0x61; crate::MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES + 1]),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &[0xff],
                    Zeroizing::new(b"separate export passphrase".to_vec()),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                randomness(),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 0);
        assert_eq!(opened.candidate_count(), 0);
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(Vec::new()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(vec![0x61; crate::MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES + 1]),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );

        let CborValue::Map(mut unsupported_version) = decode_cbor(artifact.as_bytes()).unwrap()
        else {
            panic!()
        };
        replace_cbor_field(&mut unsupported_version, 1, CborValue::Unsigned(2));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(unsupported_version)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::Unsupported)
        );

        let CborValue::Map(mut excessive_kdf) = decode_cbor(artifact.as_bytes()).unwrap() else {
            panic!()
        };
        let CborValue::Map(kdf_fields) = excessive_kdf
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(4))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        replace_cbor_field(kdf_fields, 1, CborValue::Unsigned(16 * 1024));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(excessive_kdf)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::BoundExceeded)
        );

        let CborValue::Map(mut unknown_field) = decode_cbor(artifact.as_bytes()).unwrap() else {
            panic!()
        };
        unknown_field.push((CborValue::Unsigned(8), CborValue::Null));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(unknown_field)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_export_preserves_every_current_conflict_tombstone() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x7d; 16]);
        let (publication, expected_revision_ids) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
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
        assert_eq!(session.conflicted_item_count(), 1);
        let exact_bootstrap = bootstrap.0.lock().unwrap().clone().unwrap();
        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"conflict export passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0x7e; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"conflict export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 1);
        assert_eq!(opened.candidate_count(), 2);
        let plaintext = crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"conflict export passphrase".to_vec()),
        )
        .unwrap();
        let CborValue::Map(mut snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let mut entries = take_cbor_field(&mut snapshot, 3);
        assert_eq!(take_cbor_field(&mut snapshot, 4), CborValue::Unsigned(2));
        let CborValue::Array(exported_candidates) = &mut entries else {
            panic!()
        };
        let mut actual_revision_ids = Vec::new();
        for value in exported_candidates.drain(..) {
            let CborValue::Map(mut entry) = value else {
                panic!()
            };
            assert_eq!(
                take_cbor_field(&mut entry, 1),
                CborValue::Bytes(item_id.as_bytes().to_vec())
            );
            let CborValue::Bytes(revision_id) = take_cbor_field(&mut entry, 2) else {
                panic!()
            };
            let revision_id = RevisionId::new(revision_id.try_into().unwrap());
            actual_revision_ids.push(revision_id);
            let CborValue::Bytes(mut encoded_revision) = take_cbor_field(&mut entry, 3) else {
                panic!()
            };
            let candidate = crate::decode_item_revision(revision_id, &encoded_revision).unwrap();
            assert!(matches!(candidate.state(), ItemState::Tombstone(_)));
            encoded_revision.zeroize();
        }
        assert_eq!(actual_revision_ids, expected_revision_ids);
        crate::export::zeroize_cbor(&mut entries);

        let CborValue::Map(mut reversed_snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Array(candidate_entries) = reversed_snapshot
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        candidate_entries.reverse();
        refresh_portable_snapshot_hash(&mut reversed_snapshot);
        let reversed_snapshot =
            authenticate_portable_snapshot(reversed_snapshot, b"conflict export passphrase", 0x7f);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &reversed_snapshot,
                Zeroizing::new(b"conflict export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_import_rekeys_an_independently_reopenable_new_vault() {
        let (source_locator, source_local, source_bootstrap, source_factory) = initialized();
        let exact_active = source_local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(source_active) =
            LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let conflicted_source_item = ItemId::new([0x7d; 16]);
        let (publication, _) = pending_tombstone_publication(
            &source_active,
            conflicted_source_item,
            conflicted_source_item,
            2,
            None,
        );
        *source_local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(source_active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let live_randomness = add_item_randomness(0x9a);
        let live_source_item = live_randomness.item_id();
        source_session
            .add_item(
                new_login_document(live_source_item, "Restored portal", "restored-secret"),
                900,
                live_randomness,
                &source_local,
            )
            .unwrap();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        assert_eq!(source_session.item_count(), 2);
        assert_eq!(source_session.candidate_count(), 3);
        assert_eq!(source_session.conflicted_item_count(), 1);
        let source_vault_id = source_session.vault_id();
        let source_item_ids = source_session
            .current_catalog
            .items
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let source_revision_ids = source_session
            .current_catalog
            .items
            .values()
            .flatten()
            .map(ItemCandidate::revision_id)
            .collect::<BTreeSet<_>>();
        let exact_source_bootstrap = source_bootstrap.0.lock().unwrap().clone().unwrap();
        let artifact = source_session
            .export_portable_with_passphrase(
                &exact_source_bootstrap,
                Zeroizing::new(b"restore passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0x8b; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let import_random_byte_count = crate::portable_import_random_bytes(&opened).unwrap();
        assert_eq!(import_random_byte_count, 2 * 16 + 6 * 80 + 32);
        assert_eq!(
            crate::PortableImportRandomnessV1::new(
                vec![0x91; import_random_byte_count - 1],
                &opened,
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
        let import_randomness_bytes = (0..import_random_byte_count)
            .map(|index| (index as u8).wrapping_mul(43).wrapping_add(0x91))
            .collect::<Vec<_>>();
        let import_randomness =
            crate::PortableImportRandomnessV1::new(import_randomness_bytes.clone(), &opened)
                .unwrap();
        assert_eq!(
            format!("{import_randomness:?}"),
            "PortableImportRandomnessV1(<redacted>)"
        );

        let mut target_generation_zero = generation_zero_bytes();
        for byte in &mut target_generation_zero {
            *byte = byte.wrapping_add(0x65);
        }
        let (target_locator, target_local, target_bootstrap, target_factory) = initialized_with(
            b"independent target passphrase",
            GenerationZeroRandomness::new(target_generation_zero),
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_ne!(target_session.vault_id(), source_vault_id);
        target_local.fail_next_compare(LocalStateStoreError::Unavailable);
        assert_eq!(
            target_session
                .import_opened_portable_snapshot(opened, 901, import_randomness, &target_local)
                .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(target_session.item_count(), 0);
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let import_randomness =
            crate::PortableImportRandomnessV1::new(import_randomness_bytes, &opened).unwrap();
        let imported_active = target_session
            .import_opened_portable_snapshot(opened, 901, import_randomness, &target_local)
            .unwrap();
        assert_eq!(imported_active.last_device_counter(), 2);

        drop(source_session);
        let restored = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(restored.item_count(), 2);
        assert_eq!(restored.candidate_count(), 3);
        assert_eq!(restored.conflicted_item_count(), 1);
        assert!(restored
            .current_catalog
            .items
            .keys()
            .all(|item_id| !source_item_ids.contains(item_id)));
        assert!(restored
            .current_catalog
            .items
            .values()
            .flatten()
            .all(|candidate| {
                !source_revision_ids.contains(&candidate.revision_id())
                    && candidate.causal_parents().is_empty()
            }));
        let mut tombstone_times = Vec::new();
        let mut restored_login = None;
        for candidates in restored.current_catalog.items.values() {
            for candidate in candidates {
                assert_eq!(candidate.item_id(), candidate.state().item_id());
                match candidate.state() {
                    ItemState::Live(document) => {
                        let AnyRecord::Login(login) = document.payload() else {
                            panic!("fixture must retain a login")
                        };
                        restored_login = Some((login.title.clone(), login.password.clone()));
                    }
                    ItemState::Tombstone(tombstone) => {
                        tombstone_times.push(tombstone.deleted_at_ms);
                    }
                }
            }
        }
        tombstone_times.sort_unstable();
        assert_eq!(tombstone_times, vec![100, 101]);
        assert_eq!(
            restored_login,
            Some(("Restored portal".to_owned(), "restored-secret".to_owned()))
        );
        let audit = restored.audit_verify().unwrap();
        assert_eq!(audit.commit_count(), 2);
        assert_eq!(audit.catalog_count(), 2);
        assert_eq!(audit.revision_count(), 3);
        assert_eq!(audit.item_count(), 2);
    }

    #[test]
    fn portable_import_rejects_source_and_mutated_targets_before_local_writes() {
        let (source_locator, source_local, source_bootstrap, source_factory) = initialized();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let exact_source_bootstrap = source_bootstrap.0.lock().unwrap().clone().unwrap();
        let artifact = source_session
            .export_portable_with_passphrase(
                &exact_source_bootstrap,
                Zeroizing::new(b"empty restore passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0xa2; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"empty restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(crate::portable_import_random_bytes(&opened), Ok(272));
        let randomness = crate::PortableImportRandomnessV1::new(vec![0xa3; 272], &opened).unwrap();
        let source_compare_calls = source_local.3.load(Ordering::SeqCst);
        assert_eq!(
            source_session
                .import_opened_portable_snapshot(opened, 902, randomness, &source_local)
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(source_local.3.load(Ordering::SeqCst), source_compare_calls);

        let mut target_generation_zero = generation_zero_bytes();
        for byte in &mut target_generation_zero {
            *byte = byte.wrapping_add(0x43);
        }
        let (target_locator, target_local, target_bootstrap, target_factory) = initialized_with(
            b"mutated target passphrase",
            GenerationZeroRandomness::new(target_generation_zero),
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"mutated target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        let add_randomness = add_item_randomness(0xa4);
        let target_item_id = add_randomness.item_id();
        target_session
            .add_item(
                new_login_document(target_item_id, "Existing target", "target-secret"),
                903,
                add_randomness,
                &target_local,
            )
            .unwrap();
        let target_session = open_active_vault(
            Zeroizing::new(b"mutated target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"empty restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let randomness = crate::PortableImportRandomnessV1::new(vec![0xa5; 272], &opened).unwrap();
        let target_compare_calls = target_local.3.load(Ordering::SeqCst);
        assert_eq!(
            target_session
                .import_opened_portable_snapshot(opened, 904, randomness, &target_local)
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(target_local.3.load(Ordering::SeqCst), target_compare_calls);
    }

    #[test]
    fn audit_verify_rejects_a_local_counter_without_an_exact_pinned_anchor() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        session.active = ActiveStateV1::new(
            session.active.bootstrap_locator(),
            session.active.vault_id(),
            session.active.bootstrap_id(),
            session.active.authority_fingerprint(),
            session.active.device_id(),
            session.active.device_certificate_id(),
            session.active.device_certificate_frame().clone(),
            session.active.local_secret().clone(),
            session.active.pinned_heads().clone(),
            session.active.last_device_counter() + 1,
            session.active.catalog_root(),
        )
        .unwrap();
        assert_eq!(
            session.audit_verify(),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn lifecycle_retains_locked_state_on_failure_and_drops_session_on_lock() {
        let (locator, local, bootstrap, factory) = initialized();
        let locked = crate::LockedVaultV1::new(locator);
        assert_eq!(locked.locator(), locator);
        assert_eq!(format!("{locked:?}"), "LockedVaultV1(<locked>)");

        let mut access = crate::VaultAccessV1::locked(locator);
        assert!(access.is_locked());
        assert!(!access.is_unlocked());
        assert!(matches!(
            access.as_unlocked(),
            Err(ApplicationError::Locked)
        ));
        assert_eq!(format!("{access:?}"), "VaultAccessV1::Locked(<redacted>)");

        assert_eq!(
            access.unlock(
                Zeroizing::new(b"wrong".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::AuthenticationFailed)
        );
        assert!(access.is_locked());

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert!(access.is_unlocked());
        assert_eq!(access.as_unlocked().unwrap().item_count(), 0);
        assert_eq!(format!("{access:?}"), "VaultAccessV1::Unlocked(<redacted>)");
        assert_eq!(
            access.unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert!(access.is_unlocked());

        access.lock();
        assert!(access.is_locked());
        access.lock();
        assert!(matches!(
            access.into_unlocked(),
            Err(ApplicationError::Locked)
        ));

        let mut access = crate::VaultAccessV1::locked(locator);
        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(access.into_unlocked().unwrap().item_count(), 0);
    }

    #[test]
    fn status_is_safe_while_locked_and_adds_counts_only_while_unlocked() {
        let absent_local = MemoryLocalStateStore::default();
        let absent = crate::VaultAccessV1::locked(BootstrapLocator::new([0x91; 32]));
        let status = absent.status(&absent_local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Absent);
        assert_eq!(status.item_count(), None);
        assert_eq!(status.candidate_count(), None);
        assert_eq!(status.conflicted_item_count(), None);
        assert_eq!(format!("{status:?}"), "VaultStatusV1 { state: Absent }");

        let prepared = prepare_generation_zero(
            Zeroizing::new(b"prepared passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let prepared_locator = prepared.bootstrap_locator();
        let prepared_local =
            MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        let status = crate::VaultAccessV1::locked(prepared_locator)
            .status(&prepared_local)
            .unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Prepared);
        assert_eq!(status.item_count(), None);

        let (locator, local, bootstrap, factory) = initialized();
        let mut access = crate::VaultAccessV1::locked(locator);
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Locked);
        assert_eq!(status.item_count(), None);

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Unlocked);
        assert_eq!(status.item_count(), Some(0));
        assert_eq!(status.candidate_count(), Some(0));
        assert_eq!(status.conflicted_item_count(), Some(0));
        assert_eq!(
            format!("{status:?}"),
            "VaultStatusV1 { state: Unlocked, item_count: 0, candidate_count: 0, conflicted_item_count: 0 }"
        );

        access.lock();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::RecoveryRequired);
        assert_eq!(status.item_count(), None);
        assert_eq!(
            format!("{status:?}"),
            "VaultStatusV1 { state: RecoveryRequired }"
        );
    }

    #[test]
    fn doctor_reports_coarse_locked_and_unlocked_health_states() {
        let absent_local = MemoryLocalStateStore::default();
        let absent_bootstrap = MemoryBootstrapStore::default();
        let absent = crate::VaultAccessV1::locked(BootstrapLocator::new([0x93; 32]));
        assert_eq!(
            absent.doctor(&absent_local, &absent_bootstrap).state(),
            crate::VaultDoctorStateV1::InitializationRequired
        );

        let prepared = prepare_generation_zero(
            Zeroizing::new(b"prepared passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let prepared_locator = prepared.bootstrap_locator();
        let prepared_local =
            MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        assert_eq!(
            crate::VaultAccessV1::locked(prepared_locator)
                .doctor(&prepared_local, &absent_bootstrap)
                .state(),
            crate::VaultDoctorStateV1::InitializationRequired
        );

        let (locator, local, bootstrap, factory) = initialized();
        let mut access = crate::VaultAccessV1::locked(locator);
        let locked_report = access.doctor(&local, &bootstrap);
        assert_eq!(
            locked_report.state(),
            crate::VaultDoctorStateV1::AuthenticationRequired
        );
        assert_eq!(
            format!("{locked_report:?}"),
            "VaultDoctorReportV1 { state: AuthenticationRequired }"
        );

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(
            access.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::Healthy
        );

        access.lock();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        assert_eq!(
            access.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::RecoveryRequired
        );
    }

    #[test]
    fn doctor_closes_store_version_and_integrity_failures_without_detail() {
        struct FailingLocalStateStore(LocalStateStoreError);

        impl LocalStateStore for FailingLocalStateStore {
            fn load(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
                Err(self.0)
            }

            fn compare_exchange(
                &self,
                _locator: BootstrapLocator,
                _expected: Option<&[u8]>,
                _replacement: &[u8],
            ) -> Result<(), LocalStateStoreError> {
                Err(self.0)
            }
        }

        struct FailingBootstrapStore(BootstrapStoreError);

        impl BootstrapStore for FailingBootstrapStore {
            fn load_latest(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
                Err(self.0)
            }

            fn put_generation(
                &self,
                _locator: BootstrapLocator,
                _expected_previous: Option<BootstrapId>,
                _exact_bootstrap: &[u8],
            ) -> Result<(), BootstrapStoreError> {
                Err(self.0)
            }
        }

        let locator = BootstrapLocator::new([0x94; 32]);
        let access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .doctor(
                    &FailingLocalStateStore(LocalStateStoreError::Unavailable),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::LocalStateUnavailable
        );
        assert_eq!(
            access
                .doctor(
                    &FailingLocalStateStore(LocalStateStoreError::ConcurrentHost),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(
                    &MemoryLocalStateStore::with_state(vec![0xff]),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        let (locator, local, bootstrap, _) = initialized();
        let access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .doctor(
                    &local,
                    &FailingBootstrapStore(BootstrapStoreError::Unavailable),
                )
                .state(),
            crate::VaultDoctorStateV1::BootstrapUnavailable
        );
        assert_eq!(
            access
                .doctor(
                    &local,
                    &FailingBootstrapStore(BootstrapStoreError::Conflict),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(&local, &MemoryBootstrapStore::default())
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(&local, &MemoryBootstrapStore(Mutex::new(Some(vec![0xff]))),)
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        let unsupported_local =
            replace_top_level_version(local.0.lock().unwrap().as_deref().unwrap(), 2);
        let unsupported_local = MemoryLocalStateStore::with_state(unsupported_local);
        assert_eq!(
            access.doctor(&unsupported_local, &bootstrap).state(),
            crate::VaultDoctorStateV1::UnsupportedCapability
        );

        let unsupported_bootstrap =
            replace_top_level_version(bootstrap.0.lock().unwrap().as_deref().unwrap(), 2);
        let unsupported_bootstrap = MemoryBootstrapStore(Mutex::new(Some(unsupported_bootstrap)));
        assert_eq!(
            access.doctor(&local, &unsupported_bootstrap).state(),
            crate::VaultDoctorStateV1::UnsupportedCapability
        );
    }

    #[test]
    fn unlocked_doctor_distinguishes_repository_unavailability_from_integrity() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut unavailable = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        unavailable._repository = Box::new(FailingAuditRepository(
            ApplicationRepositoryError::StorageUnavailable,
        ));
        let unavailable = crate::VaultAccessV1::Unlocked(Box::new(unavailable));
        assert_eq!(
            unavailable.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::RepositoryUnavailable
        );

        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        assert_eq!(
            unavailable.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        *local.0.lock().unwrap() = Some(exact_active);

        let mut corrupt = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        corrupt._repository = Box::new(FailingAuditRepository(
            ApplicationRepositoryError::IntegrityFailure,
        ));
        let corrupt = crate::VaultAccessV1::Unlocked(Box::new(corrupt));
        assert_eq!(
            corrupt.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        *local.0.lock().unwrap() = None;
        assert_eq!(
            corrupt.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
    }

    #[test]
    fn locked_status_closes_owner_state_failures() {
        struct FailingLocalStateStore(LocalStateStoreError);

        impl LocalStateStore for FailingLocalStateStore {
            fn load(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
                Err(self.0)
            }

            fn compare_exchange(
                &self,
                _locator: BootstrapLocator,
                _expected: Option<&[u8]>,
                _replacement: &[u8],
            ) -> Result<(), LocalStateStoreError> {
                Err(self.0)
            }
        }

        let access = crate::VaultAccessV1::locked(BootstrapLocator::new([0x92; 32]));
        for (store_error, expected) in [
            (
                LocalStateStoreError::Unavailable,
                ApplicationError::StorageUnavailable,
            ),
            (
                LocalStateStoreError::ConcurrentHost,
                ApplicationError::ConcurrentHost,
            ),
            (
                LocalStateStoreError::Corruption,
                ApplicationError::IntegrityFailure,
            ),
        ] {
            assert_eq!(
                access.status(&FailingLocalStateStore(store_error)),
                Err(expected)
            );
        }

        let corrupt = MemoryLocalStateStore::with_state(vec![0xff]);
        assert_eq!(
            access.status(&corrupt),
            Err(ApplicationError::IntegrityFailure)
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
        assert_eq!(
            session.current_item_revision(item_id).unwrap(),
            Some(session.current_catalog.items[&item_id][0].revision_id())
        );
        assert_eq!(session.get_item(ItemId::new([0x28; 16])).unwrap(), None);
        assert_eq!(
            session
                .current_item_revision(ItemId::new([0x28; 16]))
                .unwrap(),
            None
        );
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
                assert_eq!(session.current_item_revision(item_id).unwrap(), None);
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
                assert_eq!(
                    session.current_item_revision(item_id),
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
    fn conflict_candidates_are_redacted_and_choose_resolution_retains_every_parent() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2a; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
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
        let views = session.conflict_candidates(item_id).unwrap();
        assert_eq!(
            views
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<Vec<_>>()
        );
        let titles = views
            .iter()
            .map(|view| {
                let RedactedRecordView::Login { title, .. } = &view.redacted_item().unwrap().record
                else {
                    panic!("conflict fixture must contain logins")
                };
                title.as_str()
            })
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Keep left"));
        assert!(titles.contains(&"Keep right"));
        let debug = format!("{views:?}");
        for hidden in [
            "Keep left",
            "Keep right",
            "left-secret",
            "right-secret",
            &item_id.to_user_string(),
        ] {
            assert!(!debug.contains(hidden));
        }

        let selected_revision = revisions
            .iter()
            .find(|(_, title)| title == "Keep right")
            .map(|(revision_id, _)| *revision_id)
            .unwrap();
        let prior_heads = session.local_pins().clone();
        let resolved = session
            .resolve_item_conflict(
                selected_revision,
                401,
                resolve_item_conflict_randomness(0x4d),
                &local,
            )
            .unwrap();
        assert_eq!(resolved.last_device_counter(), 3);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("resolution must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("selected live conflict candidate must remain live")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("selected conflict candidate must retain its schema")
        };
        assert_eq!(login.title, "Keep right");
        assert_eq!(login.password, "right-secret");
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 401);
    }

    #[test]
    fn authored_conflict_merge_publishes_complete_parent_set_and_document() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x25; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
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
        let prior_heads = session.local_pins().clone();
        session
            .merge_item_conflict(
                new_login_document(item_id, "Merged result", "merged-secret"),
                405,
                resolve_item_conflict_randomness(0x51),
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
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored merge must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored merge must publish a live document")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("authored merge must retain the input schema")
        };
        assert_eq!(login.title, "Merged result");
        assert_eq!(login.password, "merged-secret");
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 405);
    }

    #[test]
    fn authored_conflict_merge_rejects_missing_sole_and_changed_identity_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let missing = ItemId::new([0x26; 16]);
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
            .merge_item_conflict(
                new_login_document(missing, "Missing", "missing-secret"),
                406,
                resolve_item_conflict_randomness(0x52),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_empty).unwrap()
        else {
            panic!("fixture must be active")
        };
        let sole_id = ItemId::new([0x27; 16]);
        let publication = pending_live_publication(&active, sole_id, "Sole", "sole-secret");
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
        let exact_sole = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                login_document_with_times(sole_id, "Not a conflict", "secret", 100, 300),
                407,
                resolve_item_conflict_randomness(0x53),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_sole.as_slice())
        );

        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_sole).unwrap()
        else {
            panic!("fixture must be active")
        };
        let conflict_id = ItemId::new([0x28; 16]);
        let (publication, _) = pending_live_conflict_publication(&active, conflict_id);
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
        let exact_conflict = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                login_document_with_times(conflict_id, "Changed identity", "secret", 301, 301,),
                408,
                resolve_item_conflict_randomness(0x54),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_conflict.as_slice())
        );
    }

    #[test]
    fn authored_conflict_merge_rejects_all_tombstone_conflict_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x29; 16]);
        let (publication, _) = pending_tombstone_publication(&active, item_id, item_id, 2, None);
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
        let exact_conflict = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                new_login_document(item_id, "Cannot revive", "secret"),
                409,
                resolve_item_conflict_randomness(0x55),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_conflict.as_slice())
        );
    }

    #[test]
    fn conflict_resolution_rejects_missing_or_unconflicted_revisions_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            session.conflict_candidates(ItemId::new([0x2b; 16])),
            Err(ApplicationError::NotFound)
        );
        let exact_empty = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.resolve_item_conflict(
                RevisionId::new([0x2c; 32]),
                410,
                resolve_item_conflict_randomness(0x5d),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        let add_randomness = add_item_randomness(0x6d);
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
            new_login_document(item_id, "Sole candidate", "only-secret"),
            411,
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
        assert_eq!(
            session.conflict_candidates(item_id),
            Err(ApplicationError::ConflictRequired)
        );
        let sole_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_sole = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.resolve_item_conflict(
                sole_revision,
                412,
                resolve_item_conflict_randomness(0x7d),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_sole.as_slice())
        );
    }

    #[test]
    fn conflict_resolution_can_choose_a_retained_tombstone_without_losing_parents() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2d; 16]);
        let (publication, revisions) =
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
        let views = session.conflict_candidates(item_id).unwrap();
        assert!(views.iter().all(ItemHistoryViewV1::is_deleted));
        session
            .resolve_item_conflict(
                revisions[1],
                420,
                resolve_item_conflict_randomness(0x8d),
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
        assert_eq!(reopened.get_item(item_id).unwrap(), None);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("resolution must become the sole current candidate")
        };
        assert!(matches!(candidate.state(), ItemState::Tombstone(_)));
        assert_eq!(
            candidate.causal_parents(),
            &revisions.into_iter().collect::<BTreeSet<_>>()
        );
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
    fn active_audit_item_create_is_signed_encrypted_and_atomic_with_the_commit() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        let previous_event = ObjectId::new([0xee; 32]);
        let exact_prior = LocalVaultStateV1::Active(session.active.clone())
            .encode()
            .unwrap();
        session.active = session
            .active
            .clone()
            .with_audit_event_head(previous_event)
            .unwrap();
        let exact_audited = LocalVaultStateV1::Active(session.active.clone())
            .encode()
            .unwrap();
        local
            .compare_exchange(locator, Some(&exact_prior), &exact_audited)
            .unwrap();
        let (device_public, _) = generate_keypair(session._local_secret.device_signing_seed());
        let randomness = add_item_randomness(0x42);
        let item_id = randomness.item_id();
        let active = session
            .add_item(
                new_login_document(item_id, "Audited portal", "audited-secret"),
                302,
                randomness,
                &local,
            )
            .unwrap();
        let audit_head = active.audit_event_head().unwrap();
        assert_ne!(audit_head, previous_event);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(commit.added_objects().len(), 3);
        assert!(commit.added_objects().contains(&audit_head));
        let audit_object = reopened._repository.read_object(audit_head).unwrap();
        let plaintext = open_object(
            &reopened._keys,
            ObjectKind::AuditEvent,
            audit_object.frame(),
        )
        .unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        event.verify(&device_public).unwrap();
        assert_eq!(event.event().action(), AuditActionV1::ItemCreate);
        assert_eq!(event.event().item_id(), Some(item_id));
        assert_eq!(event.event().selected_revision(), None);
        assert_eq!(event.event().previous_event(), Some(previous_event));
        assert_eq!(
            event.event().basis_heads(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(event.event().device_counter(), 2);
        assert_eq!(event.event().timestamp_ms(), 302);
        assert_eq!(
            event.event().result_revision().unwrap().as_bytes(),
            commit
                .added_objects()
                .iter()
                .find(|id| **id != audit_head && **id != commit.catalog_root())
                .unwrap()
                .as_bytes()
        );
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
    fn reveal_item_revision_is_exact_reachable_live_and_zeroizing() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x92);
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
            new_login_document(item_id, "Reveal original", "original-secret"),
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
        let original_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                original_revision,
                new_login_document(item_id, "Reveal current", "current-secret"),
                501,
                replace_item_randomness(0x93),
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
        let current_revision = session.current_catalog.items[&item_id][0].revision_id();
        let original = session.reveal_item_revision(original_revision).unwrap();
        let AnyRecord::Login(login) = original.payload() else {
            panic!("fixture must reveal a login")
        };
        assert_eq!(login.title, "Reveal original");
        assert_eq!(login.password, "original-secret");

        let revealed = session
            .reveal_item_revision_field(
                original_revision,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::Clipboard,
            )
            .unwrap();
        assert_eq!(revealed.as_bytes(), b"original-secret");
        assert_eq!(revealed.encoding(), crate::RevealedSecretEncodingV1::Utf8);
        assert!(matches!(
            session.reveal_item_revision_field(
                original_revision,
                SecretFieldV1::CardCvv,
                SecretDisclosureIntentV1::Clipboard,
            ),
            Err(ApplicationError::InvalidInput)
        ));
        assert!(matches!(
            session.reveal_item_revision_field(
                RevisionId::new([0x94; 32]),
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: false },
            ),
            Err(ApplicationError::InvalidInput)
        ));

        let mut current = session.reveal_item_revision(current_revision).unwrap();
        let AnyRecord::Login(login) = current.payload() else {
            panic!("fixture must reveal a login")
        };
        assert_eq!(login.password, "current-secret");
        current.zeroize();
        let AnyRecord::Login(login) = current.payload() else {
            panic!("zeroized fixture must retain its record variant")
        };
        assert!(login.password.is_empty());
        assert!(login.title.is_empty());
        assert!(matches!(
            session.reveal_item_revision(RevisionId::new([0x94; 32])),
            Err(ApplicationError::NotFound)
        ));

        session
            .delete_item(
                current_revision,
                502,
                503,
                delete_item_randomness(0x95),
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
        assert!(matches!(
            session.reveal_item_revision(tombstone_revision),
            Err(ApplicationError::InvalidInput)
        ));
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
