//! Bounded authenticated host-channel bootstrap and records for D18 Chief.
//!
//! This crate is a pure byte-oriented protocol kernel. Supervisors provide an
//! unlocked orchestrator identity and carry the returned bytes over a transport;
//! the crate itself opens no files, streams, sockets, or processes.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use core::fmt::{self, Display, Formatter};

use coding_adventures_vault_secure_channel::{
    Channel, ChannelInitiator, ChannelResponder, FirstMessage,
};
use coding_adventures_x3dh::{
    create_prekey_bundle, generate_identity_keypair, generate_prekey_pair, IdentityKeyPair,
    PreKeyBundle, PreKeyPair,
};

const OFFER_MAGIC: &[u8; 4] = b"D18O";
const HELLO_MAGIC: &[u8; 4] = b"D18H";
const FRAME_MAGIC: &[u8; 4] = b"D18F";
const VERSION: u8 = 1;
const MAX_HOST_ID_BYTES: usize = 64;
const MAX_FIRST_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_FRAME_BYTES: usize = 1024 * 1024;
const FRAME_HEADER_BYTES: usize = 4 + 1 + 1 + 16 + 8 + 4;
const VAULT_NEXT_MESSAGE_OVERHEAD: usize = 2 + 40 + 4 + 16;
const MAX_VAULT_CIPHERTEXT_BYTES: usize = MAX_FRAME_BYTES - FRAME_HEADER_BYTES;
const MAX_PLAINTEXT_BYTES: usize = MAX_VAULT_CIPHERTEXT_BYTES - VAULT_NEXT_MESSAGE_OVERHEAD;
// The child consumes one inner ratchet counter during ClientHello, so close one
// record before the u32 limit. This is conservative for the orchestrator too.
const SEQUENCE_EXHAUSTION_LIMIT: u64 = (u32::MAX as u64) - 1;
const MAX_OFFER_BYTES: usize = 4 + 1 + 1 + MAX_HOST_ID_BYTES + 16 + 32 + 32 + 4 + 32 + 64;
const MAX_HELLO_BYTES: usize =
    4 + 1 + 1 + MAX_HOST_ID_BYTES + 16 + 32 + 4 + MAX_FIRST_MESSAGE_BYTES;
const FRAME_DOMAIN: &[u8] = b"chief-secure-host-channel-v1";
const BOOTSTRAP_DOMAIN: &[u8] = b"chief-secure-host-bootstrap-v1";
const BOOTSTRAP_PLAINTEXT: &[u8] = b"D18 secure host channel hello v1";

/// Stable lowercase host identifier bound into every encrypted message.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostId(String);

impl HostId {
    /// Validate the D18 host-name grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, ChannelError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() < 2
            || bytes.len() > MAX_HOST_ID_BYTES
            || !bytes[0].is_ascii_lowercase()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return Err(ChannelError::InvalidHostId);
        }
        Ok(Self(value))
    }

    /// Borrow the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for HostId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Canonical binary UUID-v7 session identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId([u8; 16]);

impl SessionId {
    /// Validate UUID-v7 version and RFC 4122 variant bits.
    pub fn new(bytes: [u8; 16]) -> Result<Self, ChannelError> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(ChannelError::InvalidSessionId);
        }
        Ok(Self(bytes))
    }

    /// Return the canonical binary UUID bytes.
    pub fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// The local endpoint's role in a host channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelRole {
    /// Parent orchestrator, which publishes the X3DH prekey bundle.
    Orchestrator,
    /// Spawned child host, which initiates X3DH.
    Child,
}

impl ChannelRole {
    fn wire_tag(self) -> u8 {
        match self {
            Self::Orchestrator => 1,
            Self::Child => 2,
        }
    }

    fn peer(self) -> Self {
        match self {
            Self::Orchestrator => Self::Child,
            Self::Child => Self::Orchestrator,
        }
    }
}

/// Bounded versioned orchestrator-to-child X3DH bootstrap bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapOffer(Vec<u8>);

impl BootstrapOffer {
    /// Validate and retain received offer bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChannelError> {
        decode_offer(bytes)?;
        Ok(Self(bytes.to_vec()))
    }

    /// Borrow the encoded offer for transport.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume the wrapper and return transport bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Bounded versioned child-to-orchestrator X3DH response bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientHello(Vec<u8>);

impl ClientHello {
    /// Validate and retain received hello bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChannelError> {
        decode_hello(bytes)?;
        Ok(Self(bytes.to_vec()))
    }

    /// Borrow the encoded hello for transport.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume the wrapper and return transport bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Orchestrator-side single-use X3DH bootstrap state.
