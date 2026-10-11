//! # The broker protocol (D18S S-K7, step 6 P2.6d-1)
//!
//! One broker process serves one agent. It holds that agent's channel keys
//! and nothing else. The supervisor launched it, relays the agent's channel
//! requests to it, and answers the narrow storage callbacks it makes. This
//! crate is every byte that crosses between the two:
//!
//! ```text
//!   supervisor                                       broker (one agent's keys)
//!   ----------                                       ------
//!   Bootstrap { binding, key slots }  ───────────►   read the key descriptors
//!                                     ◄───────────   Ready { public halves }
//!   Request(Publish ...)              ───────────►
//!                                     ◄───────────   Callback(LoadDefinition)
//!   CallbackResult(Definition)        ───────────►
//!                                     ◄───────────   Callback(ReserveAppend)
//!   CallbackResult(Reserved header)   ───────────►   encrypt, sign
//!                                     ◄───────────   Callback(CommitAppend)
//!   CallbackResult(Committed)         ───────────►
//!                                     ◄───────────   Response(Published ...)
//!   Terminate                         ───────────►   exit 0
//! ```
//!
//! ## Framing
//!
//! A frame is a 4-byte big-endian length, then that many bytes, 1 byte to
//! 1 MiB: the same framing the supervisor already uses with hosts. Each
//! frame body starts with a 6-byte header:
//!
//! ```text
//!   ┌──────────┬─────────┬──────┬─────────────────────────────┐
//!   │ "D18K"   │ version │ kind │ fields of that kind ...      │
//!   │ 4 bytes  │ 1 = v1  │ 1    │                              │
//!   └──────────┴─────────┴──────┴─────────────────────────────┘
//! ```
//!
//! Kinds `0x01..=0x04` travel to the broker and `0x81..=0x83` from it, so a
//! frame sent the wrong way is refused by kind alone.
//!
//! ## Decoding is total
//!
//! The broker's peer is trusted, but the broker parses what its agent
//! caused, and a compromised broker is exactly what the supervisor must
//! survive. So both directions decode the same way:
//! - every count and length is checked against its bound *before* anything
//!   is allocated;
//! - every nested record (definition, message, header, grant, request,
//!   response, binding) is decoded by its own owning crate's total codec;
//! - trailing bytes, an unknown kind and a wrong version are all refused.
//!
//! ## What never crosses
//!
//! No frame carries an identity the broker chose. The callbacks name a
//! channel and nothing else. The daemon fills in which agent is asking from
//! the pipe the frame arrived on (S-K2).
//!
//! No secret key crosses either. The key slots name descriptors the broker
//! inherited, and `Ready` carries only public halves, so the supervisor can
//! check them against the channel definitions without holding a key.

#![forbid(unsafe_code)]

use std::fmt;
use std::io::{Read, Write};

use chief_of_staff_channel_crypto::wire::{
    decode_key_grant, decode_message, decode_message_header, encode_key_grant, encode_message,
    encode_message_header, MAX_CONTENT_TYPE_BYTES,
};
use chief_of_staff_channel_crypto::{
    ChannelId, EncryptedMessage, MessageHeader, SealedChannelKeyGrant,
};
use chief_of_staff_channel_endpoints::profile::{
    channel_definition_deserialize, channel_definition_serialize,
};
use chief_of_staff_channel_endpoints::ChannelDefinition;
use chief_of_staff_host_control_protocol::{
    decode_data_plane_request, decode_data_plane_response, encode_data_plane_request,
    encode_data_plane_response, DataPlaneRequest, DataPlaneResponse, RequestId,
    MAX_DATA_PLANE_RECORD_BYTES, MAX_LAUNCH_CHANNEL_BINDINGS,
};
use chief_of_staff_pipeline_bindings::{
    decode_host_binding, encode_host_binding, HostPipelineBinding,
};

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

