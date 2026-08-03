//! Durable membership and authorized endpoints for one-way Chief channels.
//!
//! This package binds the structural channel crypto/store primitives to the
//! identities the orchestrator authorized. A channel definition has exactly
//! one originator and a disjoint receiver set. Endpoint operations reload that
//! durable definition before every privileged action.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;

use chief_of_staff_channel_crypto::wire::{
    channel_definition_record_key, CHANNEL_STORAGE_NAMESPACE, MAX_IDENTITY_BYTES,
};
use chief_of_staff_channel_crypto::{
    decrypt_message, seal_channel_key, ChannelCryptoError, ChannelId, ChannelMasterKey, KeyEpoch,
    OriginatorSigningKey, ReceiverEpochKeys, ReceiverKeyPair, SealedChannelKeyGrant, Sequence,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore, ChannelStoreError, MessagePage};
use coding_adventures_json_value::JsonValue;
use storage_core::{Revision, StorageBackend, StorageError, StoragePutInput, StorageRecord};

const DEFINITION_MAGIC: &[u8; 4] = b"D18C";
const DEFINITION_VERSION: u8 = 1;
const DEFINITION_CONTENT_TYPE: &str =
    "application/vnd.coding-adventures.chief-channel-definition-v1";
const MAX_RECEIVERS: usize = 1024;
const MAX_CAS_ATTEMPTS: usize = 16;

/// Stable Chief entity identifier used by channel authorization.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(Vec<u8>);

impl AgentId {
    /// Validate and own a non-empty bounded identifier.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ChannelEndpointError> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.len() > MAX_IDENTITY_BYTES {
            return Err(ChannelEndpointError::InvalidDefinition(
                "agent identifier is empty or exceeds the channel bound",
            ));
        }
        Ok(Self(bytes))
    }

    /// Borrow the canonical identifier bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Originator identity and Ed25519 verification key bound to a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginatorIdentity {
    /// Authorized entity identifier.
    pub agent_id: AgentId,
    /// Ed25519 public key for message and key-grant signatures.
    pub public_key: [u8; 32],
}

/// Receiver identity and X25519 key-exchange public key bound to a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiverIdentity {
    /// Authorized entity identifier.
    pub agent_id: AgentId,
    /// X25519 public key used to seal the channel master key.
    pub public_key: [u8; 32],
}

/// One-way lifecycle of a durable channel definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelLifecycle {
    /// Endpoints may publish, receive, acknowledge, and exchange key grants.
    Active,
    /// All endpoint operations are denied; persisted history remains immutable.
    Destroyed,
}

/// Immutable durable membership and key-epoch definition for one channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelDefinition {
    channel_id: ChannelId,
    originator: OriginatorIdentity,
    receivers: Vec<ReceiverIdentity>,
    created_at_ns: u64,
    key_epoch: KeyEpoch,
    lifecycle: ChannelLifecycle,
}

impl ChannelDefinition {
    /// Create an active definition after enforcing one-way membership rules.
    pub fn new(
        channel_id: ChannelId,
        originator: OriginatorIdentity,
        receivers: Vec<ReceiverIdentity>,
        created_at_ns: u64,
        key_epoch: KeyEpoch,
    ) -> Result<Self, ChannelEndpointError> {
        Self::validated(
            channel_id,
            originator,
            receivers,
            created_at_ns,
            key_epoch,
            ChannelLifecycle::Active,
        )
    }

    /// Return the channel identifier.
    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Borrow the only entity allowed to publish.
    pub fn originator(&self) -> &OriginatorIdentity {
        &self.originator
    }

    /// Borrow the sorted authorized receiver set.
    pub fn receivers(&self) -> &[ReceiverIdentity] {
        &self.receivers
    }

    /// Return the durable creation timestamp in nanoseconds.
    pub fn created_at_ns(&self) -> u64 {
        self.created_at_ns
    }

    /// Return the only key epoch accepted for new publishes.
    pub fn key_epoch(&self) -> KeyEpoch {
        self.key_epoch
    }

    /// Return the one-way channel lifecycle.
    pub fn lifecycle(&self) -> ChannelLifecycle {
        self.lifecycle
    }

    /// Look up one authorized receiver.
    pub fn receiver(&self, agent_id: &AgentId) -> Option<&ReceiverIdentity> {
        self.receivers
            .binary_search_by(|receiver| receiver.agent_id.cmp(agent_id))
            .ok()
            .map(|index| &self.receivers[index])
    }

    fn validated(
        channel_id: ChannelId,
        originator: OriginatorIdentity,
        mut receivers: Vec<ReceiverIdentity>,
        created_at_ns: u64,
        key_epoch: KeyEpoch,
        lifecycle: ChannelLifecycle,
    ) -> Result<Self, ChannelEndpointError> {
        validate_uuid_v7(&channel_id.0).map_err(|_| {
            ChannelEndpointError::InvalidDefinition("channel ID is not a canonical UUID v7")
        })?;
        validate_agent_id(&originator.agent_id)?;
        if receivers.is_empty() || receivers.len() > MAX_RECEIVERS {
            return Err(ChannelEndpointError::InvalidDefinition(
                "channel must have between one and 1024 receivers",
            ));
        }
        receivers.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        for receiver in &receivers {
            validate_agent_id(&receiver.agent_id)?;
            if receiver.agent_id == originator.agent_id {
                return Err(ChannelEndpointError::InvalidDefinition(
                    "originator cannot also receive on the same channel",
                ));
            }
        }
        if receivers
            .windows(2)
            .any(|pair| pair[0].agent_id == pair[1].agent_id)
        {
            return Err(ChannelEndpointError::InvalidDefinition(
                "receiver identifiers must be unique",
            ));
        }
        Ok(Self {
            channel_id,
            originator,
            receivers,
            created_at_ns,
            key_epoch,
            lifecycle,
        })
    }
}