pub struct OrchestratorBootstrap<'a> {
    identity: &'a IdentityKeyPair,
    signed_prekey: PreKeyPair,
    bundle: PreKeyBundle,
    host: HostId,
    session: SessionId,
}

impl<'a> OrchestratorBootstrap<'a> {
    /// Create a fresh per-spawn signed prekey around an unlocked identity.
    pub fn new(
        identity: &'a IdentityKeyPair,
        host: HostId,
        session: SessionId,
    ) -> Result<Self, ChannelError> {
        let signed_prekey = generate_prekey_pair();
        let session_prefix = [session.0[0], session.0[1], session.0[2], session.0[3]];
        let prekey_id = u32::from_be_bytes(session_prefix);
        let bundle = create_prekey_bundle(identity, &signed_prekey, prekey_id, None);
        Ok(Self {
            identity,
            signed_prekey,
            bundle,
            host,
            session,
        })
    }

    /// Encode the public bundle and channel identity for the child.
    pub fn offer(&self) -> Result<BootstrapOffer, ChannelError> {
        Ok(BootstrapOffer(encode_offer(
            &self.host,
            self.session,
            &self.bundle,
        )))
    }

    /// Authenticate a child hello and consume this single-use bootstrap.
    pub fn accept(self, hello: &ClientHello) -> Result<SecureHostChannel, ChannelError> {
        let decoded = decode_hello(hello.as_bytes())?;
        if decoded.host != self.host {
            return Err(ChannelError::WrongHost);
        }
        if decoded.session != self.session {
            return Err(ChannelError::WrongSession);
        }
        let aad = bootstrap_aad(&self.host, self.session);
        let first = FirstMessage(decoded.first_message.to_vec());
        let (channel, plaintext) = ChannelResponder::accept(
            &first,
            self.identity,
            &self.signed_prekey,
            None,
            &decoded.child_identity,
            &aad,
        )
        .map_err(|_| ChannelError::Crypto)?;
        if plaintext != BOOTSTRAP_PLAINTEXT {
            return Err(ChannelError::Crypto);
        }
        Ok(SecureHostChannel::new(
            channel,
            ChannelRole::Orchestrator,
            self.host,
            self.session,
        ))
    }
}

/// Child-side single-use bootstrap entry point.
pub struct ChildBootstrap;

impl ChildBootstrap {
    /// Authenticate an offer and return the live child channel plus its hello.
    pub fn open(offer: &BootstrapOffer) -> Result<(SecureHostChannel, ClientHello), ChannelError> {
        let decoded = decode_offer(offer.as_bytes())?;
        let child_identity = generate_identity_keypair();
        let aad = bootstrap_aad(&decoded.host, decoded.session);
        let (channel, first) =
            ChannelInitiator::open(&child_identity, &decoded.bundle, BOOTSTRAP_PLAINTEXT, &aad)
                .map_err(|_| ChannelError::Crypto)?;
        if first.0.len() > MAX_FIRST_MESSAGE_BYTES {
            return Err(ChannelError::FrameTooLarge);
        }
        let hello = ClientHello(encode_hello(
            &decoded.host,
            decoded.session,
            &child_identity.x25519_public,
            &first.0,
        ));
        Ok((
            SecureHostChannel::new(channel, ChannelRole::Child, decoded.host, decoded.session),
            hello,
        ))
    }
}

/// Live authenticated channel with exact per-direction stream sequencing.
pub struct SecureHostChannel {
    channel: Channel,
    role: ChannelRole,
    host: HostId,
    session: SessionId,
    send_sequence: u64,
    receive_sequence: u64,
    closed: bool,
}

impl SecureHostChannel {
    fn new(channel: Channel, role: ChannelRole, host: HostId, session: SessionId) -> Self {
        Self {
            channel,
            role,
            host,
            session,
            send_sequence: 0,
            receive_sequence: 0,
            closed: false,
        }
    }