/// The frame header's magic.
pub const MAGIC: [u8; 4] = *b"D18K";
/// The protocol version this crate speaks.
pub const VERSION: u8 = 1;
/// The largest frame body, in bytes.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
/// The largest encoded host binding a Bootstrap carries.
pub const MAX_BINDING_BYTES: usize = 32 * 1024;
/// At most one receiver key per read channel, and a signing seed plus a
/// master key per write channel.
pub const MAX_KEY_SLOTS: usize = 2 * MAX_LAUNCH_CHANNEL_BINDINGS;
/// Every key the broker inherits is exactly this long.
pub const KEY_BYTES: usize = 32;
/// The descriptor that key slot 0 is inherited on; slot `i` is at
/// `FIRST_KEY_DESCRIPTOR + i`.
pub const FIRST_KEY_DESCRIPTOR: i32 = 3;
/// The most messages one receiver page returns.
pub const MAX_PAGE_MESSAGES: usize = 64;
/// The most bytes one receiver page carries, messages and grants together:
/// whole records only, inside one frame with room for the frame's own
/// fields.
pub const MAX_PAGE_BYTES: usize = 960 * 1024;
/// The most encoded *message* bytes in one page. A message never decrypts
/// into a larger data-plane record than it was stored as, so a page within
/// this bound always fits the host's response.
pub const MAX_PAGE_MESSAGE_BYTES: usize = MAX_DATA_PLANE_RECORD_BYTES - 1024;
/// The most grants, or receiver indices, one callback carries: a channel
/// has at most this many receivers.
pub const MAX_GRANTS: usize = 1024;
/// The most callbacks one relayed request may make (D18S P2.6d).
pub const MAX_CALLBACKS_PER_REQUEST: u32 = 16;

const HEADER_BYTES: usize = 6;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A frame could not be read, written, or understood. Payload-blind: no
/// variant carries the bytes it refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// The stream ended, or failed, mid-frame.
    Io,
    /// A frame length outside 1 byte to [`MAX_FRAME_BYTES`].
    FrameLength,
    /// Wrong magic or version.
    Header,
    /// A kind this direction does not carry.
    UnknownKind,
    /// A field outside its bound, a truncated body, or trailing bytes.
    Malformed,
    /// A nested record (definition, message, request, ...) failed its own
    /// codec.
    NestedRecord,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io => "broker frame: stream failed",
            Self::FrameLength => "broker frame: length out of bounds",
            Self::Header => "broker frame: wrong magic or version",
            Self::UnknownKind => "broker frame: unknown kind",
            Self::Malformed => "broker frame: malformed",
            Self::NestedRecord => "broker frame: nested record malformed",
        })
    }
}

impl std::error::Error for ProtocolError {}

// ---------------------------------------------------------------------------
// Key slots
// ---------------------------------------------------------------------------

/// What one inherited key descriptor holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyKind {
    /// A read channel's X25519 receiver private key.
    ReceiverPrivateKey,
    /// A write channel's Ed25519 originator signing seed.
    OriginatorSigningSeed,
    /// A write channel's current-epoch channel master key.
    ChannelMasterKey,
}

impl KeyKind {
    fn to_byte(self) -> u8 {
        match self {
            Self::ReceiverPrivateKey => 1,
            Self::OriginatorSigningSeed => 2,
            Self::ChannelMasterKey => 3,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        match byte {
            1 => Ok(Self::ReceiverPrivateKey),
            2 => Ok(Self::OriginatorSigningSeed),
            3 => Ok(Self::ChannelMasterKey),
            _ => Err(ProtocolError::Malformed),
        }
    }
}

/// One inherited key descriptor: slot `i` in [`ToBroker::Bootstrap`] is on
/// descriptor `FIRST_KEY_DESCRIPTOR + i`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySlot {
    /// The channel the key belongs to.
    pub channel_id: ChannelId,
    /// Which key it is.
    pub kind: KeyKind,
}

/// The public half of one key the broker loaded, for the supervisor to check
/// against the channel definition. A channel master key has no public half,
/// so only receiver keys and signing seeds are reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey {
    /// The channel.
    pub channel_id: ChannelId,
    /// `ReceiverPrivateKey` (an X25519 public key) or
    /// `OriginatorSigningSeed` (an Ed25519 public key).
    pub kind: KeyKind,
    /// The public key.
    pub public_key: [u8; 32],
}

// ---------------------------------------------------------------------------
// Callbacks
// ---------------------------------------------------------------------------

/// Which callback, as a number on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallbackOp {
    /// Load the channel's definition.
    LoadDefinition,
    /// Read the next page for the bound receiver.
    ReadReceiverPage,
    /// Move the bound receiver's cursor.
    Acknowledge,
    /// Which receivers lack a grant at the current epoch.
    LoadMissingGrants,
    /// Store new grants (write-once).
    SaveGrants,
    /// Reserve the next sequence for a plaintext hash.
    ReserveAppend,
    /// Commit an encrypted message for the pending reservation.
    CommitAppend,
    /// Abandon the pending reservation.
    AbandonAppend,
}

