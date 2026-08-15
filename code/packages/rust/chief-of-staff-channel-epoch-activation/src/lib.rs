//! Crash-safe durable D18T channel epoch activation.
//!
//! The crate composes production D18P storage records and D18Q grants without
//! assuming a multi-record storage transaction. Secret custody selects and
//! durably retains one complete successor bundle first; immutable public
//! records are then replayable until one CAS advances the same D18S record used
//! by publish reservations.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::grant_profile::{
    grant_deserialize, grant_serialize, verify_grant_signature, KeyGrantProfileError, RotationPlan,
};
use chief_of_staff_channel_crypto::wire::{
    decode_message, encode_message, key_grant_record_key, message_record_key,
    sequence_state_record_key, CHANNEL_STORAGE_NAMESPACE,
};
use chief_of_staff_channel_crypto::{
    encrypt_message_with_header, prepare_message_header, ChannelCryptoError, ChannelId,
    ChannelMasterKey, EncryptedMessage, KeyEpoch, MessageFields, MessageHeader,
    OriginatorSigningKey, Sequence,
};
use chief_of_staff_channel_endpoints::{
    ChannelDefinition, ChannelDefinitionStore, ChannelEndpointError, ChannelLifecycle,
    ReceiverIdentity,
};
use chief_of_staff_channel_store::profile::{
    channel_state_deserialize, CHANNEL_STATE_CONTENT_TYPE,
};
use coding_adventures_json_value::JsonValue;
use coding_adventures_sha256::sha256;
use storage_core::{StorageBackend, StorageError, StoragePutInput, StorageRecord};

pub mod custody;
pub mod wire;

pub use custody::{
    CustodyError, CustodySelection, EpochKeyHandle, InMemoryKeyCustody, OriginatorKeyCustody,
    PreparedEpoch, PublicPreparation,
};
pub use wire::{
    activation_plan_deserialize, activation_plan_record_key, activation_plan_serialize,
    epoch_state_deserialize, epoch_state_serialize, ActivationPlan, ActivationPlanEntry,
    EpochState, EpochWireError, ACTIVATION_PLAN_CONTENT_TYPE, EPOCH_STATE_CONTENT_TYPE,
    MAX_PENDING_HEADER_BYTES, MAX_PLAN_RECEIVERS,
};

const MAX_CAS_ATTEMPTS: usize = 16;
const MESSAGE_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-message-v1";
const GRANT_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-key-grant-v1";

/// Stable D18T failures. Display strings never include secret values.
#[derive(Debug)]
pub enum EpochActivationError {
    /// The injected public backend failed.
    Storage(StorageError),
    /// A public D18T record failed structural validation.
    Wire(EpochWireError),
    /// D18Q rejected grant bytes or cryptographic provenance.
    Grant(KeyGrantProfileError),
    /// D18F message cryptography failed.
    Crypto(ChannelCryptoError),
    /// Injected custody failed without exposing secret details.
    Custody(CustodyError),
    /// Production construction received non-durable custody.
    NonDurableCustody,
    /// No durable channel definition or state exists.
    NotInitialized,
    /// The immutable definition has been destroyed.
    ChannelDestroyed,
    /// A candidate violates D18T/D18Q or target-roster invariants.
    InvalidPlan,
    /// A stored envelope, key, metadata, or body is inconsistent.
    CorruptRecord,
    /// A publish reservation prevents activation or another reservation.
    PendingAppend,
    /// A caller named an epoch that is not active.
    UnactivatedEpoch,
    /// Custody cannot resolve the public active epoch.
    ActiveKeyMissing,
    /// Active-key import conflicts with retained bytes.
    ConflictingActiveKey,
    /// No selected recovery bundle exists for the requested epoch.
    PreparationMissing,
    /// Another prepared bundle owns the epoch slot.
    ConflictingPreparation,
    /// Another immutable activation plan owns its public key.
    ConflictingPlan,
    /// Another immutable D18G grant owns its public key.
    ConflictingGrant,
    /// A selected plan is not exactly the current successor.
    UnexpectedEpoch,
    /// Activation targeted an epoch older than the public current epoch.
    DecreasingEpoch,
    /// The public epoch cannot advance beyond `u64::MAX`.
    EpochExhausted,
    /// Sixteen revision-CAS attempts did not converge.
    ConcurrentUpdate,
}