    /// Encrypt one bounded plaintext and return a complete `D18F` record.
    pub fn send(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, ChannelError> {
        self.ensure_open()?;
        if plaintext.len() > MAX_PLAINTEXT_BYTES {
            return Err(ChannelError::FrameTooLarge);
        }
        if self.send_sequence == SEQUENCE_EXHAUSTION_LIMIT {
            self.closed = true;
            return Err(ChannelError::SequenceExhausted);
        }
        let aad = frame_aad(&self.host, self.session, self.role, self.send_sequence);
        let ciphertext = match self.channel.send(plaintext, &aad) {
            Ok(ciphertext) => ciphertext,
            Err(_) => {
                self.closed = true;
                return Err(ChannelError::Crypto);
            }
        };
        if ciphertext.len() > MAX_VAULT_CIPHERTEXT_BYTES {
            self.closed = true;
            return Err(ChannelError::FrameTooLarge);
        }
        let frame = encode_frame(self.role, self.session, self.send_sequence, &ciphertext);
        self.send_sequence += 1;
        Ok(frame)
    }

    /// Validate, authenticate, and decrypt the exact next peer `D18F` record.
    pub fn receive(&mut self, frame: &[u8]) -> Result<Vec<u8>, ChannelError> {
        self.ensure_open()?;
        if self.receive_sequence == SEQUENCE_EXHAUSTION_LIMIT {
            self.closed = true;
            return Err(ChannelError::SequenceExhausted);
        }
        let decoded = decode_frame(frame)?;
        if decoded.session != self.session {
            return Err(ChannelError::WrongSession);
        }
        let expected_role = self.role.peer();
        if decoded.direction != expected_role {
            return Err(ChannelError::WrongDirection);
        }
        if decoded.sequence != self.receive_sequence {
            return Err(ChannelError::UnexpectedSequence);
        }
        let aad = frame_aad(
            &self.host,
            self.session,
            expected_role,
            self.receive_sequence,
        );
        let plaintext = match self.channel.receive(decoded.ciphertext, &aad) {
            Ok(plaintext) => plaintext,
            Err(_) => {
                self.closed = true;
                return Err(ChannelError::Crypto);
            }
        };
        if plaintext.len() > MAX_PLAINTEXT_BYTES {
            self.closed = true;
            return Err(ChannelError::FrameTooLarge);
        }
        self.receive_sequence += 1;
        Ok(plaintext)
    }

    /// Return this endpoint's channel role.
    pub fn role(&self) -> ChannelRole {
        self.role
    }

    /// Borrow the host identity bound into AAD.
    pub fn host_id(&self) -> &HostId {
        &self.host
    }

    /// Return the UUID-v7 session identity bound into AAD.
    pub fn session_id(&self) -> SessionId {
        self.session
    }

    /// Report whether a cryptographic failure or sequence exhaustion closed the channel.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn ensure_open(&self) -> Result<(), ChannelError> {
        if self.closed {
            Err(ChannelError::Closed)
        } else {
            Ok(())
        }
    }
}

/// Bounded protocol failures with input-independent display text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelError {
    /// Host identifier did not satisfy the stable D18 grammar.
    InvalidHostId,
    /// Session bytes were not a canonical UUID-v7 value.
    InvalidSessionId,
    /// Offer or hello structure was malformed, truncated, or padded.
    MalformedBootstrap,
    /// Encrypted frame structure was malformed, truncated, or padded.
    MalformedFrame,
    /// A structurally valid record used an unsupported wire version.
    UnsupportedVersion,
    /// A declared or produced record exceeded its protocol bound.
    FrameTooLarge,
    /// Child hello identified a different host.
    WrongHost,
    /// Record identified a different session.
    WrongSession,
    /// Record was sent in the wrong channel direction.
    WrongDirection,
    /// Ordered stream record did not contain the exact next sequence.
    UnexpectedSequence,
    /// A sequence counter reached its non-wrapping terminal value.
    SequenceExhausted,
    /// X3DH, ratchet, signature, or AEAD authentication failed.
    Crypto,
    /// Channel was permanently closed after a terminal failure.
    Closed,
}

impl Display for ChannelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidHostId => "secure-host-channel: invalid host id",
            Self::InvalidSessionId => "secure-host-channel: invalid UUID-v7 session id",
            Self::MalformedBootstrap => "secure-host-channel: malformed bootstrap record",
            Self::MalformedFrame => "secure-host-channel: malformed encrypted frame",
            Self::UnsupportedVersion => "secure-host-channel: unsupported wire version",
            Self::FrameTooLarge => "secure-host-channel: record exceeds bounded frame size",
            Self::WrongHost => "secure-host-channel: bootstrap host mismatch",
            Self::WrongSession => "secure-host-channel: session mismatch",
            Self::WrongDirection => "secure-host-channel: direction mismatch",
            Self::UnexpectedSequence => "secure-host-channel: unexpected stream sequence",
            Self::SequenceExhausted => "secure-host-channel: sequence exhausted",
            Self::Crypto => "secure-host-channel: cryptographic operation failed",
            Self::Closed => "secure-host-channel: channel is closed",
        })
    }
}

impl std::error::Error for ChannelError {}

struct DecodedOffer {
    host: HostId,
    session: SessionId,
    bundle: PreKeyBundle,
}