impl CallbackOp {
    fn to_byte(self) -> u8 {
        match self {
            Self::LoadDefinition => 1,
            Self::ReadReceiverPage => 2,
            Self::Acknowledge => 3,
            Self::LoadMissingGrants => 4,
            Self::SaveGrants => 5,
            Self::ReserveAppend => 6,
            Self::CommitAppend => 7,
            Self::AbandonAppend => 8,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        Ok(match byte {
            1 => Self::LoadDefinition,
            2 => Self::ReadReceiverPage,
            3 => Self::Acknowledge,
            4 => Self::LoadMissingGrants,
            5 => Self::SaveGrants,
            6 => Self::ReserveAppend,
            7 => Self::CommitAppend,
            8 => Self::AbandonAppend,
            _ => return Err(ProtocolError::Malformed),
        })
    }
}

/// One storage callback the broker makes while serving a request.
///
/// None names who is asking: the daemon knows that from the pipe.
/// `definition_digest` is [`definition_digest`] of the definition the
/// broker is working from. A callback whose digest is no longer current is
/// refused with [`Refusal::DefinitionChanged`], so the broker never acts on a
/// stale membership.
///
/// `Debug` prints the operation and the channel only, never a message or a
/// grant.
#[derive(Clone, PartialEq, Eq)]
pub enum Callback {
    /// Load the channel's current definition.
    LoadDefinition { channel_id: ChannelId },
    /// Read up to `limit` messages after the bound receiver's cursor, with
    /// the receiver's grant for every epoch among them.
    ReadReceiverPage {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
        limit: u16,
    },
    /// Acknowledge through `sequence` for the bound receiver.
    Acknowledge {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
        sequence: u64,
    },
    /// Which receivers have no grant at the definition's epoch.
    LoadMissingGrants {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
    },
    /// Store grants the bound originator sealed.
    SaveGrants {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
        grants: Vec<SealedChannelKeyGrant>,
    },
    /// Reserve the next sequence for a plaintext with this hash. The daemon
    /// mints the message id and timestamp, and fills in the originator and
    /// epoch from the binding and the definition.
    ReserveAppend {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
        content_type: String,
        plaintext_hash: [u8; 32],
    },
    /// Commit the encrypted message for the pending reservation.
    CommitAppend {
        channel_id: ChannelId,
        definition_digest: [u8; 32],
        message: EncryptedMessage,
    },
    /// Abandon the pending reservation, if it is at `sequence`.
    AbandonAppend {
        channel_id: ChannelId,
        sequence: u64,
    },
}

impl Callback {
    /// Which callback this is.
    pub fn op(&self) -> CallbackOp {
        match self {
            Self::LoadDefinition { .. } => CallbackOp::LoadDefinition,
            Self::ReadReceiverPage { .. } => CallbackOp::ReadReceiverPage,
            Self::Acknowledge { .. } => CallbackOp::Acknowledge,
            Self::LoadMissingGrants { .. } => CallbackOp::LoadMissingGrants,
            Self::SaveGrants { .. } => CallbackOp::SaveGrants,
            Self::ReserveAppend { .. } => CallbackOp::ReserveAppend,
            Self::CommitAppend { .. } => CallbackOp::CommitAppend,
            Self::AbandonAppend { .. } => CallbackOp::AbandonAppend,
        }
    }

    /// The one channel the callback is about.
    pub fn channel_id(&self) -> ChannelId {
        match self {
            Self::LoadDefinition { channel_id }
            | Self::ReadReceiverPage { channel_id, .. }
            | Self::Acknowledge { channel_id, .. }
            | Self::LoadMissingGrants { channel_id, .. }
            | Self::SaveGrants { channel_id, .. }
            | Self::ReserveAppend { channel_id, .. }
            | Self::CommitAppend { channel_id, .. }
            | Self::AbandonAppend { channel_id, .. } => *channel_id,
        }
    }
}

/// A successful callback's answer. Each variant answers exactly one
/// [`CallbackOp`]; [`CallbackReply::op`] says which. `Debug` prints the
/// operation only.
#[derive(Clone, PartialEq, Eq)]
pub enum CallbackReply {
    /// For `LoadDefinition`.
    Definition(ChannelDefinition),
    /// For `ReadReceiverPage`: the receiver's next unread sequence, whole
    /// messages up to [`MAX_PAGE_BYTES`], and the receiver's grant for each
    /// distinct epoch among them.
    ReceiverPage {
        first_unread: u64,
        messages: Vec<EncryptedMessage>,
        grants: Vec<SealedChannelKeyGrant>,
    },
    /// For `Acknowledge`: the receiver's next unread sequence.
    Acknowledged { first_unread: u64 },
    /// For `LoadMissingGrants`: the definition's epoch, and the indices
    /// (in the definition's sorted receiver order) lacking a grant there.
    MissingGrants { key_epoch: u64, receivers: Vec<u16> },
    /// For `SaveGrants`.
    GrantsSaved,
    /// For `ReserveAppend`: the exact pending header.
    Reserved(MessageHeader),
    /// For `CommitAppend`: the committed sequence.
    Committed { sequence: u64 },
    /// For `AbandonAppend`: whether a reservation was abandoned.
    Abandoned { abandoned: bool },
}

impl CallbackReply {
    /// The callback this answers.
    pub fn op(&self) -> CallbackOp {
        match self {
            Self::Definition(_) => CallbackOp::LoadDefinition,
            Self::ReceiverPage { .. } => CallbackOp::ReadReceiverPage,
            Self::Acknowledged { .. } => CallbackOp::Acknowledge,
            Self::MissingGrants { .. } => CallbackOp::LoadMissingGrants,
            Self::GrantsSaved => CallbackOp::SaveGrants,
            Self::Reserved(_) => CallbackOp::ReserveAppend,
            Self::Committed { .. } => CallbackOp::CommitAppend,
            Self::Abandoned { .. } => CallbackOp::AbandonAppend,
        }
    }
}

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callback")
            .field("op", &self.op())
            .field("channel_id", &self.channel_id())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CallbackReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackReply")
            .field("op", &self.op())
            .finish_non_exhaustive()
    }
}