/// Canonical UUID-v7 message identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId([u8; 16]);

impl MessageId {
    /// Validate UUID-v7 version and RFC-4122/9562 variant bits.
    pub fn from_uuid_v7(bytes: [u8; 16]) -> Result<Self, ChannelEndpointError> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(ChannelEndpointError::InvalidMessageId);
        }
        Ok(Self(bytes))
    }

    /// Return the canonical 16 wire bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Injected immutable metadata for one publish operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageMetadata {
    /// UUID-v7 identity for this message.
    pub message_id: MessageId,
    /// Caller-owned monotonic nanosecond timestamp.
    pub timestamp_ns: u64,
}

/// Pluggable source for message IDs and timestamps.
///
/// The endpoint package invokes this interface but performs no direct clock or
/// operating-system random access of its own.
pub trait MessageMetadataSource: Send + Sync {
    /// Mint metadata for the next publish attempt.
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError>;
}

/// Error supplied by an injected message-metadata source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageMetadataError(String);

impl MessageMetadataError {
    /// Construct an error with a non-sensitive diagnostic.
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl core::fmt::Display for MessageMetadataError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for MessageMetadataError {}

/// Receipt returned after a ciphertext append is durably finalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublishedMessage {
    /// UUID-v7 message identity.
    pub message_id: MessageId,
    /// Durable global channel sequence.
    pub sequence: Sequence,
    /// Authenticated caller-provided timestamp.
    pub timestamp_ns: u64,
}

/// Verified and decrypted message delivered to an authorized receiver.
#[derive(Clone, PartialEq, Eq)]
pub struct ReceivedMessage {
    /// UUID-v7 message identity used for acknowledgement.
    pub message_id: MessageId,
    /// Durable global channel sequence.
    pub sequence: Sequence,
    /// Authenticated originator timestamp.
    pub timestamp_ns: u64,
    /// Authenticated MIME content type.
    pub content_type: String,
    /// Verified plaintext payload.
    pub payload: Vec<u8>,
}

/// Errors returned by durable membership and endpoint operations.
#[derive(Debug)]
pub enum ChannelEndpointError {
    /// The injected storage backend rejected an operation.
    Storage(StorageError),
    /// The encrypted channel store rejected an operation.
    Store(Box<ChannelStoreError>),
    /// Channel cryptography rejected an operation.
    Crypto(ChannelCryptoError),
    /// The injected metadata source could not mint message metadata.
    Metadata(MessageMetadataError),
    /// A durable definition violated a static membership rule.
    InvalidDefinition(&'static str),
    /// Message bytes did not represent a canonical UUID v7.
    InvalidMessageId,
    /// No durable definition exists for the requested channel.
    DefinitionNotFound,
    /// A different definition already occupies this channel ID.
    ConflictingDefinition,
    /// Persisted definition bytes or their storage envelope were invalid.
    CorruptDefinition(&'static str),
    /// A cached endpoint no longer matches the durable definition.
    DefinitionChanged,
    /// A destroyed channel denied an endpoint operation.
    ChannelDestroyed,
    /// The caller is not the definition's single originator.
    UnauthorizedOriginator,
    /// The caller is not in the definition's receiver set.
    UnauthorizedReceiver,
    /// The supplied private key does not match the definition's public key.
    PublicKeyMismatch,
    /// A receiver has no sealed key grant for a message epoch.
    MissingKeyGrant(KeyEpoch),
    /// A receiver attempted to acknowledge a message it was not delivered.
    UnknownMessageId(MessageId),
    /// An encrypted message violated the durable membership definition.
    UnauthorizedMessage,
    /// Repeated compare-and-swap races prevented lifecycle progress.
    ConcurrentUpdate,
}

impl core::fmt::Display for ChannelEndpointError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "{error}"),
            Self::Crypto(error) => write!(formatter, "{error}"),
            Self::Metadata(error) => write!(formatter, "message metadata source failed: {error}"),
            Self::InvalidDefinition(message) => {
                write!(formatter, "invalid channel definition: {message}")
            }
            Self::InvalidMessageId => formatter.write_str("message ID is not a canonical UUID v7"),
            Self::DefinitionNotFound => formatter.write_str("channel definition not found"),
            Self::ConflictingDefinition => {
                formatter.write_str("a different channel definition already exists")
            }
            Self::CorruptDefinition(message) => {
                write!(formatter, "corrupt channel definition: {message}")
            }
            Self::DefinitionChanged => formatter.write_str("durable channel definition changed"),
            Self::ChannelDestroyed => formatter.write_str("channel is destroyed"),
            Self::UnauthorizedOriginator => formatter.write_str("entity is not channel originator"),
            Self::UnauthorizedReceiver => formatter.write_str("entity is not channel receiver"),
            Self::PublicKeyMismatch => {
                formatter.write_str("endpoint private key does not match durable public key")
            }
            Self::MissingKeyGrant(epoch) => {
                write!(
                    formatter,
                    "receiver has no sealed key grant for epoch {}",
                    epoch.0
                )
            }
            Self::UnknownMessageId(message_id) => write!(
                formatter,
                "message {:02x?} was not delivered by this receiver session",
                message_id.0
            ),
            Self::UnauthorizedMessage => {
                formatter.write_str("encrypted message violates channel membership")
            }
            Self::ConcurrentUpdate => {
                formatter.write_str("channel definition changed too many times concurrently")
            }
        }
    }
}