fn encode_offer(host: &HostId, session: SessionId, bundle: &PreKeyBundle) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_OFFER_BYTES);
    output.extend_from_slice(OFFER_MAGIC);
    output.push(VERSION);
    encode_host_session(&mut output, host, session);
    output.extend_from_slice(&bundle.identity_key);
    output.extend_from_slice(&bundle.identity_key_sign);
    output.extend_from_slice(&bundle.signed_prekey_id.to_be_bytes());
    output.extend_from_slice(&bundle.signed_prekey);
    output.extend_from_slice(&bundle.signed_prekey_sig);
    output
}

fn decode_offer(bytes: &[u8]) -> Result<DecodedOffer, ChannelError> {
    if bytes.len() > MAX_OFFER_BYTES {
        return Err(ChannelError::FrameTooLarge);
    }
    let mut position = 0;
    expect_magic(
        bytes,
        &mut position,
        OFFER_MAGIC,
        ChannelError::MalformedBootstrap,
    )?;
    expect_version(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let (host, session) = decode_host_session(bytes, &mut position)?;
    let identity_key = take_array(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let identity_key_sign = take_array(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let signed_prekey_id = u32::from_be_bytes(take_array(
        bytes,
        &mut position,
        ChannelError::MalformedBootstrap,
    )?);
    let signed_prekey = take_array(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let signed_prekey_sig = take_array(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    ensure_end(bytes, position, ChannelError::MalformedBootstrap)?;
    Ok(DecodedOffer {
        host,
        session,
        bundle: PreKeyBundle {
            identity_key,
            identity_key_sign,
            signed_prekey_id,
            signed_prekey,
            signed_prekey_sig,
            one_time_prekey_id: None,
            one_time_prekey: None,
        },
    })
}

struct DecodedHello<'a> {
    host: HostId,
    session: SessionId,
    child_identity: [u8; 32],
    first_message: &'a [u8],
}

fn encode_hello(
    host: &HostId,
    session: SessionId,
    child_identity: &[u8; 32],
    first_message: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_HELLO_BYTES);
    output.extend_from_slice(HELLO_MAGIC);
    output.push(VERSION);
    encode_host_session(&mut output, host, session);
    output.extend_from_slice(child_identity);
    output.extend_from_slice(&(first_message.len() as u32).to_be_bytes());
    output.extend_from_slice(first_message);
    output
}

fn decode_hello(bytes: &[u8]) -> Result<DecodedHello<'_>, ChannelError> {
    if bytes.len() > MAX_HELLO_BYTES {
        return Err(ChannelError::FrameTooLarge);
    }
    let mut position = 0;
    expect_magic(
        bytes,
        &mut position,
        HELLO_MAGIC,
        ChannelError::MalformedBootstrap,
    )?;
    expect_version(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let (host, session) = decode_host_session(bytes, &mut position)?;
    let child_identity = take_array(bytes, &mut position, ChannelError::MalformedBootstrap)?;
    let declared = u32::from_be_bytes(take_array(
        bytes,
        &mut position,
        ChannelError::MalformedBootstrap,
    )?) as usize;
    if declared > MAX_FIRST_MESSAGE_BYTES {
        return Err(ChannelError::FrameTooLarge);
    }
    let first_message = take(
        bytes,
        &mut position,
        declared,
        ChannelError::MalformedBootstrap,
    )?;
    ensure_end(bytes, position, ChannelError::MalformedBootstrap)?;
    Ok(DecodedHello {
        host,
        session,
        child_identity,
        first_message,
    })
}

struct DecodedFrame<'a> {
    direction: ChannelRole,
    session: SessionId,
    sequence: u64,
    ciphertext: &'a [u8],
}

fn encode_frame(
    direction: ChannelRole,
    session: SessionId,
    sequence: u64,
    ciphertext: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(FRAME_HEADER_BYTES + ciphertext.len());
    output.extend_from_slice(FRAME_MAGIC);
    output.push(VERSION);
    output.push(direction.wire_tag());
    output.extend_from_slice(&session.0);
    output.extend_from_slice(&sequence.to_be_bytes());
    output.extend_from_slice(&(ciphertext.len() as u32).to_be_bytes());
    output.extend_from_slice(ciphertext);
    output
}

fn decode_frame(bytes: &[u8]) -> Result<DecodedFrame<'_>, ChannelError> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ChannelError::FrameTooLarge);
    }
    let mut position = 0;
    expect_magic(
        bytes,
        &mut position,
        FRAME_MAGIC,
        ChannelError::MalformedFrame,
    )?;
    expect_version(bytes, &mut position, ChannelError::MalformedFrame)?;
    let direction = match take(bytes, &mut position, 1, ChannelError::MalformedFrame)?[0] {
        1 => ChannelRole::Orchestrator,
        2 => ChannelRole::Child,
        _ => return Err(ChannelError::MalformedFrame),
    };
    let session = SessionId::new(take_array(
        bytes,
        &mut position,
        ChannelError::MalformedFrame,
    )?)?;
    let sequence = u64::from_be_bytes(take_array(
        bytes,
        &mut position,
        ChannelError::MalformedFrame,
    )?);
    let declared = u32::from_be_bytes(take_array(
        bytes,
        &mut position,
        ChannelError::MalformedFrame,
    )?) as usize;
    if declared > MAX_VAULT_CIPHERTEXT_BYTES {
        return Err(ChannelError::FrameTooLarge);
    }
    let ciphertext = take(bytes, &mut position, declared, ChannelError::MalformedFrame)?;
    ensure_end(bytes, position, ChannelError::MalformedFrame)?;
    Ok(DecodedFrame {
        direction,
        session,
        sequence,
        ciphertext,
    })
}

