//! # Allocation-free TLS 1.3 handshake-message framing
//!
//! A TLS handshake message is a one-byte type, a three-byte network-order body
//! length, and that many body bytes.  The complete representation is later
//! hashed into the handshake transcript, so decoding returns both the body and
//! the exact encoded bytes.  This crate does not reassemble messages across
//! records, interpret bodies, or own protocol state.
//!
//! Recognition and authorization are deliberately separate.  The enum lists
//! the core RFC 8446 wire message types, while each caller constructs an
//! explicit policy for the types and body size appropriate to its current
//! state.  Synthetic transcript-only `message_hash` is not a wire type here.

#![forbid(unsafe_code)]

use core::fmt;

/// Bytes in the TLS handshake-message header.
pub const TLS13_HANDSHAKE_HEADER_LEN: usize = 4;

/// Largest body representable by RFC 8446's `uint24` length.
pub const TLS13_MAX_HANDSHAKE_BODY_LEN: usize = 0x00ff_ffff;

/// A recognized core TLS 1.3 wire handshake-message type.
///
/// The synthetic `message_hash(254)` transcript value is intentionally absent:
/// RFC 8446 inserts it into a transcript for HelloRetryRequest processing, but
/// it is never received or emitted as a handshake wire message.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Tls13HandshakeType {
    /// Initial client parameters and key shares.
    ClientHello = 1,
    /// Server selection, including the HelloRetryRequest encoding.
    ServerHello = 2,
    /// Post-handshake resumption ticket.
    NewSessionTicket = 4,
    /// Zero-length marker ending accepted early data.
    EndOfEarlyData = 5,
    /// Encrypted server extensions.
    EncryptedExtensions = 8,
    /// Certificate chain message.
    Certificate = 11,
    /// Certificate-request parameters.
    CertificateRequest = 13,
    /// Certificate-key proof over the transcript.
    CertificateVerify = 15,
    /// Finished verification data.
    Finished = 20,
    /// Post-handshake traffic-secret update request.
    KeyUpdate = 24,
}

impl Tls13HandshakeType {
    /// Return the one-byte RFC 8446 wire value.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    const fn policy_bit(self) -> u16 {
        match self {
            Self::ClientHello => 1 << 0,
            Self::ServerHello => 1 << 1,
            Self::NewSessionTicket => 1 << 2,
            Self::EndOfEarlyData => 1 << 3,
            Self::EncryptedExtensions => 1 << 4,
            Self::Certificate => 1 << 5,
            Self::CertificateRequest => 1 << 6,
            Self::CertificateVerify => 1 << 7,
            Self::Finished => 1 << 8,
            Self::KeyUpdate => 1 << 9,
        }
    }

    fn from_wire(value: u8) -> Result<Self, HandshakeError> {
        match value {
            1 => Ok(Self::ClientHello),
            2 => Ok(Self::ServerHello),
            4 => Ok(Self::NewSessionTicket),
            5 => Ok(Self::EndOfEarlyData),
            8 => Ok(Self::EncryptedExtensions),
            11 => Ok(Self::Certificate),
            13 => Ok(Self::CertificateRequest),
            15 => Ok(Self::CertificateVerify),
            20 => Ok(Self::Finished),
            24 => Ok(Self::KeyUpdate),
            _ => Err(HandshakeError::new(
                HandshakeErrorKind::UnknownHandshakeType,
            )),
        }
    }
}

/// Stable, payload-free failure categories for policy, decoding, and encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HandshakeErrorKind {
    /// A policy allowed no handshake-message type.
    EmptyHandshakeTypePolicy,
    /// A policy repeated one handshake-message type.
    DuplicateHandshakeType,
    /// A caller body ceiling exceeded the RFC 8446 `uint24` maximum.
    BodyLimitOutOfRange,
    /// Fewer than four bytes were available for a message header.
    TruncatedHeader,
    /// The type byte is not a core TLS 1.3 wire handshake type.
    UnknownHandshakeType,
    /// The recognized type was not allowed by caller policy.
    DisallowedHandshakeType,
    /// The declared or supplied body exceeded caller policy.
    BodyTooLong,
    /// The input ended before the complete declared body.
    TruncatedBody,
    /// Exact decoding found bytes after the declared message.
    TrailingData,
    /// Caller-provided encoding storage could not hold the complete message.
    OutputTooSmall,
}

