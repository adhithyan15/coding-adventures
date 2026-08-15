//! Injected originator-key custody for D18T.
//!
//! The public contract selects one complete candidate atomically and exposes
//! only redacted handles to the orchestration layer. The included memory
//! implementation is deterministic test infrastructure and reports itself as
//! non-durable, so the production constructor rejects it.

use std::collections::BTreeMap;
use std::sync::Mutex;

use chief_of_staff_channel_crypto::{ChannelId, ChannelMasterKey, KeyEpoch};
use coding_adventures_ct_compare::ct_eq_fixed;

/// The result of an atomic create-if-absent custody operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodySelection {
    /// The complete candidate became the durable owner.
    Selected,
    /// The exact public bundle and secret key already own the slot.
    Idempotent,
    /// Different bytes already own the slot.
    Conflict,
}

/// Opaque secret-custody failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyError {
    /// The custody implementation is unavailable.
    Unavailable,
    /// A handle does not resolve to retained key material.
    MissingKey,
}

impl core::fmt::Display for CustodyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "originator key custody unavailable",
            Self::MissingKey => "originator key is unavailable",
        })
    }
}

impl std::error::Error for CustodyError {}

/// Redacted reference to one retained epoch key.
#[derive(Clone, PartialEq, Eq)]
pub struct EpochKeyHandle {
    channel_id: ChannelId,
    epoch: KeyEpoch,
}

impl EpochKeyHandle {
    /// Return the public channel identity associated with this handle.
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Return the public epoch associated with this handle.
    pub const fn epoch(&self) -> KeyEpoch {
        self.epoch
    }
}

impl core::fmt::Debug for EpochKeyHandle {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("EpochKeyHandle([REDACTED])")
    }
}

/// Secret-free recovery bundle returned after process restart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicPreparation {
    channel_id: ChannelId,
    base_epoch: KeyEpoch,
    new_epoch: KeyEpoch,
    plan_bytes: Vec<u8>,
    grants: Vec<Vec<u8>>,
}

impl PublicPreparation {
    /// Construct an exact secret-free replay bundle.
    pub fn new(
        channel_id: ChannelId,
        base_epoch: KeyEpoch,
        new_epoch: KeyEpoch,
        plan_bytes: Vec<u8>,
        grants: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            channel_id,
            base_epoch,
            new_epoch,
            plan_bytes,
            grants,
        }
    }

    /// Return the public channel identity.
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Return the public base epoch.
    pub const fn base_epoch(&self) -> KeyEpoch {
        self.base_epoch
    }

    /// Return the public successor epoch.
    pub const fn new_epoch(&self) -> KeyEpoch {
        self.new_epoch
    }

    /// Borrow the exact canonical D18T plan bytes.
    pub fn plan_bytes(&self) -> &[u8] {
        &self.plan_bytes
    }

    /// Borrow exact canonical D18G bytes in raw receiver-ID order.
    pub fn grants(&self) -> &[Vec<u8>] {
        &self.grants
    }
}

/// One indivisible candidate offered to secret custody.
pub struct PreparedEpoch {
    public: PublicPreparation,
    cmk: ChannelMasterKey,
}

impl PreparedEpoch {
    /// Own one complete public recovery bundle and its secret CMK.
    pub fn new(public: PublicPreparation, cmk: ChannelMasterKey) -> Self {
        Self { public, cmk }
    }

    /// Borrow the secret-free recovery portion.
    pub fn public(&self) -> &PublicPreparation {
        &self.public
    }

    /// Borrow the CMK only for an immediate custody operation.
    pub fn cmk(&self) -> &ChannelMasterKey {
        &self.cmk
    }

    fn into_parts(self) -> (PublicPreparation, ChannelMasterKey) {
        (self.public, self.cmk)
    }
}

impl core::fmt::Debug for PreparedEpoch {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PreparedEpoch")
            .field("channel_id", &self.public.channel_id)
            .field("base_epoch", &self.public.base_epoch)
            .field("new_epoch", &self.public.new_epoch)
            .field("plan_bytes", &self.public.plan_bytes)
            .field("grant_count", &self.public.grants.len())
            .field("cmk", &"[REDACTED]")
            .finish()
    }
}

