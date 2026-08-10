use crate::{
    open_active_vault, ApplicationError, ApplicationRepositoryFactory, BootstrapLocator,
    BootstrapStore, LocalStateStore, UnlockedVaultV1,
};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};

/// Key-free handle for one locally configured vault.
///
/// The random locator is intentionally available only through an explicit
/// accessor and remains redacted from ordinary diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LockedVaultV1 {
    locator: BootstrapLocator,
}

impl LockedVaultV1 {
    /// Construct a locked handle from the host's configured opaque locator.
    pub const fn new(locator: BootstrapLocator) -> Self {
        Self { locator }
    }

    /// Return the opaque locator needed by injected owner-state adapters.
    pub const fn locator(&self) -> BootstrapLocator {
        self.locator
    }
}

impl Debug for LockedVaultV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("LockedVaultV1(<locked>)")
    }
}

/// Explicit host lifecycle boundary for a locked or unlocked vault.
///
/// This wrapper lets a long-lived CLI or later UI retain one state object
/// without retaining live keys while locked. It does not own timers, signals,
/// prompts, stores, or process lifecycle; hosts decide when to invoke
/// [`Self::unlock`] and [`Self::lock`].
pub enum VaultAccessV1 {
    /// No live application keys or decrypted repository view are retained.
    Locked(LockedVaultV1),
    /// One authenticated session owns live keys and verified decrypted state.
    Unlocked(Box<UnlockedVaultV1>),
}

impl VaultAccessV1 {
    /// Begin in the key-free locked state.
    pub const fn locked(locator: BootstrapLocator) -> Self {
        Self::Locked(LockedVaultV1::new(locator))
    }

    /// Return whether this boundary currently retains no unlocked session.
    pub const fn is_locked(&self) -> bool {
        matches!(self, Self::Locked(_))
    }

    /// Return whether this boundary currently owns an unlocked session.
    pub const fn is_unlocked(&self) -> bool {
        matches!(self, Self::Unlocked(_))
    }

    /// Borrow the unlocked session or return the stable `Locked` error class.
    pub fn as_unlocked(&self) -> Result<&UnlockedVaultV1, ApplicationError> {
        match self {
            Self::Locked(_) => Err(ApplicationError::Locked),
            Self::Unlocked(session) => Ok(session.as_ref()),
        }
    }

    /// Consume the boundary and return its unlocked session.
    ///
    /// A locked boundary fails with the stable `Locked` class and retains no
    /// secret material that needs recovery.
    pub fn into_unlocked(self) -> Result<UnlockedVaultV1, ApplicationError> {
        match self {
            Self::Locked(_) => Err(ApplicationError::Locked),
            Self::Unlocked(session) => Ok(*session),
        }
    }

    /// Authenticate and install one unlocked session in place.
    ///
    /// The state is changed only after the complete verified open succeeds.
    /// Authentication, storage, integrity, or repository failure therefore
    /// leaves this boundary locked and immediately drops all temporary key
    /// material created by the failed attempt.
    pub fn unlock(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
        repository_factory: &dyn ApplicationRepositoryFactory,
    ) -> Result<(), ApplicationError> {
        let Self::Locked(locked) = self else {
            return Err(ApplicationError::InvalidInput);
        };
        let session = open_active_vault(
            passphrase,
            locked.locator(),
            local_state_store,
            bootstrap_store,
            repository_factory,
        )?;
        *self = Self::Unlocked(Box::new(session));
        Ok(())
    }

    /// Synchronously drop the live session and return to a key-free state.
    ///
    /// Repeated locking is idempotent. Replacing the enum before returning
    /// causes the prior unlocked session, keys, local secret, search index,
    /// decrypted catalog, and repository verifier to drop in this call.
    pub fn lock(&mut self) {
        let locator = match self {
            Self::Locked(_) => return,
            Self::Unlocked(session) => session.bootstrap_locator(),
        };
        let unlocked = core::mem::replace(self, Self::locked(locator));
        drop(unlocked);
    }
}

impl Debug for VaultAccessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locked(_) => formatter.write_str("VaultAccessV1::Locked(<redacted>)"),
            Self::Unlocked(_) => formatter.write_str("VaultAccessV1::Unlocked(<redacted>)"),
        }
    }
}