impl std::error::Error for ChannelEndpointError {}

impl From<StorageError> for ChannelEndpointError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<ChannelStoreError> for ChannelEndpointError {
    fn from(error: ChannelStoreError) -> Self {
        Self::Store(Box::new(error))
    }
}

impl From<ChannelCryptoError> for ChannelEndpointError {
    fn from(error: ChannelCryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<MessageMetadataError> for ChannelEndpointError {
    fn from(error: MessageMetadataError) -> Self {
        Self::Metadata(error)
    }
}

/// Create, load, and retire durable one-way channel definitions.
pub struct ChannelDefinitionStore<'a> {
    backend: &'a dyn StorageBackend,
}

impl<'a> ChannelDefinitionStore<'a> {
    /// Bind the definition store to an injected storage backend.
    pub fn new(backend: &'a dyn StorageBackend) -> Self {
        Self { backend }
    }

    /// Atomically create one definition and initialize its encrypted log.
    ///
    /// Repeating the byte-identical definition is idempotent. A different
    /// definition for the same channel ID is rejected.
    pub fn create(
        &self,
        definition: &ChannelDefinition,
    ) -> Result<ChannelDefinition, ChannelEndpointError> {
        if definition.lifecycle != ChannelLifecycle::Active {
            return Err(ChannelEndpointError::InvalidDefinition(
                "new channel definition must be active",
            ));
        }
        self.backend.initialize()?;
        let key = channel_definition_record_key(definition.channel_id);
        let body = encode_definition(definition);
        let input = definition_put(key.clone(), body.clone())?.with_if_absent();
        let persisted = match self.backend.put(input) {
            Ok(record) => require_definition_record(&record, definition.channel_id)?,
            Err(StorageError::Conflict { .. }) => {
                let record = self
                    .backend
                    .get(CHANNEL_STORAGE_NAMESPACE, &key)?
                    .ok_or(ChannelEndpointError::DefinitionNotFound)?;
                require_definition_content_type(&record)?;
                if record.body != body {
                    return Err(ChannelEndpointError::ConflictingDefinition);
                }
                require_definition_record(&record, definition.channel_id)?
            }
            Err(error) => return Err(error.into()),
        };
        if persisted != *definition {
            return Err(ChannelEndpointError::ConflictingDefinition);
        }
        ChannelStore::new(self.backend, definition.channel_id).initialize()?;
        self.require_current(definition)
    }

    /// Load one durable channel definition.
    pub fn load(
        &self,
        channel_id: ChannelId,
    ) -> Result<Option<ChannelDefinition>, ChannelEndpointError> {
        self.backend.initialize()?;
        Ok(self
            .load_record(channel_id)?
            .map(|loaded| loaded.definition))
    }

    /// Irreversibly mark an active channel destroyed using revision CAS.
    pub fn destroy(
        &self,
        channel_id: ChannelId,
    ) -> Result<ChannelDefinition, ChannelEndpointError> {
        for _ in 0..MAX_CAS_ATTEMPTS {
            let loaded = self
                .load_record(channel_id)?
                .ok_or(ChannelEndpointError::DefinitionNotFound)?;
            if loaded.definition.lifecycle == ChannelLifecycle::Destroyed {
                return Ok(loaded.definition);
            }
            let mut destroyed = loaded.definition;
            destroyed.lifecycle = ChannelLifecycle::Destroyed;
            let input = definition_put(
                channel_definition_record_key(channel_id),
                encode_definition(&destroyed),
            )?
            .with_if_revision(Some(loaded.revision));
            match self.backend.put(input) {
                Ok(record) => return require_definition_record(&record, channel_id),
                Err(StorageError::Conflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ChannelEndpointError::ConcurrentUpdate)
    }

    fn require_current(
        &self,
        expected: &ChannelDefinition,
    ) -> Result<ChannelDefinition, ChannelEndpointError> {
        let actual = self
            .load(expected.channel_id)?
            .ok_or(ChannelEndpointError::DefinitionNotFound)?;
        if actual.lifecycle == ChannelLifecycle::Destroyed {
            return Err(ChannelEndpointError::ChannelDestroyed);
        }
        if actual != *expected {
            return Err(ChannelEndpointError::DefinitionChanged);
        }
        Ok(actual)
    }

    fn load_record(
        &self,
        channel_id: ChannelId,
    ) -> Result<Option<LoadedDefinition>, ChannelEndpointError> {
        let key = channel_definition_record_key(channel_id);
        let Some(record) = self.backend.get(CHANNEL_STORAGE_NAMESPACE, &key)? else {
            return Ok(None);
        };
        let revision = record.revision.clone();
        let definition = require_definition_record(&record, channel_id)?;
        Ok(Some(LoadedDefinition {
            definition,
            revision,
        }))
    }
}

struct LoadedDefinition {
    definition: ChannelDefinition,
    revision: Revision,
}

/// Universal interface implemented by authorized channel originators.
pub trait Originator {
    /// Borrow the endpoint entity identifier.
    fn id(&self) -> &AgentId;
    /// Return the bound channel identifier.
    fn channel_id(&self) -> ChannelId;
    /// Return the Ed25519 public verification key.
    fn public_key(&self) -> [u8; 32];
    /// Encrypt and durably publish one rich payload.
    fn publish(
        &self,
        payload: &[u8],
        content_type: &str,
    ) -> Result<PublishedMessage, ChannelEndpointError>;
}

/// Durable implementation of the single-originator channel role.
pub struct DurableOriginator<'a> {
    backend: &'a dyn StorageBackend,
    definition: ChannelDefinition,
    signing_key: &'a OriginatorSigningKey,
    channel_key: &'a ChannelMasterKey,
    metadata_source: &'a dyn MessageMetadataSource,
}

impl<'a> DurableOriginator<'a> {
    /// Open the definition's only authorized originator endpoint.
    pub fn open(
        backend: &'a dyn StorageBackend,
        channel_id: ChannelId,
        agent_id: &AgentId,
        signing_key: &'a OriginatorSigningKey,
        channel_key: &'a ChannelMasterKey,
        metadata_source: &'a dyn MessageMetadataSource,
    ) -> Result<Self, ChannelEndpointError> {
        let definition = active_definition(backend, channel_id)?;
        if definition.originator.agent_id != *agent_id {
            return Err(ChannelEndpointError::UnauthorizedOriginator);
        }
        if definition.originator.public_key != signing_key.public_key() {
            return Err(ChannelEndpointError::PublicKeyMismatch);
        }
        ChannelStore::new(backend, channel_id).initialize()?;
        Ok(Self {
            backend,
            definition,
            signing_key,
            channel_key,
            metadata_source,
        })
    }

