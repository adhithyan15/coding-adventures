use crate::initialize::{unlock_active_material, UnlockedActiveMaterial};
use crate::{
    ActiveStateV1, ApplicationError, ApplicationRepository, ApplicationRepositoryError,
    ApplicationRepositoryFactory, BootstrapLocator, BootstrapStore, BootstrapStoreError,
    LocalSecretV1, LocalStateStore, LocalStateStoreError, LocalVaultStateV1, V1Keys,
};
use coding_adventures_vault_pm_format::{DeviceId, VaultId};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};

/// One authenticated active-vault session with live keys and a verified
/// repository view.
///
/// Dropping the session wipes its application keys and owner/device secrets.
/// The repository owns a separate wipe-on-drop verifier key set.
pub struct UnlockedVaultV1 {
    active: ActiveStateV1,
    report: OpenReport,
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
}

impl Debug for UnlockedVaultV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnlockedVaultV1")
            .field("local_pin_count", &self.active.pinned_heads().len())
            .field("verified_head_count", &self.report.heads().len())
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

    Ok(UnlockedVaultV1 {
        active,
        report,
        _keys: material.keys,
        _local_secret: material.local_secret,
        _repository: repository,
    })
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
        complete_generation_zero, encode_signed_commit, prepare_generation_zero, seal_object,
        CatalogV1, GenerationZeroPolicyV1, GenerationZeroRandomness, ObjectKind, ObjectRandomness,
        PublicationJournalV1, V1ApplicationRepositoryFactory, V1Keys, GENERATION_ZERO_RANDOM_BYTES,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_format::{AnnouncementV1, BootstrapId, CommitV1, Signature};
    use coding_adventures_vault_pm_storage::{
        FaultAction, FaultEffect, FaultInjectingObjectStore, InMemoryObjectStore, StoreOperation,
    };
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
        let signing_seed: [u8; 32] = fixture[168..200].try_into().unwrap();
        let (_, signing_secret) = generate_keypair(&signing_seed);
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &CatalogV1::empty().encode().unwrap(),
            &ObjectRandomness::new([0xd1; 32], [0xd2; 24], [0xd3; 24]),
        )
        .unwrap();
        let catalog_id = catalog_frame.id().unwrap();
        let parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
        let commit = CommitV1 {
            vault_id: active.vault_id(),
            device_id: active.device_id(),
            device_counter: active.last_device_counter() + 1,
            parents,
            catalog_root: catalog_id,
            added_objects: vec![catalog_id],
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
            vec![catalog_frame],
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
        assert_eq!(session.open_report().announcement_count(), 1);
        assert_eq!(session.open_report().commit_count(), 1);
        assert!(!session.open_report().fresh_device_unanchored());
        assert_eq!(
            format!("{session:?}"),
            "UnlockedVaultV1 { local_pin_count: 1, verified_head_count: 1, .. }"
        );
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
