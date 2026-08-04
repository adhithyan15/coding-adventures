//! Crash-safe persistence for encrypted Chief of Staff channels.
//!
//! A [`ChannelStore`] reserves and persists the exact authenticated message
//! header before encryption. This makes the channel sequence durable before it
//! can be used as part of an XChaCha20 nonce. Completed ciphertext records,
//! sealed receiver grants, and receiver cursors are all written idempotently.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::wire::{
    decode_key_grant, decode_message, decode_message_header, encode_key_grant, encode_message,
    encode_message_header, key_grant_record_key, message_record_key, message_record_prefix,
    receiver_ack_record_key, sequence_state_record_key, ChannelWireError,
    CHANNEL_STORAGE_NAMESPACE, MAX_IDENTITY_BYTES,
};
use chief_of_staff_channel_crypto::{
    encrypt_message_with_header, prepare_message_header, ChannelCryptoError, ChannelId,
    ChannelMasterKey, EncryptedMessage, KeyEpoch, MessageFields, MessageHeader,
    OriginatorSigningKey, SealedChannelKeyGrant, Sequence,
};
use coding_adventures_json_value::JsonValue;
use storage_core::{
    StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
};

const STATE_MAGIC: &[u8; 4] = b"D18S";
const ACK_MAGIC: &[u8; 4] = b"D18A";
const STORE_WIRE_VERSION: u8 = 1;
const MAX_STATE_HEADER_BYTES: usize = 16 * 1024;
const MAX_CAS_ATTEMPTS: usize = 16;

const STATE_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-state-v1";
const MESSAGE_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-message-v1";
const GRANT_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-key-grant-v1";
const ACK_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-ack-v1";

/// Immutable caller-supplied fields for a new channel append.
///
/// The store supplies `channel_id` and the never-resetting durable sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendRequest {
    /// Canonical 16-byte UUID v7 representation for the message.
    pub message_id: [u8; 16],
    /// Monotonic nanosecond timestamp authenticated with the message.
    pub timestamp_ns: u64,
    /// Entity that authored and signs this message.
    pub originator_id: Vec<u8>,
    /// CMK epoch used for this payload.
    pub key_epoch: KeyEpoch,
    /// MIME content type authenticated with the message.
    pub content_type: String,
}

/// Durable sequence state visible to recovery code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelState {
    /// First sequence that has not yet been reserved.
    pub next_sequence: Sequence,
    /// Exact pre-encryption header awaiting completion, if any.
    pub pending_header: Option<MessageHeader>,
}

/// One ordered page of encrypted channel records.
#[derive(Clone, PartialEq, Eq)]
pub struct MessagePage {
    /// Messages in ascending global channel-sequence order.
    pub messages: Vec<EncryptedMessage>,
    /// Sequence from which the caller should request the next page.
    pub next_start: Option<Sequence>,
}

