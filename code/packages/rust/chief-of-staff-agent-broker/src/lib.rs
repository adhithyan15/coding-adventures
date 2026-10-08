//! # One agent's broker (D18S S-K7, step 6 P2.6d-1)
//!
//! The broker is the only process that holds its agent's channel keys. It
//! serves that agent's three channel operations: Receive, Publish and
//! Acknowledge. The supervisor relays each request to it, and the broker
//! reaches storage only through the daemon's per-operation callbacks
//! (`chief-of-staff-broker-callbacks`):
//!
//! ```text
//!   Publish "sunny"                     what each side holds
//!   ───────────────                     ────────────────────
//!   broker: LoadDefinition ──────────►  daemon: definitions, storage
//!   broker: grants missing? seal them,  broker: signing seed, channel key
//!           SaveGrants ──────────────►
//!   broker: hash("sunny")
//!           ReserveAppend(hash) ─────►  daemon: next sequence, message id,
//!                       ◄──── header           timestamp; header pending
//!   broker: check the header, refuse a
//!           sequence it already used,
//!           encrypt and sign
//!           CommitAppend(message) ───►  daemon: pending header? signature?
//!                       ◄──── sequence         store
//! ```
//!
//! It mirrors the in-daemon dispatcher it replaces, check for check and
//! failure code for failure code. Two things differ, both deliberately:
//! - **delivery receipts are per broker**, so one agent that never
//!   acknowledges can block only itself, not everyone (the old map was
//!   shared and capped across all agents);
//! - **the sequence guard**: the broker refuses to encrypt under a sequence
//!   at or below one it already used. That is nonce-reuse protection
//!   against a daemon that replays a header, for this broker's lifetime.
//!
//! ## Fatal and ordinary failures
//!
//! An ordinary failure, such as an unknown channel, a definition that
//! changed or a store that is busy, becomes the host's `Failed` response,
//! exactly as before. A [`BrokerError`] is something an honest peer never
//! causes: a broken frame, or a reply to the wrong callback. The broker
//! exits on one, and the supervisor ends its agent.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use chief_of_staff_broker_protocol::{
    definition_digest, Callback, CallbackOp, CallbackOutcome, CallbackReply, KeyKind, KeySlot,
    ProtocolError, PublicKey, Refusal, MAX_PAGE_MESSAGES,
};
use chief_of_staff_channel_crypto::{
    encrypt_message_with_header, plaintext_hash, seal_channel_key, ChannelId, ChannelMasterKey,
    MessageHeader, OriginatorSigningKey, ReceiverEpochKeys, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    open_delivered_message, ChannelDefinition, ChannelEndpointError, MessageId,
};
use chief_of_staff_host_control_protocol::{
    validate_data_plane_response, ChannelBindingAccess, DataPlaneFailure, DataPlaneMessage,
    DataPlaneRequest, DataPlaneResponse, RequestId,
};
use chief_of_staff_pipeline_bindings::HostPipelineBinding;
use coding_adventures_zeroize::Zeroizing;

pub use chief_of_staff_broker_protocol as protocol;

/// The most messages a broker holds a delivery receipt for at once, as the
/// old shared map did across every agent.
pub const MAX_PENDING_DELIVERIES: usize = 4096;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Something an honest peer never causes. The broker exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerError {
    /// A frame failed to read, write or decode.
    Protocol(ProtocolError),
    /// A callback result answered another callback.
    WrongReply,
    /// A frame arrived out of order (a second Bootstrap, a result with no
    /// callback outstanding).
    OutOfOrder,
}

impl From<ProtocolError> for BrokerError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// The inherited keys did not match the binding, or did not load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    /// A slot names a channel the binding does not bind.
    UnboundChannel,
    /// A slot's kind does not fit the channel's direction.
    WrongDirection,
    /// Two slots name the same key.
    Duplicate,
    /// A bound channel lacks a key it needs.
    Missing,
    /// A descriptor could not be read, or failed the owner-only policy.
    Unreadable,
    /// A key of all zeros, or one the curve refuses.
    InvalidKey,
}

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