/// One redacted TLS handshake-framing failure.
///
/// The value retains only a stable category.  It never stores a wire type,
/// declared length, message body, transcript byte, or caller output buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandshakeError {
    kind: HandshakeErrorKind,
}

impl HandshakeError {
    const fn new(kind: HandshakeErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> HandshakeErrorKind {
        self.kind
    }
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 handshake framing error {:?}", self.kind)
    }
}

impl std::error::Error for HandshakeError {}

/// Caller-owned message recognition and body-work policy.
///
/// Construction rejects empty and duplicate allowlists.  A zero body ceiling
/// is valid because `EndOfEarlyData` has an empty body; body-specific rules for
/// every other type belong to later parsers and the handshake state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tls13HandshakePolicy {
    allowed_handshake_types: u16,
    maximum_body_length: usize,
}

impl Tls13HandshakePolicy {
    /// Validate one explicit handshake-message policy.
    pub fn new(
        allowed_handshake_types: &[Tls13HandshakeType],
        maximum_body_length: usize,
    ) -> Result<Self, HandshakeError> {
        if allowed_handshake_types.is_empty() {
            return Err(HandshakeError::new(
                HandshakeErrorKind::EmptyHandshakeTypePolicy,
            ));
        }
        if maximum_body_length > TLS13_MAX_HANDSHAKE_BODY_LEN {
            return Err(HandshakeError::new(HandshakeErrorKind::BodyLimitOutOfRange));
        }

        let mut allowed = 0_u16;
        for &handshake_type in allowed_handshake_types {
            let bit = handshake_type.policy_bit();
            if allowed & bit != 0 {
                return Err(HandshakeError::new(
                    HandshakeErrorKind::DuplicateHandshakeType,
                ));
            }
            allowed |= bit;
        }

        Ok(Self {
            allowed_handshake_types: allowed,
            maximum_body_length,
        })
    }

    /// Return whether this policy allows one recognized wire type.
    #[must_use]
    pub const fn allows(&self, handshake_type: Tls13HandshakeType) -> bool {
        self.allowed_handshake_types & handshake_type.policy_bit() != 0
    }

    /// Return the caller's validated body ceiling.
    #[must_use]
    pub const fn maximum_body_length(&self) -> usize {
        self.maximum_body_length
    }
}

/// One validated borrowed TLS 1.3 handshake message.
///
/// Private fields make the value constructible only through the checked
/// decoder.  `encoded` is exactly the header plus declared body, which is the
/// representation a future transcript owner must hash.
#[derive(Debug, Eq, PartialEq)]
pub struct Tls13HandshakeMessage<'a> {
    handshake_type: Tls13HandshakeType,
    body: &'a [u8],
    encoded: &'a [u8],
}

impl<'a> Tls13HandshakeMessage<'a> {
    /// Return the recognized and caller-authorized wire type.
    #[must_use]
    pub const fn handshake_type(&self) -> Tls13HandshakeType {
        self.handshake_type
    }

    /// Borrow the exact declared message body.
    #[must_use]
    pub const fn body(&self) -> &'a [u8] {
        self.body
    }

    /// Borrow the exact header-plus-body bytes covered by this message.
    #[must_use]
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }
}

/// Decode one exact TLS 1.3 handshake message.
///
/// This entry point rejects any trailing bytes.  Use
/// [`decode_tls13_handshake_prefix`] for a stream containing multiple complete
/// messages or a complete message followed by a partial next message.
pub fn decode_tls13_handshake<'a>(
    policy: &Tls13HandshakePolicy,
    input: &'a [u8],
) -> Result<Tls13HandshakeMessage<'a>, HandshakeError> {
    let (message, remainder) = decode_tls13_handshake_prefix(policy, input)?;
    if !remainder.is_empty() {
        return Err(HandshakeError::new(HandshakeErrorKind::TrailingData));
    }
    Ok(message)
}