impl EpochActivationError {
    /// Return the portable machine-readable error code.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage_error",
            Self::Wire(_) | Self::CorruptRecord => "corrupt_record",
            Self::Grant(_) | Self::Crypto(_) => "crypto_error",
            Self::Custody(_) | Self::NonDurableCustody => "custody_error",
            Self::NotInitialized => "not_initialized",
            Self::ChannelDestroyed => "channel_destroyed",
            Self::InvalidPlan => "invalid_plan",
            Self::PendingAppend => "pending_append",
            Self::UnactivatedEpoch => "unactivated_epoch",
            Self::ActiveKeyMissing => "active_key_missing",
            Self::ConflictingActiveKey => "conflicting_active_key",
            Self::PreparationMissing => "preparation_missing",
            Self::ConflictingPreparation => "conflicting_preparation",
            Self::ConflictingPlan => "conflicting_plan",
            Self::ConflictingGrant => "conflicting_grant",
            Self::UnexpectedEpoch => "unexpected_epoch",
            Self::DecreasingEpoch => "decreasing_epoch",
            Self::EpochExhausted => "epoch_exhausted",
            Self::ConcurrentUpdate => "concurrent_update",
        }
    }
}

impl core::fmt::Display for EpochActivationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for EpochActivationError {}

impl From<StorageError> for EpochActivationError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<EpochWireError> for EpochActivationError {
    fn from(error: EpochWireError) -> Self {
        Self::Wire(error)
    }
}

impl From<KeyGrantProfileError> for EpochActivationError {
    fn from(error: KeyGrantProfileError) -> Self {
        Self::Grant(error)
    }
}

impl From<ChannelCryptoError> for EpochActivationError {
    fn from(error: ChannelCryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<CustodyError> for EpochActivationError {
    fn from(error: CustodyError) -> Self {
        Self::Custody(error)
    }
}

/// Result of preparing or replaying one successor epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationOutcome {
    /// This call selected the complete bundle and persisted public records.
    Prepared,
    /// The same selected bundle and public records already existed.
    Idempotent,
}

/// Result of the one active-epoch CAS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationOutcome {
    /// This call advanced the active epoch.
    Activated,
    /// The exact selected plan had already advanced it.
    Idempotent,
}

/// Caller-owned publish metadata without a caller-selected key epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveEpochAppendRequest {
    /// Canonical message UUID v7 bytes.
    pub message_id: [u8; 16],
    /// Injected monotonic timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Authorized originator identity.
    pub originator_id: Vec<u8>,
    /// MIME content type authenticated with the message.
    pub content_type: String,
}

/// Exact reserved D18H plus a redacted handle for its active CMK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochReservation {
    header: MessageHeader,
    key_handle: EpochKeyHandle,
}

impl EpochReservation {
    /// Borrow the exact durable D18H reservation.
    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    /// Borrow the redacted active-key handle.
    pub fn key_handle(&self) -> &EpochKeyHandle {
        &self.key_handle
    }
}

/// D18T orchestration over injected public storage and secret custody.
pub struct EpochActivationStore<'a, C: OriginatorKeyCustody> {
    backend: &'a dyn StorageBackend,
    custody: &'a C,
    channel_id: ChannelId,
}

impl<'a, C: OriginatorKeyCustody> EpochActivationStore<'a, C> {
    /// Construct a production D18T store, rejecting non-durable custody.
    pub fn new(
        backend: &'a dyn StorageBackend,
        custody: &'a C,
        channel_id: ChannelId,
    ) -> Result<Self, EpochActivationError> {
        if !custody.is_durable() {
            return Err(EpochActivationError::NonDurableCustody);
        }
        backend.initialize()?;
        Ok(Self {
            backend,
            custody,
            channel_id,
        })
    }

    /// Construct a deterministic fixture/test store with explicitly
    /// non-durable custody. Production code MUST use [`Self::new`].
    pub fn new_for_testing(
        backend: &'a dyn StorageBackend,
        custody: &'a C,
        channel_id: ChannelId,
    ) -> Result<Self, EpochActivationError> {
        backend.initialize()?;
        Ok(Self {
            backend,
            custody,
            channel_id,
        })
    }