/// Atomic originator-key custody required by D18T.
pub trait OriginatorKeyCustody: Send + Sync {
    /// Report whether values survive process and machine restart.
    fn is_durable(&self) -> bool;

    /// Atomically import the CMK for an already-active version 1 channel.
    fn import_active_if_absent(
        &self,
        channel_id: ChannelId,
        epoch: KeyEpoch,
        cmk: &ChannelMasterKey,
    ) -> Result<CustodySelection, CustodyError>;

    /// Resolve one public channel/epoch to an opaque handle.
    fn resolve_handle(
        &self,
        channel_id: ChannelId,
        epoch: KeyEpoch,
    ) -> Result<Option<EpochKeyHandle>, CustodyError>;

    /// Atomically select one complete prepared-epoch bundle.
    fn prepare_if_absent(&self, prepared: PreparedEpoch) -> Result<CustodySelection, CustodyError>;

    /// Reload the exact public recovery bundle after restart.
    fn load_preparation(
        &self,
        channel_id: ChannelId,
        new_epoch: KeyEpoch,
    ) -> Result<Option<PublicPreparation>, CustodyError>;

    /// Use a resolved CMK without returning its bytes to the caller.
    fn with_key<T>(
        &self,
        handle: &EpochKeyHandle,
        operation: impl FnOnce(&ChannelMasterKey) -> T,
    ) -> Result<T, CustodyError>;

    /// Apply configured logical secret destruction for one channel.
    fn destroy_channel(&self, channel_id: ChannelId) -> Result<(), CustodyError>;
}

#[derive(Default)]
struct MemoryCustodyState {
    keys: BTreeMap<(ChannelId, KeyEpoch), ChannelMasterKey>,
    preparations: BTreeMap<(ChannelId, KeyEpoch), PublicPreparation>,
}

/// Deterministic non-durable custody for tests and fixture generation only.
#[derive(Default)]
pub struct InMemoryKeyCustody {
    state: Mutex<MemoryCustodyState>,
}

impl InMemoryKeyCustody {
    /// Create empty test-only custody.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of retained secret epoch keys for test assertions.
    pub fn retained_key_count(&self) -> Result<usize, CustodyError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| CustodyError::Unavailable)?
            .keys
            .len())
    }
}

impl OriginatorKeyCustody for InMemoryKeyCustody {
    fn is_durable(&self) -> bool {
        false
    }

    fn import_active_if_absent(
        &self,
        channel_id: ChannelId,
        epoch: KeyEpoch,
        cmk: &ChannelMasterKey,
    ) -> Result<CustodySelection, CustodyError> {
        let mut state = self.state.lock().map_err(|_| CustodyError::Unavailable)?;
        match state.keys.get(&(channel_id, epoch)) {
            Some(existing) if ct_eq_fixed(existing.as_bytes(), cmk.as_bytes()) => {
                Ok(CustodySelection::Idempotent)
            }
            Some(_) => Ok(CustodySelection::Conflict),
            None => {
                state.keys.insert(
                    (channel_id, epoch),
                    ChannelMasterKey::from_bytes(*cmk.as_bytes()),
                );
                Ok(CustodySelection::Selected)
            }
        }
    }

    fn resolve_handle(
        &self,
        channel_id: ChannelId,
        epoch: KeyEpoch,
    ) -> Result<Option<EpochKeyHandle>, CustodyError> {
        let state = self.state.lock().map_err(|_| CustodyError::Unavailable)?;
        Ok(state
            .keys
            .contains_key(&(channel_id, epoch))
            .then_some(EpochKeyHandle { channel_id, epoch }))
    }