struct OriginatorKeys {
    signing_key: OriginatorSigningKey,
    channel_key: ChannelMasterKey,
}

/// The agent's channel keys, and only those.
pub struct LoadedKeys {
    /// Receiver private keys, kept as bytes: each Receive builds its epoch
    /// keys from a fresh key pair, as the old endpoint did.
    receivers: BTreeMap<ChannelId, Zeroizing<[u8; 32]>>,
    originators: BTreeMap<ChannelId, OriginatorKeys>,
    public_keys: Vec<PublicKey>,
}

impl LoadedKeys {
    /// Check `slots` against the binding, then read each slot's key with
    /// `read(index)`.
    ///
    /// The rules the old shared authority enforced, now per broker:
    /// - every slot names a channel the binding binds, in a kind that fits
    ///   its direction: a read channel has exactly one receiver key, a write
    ///   channel exactly one signing seed and one channel key;
    /// - no key is all zeros;
    /// - a receiver key must be one the curve accepts.
    ///
    /// The slot table is checked whole before any key is read.
    pub fn load(
        binding: &HostPipelineBinding,
        slots: &[KeySlot],
        mut read: impl FnMut(usize) -> Result<Zeroizing<Vec<u8>>, KeyError>,
    ) -> Result<Self, KeyError> {
        let bound: BTreeMap<ChannelId, ChannelBindingAccess> = binding
            .launch_bindings()
            .channels()
            .iter()
            .map(|channel| (ChannelId(channel.channel_id()), channel.access()))
            .collect();
        let mut seen = BTreeSet::new();
        for slot in slots {
            let access = bound
                .get(&slot.channel_id)
                .ok_or(KeyError::UnboundChannel)?;
            let fits = match slot.kind {
                KeyKind::ReceiverPrivateKey => *access == ChannelBindingAccess::Read,
                KeyKind::OriginatorSigningSeed | KeyKind::ChannelMasterKey => {
                    *access == ChannelBindingAccess::Write
                }
            };
            if !fits {
                return Err(KeyError::WrongDirection);
            }
            if !seen.insert((slot.channel_id, slot.kind)) {
                return Err(KeyError::Duplicate);
            }
        }
        for (channel, access) in &bound {
            let needed: &[KeyKind] = match access {
                ChannelBindingAccess::Read => &[KeyKind::ReceiverPrivateKey],
                ChannelBindingAccess::Write => {
                    &[KeyKind::OriginatorSigningSeed, KeyKind::ChannelMasterKey]
                }
            };
            if needed.iter().any(|kind| !seen.contains(&(*channel, *kind))) {
                return Err(KeyError::Missing);
            }
        }

        let mut receivers = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let mut channel_keys = BTreeMap::new();
        for (index, slot) in slots.iter().enumerate() {
            let bytes = read(index)?;
            let mut key = Zeroizing::new([0u8; 32]);
            if bytes.len() != key.len() {
                return Err(KeyError::Unreadable);
            }
            key.copy_from_slice(&bytes);
            if key.iter().all(|byte| *byte == 0) {
                return Err(KeyError::InvalidKey);
            }
            match slot.kind {
                KeyKind::ReceiverPrivateKey => {
                    ReceiverKeyPair::from_private_key(*key).map_err(|_| KeyError::InvalidKey)?;
                    receivers.insert(slot.channel_id, key);
                }
                KeyKind::OriginatorSigningSeed => {
                    seeds.insert(slot.channel_id, key);
                }
                KeyKind::ChannelMasterKey => {
                    channel_keys.insert(slot.channel_id, key);
                }
            }
        }
        let mut originators = BTreeMap::new();
        for (channel, seed) in seeds {
            let channel_key = channel_keys.remove(&channel).ok_or(KeyError::Missing)?;
            originators.insert(
                channel,
                OriginatorKeys {
                    signing_key: OriginatorSigningKey::from_seed(*seed),
                    channel_key: ChannelMasterKey::from_bytes(*channel_key),
                },
            );
        }

        // Public halves in slot order, for Ready.
        let mut public_keys = Vec::new();
        for slot in slots {
            let public_key = match slot.kind {
                KeyKind::ReceiverPrivateKey => receiver_pair(&receivers[&slot.channel_id])
                    .map_err(|_| KeyError::InvalidKey)?
                    .public_key(),
                KeyKind::OriginatorSigningSeed => {
                    originators[&slot.channel_id].signing_key.public_key()
                }
                KeyKind::ChannelMasterKey => continue,
            };
            public_keys.push(PublicKey {
                channel_id: slot.channel_id,
                kind: slot.kind,
                public_key,
            });
        }
        Ok(Self {
            receivers,
            originators,
            public_keys,
        })
    }