/// Why the daemon refused a callback. Payload-blind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Not a channel, direction or operation this broker may use now.
    Unauthorized,
    /// No such channel, grant or record.
    NotFound,
    /// The definition changed since the digest the broker sent.
    DefinitionChanged,
    /// The channel was destroyed.
    ChannelDestroyed,
    /// Another append is pending on the channel.
    PendingAppend,
    /// A write-once record already holds different bytes.
    Conflict,
    /// Storage is unavailable; the request may be retried.
    Unavailable,
    /// A stored record failed its codec.
    Corrupt,
    /// A single message is larger than a page may carry.
    TooLarge,
    /// A field failed the daemon's checks: a bad signature, a foreign
    /// originator, a header that is not the pending one.
    InvalidRequest,
}

impl Refusal {
    fn to_byte(self) -> u8 {
        match self {
            Self::Unauthorized => 1,
            Self::NotFound => 2,
            Self::DefinitionChanged => 3,
            Self::ChannelDestroyed => 4,
            Self::PendingAppend => 5,
            Self::Conflict => 6,
            Self::Unavailable => 7,
            Self::Corrupt => 8,
            Self::TooLarge => 9,
            Self::InvalidRequest => 10,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        Ok(match byte {
            1 => Self::Unauthorized,
            2 => Self::NotFound,
            3 => Self::DefinitionChanged,
            4 => Self::ChannelDestroyed,
            5 => Self::PendingAppend,
            6 => Self::Conflict,
            7 => Self::Unavailable,
            8 => Self::Corrupt,
            9 => Self::TooLarge,
            10 => Self::InvalidRequest,
            _ => return Err(ProtocolError::Malformed),
        })
    }
}

/// A callback's result: an answer, or a refusal. Either way it names the
/// callback it answers, so the broker can refuse a reply to the wrong one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallbackOutcome {
    /// The callback succeeded.
    Ok(CallbackReply),
    /// The daemon refused the callback `op`.
    Refused { op: CallbackOp, refusal: Refusal },
}

impl CallbackOutcome {
    /// The callback this answers.
    pub fn op(&self) -> CallbackOp {
        match self {
            Self::Ok(reply) => reply.op(),
            Self::Refused { op, .. } => *op,
        }
    }
}

/// SHA-256 of a definition's canonical D18C bytes: how a callback names
/// the exact definition the broker is working from.
pub fn definition_digest(definition: &ChannelDefinition) -> [u8; 32] {
    coding_adventures_sha256::sha256(&channel_definition_serialize(definition))
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// A frame from the supervisor to the broker.
// A frame lives for one send or one receive, so its size does not
// matter; boxing the large variants would only add allocations.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum ToBroker {
    /// The first frame, exactly once: whom the broker serves, and what each
    /// inherited key descriptor holds.
    Bootstrap {
        binding: HostPipelineBinding,
        slots: Vec<KeySlot>,
    },
    /// One host request the supervisor has decoded, checked and
    /// rate-limited.
    Request(DataPlaneRequest),
    /// The answer to the broker's outstanding callback.
    CallbackResult {
        callback_id: u32,
        outcome: CallbackOutcome,
    },
    /// Stop: the broker exits 0.
    Terminate,
}

/// A frame from the broker to the supervisor.
// A frame lives for one send or one receive, so its size does not
// matter; boxing the large variants would only add allocations.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum FromBroker {
    /// The keys loaded; their public halves, in slot order, skipping master
    /// keys.
    Ready { public_keys: Vec<PublicKey> },
    /// A storage callback for the request in flight.
    Callback {
        callback_id: u32,
        request_id: RequestId,
        call: Callback,
    },
    /// The answer to the request in flight.
    Response(DataPlaneResponse),
}