    fn prepare_if_absent(&self, prepared: PreparedEpoch) -> Result<CustodySelection, CustodyError> {
        let key = (prepared.public.channel_id, prepared.public.new_epoch);
        let mut state = self.state.lock().map_err(|_| CustodyError::Unavailable)?;
        match (state.preparations.get(&key), state.keys.get(&key)) {
            (Some(public), Some(cmk))
                if public == &prepared.public
                    && ct_eq_fixed(cmk.as_bytes(), prepared.cmk.as_bytes()) =>
            {
                Ok(CustodySelection::Idempotent)
            }
            (None, None) => {
                let (public, cmk) = prepared.into_parts();
                state.preparations.insert(key, public);
                state.keys.insert(key, cmk);
                Ok(CustodySelection::Selected)
            }
            _ => Ok(CustodySelection::Conflict),
        }
    }

    fn load_preparation(
        &self,
        channel_id: ChannelId,
        new_epoch: KeyEpoch,
    ) -> Result<Option<PublicPreparation>, CustodyError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| CustodyError::Unavailable)?
            .preparations
            .get(&(channel_id, new_epoch))
            .cloned())
    }

    fn with_key<T>(
        &self,
        handle: &EpochKeyHandle,
        operation: impl FnOnce(&ChannelMasterKey) -> T,
    ) -> Result<T, CustodyError> {
        let state = self.state.lock().map_err(|_| CustodyError::Unavailable)?;
        let cmk = state
            .keys
            .get(&(handle.channel_id, handle.epoch))
            .ok_or(CustodyError::MissingKey)?;
        Ok(operation(cmk))
    }

    fn destroy_channel(&self, channel_id: ChannelId) -> Result<(), CustodyError> {
        let mut state = self.state.lock().map_err(|_| CustodyError::Unavailable)?;
        state
            .keys
            .retain(|(candidate, _), _| *candidate != channel_id);
        state
            .preparations
            .retain(|(candidate, _), _| *candidate != channel_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_id() -> ChannelId {
        ChannelId([
            0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ])
    }

    #[test]
    fn active_import_is_constant_time_idempotent_and_conflicting() {
        let custody = InMemoryKeyCustody::new();
        let key = ChannelMasterKey::from_bytes([0x11; 32]);
        assert_eq!(
            custody
                .import_active_if_absent(channel_id(), KeyEpoch(4), &key)
                .unwrap(),
            CustodySelection::Selected
        );
        assert_eq!(
            custody
                .import_active_if_absent(channel_id(), KeyEpoch(4), &key)
                .unwrap(),
            CustodySelection::Idempotent
        );
        assert_eq!(
            custody
                .import_active_if_absent(
                    channel_id(),
                    KeyEpoch(4),
                    &ChannelMasterKey::from_bytes([0x22; 32]),
                )
                .unwrap(),
            CustodySelection::Conflict
        );
        let handle = custody
            .resolve_handle(channel_id(), KeyEpoch(4))
            .unwrap()
            .unwrap();
        assert_eq!(format!("{handle:?}"), "EpochKeyHandle([REDACTED])");
        assert!(custody
            .with_key(&handle, |cmk| cmk.as_bytes() == &[0x11; 32])
            .unwrap());
    }

    #[test]
    fn preparation_selects_one_complete_bundle_and_destruction_erases_keys() {
        let custody = InMemoryKeyCustody::new();
        let public = PublicPreparation::new(
            channel_id(),
            KeyEpoch(4),
            KeyEpoch(5),
            b"plan".to_vec(),
            vec![b"grant".to_vec()],
        );
        let prepared = PreparedEpoch::new(public.clone(), ChannelMasterKey::from_bytes([0x33; 32]));
        let diagnostic = format!("{prepared:?}");
        assert!(diagnostic.contains("[REDACTED]"));
        assert!(!diagnostic.contains("3333333333333333"));
        assert_eq!(
            custody.prepare_if_absent(prepared).unwrap(),
            CustodySelection::Selected
        );
        assert_eq!(
            custody.load_preparation(channel_id(), KeyEpoch(5)).unwrap(),
            Some(public)
        );
        assert_eq!(custody.retained_key_count().unwrap(), 1);
        custody.destroy_channel(channel_id()).unwrap();
        assert_eq!(custody.retained_key_count().unwrap(), 0);
    }
}