fn encode_host_session(output: &mut Vec<u8>, host: &HostId, session: SessionId) {
    output.push(host.0.len() as u8);
    output.extend_from_slice(host.0.as_bytes());
    output.extend_from_slice(&session.0);
}

fn decode_host_session(
    bytes: &[u8],
    position: &mut usize,
) -> Result<(HostId, SessionId), ChannelError> {
    let host_length = take(bytes, position, 1, ChannelError::MalformedBootstrap)?[0] as usize;
    if host_length > MAX_HOST_ID_BYTES {
        return Err(ChannelError::InvalidHostId);
    }
    let host_bytes = take(
        bytes,
        position,
        host_length,
        ChannelError::MalformedBootstrap,
    )?;
    let host_text = core::str::from_utf8(host_bytes).map_err(|_| ChannelError::InvalidHostId)?;
    let host = HostId::new(host_text)?;
    let session = SessionId::new(take_array(
        bytes,
        position,
        ChannelError::MalformedBootstrap,
    )?)?;
    Ok((host, session))
}

fn expect_magic(
    bytes: &[u8],
    position: &mut usize,
    expected: &[u8; 4],
    malformed: ChannelError,
) -> Result<(), ChannelError> {
    if take(bytes, position, 4, malformed)? != expected {
        return Err(malformed);
    }
    Ok(())
}

fn expect_version(
    bytes: &[u8],
    position: &mut usize,
    malformed: ChannelError,
) -> Result<(), ChannelError> {
    let version = take(bytes, position, 1, malformed)?[0];
    if version != VERSION {
        return Err(ChannelError::UnsupportedVersion);
    }
    Ok(())
}

fn take<'a>(
    bytes: &'a [u8],
    position: &mut usize,
    length: usize,
    malformed: ChannelError,
) -> Result<&'a [u8], ChannelError> {
    let end = position.checked_add(length).ok_or(malformed)?;
    let value = bytes.get(*position..end).ok_or(malformed)?;
    *position = end;
    Ok(value)
}

fn take_array<const N: usize>(
    bytes: &[u8],
    position: &mut usize,
    malformed: ChannelError,
) -> Result<[u8; N], ChannelError> {
    take(bytes, position, N, malformed)?
        .try_into()
        .map_err(|_| malformed)
}

fn ensure_end(bytes: &[u8], position: usize, malformed: ChannelError) -> Result<(), ChannelError> {
    if position == bytes.len() {
        Ok(())
    } else {
        Err(malformed)
    }
}

fn bootstrap_aad(host: &HostId, session: SessionId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(2 + BOOTSTRAP_DOMAIN.len() + 2 + host.0.len() + 16);
    append_length_prefixed(&mut aad, BOOTSTRAP_DOMAIN);
    append_length_prefixed(&mut aad, host.0.as_bytes());
    aad.extend_from_slice(&session.0);
    aad
}

fn frame_aad(host: &HostId, session: SessionId, direction: ChannelRole, sequence: u64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(2 + FRAME_DOMAIN.len() + 2 + host.0.len() + 16 + 1 + 8);
    append_length_prefixed(&mut aad, FRAME_DOMAIN);
    append_length_prefixed(&mut aad, host.0.as_bytes());
    aad.extend_from_slice(&session.0);
    aad.push(direction.wire_tag());
    aad.extend_from_slice(&sequence.to_be_bytes());
    aad
}