const BOOTSTRAP: u8 = 0x01;
const REQUEST: u8 = 0x02;
const CALLBACK_RESULT: u8 = 0x03;
const TERMINATE: u8 = 0x04;
const READY: u8 = 0x81;
const CALLBACK: u8 = 0x82;
const RESPONSE: u8 = 0x83;

/// Encode a frame body for the broker.
pub fn encode_to_broker(frame: &ToBroker) -> Result<Vec<u8>, ProtocolError> {
    let mut out = Encoder::new();
    match frame {
        ToBroker::Bootstrap { binding, slots } => {
            out.header(BOOTSTRAP);
            out.bytes32(&encode_host_binding(binding), MAX_BINDING_BYTES)?;
            out.count16(slots.len(), MAX_KEY_SLOTS)?;
            for slot in slots {
                out.fixed(&slot.channel_id.0);
                out.u8(slot.kind.to_byte());
            }
        }
        ToBroker::Request(request) => {
            out.header(REQUEST);
            let bytes =
                encode_data_plane_request(request).map_err(|_| ProtocolError::NestedRecord)?;
            out.bytes32(&bytes, MAX_FRAME_BYTES)?;
        }
        ToBroker::CallbackResult {
            callback_id,
            outcome,
        } => {
            out.header(CALLBACK_RESULT);
            out.u32(*callback_id);
            encode_outcome(&mut out, outcome)?;
        }
        ToBroker::Terminate => out.header(TERMINATE),
    }
    out.finish()
}