    /// The public halves, in slot order, for the `Ready` frame.
    pub fn public_keys(&self) -> &[PublicKey] {
        &self.public_keys
    }
}

fn receiver_pair(private: &Zeroizing<[u8; 32]>) -> Result<ReceiverKeyPair, ()> {
    ReceiverKeyPair::from_private_key(**private).map_err(|_| ())
}

// ---------------------------------------------------------------------------
// Callbacks
// ---------------------------------------------------------------------------

/// How the broker reaches the daemon: one callback, one outcome.
pub trait ChannelCallbacks {
    /// Make `call` for request `request_id` and wait for its outcome.
    fn call(
        &mut self,
        request_id: RequestId,
        call: Callback,
    ) -> Result<CallbackOutcome, BrokerError>;
}

/// A callback's answer, or the host-facing failure its refusal maps to.
enum Answer {
    Reply(CallbackReply),
    Failed(DataPlaneFailure),
    /// The definition changed under the request; it is retried once.
    Changed,
}

fn ask(
    callbacks: &mut dyn ChannelCallbacks,
    request_id: RequestId,
    call: Callback,
) -> Result<Answer, BrokerError> {
    let op = call.op();
    let outcome = callbacks.call(request_id, call)?;
    if outcome.op() != op {
        return Err(BrokerError::WrongReply);
    }
    Ok(match outcome {
        CallbackOutcome::Ok(reply) => Answer::Reply(reply),
        CallbackOutcome::Refused { refusal, .. } => match refusal {
            Refusal::DefinitionChanged => Answer::Changed,
            Refusal::Unauthorized | Refusal::NotFound | Refusal::ChannelDestroyed => {
                Answer::Failed(DataPlaneFailure::Unauthorized)
            }
            Refusal::Unavailable => Answer::Failed(DataPlaneFailure::Unavailable),
            Refusal::PendingAppend
            | Refusal::Conflict
            | Refusal::Corrupt
            | Refusal::TooLarge
            | Refusal::InvalidRequest => Answer::Failed(DataPlaneFailure::Channel),
        },
    })
}

/// Unwrap an [`Answer`] inside an operation: a failure or a change ends
/// the attempt.
macro_rules! reply {
    ($answer:expr) => {
        match $answer? {
            Answer::Reply(reply) => reply,
            Answer::Failed(failure) => return Ok(Attempt::Done(Err(failure))),
            Answer::Changed => return Ok(Attempt::Changed),
        }
    };
}

enum Attempt {
    Done(Result<DataPlaneResponse, DataPlaneFailure>),
    Changed,
}

// ---------------------------------------------------------------------------
// The engine
// ---------------------------------------------------------------------------

/// Serves one agent's channel requests with its keys.
pub struct ChannelBroker {
    binding: HostPipelineBinding,
    keys: LoadedKeys,
    /// Delivered and not yet acknowledged: (channel, message id) to sequence.
    receipts: BTreeMap<(ChannelId, [u8; 16]), Sequence>,
    /// The highest sequence this broker has encrypted under, per channel.
    high_water: BTreeMap<ChannelId, Sequence>,
    /// Definitions whose receivers all hold a grant already.
    granted: BTreeSet<(ChannelId, [u8; 32])>,
}

impl ChannelBroker {
    /// A broker for `binding`'s agent, holding `keys`.
    pub fn new(binding: HostPipelineBinding, keys: LoadedKeys) -> Self {
        Self {
            binding,
            keys,
            receipts: BTreeMap::new(),
            high_water: BTreeMap::new(),
            granted: BTreeSet::new(),
        }
    }

