use crate::{
    ApplicationError, LocalStateStore, LocalStateStoreError, LocalVaultStateV1, VaultAccessV1,
};
use core::fmt::{self, Debug, Formatter};

/// Closed low-resolution lifecycle state returned by the safe status workflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultStatusStateV1 {
    /// No owner-private state exists for the configured locator.
    Absent,
    /// Generation zero is durably prepared but not yet fully activated.
    Prepared,
    /// Stable active owner state exists without a retained live session.
    Locked,
    /// A verified live session is retained by the lifecycle boundary.
    Unlocked,
    /// An exact pending publication must be recovered before reopening.
    RecoveryRequired,
}

/// Secret-free status projection suitable for locked CLI and UI rendering.
///
/// Exact item, candidate, and conflicted-item counts are present only for an
/// already authenticated unlocked session. Every other state omits counts and
/// never exposes vault, device, item, revision, object, or provider identities.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VaultStatusV1 {
    state: VaultStatusStateV1,
    item_count: Option<usize>,
    candidate_count: Option<usize>,
    conflicted_item_count: Option<usize>,
}

impl VaultStatusV1 {
    fn without_counts(state: VaultStatusStateV1) -> Self {
        Self {
            state,
            item_count: None,
            candidate_count: None,
            conflicted_item_count: None,
        }
    }

    fn unlocked(item_count: usize, candidate_count: usize, conflicted_item_count: usize) -> Self {
        Self {
            state: VaultStatusStateV1::Unlocked,
            item_count: Some(item_count),
            candidate_count: Some(candidate_count),
            conflicted_item_count: Some(conflicted_item_count),
        }
    }

    /// Return the closed coarse lifecycle state.
    pub const fn state(&self) -> VaultStatusStateV1 {
        self.state
    }

    /// Return the authenticated current-item count only while unlocked.
    pub const fn item_count(&self) -> Option<usize> {
        self.item_count
    }

    /// Return the authenticated retained-candidate count only while unlocked.
    pub const fn candidate_count(&self) -> Option<usize> {
        self.candidate_count
    }

    /// Return the authenticated conflicted-item count only while unlocked.
    pub const fn conflicted_item_count(&self) -> Option<usize> {
        self.conflicted_item_count
    }
}

impl Debug for VaultStatusV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut value = formatter.debug_struct("VaultStatusV1");
        value.field("state", &self.state);
        if let (Some(item_count), Some(candidate_count), Some(conflicted_item_count)) = (
            self.item_count,
            self.candidate_count,
            self.conflicted_item_count,
        ) {
            value
                .field("item_count", &item_count)
                .field("candidate_count", &candidate_count)
                .field("conflicted_item_count", &conflicted_item_count);
        }
        value.finish()
    }
}

pub(crate) fn status(
    access: &VaultAccessV1,
    local_state_store: &dyn LocalStateStore,
) -> Result<VaultStatusV1, ApplicationError> {
    match access {
        VaultAccessV1::Unlocked(session) => Ok(VaultStatusV1::unlocked(
            session.item_count(),
            session.candidate_count(),
            session.conflicted_item_count(),
        )),
        VaultAccessV1::Locked(locked) => {
            let Some(encoded) = local_state_store
                .load(locked.locator())
                .map_err(map_local_state_store)?
            else {
                return Ok(VaultStatusV1::without_counts(VaultStatusStateV1::Absent));
            };
            let state = match LocalVaultStateV1::decode(&encoded)? {
                LocalVaultStateV1::PreparedInit(_) => VaultStatusStateV1::Prepared,
                LocalVaultStateV1::Active(_) => VaultStatusStateV1::Locked,
                // Both journals mean the same thing to a locked reader: a
                // durable, exact record of work an interrupted process left
                // for the next command to finish. Distinguishing them here
                // would leak which ceremony was interrupted into a projection
                // whose whole contract is to say as little as possible.
                LocalVaultStateV1::PendingPublication { .. }
                | LocalVaultStateV1::PendingRotation(_) => VaultStatusStateV1::RecoveryRequired,
            };
            Ok(VaultStatusV1::without_counts(state))
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