/// Decode a frame body sent to the broker.
pub fn decode_to_broker(body: &[u8]) -> Result<ToBroker, ProtocolError> {
    let (kind, mut input) = Decoder::open(body)?;
    let frame = match kind {
        BOOTSTRAP => {
            let binding = decode_host_binding(input.bytes32(MAX_BINDING_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?;
            let count = input.count16(MAX_KEY_SLOTS)?;
            let mut slots = Vec::with_capacity(count);
            for _ in 0..count {
                slots.push(KeySlot {
                    channel_id: ChannelId(input.fixed()?),
                    kind: KeyKind::from_byte(input.u8()?)?,
                });
            }
            ToBroker::Bootstrap { binding, slots }
        }
        REQUEST => ToBroker::Request(
            decode_data_plane_request(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        ),
        CALLBACK_RESULT => ToBroker::CallbackResult {
            callback_id: input.u32()?,
            outcome: decode_outcome(&mut input)?,
        },
        TERMINATE => ToBroker::Terminate,
        _ => return Err(ProtocolError::UnknownKind),
    };
    input.finish()?;
    Ok(frame)
}

/// Encode a frame body from the broker.
pub fn encode_from_broker(frame: &FromBroker) -> Result<Vec<u8>, ProtocolError> {
    let mut out = Encoder::new();
    match frame {
        FromBroker::Ready { public_keys } => {
            out.header(READY);
            out.count16(public_keys.len(), MAX_KEY_SLOTS)?;
            for key in public_keys {
                if key.kind == KeyKind::ChannelMasterKey {
                    return Err(ProtocolError::Malformed);
                }
                out.fixed(&key.channel_id.0);
                out.u8(key.kind.to_byte());
                out.fixed(&key.public_key);
            }
        }
        FromBroker::Callback {
            callback_id,
            request_id,
            call,
        } => {
            out.header(CALLBACK);
            out.u32(*callback_id);
            out.u64(request_id.get());
            encode_callback(&mut out, call)?;
        }
        FromBroker::Response(response) => {
            out.header(RESPONSE);
            let bytes =
                encode_data_plane_response(response).map_err(|_| ProtocolError::NestedRecord)?;
            out.bytes32(&bytes, MAX_FRAME_BYTES)?;
        }
    }
    out.finish()
}

/// Decode a frame body sent by the broker.
pub fn decode_from_broker(body: &[u8]) -> Result<FromBroker, ProtocolError> {
    let (kind, mut input) = Decoder::open(body)?;
    let frame = match kind {
        READY => {
            let count = input.count16(MAX_KEY_SLOTS)?;
            let mut public_keys = Vec::with_capacity(count);
            for _ in 0..count {
                let channel_id = ChannelId(input.fixed()?);
                let kind = KeyKind::from_byte(input.u8()?)?;
                if kind == KeyKind::ChannelMasterKey {
                    return Err(ProtocolError::Malformed);
                }
                public_keys.push(PublicKey {
                    channel_id,
                    kind,
                    public_key: input.fixed()?,
                });
            }
            FromBroker::Ready { public_keys }
        }
        CALLBACK => FromBroker::Callback {
            callback_id: input.u32()?,
            request_id: RequestId::new(input.u64()?).map_err(|_| ProtocolError::Malformed)?,
            call: decode_callback(&mut input)?,
        },
        RESPONSE => FromBroker::Response(
            decode_data_plane_response(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        ),
        _ => return Err(ProtocolError::UnknownKind),
    };
    input.finish()?;
    Ok(frame)
}

fn encode_callback(out: &mut Encoder, call: &Callback) -> Result<(), ProtocolError> {
    out.u8(call.op().to_byte());
    out.fixed(&call.channel_id().0);
    match call {
        Callback::LoadDefinition { .. } => {}
        Callback::ReadReceiverPage {
            definition_digest,
            limit,
            ..
        } => {
            out.fixed(definition_digest);
            if !(1..=MAX_PAGE_MESSAGES as u16).contains(limit) {
                return Err(ProtocolError::Malformed);
            }
            out.u16(*limit);
        }
        Callback::Acknowledge {
            definition_digest,
            sequence,
            ..
        } => {
            out.fixed(definition_digest);
            out.u64(*sequence);
        }
        Callback::LoadMissingGrants {
            definition_digest, ..
        } => out.fixed(definition_digest),
        Callback::SaveGrants {
            definition_digest,
            grants,
            ..
        } => {
            out.fixed(definition_digest);
            encode_grants(out, grants)?;
        }
        Callback::ReserveAppend {
            definition_digest,
            content_type,
            plaintext_hash,
            ..
        } => {
            out.fixed(definition_digest);
            if content_type.is_empty() {
                return Err(ProtocolError::Malformed);
            }
            out.bytes16(content_type.as_bytes(), MAX_CONTENT_TYPE_BYTES)?;
            out.fixed(plaintext_hash);
        }
        Callback::CommitAppend {
            definition_digest,
            message,
            ..
        } => {
            out.fixed(definition_digest);
            let bytes = encode_message(message).map_err(|_| ProtocolError::NestedRecord)?;
            out.bytes32(&bytes, MAX_FRAME_BYTES)?;
        }
        Callback::AbandonAppend { sequence, .. } => out.u64(*sequence),
    }
    Ok(())
}

fn decode_callback(input: &mut Decoder<'_>) -> Result<Callback, ProtocolError> {
    let op = CallbackOp::from_byte(input.u8()?)?;
    let channel_id = ChannelId(input.fixed()?);
    Ok(match op {
        CallbackOp::LoadDefinition => Callback::LoadDefinition { channel_id },
        CallbackOp::ReadReceiverPage => Callback::ReadReceiverPage {
            channel_id,
            definition_digest: input.fixed()?,
            limit: {
                let limit = input.u16()?;
                if !(1..=MAX_PAGE_MESSAGES as u16).contains(&limit) {
                    return Err(ProtocolError::Malformed);
                }
                limit
            },
        },
        CallbackOp::Acknowledge => Callback::Acknowledge {
            channel_id,
            definition_digest: input.fixed()?,
            sequence: input.u64()?,
        },
        CallbackOp::LoadMissingGrants => Callback::LoadMissingGrants {
            channel_id,
            definition_digest: input.fixed()?,
        },
        CallbackOp::SaveGrants => Callback::SaveGrants {
            channel_id,
            definition_digest: input.fixed()?,
            grants: decode_grants(input)?,
        },
        CallbackOp::ReserveAppend => Callback::ReserveAppend {
            channel_id,
            definition_digest: input.fixed()?,
            content_type: {
                let bytes = input.bytes16(MAX_CONTENT_TYPE_BYTES)?;
                if bytes.is_empty() {
                    return Err(ProtocolError::Malformed);
                }
                String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::Malformed)?
            },
            plaintext_hash: input.fixed()?,
        },
        CallbackOp::CommitAppend => Callback::CommitAppend {
            channel_id,
            definition_digest: input.fixed()?,
            message: decode_message(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        },
        CallbackOp::AbandonAppend => Callback::AbandonAppend {
            channel_id,
            sequence: input.u64()?,
        },
    })
}

const OUTCOME_OK: u8 = 0;
const OUTCOME_REFUSED: u8 = 1;

fn encode_outcome(out: &mut Encoder, outcome: &CallbackOutcome) -> Result<(), ProtocolError> {
    out.u8(outcome.op().to_byte());
    match outcome {
        CallbackOutcome::Refused { refusal, .. } => {
            out.u8(OUTCOME_REFUSED);
            out.u8(refusal.to_byte());
        }
        CallbackOutcome::Ok(reply) => {
            out.u8(OUTCOME_OK);
            match reply {
                CallbackReply::Definition(definition) => {
                    out.bytes32(&channel_definition_serialize(definition), MAX_FRAME_BYTES)?;
                }
                CallbackReply::ReceiverPage {
                    first_unread,
                    messages,
                    grants,
                } => {
                    out.u64(*first_unread);
                    out.count16(messages.len(), MAX_PAGE_MESSAGES)?;
                    let mut total = 0usize;
                    for message in messages {
                        let bytes =
                            encode_message(message).map_err(|_| ProtocolError::NestedRecord)?;
                        total += bytes.len();
                        if total > MAX_PAGE_MESSAGE_BYTES {
                            return Err(ProtocolError::Malformed);
                        }
                        out.bytes32(&bytes, MAX_PAGE_MESSAGE_BYTES)?;
                    }
                    if grants.len() > MAX_PAGE_MESSAGES {
                        return Err(ProtocolError::Malformed);
                    }
                    for grant in grants {
                        total += encode_key_grant(grant)
                            .map_err(|_| ProtocolError::NestedRecord)?
                            .len();
                    }
                    if total > MAX_PAGE_BYTES {
                        return Err(ProtocolError::Malformed);
                    }
                    encode_grants(out, grants)?;
                }
                CallbackReply::Acknowledged { first_unread } => out.u64(*first_unread),
                CallbackReply::MissingGrants {
                    key_epoch,
                    receivers,
                } => {
                    out.u64(*key_epoch);
                    out.count16(receivers.len(), MAX_GRANTS)?;
                    for receiver in receivers {
                        out.u16(*receiver);
                    }
                }
                CallbackReply::GrantsSaved => {}
                CallbackReply::Reserved(header) => {
                    let bytes =
                        encode_message_header(header).map_err(|_| ProtocolError::NestedRecord)?;
                    out.bytes32(&bytes, MAX_FRAME_BYTES)?;
                }
                CallbackReply::Committed { sequence } => out.u64(*sequence),
                CallbackReply::Abandoned { abandoned } => out.u8(u8::from(*abandoned)),
            }
        }
    }
    Ok(())
}

fn decode_outcome(input: &mut Decoder<'_>) -> Result<CallbackOutcome, ProtocolError> {
    let op = CallbackOp::from_byte(input.u8()?)?;
    match input.u8()? {
        OUTCOME_REFUSED => {
            return Ok(CallbackOutcome::Refused {
                op,
                refusal: Refusal::from_byte(input.u8()?)?,
            })
        }
        OUTCOME_OK => {}
        _ => return Err(ProtocolError::Malformed),
    }
    let reply = match op {
        CallbackOp::LoadDefinition => CallbackReply::Definition(
            channel_definition_deserialize(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        ),
        CallbackOp::ReadReceiverPage => {
            let first_unread = input.u64()?;
            let count = input.count16(MAX_PAGE_MESSAGES)?;
            let mut messages = Vec::with_capacity(count);
            let mut total = 0usize;
            for _ in 0..count {
                let bytes = input.bytes32(MAX_PAGE_MESSAGE_BYTES)?;
                total += bytes.len();
                if total > MAX_PAGE_MESSAGE_BYTES {
                    return Err(ProtocolError::Malformed);
                }
                messages.push(decode_message(bytes).map_err(|_| ProtocolError::NestedRecord)?);
            }
            let before = input.rest.len();
            let grants = decode_grants(input)?;
            if grants.len() > MAX_PAGE_MESSAGES {
                return Err(ProtocolError::Malformed);
            }
            // Grant bytes count against the page as well (their length
            // prefixes and count included, which only tightens it).
            if total + (before - input.rest.len()) > MAX_PAGE_BYTES {
                return Err(ProtocolError::Malformed);
            }
            CallbackReply::ReceiverPage {
                first_unread,
                messages,
                grants,
            }
        }
        CallbackOp::Acknowledge => CallbackReply::Acknowledged {
            first_unread: input.u64()?,
        },
        CallbackOp::LoadMissingGrants => {
            let key_epoch = input.u64()?;
            let count = input.count16(MAX_GRANTS)?;
            let mut receivers = Vec::with_capacity(count);
            for _ in 0..count {
                receivers.push(input.u16()?);
            }
            CallbackReply::MissingGrants {
                key_epoch,
                receivers,
            }
        }
        CallbackOp::SaveGrants => CallbackReply::GrantsSaved,
        CallbackOp::ReserveAppend => CallbackReply::Reserved(
            decode_message_header(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        ),
        CallbackOp::CommitAppend => CallbackReply::Committed {
            sequence: input.u64()?,
        },
        CallbackOp::AbandonAppend => CallbackReply::Abandoned {
            abandoned: match input.u8()? {
                0 => false,
                1 => true,
                _ => return Err(ProtocolError::Malformed),
            },
        },
    };
    Ok(CallbackOutcome::Ok(reply))
}

fn encode_grants(out: &mut Encoder, grants: &[SealedChannelKeyGrant]) -> Result<(), ProtocolError> {
    out.count16(grants.len(), MAX_GRANTS)?;
    for grant in grants {
        let bytes = encode_key_grant(grant).map_err(|_| ProtocolError::NestedRecord)?;
        out.bytes32(&bytes, MAX_FRAME_BYTES)?;
    }
    Ok(())
}

fn decode_grants(input: &mut Decoder<'_>) -> Result<Vec<SealedChannelKeyGrant>, ProtocolError> {
    let count = input.count16(MAX_GRANTS)?;
    let mut grants = Vec::with_capacity(count);
    for _ in 0..count {
        grants.push(
            decode_key_grant(input.bytes32(MAX_FRAME_BYTES)?)
                .map_err(|_| ProtocolError::NestedRecord)?,
        );
    }
    Ok(grants)
}

// ---------------------------------------------------------------------------
// Streams
// ---------------------------------------------------------------------------

/// Write one frame: a 4-byte big-endian length, then the body.
pub fn write_frame(writer: &mut impl Write, body: &[u8]) -> Result<(), ProtocolError> {
    if body.is_empty() || body.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameLength);
    }
    let length = u32::try_from(body.len()).map_err(|_| ProtocolError::FrameLength)?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(body))
        .and_then(|()| writer.flush())
        .map_err(|_| ProtocolError::Io)
}

/// Read one frame body. The length is checked before anything is
/// allocated, so a hostile length costs nothing.
pub fn read_frame(reader: &mut impl Read) -> Result<Vec<u8>, ProtocolError> {
    let mut length = [0u8; 4];
    reader
        .read_exact(&mut length)
        .map_err(|_| ProtocolError::Io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameLength);
    }
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|_| ProtocolError::Io)?;
    Ok(body)
}

// ---------------------------------------------------------------------------
// The bounded encoder and decoder
// ---------------------------------------------------------------------------

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn header(&mut self, kind: u8) {
        self.bytes.extend_from_slice(&MAGIC);
        self.bytes.push(VERSION);
        self.bytes.push(kind);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn fixed(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn count16(&mut self, count: usize, max: usize) -> Result<(), ProtocolError> {
        if count > max {
            return Err(ProtocolError::Malformed);
        }
        self.u16(u16::try_from(count).map_err(|_| ProtocolError::Malformed)?);
        Ok(())
    }

    fn bytes16(&mut self, bytes: &[u8], max: usize) -> Result<(), ProtocolError> {
        self.count16(bytes.len(), max)?;
        self.fixed(bytes);
        Ok(())
    }

    fn bytes32(&mut self, bytes: &[u8], max: usize) -> Result<(), ProtocolError> {
        if bytes.len() > max {
            return Err(ProtocolError::Malformed);
        }
        self.u32(u32::try_from(bytes.len()).map_err(|_| ProtocolError::Malformed)?);
        self.fixed(bytes);
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, ProtocolError> {
        if self.bytes.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameLength);
        }
        Ok(self.bytes)
    }
}

struct Decoder<'a> {
    rest: &'a [u8],
}

