//! Bounded decoding for one TLS 1.3 `ServerHello` handshake body.
//!
//! [`decode_tls13_server_hello`] validates the fixed RFC 8446 legacy fields,
//! retains the random, session-id echo, and cipher-suite identifier as exact
//! borrowed or scalar wire values, and delegates the final extension vector to
//! `tls13-extension-block` under caller-owned byte and entry-count ceilings.
//!
//! This crate does not classify `HelloRetryRequest`, interpret extensions,
//! validate a server selection against a `ClientHello`, advance a handshake,
//! invoke cryptography, perform I/O, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::{
    decode_extension_block, ExtensionBlock, ExtensionBlockError, ExtensionBlockLimits,
};

/// TLS 1.3's fixed legacy version value in `ServerHello`.
pub const TLS13_SERVER_HELLO_LEGACY_VERSION: u16 = 0x0303;

/// TLS 1.3's fixed legacy compression method in `ServerHello`.
pub const TLS13_SERVER_HELLO_LEGACY_COMPRESSION_METHOD: u8 = 0;

/// Exact random-field length in bytes.
pub const TLS13_SERVER_HELLO_RANDOM_LENGTH: usize = 32;

/// Largest session-id echo representable by the TLS 1.3 structure.
pub const TLS13_SERVER_HELLO_MAX_SESSION_ID_ECHO_LENGTH: usize = 32;

/// Smallest extension-vector payload admitted by the TLS 1.3 structure.
pub const TLS13_SERVER_HELLO_MIN_EXTENSIONS_PAYLOAD_LENGTH: usize = 6;

/// Largest complete `ServerHello` body representable by the structure.
pub const TLS13_SERVER_HELLO_MAX_MESSAGE_LENGTH: usize = 65_607;

/// Smallest complete `ServerHello` body representable by the structure.
pub const TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH: usize = 46;

/// Caller-owned resource ceilings for one `ServerHello` body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tls13ServerHelloLimits {
    max_message_length: usize,
    extension_limits: ExtensionBlockLimits,
}

impl Tls13ServerHelloLimits {
    /// Validate body, extension-payload, and extension-count ceilings.
    pub fn new(
        max_message_length: usize,
        max_extensions_payload_length: usize,
        max_extensions: usize,
    ) -> Result<Self, Tls13ServerHelloError> {
        if !(TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH..=TLS13_SERVER_HELLO_MAX_MESSAGE_LENGTH)
            .contains(&max_message_length)
            || max_extensions_payload_length < TLS13_SERVER_HELLO_MIN_EXTENSIONS_PAYLOAD_LENGTH
        {
            return Err(Tls13ServerHelloError::new(
                Tls13ServerHelloErrorKind::InvalidLimits,
                0,
            ));
        }

        let extension_limits =
            ExtensionBlockLimits::new(max_extensions_payload_length, max_extensions).map_err(
                |_| Tls13ServerHelloError::new(Tls13ServerHelloErrorKind::InvalidLimits, 0),
            )?;

        Ok(Self {
            max_message_length,
            extension_limits,
        })
    }

    /// Return the maximum accepted complete body length.
    #[must_use]
    pub const fn max_message_length(self) -> usize {
        self.max_message_length
    }

    /// Return the maximum accepted extension-vector payload length.
    #[must_use]
    pub const fn max_extensions_payload_length(self) -> usize {
        self.extension_limits.max_payload_length()
    }

    /// Return the maximum accepted number of extension entries.
    #[must_use]
    pub const fn max_extensions(self) -> usize {
        self.extension_limits.max_extensions()
    }
}

/// The field whose bytes were absent or incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13ServerHelloField {
    LegacyVersion,
    Random,
    SessionIdEchoLength,
    SessionIdEcho,
    CipherSuite,
    LegacyCompressionMethod,
}

/// Stable payload-free failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13ServerHelloErrorKind {
    InvalidLimits,
    MessageTooLong,
    Truncated(Tls13ServerHelloField),
    InvalidLegacyVersion,
    SessionIdEchoTooLong,
    InvalidLegacyCompressionMethod,
    ExtensionsTooShort,
    InvalidExtensions(ExtensionBlockError),
}

/// A redacted failure at an offset local to the `ServerHello` body.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13ServerHelloError {
    kind: Tls13ServerHelloErrorKind,
    offset: usize,
}

impl Tls13ServerHelloError {
    const fn new(kind: Tls13ServerHelloErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> Tls13ServerHelloErrorKind {
        self.kind
    }

    /// Return the byte offset at which validation failed.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Debug for Tls13ServerHelloError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13ServerHelloError")
            .field("kind", &self.kind)
            .field("offset", &self.offset)
            .finish()
    }
}

impl fmt::Display for Tls13ServerHelloError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 ServerHello error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl std::error::Error for Tls13ServerHelloError {}

/// One exact unclassified cipher-suite wire identifier.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CipherSuiteId(u16);

impl CipherSuiteId {
    /// Retain a wire identifier without claiming support or selection validity.
    #[must_use]
    pub const fn from_wire_value(value: u16) -> Self {
        Self(value)
    }

    /// Return the exact two-byte wire identifier.
    #[must_use]
    pub const fn wire_value(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for CipherSuiteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CipherSuiteId")
            .field(&self.0)
            .finish()
    }
}