    /// Publish with caller-selected metadata, primarily for idempotent recovery.
    pub fn publish_with_metadata(
        &self,
        metadata: MessageMetadata,
        payload: &[u8],
        content_type: &str,
    ) -> Result<PublishedMessage, ChannelEndpointError> {
        ChannelDefinitionStore::new(self.backend).require_current(&self.definition)?;
        let message = ChannelStore::new(self.backend, self.definition.channel_id).append(
            AppendRequest {
                message_id: metadata.message_id.0,
                timestamp_ns: metadata.timestamp_ns,
                originator_id: self.definition.originator.agent_id.0.clone(),
                key_epoch: self.definition.key_epoch,
                content_type: content_type.to_owned(),
            },
            payload,
            self.channel_key,
            self.signing_key,
        )?;
        Ok(PublishedMessage {
            message_id: metadata.message_id,
            sequence: message.header.fields.sequence,
            timestamp_ns: metadata.timestamp_ns,
        })
    }

    /// Seal and idempotently persist the current epoch key for one receiver.
    pub fn grant_receiver(
        &self,
        receiver_id: &AgentId,
    ) -> Result<SealedChannelKeyGrant, ChannelEndpointError> {
        let definition =
            ChannelDefinitionStore::new(self.backend).require_current(&self.definition)?;
        let receiver = definition
            .receiver(receiver_id)
            .ok_or(ChannelEndpointError::UnauthorizedReceiver)?;
        let store = ChannelStore::new(self.backend, definition.channel_id);
        if let Some(existing) = store.key_grant(definition.key_epoch, receiver_id.as_bytes())? {
            if existing.originator_id != definition.originator.agent_id.0 {
                return Err(ChannelEndpointError::UnauthorizedMessage);
            }
            return Ok(existing);
        }
        let grant = seal_channel_key(
            definition.originator.agent_id.as_bytes(),
            receiver.agent_id.as_bytes(),
            definition.channel_id,
            definition.key_epoch,
            self.channel_key,
            &receiver.public_key,
            self.signing_key,
        )?;
        store.save_key_grant(&grant)?;
        Ok(grant)
    }
}

impl Originator for DurableOriginator<'_> {
    fn id(&self) -> &AgentId {
        &self.definition.originator.agent_id
    }

    fn channel_id(&self) -> ChannelId {
        self.definition.channel_id
    }

    fn public_key(&self) -> [u8; 32] {
        self.signing_key.public_key()
    }

    fn publish(
        &self,
        payload: &[u8],
        content_type: &str,
    ) -> Result<PublishedMessage, ChannelEndpointError> {
        let metadata = self.metadata_source.next_metadata()?;
        self.publish_with_metadata(metadata, payload, content_type)
    }
}

/// Universal interface implemented by authorized channel receivers.
pub trait Receiver {
    /// Borrow the endpoint entity identifier.
    fn id(&self) -> &AgentId;
    /// Return the bound channel identifier.
    fn channel_id(&self) -> ChannelId;
    /// Return the X25519 key-exchange public key.
    fn public_key(&self) -> [u8; 32];
    /// Deliver an ordered page of verified plaintext messages.
    fn receive(&mut self, limit: usize) -> Result<Vec<ReceivedMessage>, ChannelEndpointError>;
    /// Monotonically acknowledge a message delivered by this endpoint session.
    fn acknowledge(&mut self, message_id: MessageId) -> Result<Sequence, ChannelEndpointError>;
}

/// Durable implementation of one authorized receiver role.
pub struct DurableReceiver<'a> {
    backend: &'a dyn StorageBackend,
    definition: ChannelDefinition,
    receiver_id: AgentId,
    epoch_keys: ReceiverEpochKeys,
    delivered: BTreeMap<MessageId, Sequence>,
}