fn append_length_prefixed(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u16).to_be_bytes());
    output.extend_from_slice(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(value: &str) -> HostId {
        HostId::new(value).unwrap()
    }

    fn session(last: u8) -> SessionId {
        let mut bytes = [0u8; 16];
        bytes[0..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        SessionId::new(bytes).unwrap()
    }

    fn pending_handshake<'a>(
        identity: &'a IdentityKeyPair,
        host_id: &str,
        session_id: SessionId,
    ) -> (OrchestratorBootstrap<'a>, SecureHostChannel, ClientHello) {
        let bootstrap = OrchestratorBootstrap::new(identity, host(host_id), session_id).unwrap();
        let offer = bootstrap.offer().unwrap();
        let (child, hello) = ChildBootstrap::open(&offer).unwrap();
        (bootstrap, child, hello)
    }

    fn channel_pair() -> (SecureHostChannel, SecureHostChannel) {
        let identity = generate_identity_keypair();
        let (bootstrap, child, hello) = pending_handshake(&identity, "host-a", session(1));
        let orchestrator = bootstrap.accept(&hello).unwrap();
        (orchestrator, child)
    }

    #[test]
    fn validates_host_and_session_identifiers() {
        assert_eq!(host("host-7").as_str(), "host-7");
        assert_eq!(host("host-7").to_string(), "host-7");
        for invalid in ["", "a", "7host", "Host", "host_name"] {
            assert_eq!(HostId::new(invalid), Err(ChannelError::InvalidHostId));
        }
        assert_eq!(
            HostId::new("a".repeat(MAX_HOST_ID_BYTES + 1)),
            Err(ChannelError::InvalidHostId)
        );
        let id = session(9);
        assert_eq!(id.as_bytes()[15], 9);
        let mut bad_version = id.as_bytes();
        bad_version[6] = 0x60;
        assert_eq!(
            SessionId::new(bad_version),
            Err(ChannelError::InvalidSessionId)
        );
        let mut bad_variant = id.as_bytes();
        bad_variant[8] = 0x40;
        assert_eq!(
            SessionId::new(bad_variant),
            Err(ChannelError::InvalidSessionId)
        );
    }

    #[test]
    fn offer_codec_is_bounded_and_strict() {
        let identity = generate_identity_keypair();
        let bootstrap = OrchestratorBootstrap::new(&identity, host("host-a"), session(1)).unwrap();
        let offer = bootstrap.offer().unwrap();
        let parsed = BootstrapOffer::from_bytes(offer.as_bytes()).unwrap();
        assert_eq!(parsed.as_bytes(), offer.as_bytes());
        assert_eq!(parsed.clone().into_bytes(), offer.as_bytes());
        for end in 0..offer.as_bytes().len() {
            assert!(BootstrapOffer::from_bytes(&offer.as_bytes()[..end]).is_err());
        }
        let mut trailing = offer.as_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            BootstrapOffer::from_bytes(&trailing),
            Err(ChannelError::MalformedBootstrap)
        );
        let mut wrong_version = offer.as_bytes().to_vec();
        wrong_version[4] = 2;
        assert_eq!(
            BootstrapOffer::from_bytes(&wrong_version),
            Err(ChannelError::UnsupportedVersion)
        );
        assert_eq!(
            BootstrapOffer::from_bytes(&vec![0; MAX_OFFER_BYTES + 1]),
            Err(ChannelError::FrameTooLarge)
        );
    }

    #[test]
    fn offer_rejects_invalid_host_and_session_fields() {
        let identity = generate_identity_keypair();
        let bootstrap = OrchestratorBootstrap::new(&identity, host("host-a"), session(1)).unwrap();
        let offer = bootstrap.offer().unwrap();
        let mut invalid_utf8 = offer.as_bytes().to_vec();
        invalid_utf8[6] = 0xff;
        assert_eq!(
            BootstrapOffer::from_bytes(&invalid_utf8),
            Err(ChannelError::InvalidHostId)
        );
        let mut invalid_session = offer.as_bytes().to_vec();
        let session_start = 6 + "host-a".len();
        invalid_session[session_start + 6] = 0x60;
        assert_eq!(
            BootstrapOffer::from_bytes(&invalid_session),
            Err(ChannelError::InvalidSessionId)
        );
        let mut zero_host = offer.as_bytes().to_vec();
        zero_host[5] = 0;
        assert_eq!(
            BootstrapOffer::from_bytes(&zero_host),
            Err(ChannelError::InvalidHostId)
        );
    }

    #[test]
    fn tampered_bundle_signature_fails_child_bootstrap() {
        let identity = generate_identity_keypair();
        let bootstrap = OrchestratorBootstrap::new(&identity, host("host-a"), session(1)).unwrap();
        let offer = bootstrap.offer().unwrap();
        let mut bytes = offer.into_bytes();
        *bytes.last_mut().unwrap() ^= 1;
        let tampered = BootstrapOffer::from_bytes(&bytes).unwrap();
        assert!(matches!(
            ChildBootstrap::open(&tampered),
            Err(ChannelError::Crypto)
        ));
    }

    #[test]
    fn hello_codec_is_bounded_and_strict() {
        let identity = generate_identity_keypair();
        let (_, _, hello) = pending_handshake(&identity, "host-a", session(1));
        let parsed = ClientHello::from_bytes(hello.as_bytes()).unwrap();
        assert_eq!(parsed.as_bytes(), hello.as_bytes());
        assert_eq!(parsed.clone().into_bytes(), hello.as_bytes());
        for end in 0..hello.as_bytes().len() {
            assert!(ClientHello::from_bytes(&hello.as_bytes()[..end]).is_err());
        }
        let mut trailing = hello.as_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            ClientHello::from_bytes(&trailing),
            Err(ChannelError::MalformedBootstrap)
        );
        let mut too_large = hello.as_bytes().to_vec();
        let length_offset = 4 + 1 + 1 + "host-a".len() + 16 + 32;
        too_large[length_offset..length_offset + 4]
            .copy_from_slice(&((MAX_FIRST_MESSAGE_BYTES as u32) + 1).to_be_bytes());
        assert_eq!(
            ClientHello::from_bytes(&too_large),
            Err(ChannelError::FrameTooLarge)
        );
        assert_eq!(
            ClientHello::from_bytes(&vec![0; MAX_HELLO_BYTES + 1]),
            Err(ChannelError::FrameTooLarge)
        );
    }

    #[test]
    fn handshake_binds_host_and_session() {
        let identity = generate_identity_keypair();
        let (bootstrap, _, hello) = pending_handshake(&identity, "host-a", session(1));
        let mut wrong_host = hello.into_bytes();
        let host_start = 6;
        wrong_host[host_start..host_start + 6].copy_from_slice(b"host-b");
        let wrong_host = ClientHello::from_bytes(&wrong_host).unwrap();
        assert!(matches!(
            bootstrap.accept(&wrong_host),
            Err(ChannelError::WrongHost)
        ));

        let (bootstrap, _, hello) = pending_handshake(&identity, "host-a", session(1));
        let mut wrong_session = hello.into_bytes();
        let session_start = 6 + "host-a".len();
        wrong_session[session_start + 15] ^= 1;
        let wrong_session = ClientHello::from_bytes(&wrong_session).unwrap();
        assert!(matches!(
            bootstrap.accept(&wrong_session),
            Err(ChannelError::WrongSession)
        ));
    }

    #[test]
    fn bidirectional_messages_round_trip_with_identity_accessors() {
        let (mut orchestrator, mut child) = channel_pair();
        assert_eq!(orchestrator.role(), ChannelRole::Orchestrator);
        assert_eq!(child.role(), ChannelRole::Child);
        assert_eq!(child.host_id(), &host("host-a"));
        assert_eq!(child.session_id(), session(1));
        assert!(!child.is_closed());
        for index in 0..64u8 {
            let child_message = vec![index; usize::from(index) + 1];
            let frame = child.send(&child_message).unwrap();
            assert_eq!(orchestrator.receive(&frame).unwrap(), child_message);
            let response = vec![255 - index; usize::from(index) + 2];
            let frame = orchestrator.send(&response).unwrap();
            assert_eq!(child.receive(&frame).unwrap(), response);
        }
    }

    #[test]
    fn every_truncated_frame_and_trailing_byte_is_rejected() {
        let (mut orchestrator, mut child) = channel_pair();
        let frame = child.send(b"complete").unwrap();
        for end in 0..frame.len() {
            assert!(decode_frame(&frame[..end]).is_err());
        }
        let mut trailing = frame.clone();
        trailing.push(0);
        assert_eq!(
            orchestrator.receive(&trailing),
            Err(ChannelError::MalformedFrame)
        );
        assert_eq!(orchestrator.receive(&frame).unwrap(), b"complete");
    }

    #[test]
    fn structural_rejections_do_not_advance_receive_state() {
        let (mut orchestrator, mut child) = channel_pair();
        let frame = child.send(b"expected").unwrap();

        let mut wrong_direction = frame.clone();
        wrong_direction[5] = ChannelRole::Orchestrator.wire_tag();
        assert_eq!(
            orchestrator.receive(&wrong_direction),
            Err(ChannelError::WrongDirection)
        );

        let mut wrong_session = frame.clone();
        wrong_session[6 + 15] ^= 1;
        assert_eq!(
            orchestrator.receive(&wrong_session),
            Err(ChannelError::WrongSession)
        );

        let mut wrong_sequence = frame.clone();
        wrong_sequence[22..30].copy_from_slice(&7u64.to_be_bytes());
        assert_eq!(
            orchestrator.receive(&wrong_sequence),
            Err(ChannelError::UnexpectedSequence)
        );
        assert_eq!(orchestrator.receive(&frame).unwrap(), b"expected");
        assert_eq!(
            orchestrator.receive(&frame),
            Err(ChannelError::UnexpectedSequence)
        );
    }

    #[test]
    fn malformed_frame_fields_are_rejected_before_decryption() {
        let (mut orchestrator, mut child) = channel_pair();
        let frame = child.send(b"expected").unwrap();

        let mut wrong_magic = frame.clone();
        wrong_magic[0] = b'X';
        assert_eq!(
            orchestrator.receive(&wrong_magic),
            Err(ChannelError::MalformedFrame)
        );
        let mut wrong_version = frame.clone();
        wrong_version[4] = 2;
        assert_eq!(
            orchestrator.receive(&wrong_version),
            Err(ChannelError::UnsupportedVersion)
        );
        let mut bad_direction = frame.clone();
        bad_direction[5] = 7;
        assert_eq!(
            orchestrator.receive(&bad_direction),
            Err(ChannelError::MalformedFrame)
        );
        let mut invalid_session = frame.clone();
        invalid_session[6 + 6] = 0x60;
        assert_eq!(
            orchestrator.receive(&invalid_session),
            Err(ChannelError::InvalidSessionId)
        );
        let mut oversized = frame.clone();
        oversized[30..34].copy_from_slice(&((MAX_VAULT_CIPHERTEXT_BYTES as u32) + 1).to_be_bytes());
        assert_eq!(
            orchestrator.receive(&oversized),
            Err(ChannelError::FrameTooLarge)
        );
        assert_eq!(
            orchestrator.receive(&vec![0; MAX_FRAME_BYTES + 1]),
            Err(ChannelError::FrameTooLarge)
        );
        assert_eq!(orchestrator.receive(&frame).unwrap(), b"expected");
    }

    #[test]
    fn authentication_failure_permanently_closes_channel() {
        let (mut orchestrator, mut child) = channel_pair();
        let mut frame = child.send(b"authentic").unwrap();
        *frame.last_mut().unwrap() ^= 1;
        assert_eq!(orchestrator.receive(&frame), Err(ChannelError::Crypto));
        assert!(orchestrator.is_closed());
        assert_eq!(orchestrator.send(b"after"), Err(ChannelError::Closed));
        assert_eq!(orchestrator.receive(&frame), Err(ChannelError::Closed));
    }

    #[test]
    fn host_is_bound_into_frame_aad() {
        let (mut orchestrator, mut child) = channel_pair();
        let frame = child.send(b"bound").unwrap();
        orchestrator.host = host("host-b");
        assert_eq!(orchestrator.receive(&frame), Err(ChannelError::Crypto));
    }

    #[test]
    fn plaintext_and_sequence_bounds_fail_closed_as_specified() {
        let (mut orchestrator, mut child) = channel_pair();
        assert_eq!(
            child.send(&vec![0; MAX_PLAINTEXT_BYTES + 1]),
            Err(ChannelError::FrameTooLarge)
        );
        assert!(!child.is_closed());
        child.send_sequence = SEQUENCE_EXHAUSTION_LIMIT;
        assert_eq!(child.send(b"never"), Err(ChannelError::SequenceExhausted));
        assert!(child.is_closed());

        orchestrator.receive_sequence = SEQUENCE_EXHAUSTION_LIMIT;
        assert_eq!(
            orchestrator.receive(b"never"),
            Err(ChannelError::SequenceExhausted)
        );
        assert!(orchestrator.is_closed());
    }

    #[test]
    fn error_messages_are_literal_and_safe() {
        let errors = [
            ChannelError::InvalidHostId,
            ChannelError::InvalidSessionId,
            ChannelError::MalformedBootstrap,
            ChannelError::MalformedFrame,
            ChannelError::UnsupportedVersion,
            ChannelError::FrameTooLarge,
            ChannelError::WrongHost,
            ChannelError::WrongSession,
            ChannelError::WrongDirection,
            ChannelError::UnexpectedSequence,
            ChannelError::SequenceExhausted,
            ChannelError::Crypto,
            ChannelError::Closed,
        ];
        for error in errors {
            assert!(error.to_string().starts_with("secure-host-channel:"));
            assert!(std::error::Error::source(&error).is_none());
        }
    }
}