    /// Serve one relayed request. `Err` is fatal; every ordinary failure is
    /// a `Failed` response.
    pub fn serve(
        &mut self,
        request: &DataPlaneRequest,
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<DataPlaneResponse, BrokerError> {
        let id = request.id();
        // A definition that changes mid-request is reloaded once, as the
        // old endpoint re-read it; a second change is a refusal.
        for _ in 0..2 {
            let attempt = match request {
                DataPlaneRequest::Receive {
                    channel_id, limit, ..
                } => self.receive(id, ChannelId(*channel_id), *limit, callbacks)?,
                DataPlaneRequest::Publish {
                    channel_id,
                    content_type,
                    payload,
                    ..
                } => self.publish(id, ChannelId(*channel_id), content_type, payload, callbacks)?,
                DataPlaneRequest::Acknowledge {
                    channel_id,
                    message_id,
                    ..
                } => self.acknowledge(id, ChannelId(*channel_id), *message_id, callbacks)?,
                _ => Attempt::Done(Err(DataPlaneFailure::Unavailable)),
            };
            if let Attempt::Done(result) = attempt {
                return Ok(
                    result.unwrap_or_else(|failure| DataPlaneResponse::Failed { id, failure })
                );
            }
        }
        Ok(DataPlaneResponse::Failed {
            id,
            failure: DataPlaneFailure::Unauthorized,
        })
    }

    /// Load the definition and check this agent's place in it.
    fn definition(
        &self,
        id: RequestId,
        channel: ChannelId,
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<Answer, BrokerError> {
        let answer = ask(
            callbacks,
            id,
            Callback::LoadDefinition {
                channel_id: channel,
            },
        )?;
        Ok(match answer {
            Answer::Reply(CallbackReply::Definition(definition)) => {
                if definition.channel_id() == channel {
                    Answer::Reply(CallbackReply::Definition(definition))
                } else {
                    return Err(BrokerError::WrongReply);
                }
            }
            Answer::Reply(_) => return Err(BrokerError::WrongReply),
            other => other,
        })
    }

    fn receive(
        &mut self,
        id: RequestId,
        channel: ChannelId,
        limit: u16,
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<Attempt, BrokerError> {
        let Some(private) = self.keys.receivers.get(&channel) else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
        };
        let Ok(key_pair) = receiver_pair(private) else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Internal)));
        };
        let CallbackReply::Definition(definition) = reply!(self.definition(id, channel, callbacks))
        else {
            return Err(BrokerError::WrongReply);
        };
        let agent = self.binding.agent_id();
        match definition.receiver(agent) {
            Some(receiver) if receiver.public_key == key_pair.public_key() => {}
            _ => return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized))),
        }
        let page_limit = limit.min(MAX_PAGE_MESSAGES as u16);
        let CallbackReply::ReceiverPage {
            messages, grants, ..
        } = reply!(ask(
            callbacks,
            id,
            Callback::ReadReceiverPage {
                channel_id: channel,
                definition_digest: definition_digest(&definition),
                limit: page_limit,
            }
        ))
        else {
            return Err(BrokerError::WrongReply);
        };
        if messages.len() > usize::from(page_limit) {
            return Err(BrokerError::WrongReply);
        }

        let mut epoch_keys = ReceiverEpochKeys::new(
            definition.originator().agent_id.as_bytes().to_vec(),
            agent.as_bytes().to_vec(),
            channel,
            key_pair,
            definition.originator().public_key,
        );
        let mut delivered = Vec::with_capacity(messages.len());
        for message in &messages {
            let opened =
                match open_delivered_message(&definition, &mut epoch_keys, message, |epoch| {
                    Ok(grants
                        .iter()
                        .find(|grant| {
                            grant.key_epoch == epoch && grant.receiver_id == agent.as_bytes()
                        })
                        .cloned())
                }) {
                    Ok(opened) => opened,
                    Err(error) => return Ok(Attempt::Done(Err(endpoint_failure(error)))),
                };
            delivered.push(opened);
        }

        // Receipts: the cross-message duplicate check, which
        // open_delivered_message leaves to its caller, and the cap.
        let new = delivered
            .iter()
            .filter(|message| {
                !self
                    .receipts
                    .contains_key(&(channel, *message.message_id.as_bytes()))
            })
            .count();
        if self.receipts.len() + new > MAX_PENDING_DELIVERIES {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unavailable)));
        }
        let mut out = Vec::with_capacity(delivered.len());
        for message in delivered {
            let key = (channel, *message.message_id.as_bytes());
            if self
                .receipts
                .insert(key, message.sequence)
                .is_some_and(|previous| previous != message.sequence)
            {
                return Ok(Attempt::Done(Err(DataPlaneFailure::Internal)));
            }
            out.push(DataPlaneMessage {
                message_id: *message.message_id.as_bytes(),
                sequence: message.sequence.0,
                timestamp_ns: message.timestamp_ns,
                content_type: message.content_type,
                payload: message.payload,
            });
        }
        let response = DataPlaneResponse::Received { id, messages: out };
        if validate_data_plane_response(&response).is_err() {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Channel)));
        }
        Ok(Attempt::Done(Ok(response)))
    }

    fn publish(
        &mut self,
        id: RequestId,
        channel: ChannelId,
        content_type: &str,
        payload: &[u8],
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<Attempt, BrokerError> {
        let Some(keys) = self.keys.originators.get(&channel) else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
        };
        let CallbackReply::Definition(definition) = reply!(self.definition(id, channel, callbacks))
        else {
            return Err(BrokerError::WrongReply);
        };
        let agent = self.binding.agent_id();
        if definition.originator().agent_id != *agent
            || definition.originator().public_key != keys.signing_key.public_key()
        {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
        }
        let digest = definition_digest(&definition);

        // Every receiver needs a grant at the current epoch before the
        // first publish; after that, this definition is known granted.
        if !self.granted.contains(&(channel, digest)) {
            let CallbackReply::MissingGrants {
                key_epoch,
                receivers,
            } = reply!(ask(
                callbacks,
                id,
                Callback::LoadMissingGrants {
                    channel_id: channel,
                    definition_digest: digest,
                }
            ))
            else {
                return Err(BrokerError::WrongReply);
            };
            if key_epoch != definition.key_epoch().0 {
                return Err(BrokerError::WrongReply);
            }
            let mut grants = Vec::with_capacity(receivers.len());
            for index in receivers {
                let receiver = definition
                    .receivers()
                    .get(usize::from(index))
                    .ok_or(BrokerError::WrongReply)?;
                match seal_channel_key(
                    agent.as_bytes(),
                    receiver.agent_id.as_bytes(),
                    channel,
                    definition.key_epoch(),
                    &keys.channel_key,
                    &receiver.public_key,
                    &keys.signing_key,
                ) {
                    Ok(grant) => grants.push(grant),
                    Err(_) => return Ok(Attempt::Done(Err(DataPlaneFailure::Channel))),
                }
            }
            if !grants.is_empty() {
                let CallbackReply::GrantsSaved = reply!(ask(
                    callbacks,
                    id,
                    Callback::SaveGrants {
                        channel_id: channel,
                        definition_digest: digest,
                        grants,
                    }
                )) else {
                    return Err(BrokerError::WrongReply);
                };
            }
            self.granted.insert((channel, digest));
        }

        let hash = plaintext_hash(payload);
        let CallbackReply::Reserved(header) = reply!(ask(
            callbacks,
            id,
            Callback::ReserveAppend {
                channel_id: channel,
                definition_digest: digest,
                content_type: content_type.to_owned(),
                plaintext_hash: hash,
            }
        )) else {
            return Err(BrokerError::WrongReply);
        };
        let sequence = header.fields().sequence();

        // The daemon chose the sequence, and the sequence is the nonce.
        // Before encrypting, check the header is the one asked for, and
        // that this broker has never encrypted under that sequence.
        let sound = header_is_sound(
            &header,
            channel,
            agent.as_bytes(),
            content_type,
            hash,
            &definition,
        ) && self
            .high_water
            .get(&channel)
            .is_none_or(|used| sequence > *used);
        if !sound {
            self.abandon(id, channel, sequence, callbacks)?;
            return Ok(Attempt::Done(Err(DataPlaneFailure::Internal)));
        }
        self.high_water.insert(channel, sequence);
        let message = match encrypt_message_with_header(
            header.clone(),
            payload,
            &keys.channel_key,
            &keys.signing_key,
        ) {
            Ok(message) => message,
            Err(_) => {
                self.abandon(id, channel, sequence, callbacks)?;
                return Ok(Attempt::Done(Err(DataPlaneFailure::Channel)));
            }
        };
        match ask(
            callbacks,
            id,
            Callback::CommitAppend {
                channel_id: channel,
                definition_digest: digest,
                message,
            },
        )? {
            Answer::Reply(CallbackReply::Committed {
                sequence: committed,
            }) if committed == sequence.0 => {}
            Answer::Reply(_) => return Err(BrokerError::WrongReply),
            // A refused commit leaves the reservation pending; give it back,
            // or every later publish on the channel would wait behind it.
            Answer::Failed(failure) => {
                self.abandon(id, channel, sequence, callbacks)?;
                return Ok(Attempt::Done(Err(failure)));
            }
            Answer::Changed => {
                self.abandon(id, channel, sequence, callbacks)?;
                return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
            }
        }
        Ok(Attempt::Done(Ok(DataPlaneResponse::Published {
            id,
            message_id: header.fields().message_id(),
            sequence: sequence.0,
            timestamp_ns: header.fields().timestamp_ns(),
        })))
    }

    /// Give a reservation back. A refusal here changes nothing for the host:
    /// the request has already failed.
    fn abandon(
        &self,
        id: RequestId,
        channel: ChannelId,
        sequence: Sequence,
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<(), BrokerError> {
        ask(
            callbacks,
            id,
            Callback::AbandonAppend {
                channel_id: channel,
                sequence: sequence.0,
            },
        )?;
        Ok(())
    }

    fn acknowledge(
        &mut self,
        id: RequestId,
        channel: ChannelId,
        message_id: [u8; 16],
        callbacks: &mut dyn ChannelCallbacks,
    ) -> Result<Attempt, BrokerError> {
        let Some(private) = self.keys.receivers.get(&channel) else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
        };
        let Ok(key_pair) = receiver_pair(private) else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Internal)));
        };
        if MessageId::from_uuid_v7(message_id).is_err() {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Channel)));
        }
        let CallbackReply::Definition(definition) = reply!(self.definition(id, channel, callbacks))
        else {
            return Err(BrokerError::WrongReply);
        };
        match definition.receiver(self.binding.agent_id()) {
            Some(receiver) if receiver.public_key == key_pair.public_key() => {}
            _ => return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized))),
        }
        let Some(sequence) = self.receipts.get(&(channel, message_id)).copied() else {
            return Ok(Attempt::Done(Err(DataPlaneFailure::Unauthorized)));
        };
        let first_unread = match ask(
            callbacks,
            id,
            Callback::Acknowledge {
                channel_id: channel,
                definition_digest: definition_digest(&definition),
                sequence: sequence.0,
            },
        )? {
            Answer::Reply(CallbackReply::Acknowledged { first_unread }) => first_unread,
            Answer::Reply(_) => return Err(BrokerError::WrongReply),
            Answer::Changed => return Ok(Attempt::Changed),
            // Any acknowledgement failure is a channel failure, as before.
            Answer::Failed(_) => return Ok(Attempt::Done(Err(DataPlaneFailure::Channel))),
        };
        self.receipts.retain(|(candidate, _), delivered| {
            *candidate != channel || delivered.0 >= first_unread
        });
        Ok(Attempt::Done(Ok(DataPlaneResponse::Acknowledged {
            id,
            sequence: first_unread,
        })))
    }
}