impl<'a> Decoder<'a> {
    /// Check the header; return the kind and a decoder for the fields.
    fn open(body: &'a [u8]) -> Result<(u8, Self), ProtocolError> {
        if body.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameLength);
        }
        if body.len() < HEADER_BYTES {
            return Err(ProtocolError::Malformed);
        }
        if body[..4] != MAGIC || body[4] != VERSION {
            return Err(ProtocolError::Header);
        }
        Ok((
            body[5],
            Self {
                rest: &body[HEADER_BYTES..],
            },
        ))
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        if length > self.rest.len() {
            return Err(ProtocolError::Malformed);
        }
        let (taken, rest) = self.rest.split_at(length);
        self.rest = rest;
        Ok(taken)
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], ProtocolError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(self.fixed()?))
    }

    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_be_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }

    fn count16(&mut self, max: usize) -> Result<usize, ProtocolError> {
        let count = usize::from(self.u16()?);
        if count > max {
            return Err(ProtocolError::Malformed);
        }
        Ok(count)
    }

    fn bytes16(&mut self, max: usize) -> Result<&'a [u8], ProtocolError> {
        let length = self.count16(max)?;
        self.take(length)
    }

    fn bytes32(&mut self, max: usize) -> Result<&'a [u8], ProtocolError> {
        let length = self.u32()? as usize;
        if length > max {
            return Err(ProtocolError::Malformed);
        }
        self.take(length)
    }

    fn finish(self) -> Result<(), ProtocolError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Malformed)
        }
    }
}
