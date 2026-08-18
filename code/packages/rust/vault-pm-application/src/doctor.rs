use crate::initialize::verify_active_bootstrap;
use crate::{
    ApplicationError, BootstrapStore, BootstrapStoreError, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, VaultAccessV1,
};
use core::fmt::{self, Debug, Formatter};

/// Closed coarse outcome from the read-only doctor workflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultDoctorStateV1 {
    /// Every check available to an authenticated session passed.
    Healthy,
    /// No active vault exists yet, or initialization is still prepared.
    InitializationRequired,
    /// A durable pending publication must be replayed before ordinary open.
    RecoveryRequired,
    /// The owner-private state provider could not be read.
    LocalStateUnavailable,
    /// The public bootstrap provider could not be read.
    BootstrapUnavailable,
    /// The encrypted immutable repository could not be read.
    RepositoryUnavailable,
    /// A required provider capability or persisted version is unsupported.
    UnsupportedCapability,
    /// Repository integrity cannot be checked until the vault is unlocked.
    AuthenticationRequired,
    /// Persisted state, authenticated bytes, identities, pins, or graph failed closed.
    IntegrityFailure,
}

/// Secret-free doctor report suitable for CLI and later UI rendering.
///
/// The report intentionally contains one coarse state only. It never includes
/// vault, device, item, revision, object, locator, or provider identities.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VaultDoctorReportV1 {
    state: VaultDoctorStateV1,
}

impl VaultDoctorReportV1 {
    const fn new(state: VaultDoctorStateV1) -> Self {
        Self { state }
    }

    /// Return the closed coarse doctor state.
    pub const fn state(&self) -> VaultDoctorStateV1 {
        self.state
    }
}

impl Debug for VaultDoctorReportV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VaultDoctorReportV1")
            .field("state", &self.state)
            .finish()
    }
}

pub(crate) fn doctor(
    access: &VaultAccessV1,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
) -> VaultDoctorReportV1 {
    let locator = match access {
        VaultAccessV1::Locked(locked) => locked.locator(),
        VaultAccessV1::Unlocked(session) => session.bootstrap_locator(),
    };
    let exact_local = match local_state_store.load(locator) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return VaultDoctorReportV1::new(match access {
                VaultAccessV1::Locked(_) => VaultDoctorStateV1::InitializationRequired,
                VaultAccessV1::Unlocked(_) => VaultDoctorStateV1::IntegrityFailure,
            })
        }
        Err(LocalStateStoreError::Unavailable) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::LocalStateUnavailable)
        }
        Err(LocalStateStoreError::ConcurrentHost | LocalStateStoreError::Corruption) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure)
        }
    };
    let local = match LocalVaultStateV1::decode(&exact_local) {
        Ok(value) => value,
        Err(ApplicationError::Unsupported) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::UnsupportedCapability)
        }
        Err(_) => return VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure),
    };
    let active = match (&local, access) {
        (LocalVaultStateV1::PreparedInit(_), VaultAccessV1::Locked(_)) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::InitializationRequired)
        }
        (
            LocalVaultStateV1::PendingPublication { .. } | LocalVaultStateV1::PendingRotation(_),
            VaultAccessV1::Locked(_),
        ) => return VaultDoctorReportV1::new(VaultDoctorStateV1::RecoveryRequired),
        (LocalVaultStateV1::Active(active), VaultAccessV1::Locked(_)) => active,
        (LocalVaultStateV1::Active(active), VaultAccessV1::Unlocked(session))
            if active == session.active_state() =>
        {
            active
        }
        _ => return VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure),
    };

    let exact_bootstrap = match bootstrap_store.load_latest(locator) {
        Ok(Some(value)) => value,
        Ok(None) => return VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure),
        Err(BootstrapStoreError::Unavailable) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::BootstrapUnavailable)
        }
        Err(BootstrapStoreError::Conflict | BootstrapStoreError::Corruption) => {
            return VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure)
        }
    };
    if let Err(error) = verify_active_bootstrap(active, &exact_bootstrap) {
        return VaultDoctorReportV1::new(match error {
            ApplicationError::Unsupported => VaultDoctorStateV1::UnsupportedCapability,
            _ => VaultDoctorStateV1::IntegrityFailure,
        });
    }

    let VaultAccessV1::Unlocked(session) = access else {
        return VaultDoctorReportV1::new(VaultDoctorStateV1::AuthenticationRequired);
    };
    match session.audit_verify() {
        Ok(_) => VaultDoctorReportV1::new(VaultDoctorStateV1::Healthy),
        Err(ApplicationError::StorageUnavailable) => {
            VaultDoctorReportV1::new(VaultDoctorStateV1::RepositoryUnavailable)
        }
        Err(ApplicationError::Unsupported) => {
            VaultDoctorReportV1::new(VaultDoctorStateV1::UnsupportedCapability)
        }
        Err(_) => VaultDoctorReportV1::new(VaultDoctorStateV1::IntegrityFailure),
    }
}