impl<'a> DurableReceiver<'a> {
    /// Open one authorized receiver endpoint and take ownership of its private key.
    pub fn open(
        backend: &'a dyn StorageBackend,
        channel_id: ChannelId,
        receiver_id: AgentId,
        receiver_key_pair: ReceiverKeyPair,
    ) -> Result<Self, ChannelEndpointError> {
        let definition = active_definition(backend, channel_id)?;
        let receiver = definition
            .receiver(&receiver_id)
            .ok_or(ChannelEndpointError::UnauthorizedReceiver)?;
        if receiver.public_key != receiver_key_pair.public_key() {
            return Err(ChannelEndpointError::PublicKeyMismatch);
        }
        let epoch_keys = ReceiverEpochKeys::new(
            definition.originator.agent_id.0.clone(),
            receiver_id.0.clone(),
            channel_id,
            receiver_key_pair,
            definition.originator.public_key,
        );
        ChannelStore::new(backend, channel_id).initialize()?;
        Ok(Self {
            backend,
            definition,
            receiver_id,
            epoch_keys,
            delivered: BTreeMap::new(),
        })
    }

    fn load_page(
        &mut self,
        page: MessagePage,
    ) -> Result<Vec<ReceivedMessage>, ChannelEndpointError> {
        let mut delivered = Vec::with_capacity(page.messages.len());
        let store = ChannelStore::new(self.backend, self.definition.channel_id);
        for encrypted in page.messages {
            let fields = &encrypted.header.fields;
            if fields.channel_id != self.definition.channel_id
                || fields.originator_id != self.definition.originator.agent_id.0
                || fields.key_epoch > self.definition.key_epoch
            {
                return Err(ChannelEndpointError::UnauthorizedMessage);
            }
            if self.epoch_keys.epoch_key(fields.key_epoch).is_none() {
                let grant = store
                    .key_grant(fields.key_epoch, self.receiver_id.as_bytes())?
                    .ok_or(ChannelEndpointError::MissingKeyGrant(fields.key_epoch))?;
                self.epoch_keys.install_grant(grant)?;
            }
            let key = self
                .epoch_keys
                .epoch_key(fields.key_epoch)
                .ok_or(ChannelEndpointError::MissingKeyGrant(fields.key_epoch))?;
            let payload = decrypt_message(&encrypted, key, &self.definition.originator.public_key)?;
            let message_id = MessageId::from_uuid_v7(fields.message_id)?;
            if self
                .delivered
                .insert(message_id, fields.sequence)
                .is_some_and(|previous| previous != fields.sequence)
            {
                return Err(ChannelEndpointError::UnauthorizedMessage);
            }
            delivered.push(ReceivedMessage {
                message_id,
                sequence: fields.sequence,
                timestamp_ns: fields.timestamp_ns,
                content_type: fields.content_type.clone(),
                payload,
            });
        }
        Ok(delivered)
    }
}

impl Receiver for DurableReceiver<'_> {
    fn id(&self) -> &AgentId {
        &self.receiver_id
    }

    fn channel_id(&self) -> ChannelId {
        self.definition.channel_id
    }

    fn public_key(&self) -> [u8; 32] {
        self.epoch_keys.receiver_public_key()
    }

    fn receive(&mut self, limit: usize) -> Result<Vec<ReceivedMessage>, ChannelEndpointError> {
        ChannelDefinitionStore::new(self.backend).require_current(&self.definition)?;
        let page = ChannelStore::new(self.backend, self.definition.channel_id)
            .read_for_receiver(self.receiver_id.as_bytes(), limit)?;
        self.load_page(page)
    }

    fn acknowledge(&mut self, message_id: MessageId) -> Result<Sequence, ChannelEndpointError> {
        ChannelDefinitionStore::new(self.backend).require_current(&self.definition)?;
        let sequence = self
            .delivered
            .get(&message_id)
            .copied()
            .ok_or(ChannelEndpointError::UnknownMessageId(message_id))?;
        Ok(ChannelStore::new(self.backend, self.definition.channel_id)
            .acknowledge(self.receiver_id.as_bytes(), sequence)?)
    }
}

fn active_definition(
    backend: &dyn StorageBackend,
    channel_id: ChannelId,
) -> Result<ChannelDefinition, ChannelEndpointError> {
    let definition = ChannelDefinitionStore::new(backend)
        .load(channel_id)?
        .ok_or(ChannelEndpointError::DefinitionNotFound)?;
    if definition.lifecycle == ChannelLifecycle::Destroyed {
        return Err(ChannelEndpointError::ChannelDestroyed);
    }
    Ok(definition)
}

fn validate_agent_id(agent_id: &AgentId) -> Result<(), ChannelEndpointError> {
    if agent_id.0.is_empty() || agent_id.0.len() > MAX_IDENTITY_BYTES {
        return Err(ChannelEndpointError::InvalidDefinition(
            "agent identifier is empty or exceeds the channel bound",
        ));
    }
    Ok(())
}

fn validate_uuid_v7(bytes: &[u8; 16]) -> Result<(), ChannelEndpointError> {
    if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
        return Err(ChannelEndpointError::InvalidMessageId);
    }
    Ok(())
}

fn definition_put(key: String, body: Vec<u8>) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(
        CHANNEL_STORAGE_NAMESPACE,
        key,
        DEFINITION_CONTENT_TYPE,
        JsonValue::Object(vec![]),
        body,
    )
}

fn require_definition_content_type(record: &StorageRecord) -> Result<(), ChannelEndpointError> {
    if record.content_type != DEFINITION_CONTENT_TYPE {
        return Err(ChannelEndpointError::CorruptDefinition(
            "definition content type is invalid",
        ));
    }
    Ok(())
}