/// The reserved header must be exactly what the broker asked for: this
/// channel, this agent as originator, this content type and plaintext hash,
/// the definition's epoch, and a valid message id.
fn header_is_sound(
    header: &MessageHeader,
    channel: ChannelId,
    agent: &[u8],
    content_type: &str,
    hash: [u8; 32],
    definition: &ChannelDefinition,
) -> bool {
    let fields = header.fields();
    fields.channel_id() == channel
        && fields.originator_id() == agent
        && fields.content_type() == content_type
        && header.plaintext_hash() == hash
        && fields.key_epoch() == definition.key_epoch()
        && MessageId::from_uuid_v7(fields.message_id()).is_ok()
}

/// The old dispatcher's mapping, unchanged.
fn endpoint_failure(error: ChannelEndpointError) -> DataPlaneFailure {
    match error {
        ChannelEndpointError::DefinitionNotFound
        | ChannelEndpointError::DefinitionChanged
        | ChannelEndpointError::ChannelDestroyed
        | ChannelEndpointError::UnauthorizedOriginator
        | ChannelEndpointError::UnauthorizedReceiver
        | ChannelEndpointError::PublicKeyMismatch
        | ChannelEndpointError::UnauthorizedMessage => DataPlaneFailure::Unauthorized,
        ChannelEndpointError::Storage(_) | ChannelEndpointError::ConcurrentUpdate => {
            DataPlaneFailure::Unavailable
        }
        _ => DataPlaneFailure::Channel,
    }
}