/// One structurally valid borrowed TLS 1.3 `ServerHello` body.
pub struct Tls13ServerHello<'input> {
    encoded: &'input [u8],
    random: &'input [u8; TLS13_SERVER_HELLO_RANDOM_LENGTH],
    session_id_echo: &'input [u8],
    cipher_suite: CipherSuiteId,
    extensions: ExtensionBlock<'input>,
}

impl<'input> Tls13ServerHello<'input> {
    /// Borrow the exact already-delimited body bytes.
    #[must_use]
    pub const fn encoded(&self) -> &'input [u8] {
        self.encoded
    }

    /// Borrow the exact 32-byte random field without classifying HRR.
    #[must_use]
    pub const fn random(&self) -> &'input [u8; TLS13_SERVER_HELLO_RANDOM_LENGTH] {
        self.random
    }

    /// Borrow the exact legacy session-id echo.
    #[must_use]
    pub const fn session_id_echo(&self) -> &'input [u8] {
        self.session_id_echo
    }

    /// Return the exact unclassified cipher-suite identifier.
    #[must_use]
    pub const fn cipher_suite(&self) -> CipherSuiteId {
        self.cipher_suite
    }

    /// Borrow the structurally decoded extension vector.
    #[must_use]
    pub const fn extensions(&self) -> &ExtensionBlock<'input> {
        &self.extensions
    }
}

impl fmt::Debug for Tls13ServerHello<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13ServerHello")
            .field("encoded_length", &self.encoded.len())
            .field("random", &"[OPAQUE]")
            .field("session_id_echo_length", &self.session_id_echo.len())
            .field("session_id_echo", &"[OPAQUE]")
            .field("cipher_suite", &self.cipher_suite)
            .field("extensions", &self.extensions)
            .finish()
    }
}

/// Decode one exact already-delimited TLS 1.3 `ServerHello` body.
///
/// The fixed legacy fields and structural vector bounds are enforced. The
/// cipher-suite identifier, random value, and extension contents remain data
/// for later context-specific handshake policy.
pub fn decode_tls13_server_hello(
    input: &[u8],
    limits: Tls13ServerHelloLimits,
) -> Result<Tls13ServerHello<'_>, Tls13ServerHelloError> {
    if input.len() > limits.max_message_length() {
        return Err(Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::MessageTooLong,
            0,
        ));
    }

    let mut offset = 0;
    let legacy_version = take(input, &mut offset, 2, Tls13ServerHelloField::LegacyVersion)?;
    if u16::from_be_bytes([legacy_version[0], legacy_version[1]])
        != TLS13_SERVER_HELLO_LEGACY_VERSION
    {
        return Err(Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::InvalidLegacyVersion,
            0,
        ));
    }

    let random_offset = offset;
    let random = take(
        input,
        &mut offset,
        TLS13_SERVER_HELLO_RANDOM_LENGTH,
        Tls13ServerHelloField::Random,
    )?;
    let random: &[u8; TLS13_SERVER_HELLO_RANDOM_LENGTH] = random.try_into().map_err(|_| {
        Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::Truncated(Tls13ServerHelloField::Random),
            random_offset,
        )
    })?;

    let session_length_offset = offset;
    let session_id_echo_length = take(
        input,
        &mut offset,
        1,
        Tls13ServerHelloField::SessionIdEchoLength,
    )?[0] as usize;
    if session_id_echo_length > TLS13_SERVER_HELLO_MAX_SESSION_ID_ECHO_LENGTH {
        return Err(Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::SessionIdEchoTooLong,
            session_length_offset,
        ));
    }
    let session_id_echo = take(
        input,
        &mut offset,
        session_id_echo_length,
        Tls13ServerHelloField::SessionIdEcho,
    )?;

    let cipher_suite = take(input, &mut offset, 2, Tls13ServerHelloField::CipherSuite)?;
    let cipher_suite =
        CipherSuiteId::from_wire_value(u16::from_be_bytes([cipher_suite[0], cipher_suite[1]]));

    let compression_offset = offset;
    let compression = take(
        input,
        &mut offset,
        1,
        Tls13ServerHelloField::LegacyCompressionMethod,
    )?[0];
    if compression != TLS13_SERVER_HELLO_LEGACY_COMPRESSION_METHOD {
        return Err(Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::InvalidLegacyCompressionMethod,
            compression_offset,
        ));
    }

    let extensions_offset = offset;
    let extensions = decode_extension_block(&input[extensions_offset..], limits.extension_limits)
        .map_err(|error| {
        Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::InvalidExtensions(error),
            extensions_offset,
        )
    })?;
    if extensions.payload_length() < TLS13_SERVER_HELLO_MIN_EXTENSIONS_PAYLOAD_LENGTH {
        return Err(Tls13ServerHelloError::new(
            Tls13ServerHelloErrorKind::ExtensionsTooShort,
            extensions_offset,
        ));
    }

    Ok(Tls13ServerHello {
        encoded: input,
        random,
        session_id_echo,
        cipher_suite,
        extensions,
    })
}

fn take<'input>(
    input: &'input [u8],
    offset: &mut usize,
    length: usize,
    field: Tls13ServerHelloField,
) -> Result<&'input [u8], Tls13ServerHelloError> {
    let start = *offset;
    let end = start.checked_add(length).ok_or_else(|| {
        Tls13ServerHelloError::new(Tls13ServerHelloErrorKind::Truncated(field), start)
    })?;
    let value = input.get(start..end).ok_or_else(|| {
        Tls13ServerHelloError::new(Tls13ServerHelloErrorKind::Truncated(field), start)
    })?;
    *offset = end;
    Ok(value)
}
