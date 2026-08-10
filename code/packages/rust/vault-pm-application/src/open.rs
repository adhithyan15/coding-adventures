use crate::initialize::unlock_active_material;
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
        complete_generation_zero, prepare_generation_zero, GenerationZeroPolicyV1,
        GenerationZeroRandomness, V1ApplicationRepositoryFactory, GENERATION_ZERO_RANDOM_BYTES,
    };
    use coding_adventures_vault_pm_format::BootstrapId;
    use coding_adventures_vault_pm_storage::InMemoryObjectStore;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MemoryLocalStateStore(Mutex<Option<Vec<u8>>>);

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

    fn randomness() -> GenerationZeroRandomness {
        let mut bytes = [0; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(7);
        }
        GenerationZeroRandomness::new(bytes)
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
        let local =
            MemoryLocalStateStore(Mutex::new(Some(prepared.owner_state().encode().unwrap())));
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
}