/// Errors returned by durable channel-store operations.
#[derive(Debug)]
pub enum ChannelStoreError {
    /// The injected storage backend rejected an operation.
    Storage(StorageError),
    /// A persisted channel record failed structural decoding.
    Wire(ChannelWireError),
    /// Channel cryptography rejected an operation.
    Crypto(ChannelCryptoError),
    /// [`ChannelStore::initialize`] has not created durable channel state.
    NotInitialized,
    /// A record was structurally valid but violated a store invariant.
    CorruptRecord(&'static str),
    /// A previous append must be completed or abandoned first.
    PendingAppend(Box<MessageHeader>),
    /// No matching durable reservation exists for this completion attempt.
    NoPendingAppend,
    /// The completion header differs from the durable pending reservation.
    PendingHeaderMismatch,
    /// A create-if-absent record already contains different bytes.
    ConflictingRecord(&'static str),
    /// Repeated compare-and-swap races prevented progress.
    ConcurrentUpdate,
    /// A receiver identifier was empty or exceeded the channel codec bound.
    InvalidReceiverId,
    /// Ordered reads require a nonzero page size.
    InvalidPageSize,
    /// An acknowledgement attempted to move a receiver cursor backwards.
    AcknowledgementRegression {
        /// Current first unread sequence.
        current: Sequence,
        /// Requested first unread sequence.
        attempted: Sequence,
    },
    /// An acknowledgement named a sequence that has not been reserved.
    AcknowledgementAhead {
        /// Durable first unreserved channel sequence.
        next_sequence: Sequence,
        /// Sequence the caller attempted to acknowledge.
        attempted: Sequence,
    },
    /// An acknowledgement would skip a reservation that may still commit.
    AcknowledgementPending {
        /// Sequence whose append has not finished or been abandoned.
        pending: Sequence,
        /// Sequence the caller attempted to acknowledge.
        attempted: Sequence,
    },
}

impl core::fmt::Display for ChannelStoreError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Wire(error) => write!(formatter, "{error}"),
            Self::Crypto(error) => write!(formatter, "{error}"),
            Self::NotInitialized => formatter.write_str("durable channel state is not initialized"),
            Self::CorruptRecord(message) => write!(formatter, "corrupt channel record: {message}"),
            Self::PendingAppend(_) => formatter.write_str("a channel append is already pending"),
            Self::NoPendingAppend => formatter.write_str("no matching channel append is pending"),
            Self::PendingHeaderMismatch => {
                formatter.write_str("completion header differs from pending channel append")
            }
            Self::ConflictingRecord(kind) => {
                write!(formatter, "conflicting durable {kind} record")
            }
            Self::ConcurrentUpdate => {
                formatter.write_str("channel state changed too many times concurrently")
            }
            Self::InvalidReceiverId => formatter.write_str("receiver identifier is invalid"),
            Self::InvalidPageSize => formatter.write_str("message page size must be nonzero"),
            Self::AcknowledgementRegression { current, attempted } => write!(
                formatter,
                "receiver cursor would regress from {} to {}",
                current.0, attempted.0
            ),
            Self::AcknowledgementAhead {
                next_sequence,
                attempted,
            } => write!(
                formatter,
                "cannot acknowledge sequence {} when next unreserved sequence is {}",
                attempted.0, next_sequence.0
            ),
            Self::AcknowledgementPending { pending, attempted } => write!(
                formatter,
                "cannot acknowledge sequence {} while sequence {} is pending",
                attempted.0, pending.0
            ),
        }
    }
}

impl std::error::Error for ChannelStoreError {}

impl From<StorageError> for ChannelStoreError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<ChannelWireError> for ChannelStoreError {
    fn from(error: ChannelWireError) -> Self {
        Self::Wire(error)
    }
}

impl From<ChannelCryptoError> for ChannelStoreError {
    fn from(error: ChannelCryptoError) -> Self {
        Self::Crypto(error)
    }
}

/// CAS-backed durable view of one encrypted channel.
pub struct ChannelStore<'a> {
    backend: &'a dyn StorageBackend,
    channel_id: ChannelId,
}

impl<'a> ChannelStore<'a> {
    /// Bind one channel to an injected repository-owned storage backend.
    pub fn new(backend: &'a dyn StorageBackend, channel_id: ChannelId) -> Self {
        Self {
            backend,
            channel_id,
        }
    }

    /// Initialize the backend and atomically create sequence state at zero.
    ///
    /// Reopening an existing channel is idempotent and preserves its state.
    pub fn initialize(&self) -> Result<ChannelState, ChannelStoreError> {
        self.backend.initialize()?;
        if let Some(record) = self.state_record()? {
            return decode_state_record(&record, self.channel_id);
        }

        let initial = ChannelState {
            next_sequence: Sequence(0),
            pending_header: None,
        };
        let input = put_input(
            sequence_state_record_key(self.channel_id),
            STATE_CONTENT_TYPE,
            encode_state(&initial)?,
        )?
        .with_if_absent();
        match self.backend.put(input) {
            Ok(record) => decode_state_record(&record, self.channel_id),
            Err(StorageError::Conflict { .. }) => self.state(),
            Err(error) => Err(error.into()),
        }
    }

    /// Load the current durable sequence and pending-append state.
    pub fn state(&self) -> Result<ChannelState, ChannelStoreError> {
        let record = self
            .state_record()?
            .ok_or(ChannelStoreError::NotInitialized)?;
        decode_state_record(&record, self.channel_id)
    }