    /// Upgrade absent or D18S version 1 state after proving current-key
    /// custody. An already-valid version 2 state is idempotent.
    pub fn migrate_epoch_state(
        &self,
        definition: &ChannelDefinition,
        current_cmk: Option<&ChannelMasterKey>,
    ) -> Result<EpochState, EpochActivationError> {
        self.require_definition(definition, false)?;
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self.state_record()?;
            if let Some(record) = &record {
                if record.content_type == EPOCH_STATE_CONTENT_TYPE {
                    let state = self.decode_v2_state_record(record)?;
                    if self
                        .custody
                        .resolve_handle(self.channel_id, state.active_epoch())?
                        .is_none()
                    {
                        return Err(EpochActivationError::ActiveKeyMissing);
                    }
                    return Ok(state);
                }
            }

            self.ensure_initial_key(definition.key_epoch(), current_cmk)?;
            let state = match &record {
                None => {
                    EpochState::new(self.channel_id, definition.key_epoch(), Sequence(0), None)?
                }
                Some(record) => {
                    self.require_record_envelope(
                        record,
                        &sequence_state_record_key(self.channel_id),
                        CHANNEL_STATE_CONTENT_TYPE,
                    )?;
                    let prior = channel_state_deserialize(&record.body, self.channel_id)
                        .map_err(|_| EpochActivationError::CorruptRecord)?;
                    EpochState::new(
                        self.channel_id,
                        definition.key_epoch(),
                        prior.next_sequence,
                        prior.pending_header,
                    )?
                }
            };
            let mut input = public_put(
                sequence_state_record_key(self.channel_id),
                EPOCH_STATE_CONTENT_TYPE,
                epoch_state_serialize(&state)?,
            )?;
            input = match &record {
                None => input.with_if_absent(),
                Some(record) => input.with_if_revision(Some(record.revision.clone())),
            };
            match self.backend.put(input) {
                Ok(stored) => return self.decode_v2_state_record(&stored),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(EpochActivationError::ConcurrentUpdate)
    }

    /// Load the current D18S version 2 state.
    pub fn state(&self) -> Result<EpochState, EpochActivationError> {
        let record = self
            .state_record()?
            .ok_or(EpochActivationError::NotInitialized)?;
        self.decode_v2_state_record(&record)
    }

    /// Select one D18Q rotation in custody, then replay its exact immutable
    /// plan and grants into public storage.
    pub fn prepare_rotation(
        &self,
        definition: &ChannelDefinition,
        target_roster: &[ReceiverIdentity],
        rotation: RotationPlan,
    ) -> Result<PreparationOutcome, EpochActivationError> {
        self.require_definition(definition, false)?;
        let state = self.state()?;
        if state.pending_header().is_some() {
            return Err(EpochActivationError::PendingAppend);
        }
        let expected = state
            .active_epoch()
            .0
            .checked_add(1)
            .map(KeyEpoch)
            .ok_or(EpochActivationError::EpochExhausted)?;
        if rotation.new_epoch() != expected {
            return Err(EpochActivationError::UnexpectedEpoch);
        }
        let prepared =
            prepare_rotation_candidate(definition, state.active_epoch(), target_roster, rotation)?;
        let selection = self.custody.prepare_if_absent(prepared)?;
        let outcome = match selection {
            CustodySelection::Selected => PreparationOutcome::Prepared,
            CustodySelection::Idempotent => PreparationOutcome::Idempotent,
            CustodySelection::Conflict => return Err(EpochActivationError::ConflictingPreparation),
        };
        self.replay_preparation(definition, expected)?;
        Ok(outcome)
    }

    /// Replay one custody-selected plan and every exact grant after restart.
    pub fn recover_preparation(
        &self,
        definition: &ChannelDefinition,
        new_epoch: KeyEpoch,
    ) -> Result<PreparationOutcome, EpochActivationError> {
        self.require_definition(definition, false)?;
        let active_epoch = self.state()?.active_epoch();
        if new_epoch < active_epoch {
            return Err(EpochActivationError::DecreasingEpoch);
        }
        if new_epoch != active_epoch {
            let successor = active_epoch
                .0
                .checked_add(1)
                .map(KeyEpoch)
                .ok_or(EpochActivationError::EpochExhausted)?;
            if new_epoch != successor {
                return Err(EpochActivationError::UnexpectedEpoch);
            }
        }
        self.replay_preparation(definition, new_epoch)?;
        Ok(PreparationOutcome::Idempotent)
    }

    /// Advance the active epoch exactly once after replaying and verifying the
    /// selected public bundle.
    pub fn activate_prepared_epoch(
        &self,
        definition: &ChannelDefinition,
        new_epoch: KeyEpoch,
    ) -> Result<ActivationOutcome, EpochActivationError> {
        self.require_definition(definition, false)?;
        let prepared = self
            .custody
            .load_preparation(self.channel_id, new_epoch)?
            .ok_or(EpochActivationError::PreparationMissing)?;
        for _ in 0..MAX_CAS_ATTEMPTS {
            self.require_definition(definition, false)?;
            let record = self
                .state_record()?
                .ok_or(EpochActivationError::NotInitialized)?;
            let state = self.decode_v2_state_record(&record)?;
            if state.active_epoch() == new_epoch {
                self.validate_and_replay(definition, &prepared)?;
                if self
                    .custody
                    .resolve_handle(self.channel_id, new_epoch)?
                    .is_none()
                {
                    return Err(EpochActivationError::ActiveKeyMissing);
                }
                return Ok(ActivationOutcome::Idempotent);
            }
            if state.active_epoch() > new_epoch {
                return Err(EpochActivationError::DecreasingEpoch);
            }
            let expected = state
                .active_epoch()
                .0
                .checked_add(1)
                .map(KeyEpoch)
                .ok_or(EpochActivationError::EpochExhausted)?;
            if expected != new_epoch
                || prepared.base_epoch() != state.active_epoch()
                || prepared.new_epoch() != new_epoch
            {
                return Err(EpochActivationError::UnexpectedEpoch);
            }
            self.validate_and_replay(definition, &prepared)?;
            if self
                .custody
                .resolve_handle(self.channel_id, new_epoch)?
                .is_none()
            {
                return Err(EpochActivationError::ActiveKeyMissing);
            }
            if state.pending_header().is_some() {
                return Err(EpochActivationError::PendingAppend);
            }
            let updated = state.with_active_epoch(self.channel_id, new_epoch)?;
            let input = public_put(
                sequence_state_record_key(self.channel_id),
                EPOCH_STATE_CONTENT_TYPE,
                epoch_state_serialize(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(stored) => {
                    let committed = self.decode_v2_state_record(&stored)?;
                    if committed != updated {
                        return Err(EpochActivationError::CorruptRecord);
                    }
                    return Ok(ActivationOutcome::Activated);
                }
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(EpochActivationError::ConcurrentUpdate)
    }

    /// Reserve one publish using only the epoch currently authoritative in
    /// D18S version 2. No encryption occurs before the reservation CAS.
    pub fn reserve_publish_using_active_epoch(
        &self,
        definition: &ChannelDefinition,
        request: ActiveEpochAppendRequest,
        plaintext: &[u8],
    ) -> Result<EpochReservation, EpochActivationError> {
        self.require_definition(definition, false)?;
        if request.originator_id != definition.originator().agent_id.as_bytes() {
            return Err(EpochActivationError::InvalidPlan);
        }
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(EpochActivationError::NotInitialized)?;
            let state = self.decode_v2_state_record(&record)?;
            let key_handle = self
                .custody
                .resolve_handle(self.channel_id, state.active_epoch())?
                .ok_or(EpochActivationError::ActiveKeyMissing)?;
            if state.pending_header().is_some() {
                return Err(EpochActivationError::PendingAppend);
            }
            let next_sequence = state
                .next_sequence()
                .0
                .checked_add(1)
                .map(Sequence)
                .ok_or(ChannelCryptoError::SequenceExhausted)?;
            let header = prepare_message_header(
                MessageFields::new(
                    request.message_id,
                    request.timestamp_ns,
                    request.originator_id.clone(),
                    self.channel_id,
                    state.next_sequence(),
                    state.active_epoch(),
                    request.content_type.clone(),
                ),
                plaintext,
            );
            let updated =
                state.with_pending(self.channel_id, next_sequence, Some(header.clone()))?;
            let input = public_put(
                sequence_state_record_key(self.channel_id),
                EPOCH_STATE_CONTENT_TYPE,
                epoch_state_serialize(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(EpochReservation { header, key_handle }),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(EpochActivationError::ConcurrentUpdate)
    }

    /// Encrypt, idempotently persist, and clear one exact D18T reservation.
    pub fn commit_reserved(
        &self,
        definition: &ChannelDefinition,
        reservation: &EpochReservation,
        plaintext: &[u8],
        signing_key: &OriginatorSigningKey,
    ) -> Result<EncryptedMessage, EpochActivationError> {
        self.require_definition(definition, false)?;
        let header = reservation.header();
        if header.fields().channel_id() != self.channel_id
            || header.fields().key_epoch() != reservation.key_handle().epoch()
            || reservation.key_handle().channel_id() != self.channel_id
        {
            return Err(EpochActivationError::InvalidPlan);
        }
        let state = self.state()?;
        match state.pending_header() {
            Some(pending) if pending == header => {}
            Some(_) => return Err(EpochActivationError::InvalidPlan),
            None => {
                let stored = self
                    .backend
                    .get(
                        CHANNEL_STORAGE_NAMESPACE,
                        &message_record_key(self.channel_id, header.fields().sequence()),
                    )?
                    .ok_or(EpochActivationError::CorruptRecord)?;
                self.require_record_envelope(
                    &stored,
                    &message_record_key(self.channel_id, header.fields().sequence()),
                    MESSAGE_CONTENT_TYPE,
                )?;
                let message = decode_message(&stored.body)
                    .map_err(|_| EpochActivationError::CorruptRecord)?;
                if message.header() != header {
                    return Err(EpochActivationError::CorruptRecord);
                }
                let expected = self.encrypt_with_handle(
                    reservation.key_handle(),
                    header.clone(),
                    plaintext,
                    signing_key,
                )?;
                if encode_message(&expected).map_err(|_| EpochActivationError::CorruptRecord)?
                    != stored.body
                {
                    return Err(EpochActivationError::CorruptRecord);
                }
                return Ok(message);
            }
        }

        let message = self.encrypt_with_handle(
            reservation.key_handle(),
            header.clone(),
            plaintext,
            signing_key,
        )?;
        self.put_immutable(
            message_record_key(self.channel_id, header.fields().sequence()),
            MESSAGE_CONTENT_TYPE,
            encode_message(&message).map_err(|_| EpochActivationError::CorruptRecord)?,
            EpochActivationError::CorruptRecord,
        )?;
        self.clear_pending(header)?;
        Ok(message)
    }

    /// Clear a pending reservation without decrementing its consumed sequence.
    pub fn abandon_pending(&self) -> Result<Option<MessageHeader>, EpochActivationError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(EpochActivationError::NotInitialized)?;
            let state = self.decode_v2_state_record(&record)?;
            let Some(header) = state.pending_header().cloned() else {
                return Ok(None);
            };
            let updated = state.with_pending(self.channel_id, state.next_sequence(), None)?;
            let input = public_put(
                sequence_state_record_key(self.channel_id),
                EPOCH_STATE_CONTENT_TYPE,
                epoch_state_serialize(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(Some(header)),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(EpochActivationError::ConcurrentUpdate)
    }

    /// Load one immutable public activation plan, if present.
    pub fn activation_plan(
        &self,
        new_epoch: KeyEpoch,
    ) -> Result<Option<ActivationPlan>, EpochActivationError> {
        let key = activation_plan_record_key(self.channel_id, new_epoch);
        let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
            return Ok(None);
        };
        self.require_record_envelope(&record, &key, ACTIVATION_PLAN_CONTENT_TYPE)?;
        let plan = activation_plan_deserialize(&record.body)?;
        if plan.channel_id() != self.channel_id || plan.new_epoch() != new_epoch {
            return Err(EpochActivationError::CorruptRecord);
        }
        Ok(Some(plan))
    }

    /// Erase locally retained originator secrets after the durable definition
    /// is destroyed. Public definitions, plans, grants, and messages remain.
    pub fn apply_destruction(
        &self,
        definition: &ChannelDefinition,
    ) -> Result<(), EpochActivationError> {
        self.require_definition(definition, true)?;
        self.custody.destroy_channel(self.channel_id)?;
        Ok(())
    }

    fn ensure_initial_key(
        &self,
        epoch: KeyEpoch,
        current_cmk: Option<&ChannelMasterKey>,
    ) -> Result<(), EpochActivationError> {
        if self
            .custody
            .resolve_handle(self.channel_id, epoch)?
            .is_some()
        {
            return Ok(());
        }
        let cmk = current_cmk.ok_or(EpochActivationError::ActiveKeyMissing)?;
        match self
            .custody
            .import_active_if_absent(self.channel_id, epoch, cmk)?
        {
            CustodySelection::Selected | CustodySelection::Idempotent => Ok(()),
            CustodySelection::Conflict => Err(EpochActivationError::ConflictingActiveKey),
        }
    }

    fn replay_preparation(
        &self,
        definition: &ChannelDefinition,
        new_epoch: KeyEpoch,
    ) -> Result<(), EpochActivationError> {
        let prepared = self
            .custody
            .load_preparation(self.channel_id, new_epoch)?
            .ok_or(EpochActivationError::PreparationMissing)?;
        self.validate_and_replay(definition, &prepared)
    }

    fn validate_and_replay(
        &self,
        definition: &ChannelDefinition,
        prepared: &PublicPreparation,
    ) -> Result<(), EpochActivationError> {
        let plan = validate_public_preparation(definition, prepared)?;
        self.put_immutable(
            activation_plan_record_key(self.channel_id, plan.new_epoch()),
            ACTIVATION_PLAN_CONTENT_TYPE,
            prepared.plan_bytes().to_vec(),
            EpochActivationError::ConflictingPlan,
        )?;
        for bytes in prepared.grants() {
            let grant = grant_deserialize(bytes)?;
            self.put_immutable(
                key_grant_record_key(self.channel_id, grant.key_epoch(), grant.receiver_id()),
                GRANT_CONTENT_TYPE,
                bytes.clone(),
                EpochActivationError::ConflictingGrant,
            )?;
        }
        let stored = self
            .activation_plan(plan.new_epoch())?
            .ok_or(EpochActivationError::CorruptRecord)?;
        if stored != plan {
            return Err(EpochActivationError::CorruptRecord);
        }
        for bytes in prepared.grants() {
            let grant = grant_deserialize(bytes)?;
            let key = key_grant_record_key(self.channel_id, grant.key_epoch(), grant.receiver_id());
            let record = self
                .backend
                .get(CHANNEL_STORAGE_NAMESPACE, &key)?
                .ok_or(EpochActivationError::CorruptRecord)?;
            self.require_record_envelope(&record, &key, GRANT_CONTENT_TYPE)?;
            if record.body != *bytes {
                return Err(EpochActivationError::CorruptRecord);
            }
        }
        Ok(())
    }

    fn encrypt_with_handle(
        &self,
        handle: &EpochKeyHandle,
        header: MessageHeader,
        plaintext: &[u8],
        signing_key: &OriginatorSigningKey,
    ) -> Result<EncryptedMessage, EpochActivationError> {
        self.custody
            .with_key(handle, |cmk| {
                encrypt_message_with_header(header, plaintext, cmk, signing_key)
            })?
            .map_err(EpochActivationError::Crypto)
    }

    fn clear_pending(&self, expected: &MessageHeader) -> Result<(), EpochActivationError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(EpochActivationError::NotInitialized)?;
            let state = self.decode_v2_state_record(&record)?;
            match state.pending_header() {
                None => return Ok(()),
                Some(pending) if pending == expected => {}
                Some(_) => return Err(EpochActivationError::InvalidPlan),
            }
            let updated = state.with_pending(self.channel_id, state.next_sequence(), None)?;
            let input = public_put(
                sequence_state_record_key(self.channel_id),
                EPOCH_STATE_CONTENT_TYPE,
                epoch_state_serialize(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(()),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(EpochActivationError::ConcurrentUpdate)
    }

    fn require_definition(
        &self,
        expected: &ChannelDefinition,
        require_destroyed: bool,
    ) -> Result<(), EpochActivationError> {
        if expected.channel_id() != self.channel_id {
            return Err(EpochActivationError::InvalidPlan);
        }
        let actual = ChannelDefinitionStore::new(self.backend)
            .load(self.channel_id)
            .map_err(map_definition_error)?
            .ok_or(EpochActivationError::NotInitialized)?;
        if actual != *expected {
            return Err(EpochActivationError::InvalidPlan);
        }
        match (actual.lifecycle(), require_destroyed) {
            (ChannelLifecycle::Destroyed, false) => Err(EpochActivationError::ChannelDestroyed),
            (ChannelLifecycle::Active, true) => Err(EpochActivationError::InvalidPlan),
            _ => Ok(()),
        }
    }

    fn state_record(&self) -> Result<Option<StorageRecord>, EpochActivationError> {
        Ok(self.backend.get(
            CHANNEL_STORAGE_NAMESPACE,
            &sequence_state_record_key(self.channel_id),
        )?)
    }

    fn decode_v2_state_record(
        &self,
        record: &StorageRecord,
    ) -> Result<EpochState, EpochActivationError> {
        self.require_record_envelope(
            record,
            &sequence_state_record_key(self.channel_id),
            EPOCH_STATE_CONTENT_TYPE,
        )?;
        Ok(epoch_state_deserialize(&record.body, self.channel_id)?)
    }

    fn require_record_envelope(
        &self,
        record: &StorageRecord,
        expected_key: &str,
        expected_content_type: &str,
    ) -> Result<(), EpochActivationError> {
        if record.namespace != CHANNEL_STORAGE_NAMESPACE
            || record.key != expected_key
            || record.content_type != expected_content_type
            || record.metadata != empty_metadata()
        {
            return Err(EpochActivationError::CorruptRecord);
        }
        Ok(())
    }

    fn put_immutable(
        &self,
        key: String,
        content_type: &'static str,
        body: Vec<u8>,
        conflict: EpochActivationError,
    ) -> Result<(), EpochActivationError> {
        let input = public_put(key.clone(), content_type, body.clone())?.with_if_absent();
        match self.backend.put(input) {
            Ok(record) => {
                self.require_record_envelope(&record, &key, content_type)?;
                if record.body != body {
                    return Err(EpochActivationError::CorruptRecord);
                }
                Ok(())
            }
            Err(StorageError::Conflict { .. }) => {
                let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
                    return Err(EpochActivationError::CorruptRecord);
                };
                self.require_record_envelope(&record, &key, content_type)?;
                if record.body == body {
                    Ok(())
                } else {
                    Err(conflict)
                }
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// Validate and construct one complete D18Q candidate before custody selection.
///
/// This pure helper is public so deterministic conformance generators can
/// model crashes between custody selection and each public-record replay step.
pub fn prepare_rotation_candidate(
    definition: &ChannelDefinition,
    base_epoch: KeyEpoch,
    target_roster: &[ReceiverIdentity],
    rotation: RotationPlan,
) -> Result<PreparedEpoch, EpochActivationError> {
    if target_roster.is_empty()
        || target_roster.len() > MAX_PLAN_RECEIVERS
        || target_roster.len() != rotation.grants().len()
    {
        return Err(EpochActivationError::InvalidPlan);
    }
    let mut roster = target_roster.to_vec();
    roster.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
    if roster
        .windows(2)
        .any(|pair| pair[0].agent_id == pair[1].agent_id)
    {
        return Err(EpochActivationError::InvalidPlan);
    }
    for (receiver, grant) in roster.iter().zip(rotation.grants()) {
        if receiver.agent_id.as_bytes() != grant.receiver_id() {
            return Err(EpochActivationError::InvalidPlan);
        }
        verify_grant_signature(
            grant,
            definition.originator().agent_id.as_bytes(),
            receiver.agent_id.as_bytes(),
            definition.channel_id(),
            &definition.originator().public_key,
        )?;
        if grant.key_epoch() != rotation.new_epoch() {
            return Err(EpochActivationError::InvalidPlan);
        }
    }
    let (new_epoch, cmk, grants) = rotation.into_parts();
    if base_epoch.0.checked_add(1).map(KeyEpoch) != Some(new_epoch) {
        return Err(EpochActivationError::UnexpectedEpoch);
    }
    let mut grant_bytes = Vec::with_capacity(grants.len());
    let mut entries = Vec::with_capacity(grants.len());
    for grant in &grants {
        let encoded = grant_serialize(grant)?;
        entries.push(ActivationPlanEntry::new(
            sha256(grant.receiver_id()),
            sha256(&encoded),
        ));
        grant_bytes.push(encoded);
    }
    let plan = ActivationPlan::new(definition.channel_id(), base_epoch, new_epoch, entries)?;
    let public = PublicPreparation::new(
        definition.channel_id(),
        base_epoch,
        new_epoch,
        activation_plan_serialize(&plan),
        grant_bytes,
    );
    Ok(PreparedEpoch::new(public, cmk))
}

fn validate_public_preparation(
    definition: &ChannelDefinition,
    prepared: &PublicPreparation,
) -> Result<ActivationPlan, EpochActivationError> {
    if prepared.channel_id() != definition.channel_id()
        || prepared.new_epoch().0
            != prepared
                .base_epoch()
                .0
                .checked_add(1)
                .ok_or(EpochActivationError::EpochExhausted)?
        || prepared.grants().is_empty()
        || prepared.grants().len() > MAX_PLAN_RECEIVERS
    {
        return Err(EpochActivationError::InvalidPlan);
    }
    let plan = activation_plan_deserialize(prepared.plan_bytes())?;
    if plan.channel_id() != prepared.channel_id()
        || plan.base_epoch() != prepared.base_epoch()
        || plan.new_epoch() != prepared.new_epoch()
        || plan.receivers().len() != prepared.grants().len()
    {
        return Err(EpochActivationError::InvalidPlan);
    }
    let mut prior_receiver: Option<Vec<u8>> = None;
    let mut entries = Vec::with_capacity(prepared.grants().len());
    for bytes in prepared.grants() {
        let grant = grant_deserialize(bytes)?;
        if grant.channel_id() != prepared.channel_id()
            || grant.key_epoch() != prepared.new_epoch()
            || prior_receiver
                .as_deref()
                .is_some_and(|prior| prior >= grant.receiver_id())
        {
            return Err(EpochActivationError::InvalidPlan);
        }
        verify_grant_signature(
            &grant,
            definition.originator().agent_id.as_bytes(),
            grant.receiver_id(),
            definition.channel_id(),
            &definition.originator().public_key,
        )?;
        prior_receiver = Some(grant.receiver_id().to_vec());
        entries.push(ActivationPlanEntry::new(
            sha256(grant.receiver_id()),
            sha256(bytes),
        ));
    }
    let expected = ActivationPlan::new(
        prepared.channel_id(),
        prepared.base_epoch(),
        prepared.new_epoch(),
        entries,
    )?;
    if plan != expected {
        return Err(EpochActivationError::InvalidPlan);
    }
    Ok(plan)
}

fn public_put(
    key: String,
    content_type: &'static str,
    body: Vec<u8>,
) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(
        CHANNEL_STORAGE_NAMESPACE,
        key,
        content_type,
        empty_metadata(),
        body,
    )
}

fn map_definition_error(error: ChannelEndpointError) -> EpochActivationError {
    match error {
        ChannelEndpointError::Storage(error) => EpochActivationError::Storage(error),
        ChannelEndpointError::ChannelDestroyed => EpochActivationError::ChannelDestroyed,
        ChannelEndpointError::DefinitionNotFound => EpochActivationError::NotInitialized,
        ChannelEndpointError::CorruptDefinition(_) | ChannelEndpointError::Store(_) => {
            EpochActivationError::CorruptRecord
        }
        _ => EpochActivationError::InvalidPlan,
    }
}

fn empty_metadata() -> JsonValue {
    JsonValue::Object(vec![])
}