fn require_definition_record(
    record: &StorageRecord,
    expected_channel_id: ChannelId,
) -> Result<ChannelDefinition, ChannelEndpointError> {
    require_definition_content_type(record)?;
    let definition = decode_definition(&record.body)?;
    if definition.channel_id != expected_channel_id
        || record.key != channel_definition_record_key(expected_channel_id)
    {
        return Err(ChannelEndpointError::CorruptDefinition(
            "definition body does not match storage key",
        ));
    }
    Ok(definition)
}

fn encode_definition(definition: &ChannelDefinition) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(DEFINITION_MAGIC);
    bytes.push(DEFINITION_VERSION);
    bytes.extend_from_slice(&definition.channel_id.0);
    put_bytes(&mut bytes, definition.originator.agent_id.as_bytes());
    bytes.extend_from_slice(&definition.originator.public_key);
    bytes.extend_from_slice(&(definition.receivers.len() as u32).to_be_bytes());
    for receiver in &definition.receivers {
        put_bytes(&mut bytes, receiver.agent_id.as_bytes());
        bytes.extend_from_slice(&receiver.public_key);
    }
    bytes.extend_from_slice(&definition.created_at_ns.to_be_bytes());
    bytes.extend_from_slice(&definition.key_epoch.0.to_be_bytes());
    bytes.push(match definition.lifecycle {
        ChannelLifecycle::Active => 0,
        ChannelLifecycle::Destroyed => 1,
    });
    bytes
}

fn decode_definition(bytes: &[u8]) -> Result<ChannelDefinition, ChannelEndpointError> {
    let mut decoder = DefinitionDecoder::new(bytes);
    if decoder.read_array::<4>()? != *DEFINITION_MAGIC {
        return Err(ChannelEndpointError::CorruptDefinition(
            "definition magic is invalid",
        ));
    }
    if decoder.read_u8()? != DEFINITION_VERSION {
        return Err(ChannelEndpointError::CorruptDefinition(
            "definition version is unsupported",
        ));
    }
    let channel_id = ChannelId(decoder.read_array()?);
    let originator = OriginatorIdentity {
        agent_id: decode_agent_id(decoder.read_vec(MAX_IDENTITY_BYTES)?)?,
        public_key: decoder.read_array()?,
    };
    let receiver_count = decoder.read_u32()? as usize;
    if receiver_count == 0 || receiver_count > MAX_RECEIVERS {
        return Err(ChannelEndpointError::CorruptDefinition(
            "receiver count is outside the durable bound",
        ));
    }
    let mut receivers = Vec::with_capacity(receiver_count);
    for _ in 0..receiver_count {
        receivers.push(ReceiverIdentity {
            agent_id: decode_agent_id(decoder.read_vec(MAX_IDENTITY_BYTES)?)?,
            public_key: decoder.read_array()?,
        });
    }
    let created_at_ns = decoder.read_u64()?;
    let key_epoch = KeyEpoch(decoder.read_u64()?);
    let lifecycle = match decoder.read_u8()? {
        0 => ChannelLifecycle::Active,
        1 => ChannelLifecycle::Destroyed,
        _ => {
            return Err(ChannelEndpointError::CorruptDefinition(
                "channel lifecycle is invalid",
            ))
        }
    };
    decoder.finish()?;
    ChannelDefinition::validated(
        channel_id,
        originator,
        receivers,
        created_at_ns,
        key_epoch,
        lifecycle,
    )
    .map_err(|_| ChannelEndpointError::CorruptDefinition("membership invariant failed"))
}

fn decode_agent_id(bytes: Vec<u8>) -> Result<AgentId, ChannelEndpointError> {
    AgentId::new(bytes)
        .map_err(|_| ChannelEndpointError::CorruptDefinition("agent identifier is invalid"))
}

fn put_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    output.extend_from_slice(bytes);
}