/// Decode the first complete TLS 1.3 handshake message and return the stream
/// tail without copying either slice.
///
/// A truncated result means the caller must retain the bytes and reassemble
/// more authenticated record content before retrying.  This function owns no
/// buffering and consumes no input on failure.
pub fn decode_tls13_handshake_prefix<'a>(
    policy: &Tls13HandshakePolicy,
    input: &'a [u8],
) -> Result<(Tls13HandshakeMessage<'a>, &'a [u8]), HandshakeError> {
    if input.len() < TLS13_HANDSHAKE_HEADER_LEN {
        return Err(HandshakeError::new(HandshakeErrorKind::TruncatedHeader));
    }

    let handshake_type = Tls13HandshakeType::from_wire(input[0])?;
    if !policy.allows(handshake_type) {
        return Err(HandshakeError::new(
            HandshakeErrorKind::DisallowedHandshakeType,
        ));
    }

    let body_length = decode_u24([input[1], input[2], input[3]]);
    if body_length > policy.maximum_body_length {
        return Err(HandshakeError::new(HandshakeErrorKind::BodyTooLong));
    }
    let encoded_length = TLS13_HANDSHAKE_HEADER_LEN + body_length;
    if input.len() < encoded_length {
        return Err(HandshakeError::new(HandshakeErrorKind::TruncatedBody));
    }

    let (encoded, remainder) = input.split_at(encoded_length);
    let body = &encoded[TLS13_HANDSHAKE_HEADER_LEN..];
    Ok((
        Tls13HandshakeMessage {
            handshake_type,
            body,
            encoded,
        },
        remainder,
    ))
}

/// Encode one complete TLS 1.3 handshake message into caller-owned storage.
///
/// Every policy, length, and capacity check happens before the first output
/// byte changes.  Success returns the exact initialized prefix length.
pub fn encode_tls13_handshake(
    policy: &Tls13HandshakePolicy,
    handshake_type: Tls13HandshakeType,
    body: &[u8],
    output: &mut [u8],
) -> Result<usize, HandshakeError> {
    if !policy.allows(handshake_type) {
        return Err(HandshakeError::new(
            HandshakeErrorKind::DisallowedHandshakeType,
        ));
    }
    if body.len() > policy.maximum_body_length {
        return Err(HandshakeError::new(HandshakeErrorKind::BodyTooLong));
    }

    let encoded_length = TLS13_HANDSHAKE_HEADER_LEN + body.len();
    if output.len() < encoded_length {
        return Err(HandshakeError::new(HandshakeErrorKind::OutputTooSmall));
    }

    output[0] = handshake_type.wire_value();
    let length_bytes = encode_u24(body.len());
    output[1..TLS13_HANDSHAKE_HEADER_LEN].copy_from_slice(&length_bytes);
    output[TLS13_HANDSHAKE_HEADER_LEN..encoded_length].copy_from_slice(body);
    Ok(encoded_length)
}

const fn decode_u24(bytes: [u8; 3]) -> usize {
    ((bytes[0] as usize) << 16) | ((bytes[1] as usize) << 8) | bytes[2] as usize
}

fn encode_u24(value: usize) -> [u8; 3] {
    debug_assert!(value <= TLS13_MAX_HANDSHAKE_BODY_LEN);
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::{decode_u24, encode_u24, TLS13_MAX_HANDSHAKE_BODY_LEN};

    #[test]
    fn uint24_helpers_cover_boundaries() {
        for value in [0, 1, 0xff, 0x0100, 0x010203, TLS13_MAX_HANDSHAKE_BODY_LEN] {
            assert_eq!(decode_u24(encode_u24(value)), value);
        }
        assert_eq!(encode_u24(TLS13_MAX_HANDSHAKE_BODY_LEN), [0xff; 3]);
    }
}
