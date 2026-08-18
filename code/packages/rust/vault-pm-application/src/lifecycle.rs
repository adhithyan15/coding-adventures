use crate::{
    open_active_vault, recover_pending_publication, ApplicationError, ApplicationRepositoryFactory,
    BootstrapLocator, BootstrapStore, LocalStateStore, UnlockedVaultV1, VaultStatusStateV1,
    VaultStatusV1,
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

/// What an unlock had to finish before it could open the vault.
///
/// A crash inside a mutation publication is invisible to the person who caused
/// it — the machine simply stopped — so the host that repairs it is the only
/// party in a position to mention it afterwards. This closed enum is the whole
/// vocabulary needed for that: it carries no vault, item, revision, object, or
/// provider identity, and no count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnlockRecoveryV1 {
    /// Durable owner state was already `Active`; nothing was published.
    AlreadyActive,
    /// One exact `PendingPublication` journal was replayed to completion
    /// before the vault was opened.
    RecoveredPendingPublication,
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

    /// Return a secret-free low-resolution status projection.
    ///
    /// Locked access reads and strictly decodes only the owner-private state
    /// needed to distinguish absent, prepared, active, and recovery-required
    /// states. Unlocked access reports authenticated aggregate counts from the
    /// retained session and does not consult an external provider.
    pub fn status(
        &self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<VaultStatusV1, ApplicationError> {
        crate::status::status(self, local_state_store)
    }

    /// Run read-only low-resolution health checks without repairing state.
    ///
    /// Locked access checks owner-state and public-bootstrap availability, then
    /// reports `AuthenticationRequired` because repository addressing and
    /// verification require authenticated secrets. Unlocked access additionally
    /// rechecks exact local/bootstrap binding and runs the complete vault audit.
    pub fn doctor(
        &self,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
    ) -> crate::VaultDoctorReportV1 {
        crate::doctor::doctor(self, local_state_store, bootstrap_store)
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

    /// Finish an interrupted publication if one is durable, then unlock.
    ///
    /// # Why this exists
    ///
    /// `VLT-PM05-application.md` §8 step 2 requires an open to "resume a
    /// prepared initialization or pending publication when present". The
    /// `PreparedInit` half of that sentence is `init`'s resume path. This is
    /// the other half, and until VLT-PM42 nothing in the product performed it:
    /// [`Self::unlock`] refuses every owner state but `Active`, so a process
    /// killed inside [`crate::UnlockedVaultV1`]'s publication path left a vault
    /// that was intact, exactly journalled, correctly diagnosed — and that no
    /// command could open. VLT-PM41 §8 found that and measured it as an
    /// availability defect in shipped code.
    ///
    /// # What it does
    ///
    /// ```text
    ///   PendingPublication ──▶ recover_pending_publication ──┐
    ///                                                        ├──▶ open_active_vault
    ///   anything else ───────────────────────────────────────┘
    /// ```
    ///
    /// The recovery replays the already-signed bytes idempotently and advances
    /// the owner state only after the repository returns exactly the heads the
    /// journal expected. Its result is then deliberately *discarded*: the vault
    /// is opened from durable state by the ordinary strict
    /// [`open_active_vault`], so every check a later, uninvolved process would
    /// perform runs here too, on the repaired bytes. A repair that produced a
    /// session only the repairing process could reproduce would be worth less
    /// than no repair at all.
    ///
    /// # What it costs
    ///
    /// One extra Argon2id derivation, and only when a repair actually happens:
    /// recovery and open each authenticate the passphrase against the bootstrap
    /// root wrap. The `AlreadyActive` path derives exactly once, as before. A
    /// process that finds a wedged vault has already survived a crash; paying
    /// one more key derivation to reopen it from scratch is the right trade.
    ///
    /// # Secret handling
    ///
    /// Both callees consume a passphrase by value, and
    /// [`Zeroizing`] deliberately implements neither `Clone` nor `Debug` so
    /// that duplicating a secret cannot happen by accident. The duplicate below
    /// is therefore constructed by name, exists only inside the recovering
    /// branch, and is wiped on drop — including while unwinding from a panic.
    ///
    /// # Failure
    ///
    /// A wrong passphrase is rejected by the recovery *before* any publication,
    /// with the same closed authentication class an ordinary unlock returns,
    /// and leaves the exact journal in place for a later correct attempt. A
    /// `PreparedInit` state is refused: it belongs to `init`. Any failure
    /// leaves this boundary locked, and repeating the whole call is always
    /// sound because the recovery is idempotent.
    pub fn unlock_recovering_pending_publication(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
        repository_factory: &dyn ApplicationRepositoryFactory,
    ) -> Result<UnlockRecoveryV1, ApplicationError> {
        let Self::Locked(locked) = self else {
            return Err(ApplicationError::InvalidInput);
        };
        let locator = locked.locator();
        // The same read-only projection `status` exposes to a locked host, so
        // there is exactly one place in this crate that decides which durable
        // owner states mean "recovery required".
        let recovery_required = matches!(
            self.status(local_state_store)?.state(),
            VaultStatusStateV1::RecoveryRequired
        );
        let outcome = if recovery_required {
            recover_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                locator,
                local_state_store,
                bootstrap_store,
                repository_factory,
            )?;
            UnlockRecoveryV1::RecoveredPendingPublication
        } else {
            UnlockRecoveryV1::AlreadyActive
        };
        self.unlock(
            passphrase,
            local_state_store,
            bootstrap_store,
            repository_factory,
        )?;
        Ok(outcome)
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