struct DefinitionDecoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> DefinitionDecoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ChannelEndpointError> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(ChannelEndpointError::CorruptDefinition(
                "definition length overflow",
            ))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(ChannelEndpointError::CorruptDefinition(
                "definition is truncated",
            ))?
            .try_into()
            .map_err(|_| ChannelEndpointError::CorruptDefinition("definition is truncated"))?;
        self.position = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, ChannelEndpointError> {
        Ok(self.read_array::<1>()?[0])
    }

    fn read_u32(&mut self) -> Result<u32, ChannelEndpointError> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    fn read_u64(&mut self) -> Result<u64, ChannelEndpointError> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    fn read_vec(&mut self, maximum: usize) -> Result<Vec<u8>, ChannelEndpointError> {
        let length = self.read_u32()? as usize;
        if length > maximum {
            return Err(ChannelEndpointError::CorruptDefinition(
                "identity exceeds the durable bound",
            ));
        }
        let end =
            self.position
                .checked_add(length)
                .ok_or(ChannelEndpointError::CorruptDefinition(
                    "definition length overflow",
                ))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(ChannelEndpointError::CorruptDefinition(
                "definition is truncated",
            ))?
            .to_vec();
        self.position = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), ChannelEndpointError> {
        if self.position != self.bytes.len() {
            return Err(ChannelEndpointError::CorruptDefinition(
                "definition has trailing bytes",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use storage_core::InMemoryStorageBackend;
    use storage_local_folder::LocalFolderStorageBackend;

    struct FixedMetadataSource {
        values: Mutex<VecDeque<MessageMetadata>>,
    }

    impl FixedMetadataSource {
        fn new(values: Vec<MessageMetadata>) -> Self {
            Self {
                values: Mutex::new(values.into()),
            }
        }
    }

    impl MessageMetadataSource for FixedMetadataSource {
        fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
            self.values
                .lock()
                .expect("metadata mutex poisoned")
                .pop_front()
                .ok_or_else(|| MessageMetadataError::new("metadata exhausted"))
        }
    }

    fn channel_id() -> ChannelId {
        let mut bytes = [0x61; 16];
        bytes[6] = 0x71;
        bytes[8] = 0xa1;
        ChannelId(bytes)
    }

    fn message_id(byte: u8) -> MessageId {
        let mut bytes = [byte; 16];
        bytes[6] = 0x70 | (byte & 0x0f);
        bytes[8] = 0x80 | (byte & 0x3f);
        MessageId::from_uuid_v7(bytes).unwrap()
    }

    fn originator_id() -> AgentId {
        AgentId::new(b"originator".to_vec()).unwrap()
    }

    fn receiver_id() -> AgentId {
        AgentId::new(b"receiver".to_vec()).unwrap()
    }

    fn identities() -> (OriginatorSigningKey, ReceiverKeyPair) {
        (
            OriginatorSigningKey::from_seed([0x31; 32]),
            ReceiverKeyPair::from_private_key([0x42; 32]).unwrap(),
        )
    }

    fn definition(
        signing_key: &OriginatorSigningKey,
        receiver_key: &ReceiverKeyPair,
    ) -> ChannelDefinition {
        ChannelDefinition::new(
            channel_id(),
            OriginatorIdentity {
                agent_id: originator_id(),
                public_key: signing_key.public_key(),
            },
            vec![ReceiverIdentity {
                agent_id: receiver_id(),
                public_key: receiver_key.public_key(),
            }],
            1_725_000_000_000_000_000,
            KeyEpoch(0),
        )
        .unwrap()
    }

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chief-channel-endpoints-{label}-{}-{}",
            std::process::id(),
            storage_core::now_utc_ms()
        ))
    }

    #[test]
    fn membership_rejects_overlap_duplicates_and_empty_receivers() {
        let (signing_key, receiver_key) = identities();
        let originator = OriginatorIdentity {
            agent_id: originator_id(),
            public_key: signing_key.public_key(),
        };
        assert!(matches!(
            ChannelDefinition::new(channel_id(), originator.clone(), vec![], 1, KeyEpoch(0)),
            Err(ChannelEndpointError::InvalidDefinition(_))
        ));
        assert!(matches!(
            ChannelDefinition::new(
                ChannelId([0x61; 16]),
                originator.clone(),
                vec![ReceiverIdentity {
                    agent_id: receiver_id(),
                    public_key: receiver_key.public_key(),
                }],
                1,
                KeyEpoch(0)
            ),
            Err(ChannelEndpointError::InvalidDefinition(_))
        ));
        let overlapping = ReceiverIdentity {
            agent_id: originator_id(),
            public_key: receiver_key.public_key(),
        };
        assert!(matches!(
            ChannelDefinition::new(
                channel_id(),
                originator.clone(),
                vec![overlapping],
                1,
                KeyEpoch(0)
            ),
            Err(ChannelEndpointError::InvalidDefinition(_))
        ));
        let receiver = ReceiverIdentity {
            agent_id: receiver_id(),
            public_key: receiver_key.public_key(),
        };
        assert!(matches!(
            ChannelDefinition::new(
                channel_id(),
                originator,
                vec![receiver.clone(), receiver],
                1,
                KeyEpoch(0)
            ),
            Err(ChannelEndpointError::InvalidDefinition(_))
        ));
    }

    #[test]
    fn definition_create_is_idempotent_conflict_safe_and_restart_durable() {
        let root = temp_root("definition");
        let (signing_key, receiver_key) = identities();
        let expected = definition(&signing_key, &receiver_key);
        {
            let backend = LocalFolderStorageBackend::new(&root);
            let store = ChannelDefinitionStore::new(&backend);
            assert_eq!(store.create(&expected).unwrap(), expected);
            assert_eq!(store.create(&expected).unwrap(), expected);
        }
        {
            let backend = LocalFolderStorageBackend::new(&root);
            let store = ChannelDefinitionStore::new(&backend);
            assert_eq!(store.load(channel_id()).unwrap(), Some(expected.clone()));
            let mut conflicting = expected.clone();
            conflicting.created_at_ns += 1;
            assert!(matches!(
                store.create(&conflicting),
                Err(ChannelEndpointError::ConflictingDefinition)
            ));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn encrypted_originator_receiver_round_trip_and_ack() {
        let backend = InMemoryStorageBackend::new();
        let (signing_key, receiver_key) = identities();
        let receiver_private = [0x42; 32];
        let definition = definition(&signing_key, &receiver_key);
        ChannelDefinitionStore::new(&backend)
            .create(&definition)
            .unwrap();
        let metadata = MessageMetadata {
            message_id: message_id(7),
            timestamp_ns: 77,
        };
        let source = FixedMetadataSource::new(vec![metadata]);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let originator = DurableOriginator::open(
            &backend,
            channel_id(),
            &originator_id(),
            &signing_key,
            &cmk,
            &source,
        )
        .unwrap();
        let first_grant = originator.grant_receiver(&receiver_id()).unwrap();
        let retry_grant = originator.grant_receiver(&receiver_id()).unwrap();
        assert!(first_grant == retry_grant);
        let published = originator.publish(b"hello receiver", "text/plain").unwrap();
        assert_eq!(published.message_id, metadata.message_id);

        let mut receiver = DurableReceiver::open(
            &backend,
            channel_id(),
            receiver_id(),
            ReceiverKeyPair::from_private_key(receiver_private).unwrap(),
        )
        .unwrap();
        let messages = receiver.receive(10).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id, metadata.message_id);
        assert_eq!(messages[0].content_type, "text/plain");
        assert_eq!(messages[0].payload, b"hello receiver");
        assert_eq!(
            receiver.acknowledge(metadata.message_id).unwrap(),
            Sequence(1)
        );
        assert!(receiver.receive(10).unwrap().is_empty());
        assert_eq!(
            receiver.acknowledge(metadata.message_id).unwrap(),
            Sequence(1)
        );
    }

    #[test]
    fn endpoint_open_rejects_wrong_role_and_private_key() {
        let backend = InMemoryStorageBackend::new();
        let (signing_key, receiver_key) = identities();
        ChannelDefinitionStore::new(&backend)
            .create(&definition(&signing_key, &receiver_key))
            .unwrap();
        let source = FixedMetadataSource::new(vec![]);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        assert!(matches!(
            DurableOriginator::open(
                &backend,
                channel_id(),
                &AgentId::new(b"intruder".to_vec()).unwrap(),
                &signing_key,
                &cmk,
                &source
            ),
            Err(ChannelEndpointError::UnauthorizedOriginator)
        ));
        assert!(matches!(
            DurableReceiver::open(
                &backend,
                channel_id(),
                receiver_id(),
                ReceiverKeyPair::from_private_key([0x99; 32]).unwrap()
            ),
            Err(ChannelEndpointError::PublicKeyMismatch)
        ));
    }

    #[test]
    fn receiver_cannot_acknowledge_an_undelivered_message() {
        let backend = InMemoryStorageBackend::new();
        let (signing_key, receiver_key) = identities();
        ChannelDefinitionStore::new(&backend)
            .create(&definition(&signing_key, &receiver_key))
            .unwrap();
        let mut receiver =
            DurableReceiver::open(&backend, channel_id(), receiver_id(), receiver_key).unwrap();
        assert!(matches!(
            receiver.acknowledge(message_id(8)),
            Err(ChannelEndpointError::UnknownMessageId(_))
        ));
    }

    #[test]
    fn receiver_fails_closed_until_originator_persists_its_grant() {
        let backend = InMemoryStorageBackend::new();
        let (signing_key, receiver_key) = identities();
        ChannelDefinitionStore::new(&backend)
            .create(&definition(&signing_key, &receiver_key))
            .unwrap();
        let metadata = MessageMetadata {
            message_id: message_id(12),
            timestamp_ns: 12,
        };
        let source = FixedMetadataSource::new(vec![metadata]);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let originator = DurableOriginator::open(
            &backend,
            channel_id(),
            &originator_id(),
            &signing_key,
            &cmk,
            &source,
        )
        .unwrap();
        originator.publish(b"still sealed", "text/plain").unwrap();
        let mut receiver =
            DurableReceiver::open(&backend, channel_id(), receiver_id(), receiver_key).unwrap();
        assert!(matches!(
            receiver.receive(1),
            Err(ChannelEndpointError::MissingKeyGrant(KeyEpoch(0)))
        ));
        originator.grant_receiver(&receiver_id()).unwrap();
        assert_eq!(receiver.receive(1).unwrap()[0].payload, b"still sealed");
    }

    #[test]
    fn destruction_is_idempotent_and_revokes_open_endpoints() {
        let backend = InMemoryStorageBackend::new();
        let (signing_key, receiver_key) = identities();
        let definition = definition(&signing_key, &receiver_key);
        let definitions = ChannelDefinitionStore::new(&backend);
        definitions.create(&definition).unwrap();
        let source = FixedMetadataSource::new(vec![MessageMetadata {
            message_id: message_id(9),
            timestamp_ns: 9,
        }]);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let originator = DurableOriginator::open(
            &backend,
            channel_id(),
            &originator_id(),
            &signing_key,
            &cmk,
            &source,
        )
        .unwrap();
        assert_eq!(
            definitions.destroy(channel_id()).unwrap().lifecycle(),
            ChannelLifecycle::Destroyed
        );
        assert_eq!(
            definitions.destroy(channel_id()).unwrap().lifecycle(),
            ChannelLifecycle::Destroyed
        );
        assert!(matches!(
            originator.publish(b"denied", "text/plain"),
            Err(ChannelEndpointError::ChannelDestroyed)
        ));
    }

    #[test]
    fn malformed_definition_fails_closed() {
        let backend = InMemoryStorageBackend::new();
        backend.initialize().unwrap();
        backend
            .put(
                definition_put(
                    channel_definition_record_key(channel_id()),
                    b"not a definition".to_vec(),
                )
                .unwrap(),
            )
            .unwrap();
        assert!(matches!(
            ChannelDefinitionStore::new(&backend).load(channel_id()),
            Err(ChannelEndpointError::CorruptDefinition(_))
        ));
    }

    #[test]
    fn definition_codec_rejects_every_truncated_prefix_and_trailing_bytes() {
        let (signing_key, receiver_key) = identities();
        let expected = definition(&signing_key, &receiver_key);
        let encoded = encode_definition(&expected);
        assert_eq!(decode_definition(&encoded).unwrap(), expected);
        for end in 0..encoded.len() {
            assert!(decode_definition(&encoded[..end]).is_err(), "prefix {end}");
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(matches!(
            decode_definition(&trailing),
            Err(ChannelEndpointError::CorruptDefinition(_))
        ));
    }
}