    /// Atomically reserve a sequence and persist its exact authenticated header.
    ///
    /// No encryption occurs until this method returns successfully.
    pub fn reserve_append(
        &self,
        request: AppendRequest,
        plaintext: &[u8],
    ) -> Result<MessageHeader, ChannelStoreError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(ChannelStoreError::NotInitialized)?;
            let state = decode_state_record(&record, self.channel_id)?;
            if let Some(header) = state.pending_header {
                return Err(ChannelStoreError::PendingAppend(Box::new(header)));
            }
            let next = state
                .next_sequence
                .0
                .checked_add(1)
                .map(Sequence)
                .ok_or(ChannelCryptoError::SequenceExhausted)?;
            let header = prepare_message_header(
                MessageFields {
                    message_id: request.message_id,
                    timestamp_ns: request.timestamp_ns,
                    originator_id: request.originator_id.clone(),
                    channel_id: self.channel_id,
                    sequence: state.next_sequence,
                    key_epoch: request.key_epoch,
                    content_type: request.content_type.clone(),
                },
                plaintext,
            );
            let updated = ChannelState {
                next_sequence: next,
                pending_header: Some(header.clone()),
            };
            let input = put_input(
                sequence_state_record_key(self.channel_id),
                STATE_CONTENT_TYPE,
                encode_state(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(header),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ChannelStoreError::ConcurrentUpdate)
    }

    /// Encrypt and idempotently persist one previously reserved append.
    ///
    /// A retry after the message write but before pending-state cleanup returns
    /// the same deterministic ciphertext record and completes cleanup.
    pub fn commit_reserved(
        &self,
        header: &MessageHeader,
        plaintext: &[u8],
        cmk: &ChannelMasterKey,
        signing_key: &OriginatorSigningKey,
    ) -> Result<EncryptedMessage, ChannelStoreError> {
        if header.fields.channel_id != self.channel_id {
            return Err(ChannelStoreError::PendingHeaderMismatch);
        }
        let state = self.state()?;
        match state.pending_header {
            Some(ref pending) if pending == header => {}
            Some(_) => return Err(ChannelStoreError::PendingHeaderMismatch),
            None => {
                let key = message_record_key(self.channel_id, header.fields.sequence);
                let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
                    return Err(ChannelStoreError::NoPendingAppend);
                };
                require_content_type(&record, MESSAGE_CONTENT_TYPE)?;
                let stored = decode_message(&record.body)?;
                if stored.header != *header {
                    return Err(ChannelStoreError::ConflictingRecord("message"));
                }
                let expected =
                    encrypt_message_with_header(header.clone(), plaintext, cmk, signing_key)?;
                if encode_message(&expected)? != record.body {
                    return Err(ChannelStoreError::ConflictingRecord("message"));
                }
                return Ok(stored);
            }
        }