// ---------------------------------------------------------------------------
// Callbacks over frames
// ---------------------------------------------------------------------------

/// [`ChannelCallbacks`] over the broker's stdin and stdout.
pub struct FramedCallbacks<R, W> {
    reader: R,
    writer: W,
    next_callback: u32,
}

impl<R: std::io::Read, W: std::io::Write> FramedCallbacks<R, W> {
    /// Callbacks read from `reader` and written to `writer`.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            next_callback: 0,
        }
    }

    /// The next frame from the supervisor.
    pub fn receive(&mut self) -> Result<protocol::ToBroker, BrokerError> {
        let body = protocol::read_frame(&mut self.reader)?;
        Ok(protocol::decode_to_broker(&body)?)
    }

    /// Send a frame to the supervisor.
    pub fn send(&mut self, frame: &protocol::FromBroker) -> Result<(), BrokerError> {
        let body = protocol::encode_from_broker(frame)?;
        Ok(protocol::write_frame(&mut self.writer, &body)?)
    }
}

impl<R: std::io::Read, W: std::io::Write> ChannelCallbacks for FramedCallbacks<R, W> {
    fn call(
        &mut self,
        request_id: RequestId,
        call: Callback,
    ) -> Result<CallbackOutcome, BrokerError> {
        self.next_callback = self.next_callback.wrapping_add(1).max(1);
        let callback_id = self.next_callback;
        self.send(&protocol::FromBroker::Callback {
            callback_id,
            request_id,
            call,
        })?;
        match self.receive()? {
            protocol::ToBroker::CallbackResult {
                callback_id: answered,
                outcome,
            } if answered == callback_id => Ok(outcome),
            protocol::ToBroker::CallbackResult { .. } => Err(BrokerError::WrongReply),
            _ => Err(BrokerError::OutOfOrder),
        }
    }
}

/// Which callbacks an operation may need, for documentation and tests.
pub fn callbacks_for(request: &DataPlaneRequest) -> &'static [CallbackOp] {
    match request {
        DataPlaneRequest::Receive { .. } => {
            &[CallbackOp::LoadDefinition, CallbackOp::ReadReceiverPage]
        }
        DataPlaneRequest::Acknowledge { .. } => {
            &[CallbackOp::LoadDefinition, CallbackOp::Acknowledge]
        }
        DataPlaneRequest::Publish { .. } => &[
            CallbackOp::LoadDefinition,
            CallbackOp::LoadMissingGrants,
            CallbackOp::SaveGrants,
            CallbackOp::ReserveAppend,
            CallbackOp::CommitAppend,
            CallbackOp::AbandonAppend,
        ],
        _ => &[],
    }
}