        let message = encrypt_message_with_header(header.clone(), plaintext, cmk, signing_key)?;
        let encoded = encode_message(&message)?;
        self.put_idempotent(
            message_record_key(self.channel_id, header.fields.sequence),
            MESSAGE_CONTENT_TYPE,
            encoded,
            "message",
        )?;
        self.clear_pending(header)?;
        Ok(message)
    }

    /// Reserve, encrypt, persist, and finalize one append.
    pub fn append(
        &self,
        request: AppendRequest,
        plaintext: &[u8],
        cmk: &ChannelMasterKey,
        signing_key: &OriginatorSigningKey,
    ) -> Result<EncryptedMessage, ChannelStoreError> {
        let header = self.reserve_append(request, plaintext)?;
        self.commit_reserved(&header, plaintext, cmk, signing_key)
    }

    /// Clear a pending append while permanently consuming its sequence.
    ///
    /// Returns the abandoned header, or `None` when no append was pending.
    pub fn abandon_pending(&self) -> Result<Option<MessageHeader>, ChannelStoreError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(ChannelStoreError::NotInitialized)?;
            let state = decode_state_record(&record, self.channel_id)?;
            let Some(header) = state.pending_header else {
                return Ok(None);
            };
            let updated = ChannelState {
                next_sequence: state.next_sequence,
                pending_header: None,
            };
            let input = put_input(
                sequence_state_record_key(self.channel_id),
                STATE_CONTENT_TYPE,
                encode_state(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(Some(header)),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ChannelStoreError::ConcurrentUpdate)
    }

    /// Read encrypted messages in ascending sequence order.
    ///
    /// Abandoned sequence gaps are omitted. `next_start` is populated only
    /// when the backend reports another ordered page.
    pub fn read_messages(
        &self,
        start: Sequence,
        page_size: usize,
    ) -> Result<MessagePage, ChannelStoreError> {
        if page_size == 0 {
            return Err(ChannelStoreError::InvalidPageSize);
        }
        let prefix = message_record_prefix(self.channel_id);
        let cursor = start
            .0
            .checked_sub(1)
            .map(|previous| message_record_key(self.channel_id, Sequence(previous)));
        let page = self.backend.list(
            CHANNEL_STORAGE_NAMESPACE,
            StorageListOptions {
                prefix: Some(prefix),
                recursive: true,
                page_size: Some(page_size),
                cursor,
            },
        )?;
        let has_more = page.next_cursor.is_some();
        let mut messages = Vec::with_capacity(page.records.len());
        for record in page.records {
            require_content_type(&record, MESSAGE_CONTENT_TYPE)?;
            let message = decode_message(&record.body)?;
            if message.header.fields.channel_id != self.channel_id
                || message.header.fields.sequence < start
                || record.key != message_record_key(self.channel_id, message.header.fields.sequence)
            {
                return Err(ChannelStoreError::CorruptRecord(
                    "message body does not match storage key",
                ));
            }
            if messages.last().is_some_and(|previous: &EncryptedMessage| {
                previous.header.fields.sequence >= message.header.fields.sequence
            }) {
                return Err(ChannelStoreError::CorruptRecord(
                    "message records are not strictly ordered",
                ));
            }
            messages.push(message);
        }
        let next_start = if has_more {
            let last = messages.last().ok_or(ChannelStoreError::CorruptRecord(
                "backend returned an empty page with a continuation",
            ))?;
            Some(Sequence(
                last.header.fields.sequence.0.checked_add(1).ok_or(
                    ChannelStoreError::CorruptRecord("message continuation exceeds sequence range"),
                )?,
            ))
        } else {
            None
        };
        Ok(MessagePage {
            messages,
            next_start,
        })
    }

    /// Read one ordered page beginning at a receiver's first unread sequence.
    pub fn read_for_receiver(
        &self,
        receiver_id: &[u8],
        page_size: usize,
    ) -> Result<MessagePage, ChannelStoreError> {
        let cursor = self.receiver_cursor(receiver_id)?;
        self.read_messages(cursor, page_size)
    }

    /// Return the receiver's first unread sequence, defaulting to zero.
    pub fn receiver_cursor(&self, receiver_id: &[u8]) -> Result<Sequence, ChannelStoreError> {
        validate_receiver_id(receiver_id)?;
        let key = receiver_ack_record_key(self.channel_id, receiver_id);
        let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
            return Ok(Sequence(0));
        };
        require_content_type(&record, ACK_CONTENT_TYPE)?;
        decode_cursor(&record.body)
    }

    /// Monotonically acknowledge every sequence through `acknowledged`.
    ///
    /// The persisted cursor is the following sequence, which becomes the first
    /// unread sequence on the receiver's next read.
    pub fn acknowledge(
        &self,
        receiver_id: &[u8],
        acknowledged: Sequence,
    ) -> Result<Sequence, ChannelStoreError> {
        validate_receiver_id(receiver_id)?;
        let state = self.state()?;
        if acknowledged >= state.next_sequence {
            return Err(ChannelStoreError::AcknowledgementAhead {
                next_sequence: state.next_sequence,
                attempted: acknowledged,
            });
        }
        if let Some(pending) = state.pending_header {
            if acknowledged >= pending.fields.sequence {
                return Err(ChannelStoreError::AcknowledgementPending {
                    pending: pending.fields.sequence,
                    attempted: acknowledged,
                });
            }
        }
        let desired = acknowledged
            .0
            .checked_add(1)
            .map(Sequence)
            .ok_or(ChannelCryptoError::SequenceExhausted)?;
        let key = receiver_ack_record_key(self.channel_id, receiver_id);
        for _ in 0..MAX_CAS_ATTEMPTS {
            match self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? {
                None => {
                    let input = put_input(key.clone(), ACK_CONTENT_TYPE, encode_cursor(desired))?
                        .with_if_absent();
                    match self.backend.put(input) {
                        Ok(_) => return Ok(desired),
                        Err(StorageError::Conflict { .. }) => continue,
                        Err(error) => return Err(error.into()),
                    }
                }
                Some(record) => {
                    require_content_type(&record, ACK_CONTENT_TYPE)?;
                    let current = decode_cursor(&record.body)?;
                    if desired < current {
                        return Err(ChannelStoreError::AcknowledgementRegression {
                            current,
                            attempted: desired,
                        });
                    }
                    if desired == current {
                        return Ok(current);
                    }
                    let input = put_input(key.clone(), ACK_CONTENT_TYPE, encode_cursor(desired))?
                        .with_if_revision(Some(record.revision));
                    match self.backend.put(input) {
                        Ok(_) => return Ok(desired),
                        Err(StorageError::Conflict { .. }) => continue,
                        Err(error) => return Err(error.into()),
                    }
                }
            }
        }
        Err(ChannelStoreError::ConcurrentUpdate)
    }

    /// Idempotently persist a receiver-bound sealed key grant.
    pub fn save_key_grant(&self, grant: &SealedChannelKeyGrant) -> Result<(), ChannelStoreError> {
        if grant.channel_id != self.channel_id {
            return Err(ChannelStoreError::CorruptRecord(
                "key grant belongs to another channel",
            ));
        }
        validate_receiver_id(&grant.receiver_id)?;
        self.put_idempotent(
            key_grant_record_key(self.channel_id, grant.key_epoch, &grant.receiver_id),
            GRANT_CONTENT_TYPE,
            encode_key_grant(grant)?,
            "key grant",
        )
    }

    /// Load a sealed key grant for one receiver and epoch.
    pub fn key_grant(
        &self,
        key_epoch: KeyEpoch,
        receiver_id: &[u8],
    ) -> Result<Option<SealedChannelKeyGrant>, ChannelStoreError> {
        validate_receiver_id(receiver_id)?;
        let key = key_grant_record_key(self.channel_id, key_epoch, receiver_id);
        let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
            return Ok(None);
        };
        require_content_type(&record, GRANT_CONTENT_TYPE)?;
        let grant = decode_key_grant(&record.body)?;
        if grant.channel_id != self.channel_id
            || grant.key_epoch != key_epoch
            || grant.receiver_id != receiver_id
        {
            return Err(ChannelStoreError::CorruptRecord(
                "key grant body does not match storage key",
            ));
        }
        Ok(Some(grant))
    }

    fn state_record(&self) -> Result<Option<StorageRecord>, ChannelStoreError> {
        Ok(self.backend.get(
            CHANNEL_STORAGE_NAMESPACE,
            &sequence_state_record_key(self.channel_id),
        )?)
    }

    fn put_idempotent(
        &self,
        key: String,
        content_type: &'static str,
        body: Vec<u8>,
        kind: &'static str,
    ) -> Result<(), ChannelStoreError> {
        let input = put_input(key.clone(), content_type, body.clone())?.with_if_absent();
        match self.backend.put(input) {
            Ok(_) => Ok(()),
            Err(StorageError::Conflict { .. }) => {
                if self.record_matches(&key, content_type, &body)? {
                    Ok(())
                } else {
                    Err(ChannelStoreError::ConflictingRecord(kind))
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    fn record_matches(
        &self,
        key: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<bool, ChannelStoreError> {
        Ok(self
            .backend
            .get(CHANNEL_STORAGE_NAMESPACE, key)?
            .is_some_and(|record| record.content_type == content_type && record.body == body))
    }

    fn clear_pending(&self, expected: &MessageHeader) -> Result<(), ChannelStoreError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let record = self
                .state_record()?
                .ok_or(ChannelStoreError::NotInitialized)?;
            let state = decode_state_record(&record, self.channel_id)?;
            match state.pending_header {
                None => return Ok(()),
                Some(ref pending) if pending == expected => {}
                Some(_) => return Err(ChannelStoreError::PendingHeaderMismatch),
            }
            let updated = ChannelState {
                next_sequence: state.next_sequence,
                pending_header: None,
            };
            let input = put_input(
                sequence_state_record_key(self.channel_id),
                STATE_CONTENT_TYPE,
                encode_state(&updated)?,
            )?
            .with_if_revision(Some(record.revision));
            match self.backend.put(input) {
                Ok(_) => return Ok(()),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ChannelStoreError::ConcurrentUpdate)
    }
}

fn put_input(
    key: String,
    content_type: &'static str,
    body: Vec<u8>,
) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(
        CHANNEL_STORAGE_NAMESPACE,
        key,
        content_type,
        JsonValue::Object(vec![]),
        body,
    )
}

fn validate_receiver_id(receiver_id: &[u8]) -> Result<(), ChannelStoreError> {
    if receiver_id.is_empty() || receiver_id.len() > MAX_IDENTITY_BYTES {
        return Err(ChannelStoreError::InvalidReceiverId);
    }
    Ok(())
}

fn require_content_type(record: &StorageRecord, expected: &str) -> Result<(), ChannelStoreError> {
    if record.content_type != expected {
        return Err(ChannelStoreError::CorruptRecord(
            "record content type does not match its key",
        ));
    }
    Ok(())
}

fn encode_state(state: &ChannelState) -> Result<Vec<u8>, ChannelStoreError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(STATE_MAGIC);
    bytes.push(STORE_WIRE_VERSION);
    bytes.extend_from_slice(&state.next_sequence.0.to_be_bytes());
    match &state.pending_header {
        None => bytes.push(0),
        Some(header) => {
            let encoded_header = encode_message_header(header)?;
            if encoded_header.len() > MAX_STATE_HEADER_BYTES {
                return Err(ChannelStoreError::CorruptRecord(
                    "pending message header exceeds store bound",
                ));
            }
            bytes.push(1);
            bytes.extend_from_slice(&(encoded_header.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&encoded_header);
        }
    }
    Ok(bytes)
}

fn decode_state_record(
    record: &StorageRecord,
    channel_id: ChannelId,
) -> Result<ChannelState, ChannelStoreError> {
    require_content_type(record, STATE_CONTENT_TYPE)?;
    decode_state(&record.body, channel_id)
}

fn decode_state(bytes: &[u8], channel_id: ChannelId) -> Result<ChannelState, ChannelStoreError> {
    if bytes.len() < 14 || &bytes[..4] != STATE_MAGIC {
        return Err(ChannelStoreError::CorruptRecord("invalid state envelope"));
    }
    if bytes[4] != STORE_WIRE_VERSION {
        return Err(ChannelStoreError::CorruptRecord(
            "unsupported state version",
        ));
    }
    let next_sequence =
        Sequence(u64::from_be_bytes(bytes[5..13].try_into().map_err(
            |_| ChannelStoreError::CorruptRecord("truncated state sequence"),
        )?));
    let pending_header = match bytes[13] {
        0 if bytes.len() == 14 => None,
        1 if bytes.len() >= 18 => {
            let length = u32::from_be_bytes(
                bytes[14..18]
                    .try_into()
                    .map_err(|_| ChannelStoreError::CorruptRecord("truncated header length"))?,
            ) as usize;
            if length > MAX_STATE_HEADER_BYTES || bytes.len() != 18 + length {
                return Err(ChannelStoreError::CorruptRecord(
                    "invalid pending header length",
                ));
            }
            Some(decode_message_header(&bytes[18..])?)
        }
        _ => {
            return Err(ChannelStoreError::CorruptRecord(
                "invalid pending state flag",
            ))
        }
    };
    if let Some(header) = &pending_header {
        if header.fields.channel_id != channel_id
            || header.fields.sequence.0.checked_add(1).map(Sequence) != Some(next_sequence)
        {
            return Err(ChannelStoreError::CorruptRecord(
                "pending header violates channel sequence state",
            ));
        }
    }
    Ok(ChannelState {
        next_sequence,
        pending_header,
    })
}

fn encode_cursor(cursor: Sequence) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(13);
    bytes.extend_from_slice(ACK_MAGIC);
    bytes.push(STORE_WIRE_VERSION);
    bytes.extend_from_slice(&cursor.0.to_be_bytes());
    bytes
}

fn decode_cursor(bytes: &[u8]) -> Result<Sequence, ChannelStoreError> {
    if bytes.len() != 13 || &bytes[..4] != ACK_MAGIC {
        return Err(ChannelStoreError::CorruptRecord(
            "invalid receiver cursor envelope",
        ));
    }
    if bytes[4] != STORE_WIRE_VERSION {
        return Err(ChannelStoreError::CorruptRecord(
            "unsupported receiver cursor version",
        ));
    }
    Ok(Sequence(u64::from_be_bytes(
        bytes[5..13]
            .try_into()
            .map_err(|_| ChannelStoreError::CorruptRecord("truncated receiver cursor"))?,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{decrypt_message, seal_channel_key, ReceiverKeyPair};
    use std::path::PathBuf;
    use storage_core::InMemoryStorageBackend;
    use storage_local_folder::LocalFolderStorageBackend;

    fn channel_id() -> ChannelId {
        ChannelId([0x51; 16])
    }

    fn request(message_id_byte: u8) -> AppendRequest {
        AppendRequest {
            message_id: [message_id_byte; 16],
            timestamp_ns: 1_725_000_000_000_000_000 + u64::from(message_id_byte),
            originator_id: b"originator".to_vec(),
            key_epoch: KeyEpoch(3),
            content_type: "text/plain".to_owned(),
        }
    }

    fn keys() -> (ChannelMasterKey, OriginatorSigningKey) {
        (
            ChannelMasterKey::from_bytes([0xa5; 32]),
            OriginatorSigningKey::from_seed([0x37; 32]),
        )
    }

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chief-channel-store-{label}-{}-{}",
            std::process::id(),
            storage_core::now_utc_ms()
        ))
    }

    #[test]
    fn reserves_before_encrypting_and_clears_after_commit() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        assert_eq!(store.initialize().unwrap().next_sequence, Sequence(0));
        let header = store.reserve_append(request(1), b"hello").unwrap();
        assert_eq!(header.fields.sequence, Sequence(0));
        assert_eq!(store.state().unwrap().pending_header, Some(header.clone()));
        assert!(backend
            .get(
                CHANNEL_STORAGE_NAMESPACE,
                &message_record_key(channel_id(), Sequence(0))
            )
            .unwrap()
            .is_none());

        let (cmk, signing_key) = keys();
        let message = store
            .commit_reserved(&header, b"hello", &cmk, &signing_key)
            .unwrap();
        assert_eq!(store.state().unwrap().pending_header, None);
        assert_eq!(
            decrypt_message(&message, &cmk, &signing_key.public_key()).unwrap(),
            b"hello"
        );
    }

    #[test]
    fn restart_recovers_exact_header_and_commit_retry_is_idempotent() {
        let backend = InMemoryStorageBackend::new();
        let header = {
            let first = ChannelStore::new(&backend, channel_id());
            first.initialize().unwrap();
            first.reserve_append(request(2), b"recover me").unwrap()
        };

        let recovered = ChannelStore::new(&backend, channel_id());
        assert_eq!(
            recovered.initialize().unwrap().pending_header,
            Some(header.clone())
        );
        let (cmk, signing_key) = keys();
        assert!(matches!(
            recovered.commit_reserved(&header, b"wrong", &cmk, &signing_key),
            Err(ChannelStoreError::Crypto(
                ChannelCryptoError::PlaintextHashMismatch
            ))
        ));
        let first_commit = recovered
            .commit_reserved(&header, b"recover me", &cmk, &signing_key)
            .unwrap();
        let retry = recovered
            .commit_reserved(&header, b"recover me", &cmk, &signing_key)
            .unwrap();
        assert!(first_commit == retry);
    }

    #[test]
    fn retry_after_ciphertext_write_finishes_pending_cleanup() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        store.initialize().unwrap();
        let header = store.reserve_append(request(9), b"written once").unwrap();
        let (cmk, signing_key) = keys();
        let written =
            encrypt_message_with_header(header.clone(), b"written once", &cmk, &signing_key)
                .unwrap();
        backend
            .put(
                put_input(
                    message_record_key(channel_id(), header.fields.sequence),
                    MESSAGE_CONTENT_TYPE,
                    encode_message(&written).unwrap(),
                )
                .unwrap()
                .with_if_absent(),
            )
            .unwrap();

        let recovered = ChannelStore::new(&backend, channel_id());
        let committed = recovered
            .commit_reserved(&header, b"written once", &cmk, &signing_key)
            .unwrap();
        assert!(committed == written);
        assert_eq!(recovered.state().unwrap().pending_header, None);
    }

    #[test]
    fn local_folder_restart_recovers_and_commits_reservation() {
        let root = temp_root("restart");
        let header = {
            let backend = LocalFolderStorageBackend::new(&root);
            let store = ChannelStore::new(&backend, channel_id());
            store.initialize().unwrap();
            store.reserve_append(request(10), b"disk recovery").unwrap()
        };

        {
            let backend = LocalFolderStorageBackend::new(&root);
            let store = ChannelStore::new(&backend, channel_id());
            assert_eq!(
                store.initialize().unwrap().pending_header,
                Some(header.clone())
            );
            let (cmk, signing_key) = keys();
            store
                .commit_reserved(&header, b"disk recovery", &cmk, &signing_key)
                .unwrap();
            assert_eq!(
                store.read_messages(Sequence(0), 10).unwrap().messages.len(),
                1
            );
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn abandoned_sequences_stay_consumed_and_reads_skip_gaps() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        store.initialize().unwrap();
        let abandoned = store.reserve_append(request(3), b"abandon").unwrap();
        assert_eq!(store.abandon_pending().unwrap(), Some(abandoned));

        let (cmk, signing_key) = keys();
        let committed = store
            .append(request(4), b"kept", &cmk, &signing_key)
            .unwrap();
        assert_eq!(committed.header.fields.sequence, Sequence(1));
        let page = store.read_messages(Sequence(0), 10).unwrap();
        assert!(page.messages == vec![committed]);
        assert_eq!(page.next_start, None);
    }

    #[test]
    fn ordered_pages_and_receiver_cursors_are_monotonic() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        store.initialize().unwrap();
        let (cmk, signing_key) = keys();
        for byte in 0..3 {
            store
                .append(request(byte), &[byte], &cmk, &signing_key)
                .unwrap();
        }
        let first = store.read_for_receiver(b"receiver", 2).unwrap();
        assert_eq!(
            first
                .messages
                .iter()
                .map(|message| message.header.fields.sequence)
                .collect::<Vec<_>>(),
            vec![Sequence(0), Sequence(1)]
        );
        assert_eq!(first.next_start, Some(Sequence(2)));
        assert_eq!(
            store.acknowledge(b"receiver", Sequence(1)).unwrap(),
            Sequence(2)
        );
        assert_eq!(store.receiver_cursor(b"receiver").unwrap(), Sequence(2));
        assert_eq!(
            store.read_for_receiver(b"receiver", 2).unwrap().messages[0]
                .header
                .fields
                .sequence,
            Sequence(2)
        );
        assert!(matches!(
            store.acknowledge(b"receiver", Sequence(0)),
            Err(ChannelStoreError::AcknowledgementRegression { .. })
        ));
        assert!(matches!(
            store.acknowledge(b"receiver", Sequence(3)),
            Err(ChannelStoreError::AcknowledgementAhead { .. })
        ));
    }

    #[test]
    fn receiver_cannot_acknowledge_a_pending_append() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        store.initialize().unwrap();
        store.reserve_append(request(11), b"not committed").unwrap();
        assert!(matches!(
            store.acknowledge(b"receiver", Sequence(0)),
            Err(ChannelStoreError::AcknowledgementPending {
                pending: Sequence(0),
                attempted: Sequence(0)
            })
        ));
    }

    #[test]
    fn sealed_grants_round_trip_idempotently() {
        let backend = InMemoryStorageBackend::new();
        let store = ChannelStore::new(&backend, channel_id());
        store.initialize().unwrap();
        let (cmk, signing_key) = keys();
        let receiver = ReceiverKeyPair::from_private_key([0x92; 32]).unwrap();
        let grant = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(3),
            &cmk,
            &receiver.public_key(),
            &signing_key,
        )
        .unwrap();
        store.save_key_grant(&grant).unwrap();
        store.save_key_grant(&grant).unwrap();
        assert!(store.key_grant(KeyEpoch(3), b"receiver").unwrap() == Some(grant));
    }

    #[test]
    fn corrupt_state_fails_closed() {
        let backend = InMemoryStorageBackend::new();
        backend.initialize().unwrap();
        backend
            .put(
                put_input(
                    sequence_state_record_key(channel_id()),
                    STATE_CONTENT_TYPE,
                    b"not state".to_vec(),
                )
                .unwrap(),
            )
            .unwrap();
        let store = ChannelStore::new(&backend, channel_id());
        assert!(matches!(
            store.initialize(),
            Err(ChannelStoreError::CorruptRecord(_))
        ));
    }
}
