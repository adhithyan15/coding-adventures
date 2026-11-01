//! Exact, bounded TLS 1.3 `ClientHello` message-body decoding.
//!
//! RFC 8446 section 4.1.2 fixes the legacy version and compression fields,
//! bounds the legacy session ID, and carries preference-ordered cipher suites
//! plus a mandatory extension vector. This crate owns only that structural
//! body shape and delegates extension framing to `tls13-extension-block`.

#![forbid(unsafe_code)]

use core::{fmt, iter::FusedIterator};
use tls13_extension_block::{
    decode_extension_block, ExtensionBlock, ExtensionBlockError, ExtensionBlockLimits,
};

/// TLS 1.3's fixed ClientHello legacy version (`TLS 1.2`).
pub const CLIENT_HELLO_LEGACY_VERSION: u16 = 0x0303;

/// The fixed ClientHello random length.
pub const CLIENT_RANDOM_LENGTH: usize = 32;

/// RFC 8446's legacy-session-ID ceiling.
pub const MAX_LEGACY_SESSION_ID_LENGTH: usize = 32;

/// RFC 8446's largest possible count of two-byte cipher-suite identifiers.
pub const MAX_CIPHER_SUITE_COUNT: usize = 32_767;

/// RFC 8446's minimum ClientHello extension-vector payload length.
pub const MIN_CLIENT_HELLO_EXTENSION_PAYLOAD_LENGTH: usize = 8;

/// Caller-selected resource limits for one ClientHello body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientHelloLimits {
    max_cipher_suite_count: usize,
    extension_limits: ExtensionBlockLimits,
}

impl ClientHelloLimits {
    /// Validate the suite-count ceiling and retain validated extension limits.
    pub const fn new(
        max_cipher_suite_count: usize,
        extension_limits: ExtensionBlockLimits,
    ) -> Result<Self, ClientHelloError> {
        if max_cipher_suite_count == 0 || max_cipher_suite_count > MAX_CIPHER_SUITE_COUNT {
            return Err(ClientHelloError::InvalidCipherSuiteLimit);
        }
        Ok(Self {
            max_cipher_suite_count,
            extension_limits,
        })
    }

    /// Return the maximum accepted cipher-suite count.
    #[must_use]
    pub const fn max_cipher_suite_count(self) -> usize {
        self.max_cipher_suite_count
    }

    /// Return the extension-block limits applied by the decoder.
    #[must_use]
    pub const fn extension_limits(self) -> ExtensionBlockLimits {
        self.extension_limits
    }
}

/// Stable, payload-free ClientHello framing and resource failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientHelloError {
    /// The caller supplied a zero or out-of-protocol cipher-suite limit.
    InvalidCipherSuiteLimit,
    /// The body omitted part of the fixed legacy version, random, or session length.
    MissingFixedFields,
    /// The legacy version was not the TLS 1.3-required `0x0303` value.
    InvalidLegacyVersion,
    /// The declared legacy session ID exceeded 32 bytes.
    LegacySessionIdTooLong,
    /// The body ended inside the declared legacy session ID.
    TruncatedLegacySessionId,
    /// The two-byte cipher-suite vector length was absent or incomplete.
    MissingCipherSuiteLength,
    /// The cipher-suite vector contained no identifiers.
    EmptyCipherSuiteList,
    /// The cipher-suite vector length was not an even number of bytes.
    OddCipherSuiteLength,
    /// The declared cipher-suite count exceeded the caller limit.
    TooManyCipherSuites,
    /// The body ended inside the declared cipher-suite vector.
    TruncatedCipherSuites,
    /// The one-byte compression-method vector length was absent.
    MissingCompressionMethodsLength,
    /// The body ended inside the declared compression-method vector.
    TruncatedCompressionMethods,
    /// The vector was not exactly the single null compression method.
    InvalidCompressionMethods,
    /// The decoded extension payload was shorter than the ClientHello minimum.
    ExtensionVectorTooShort,
    /// Generic extension-vector framing or resource validation failed.
    Extensions(ExtensionBlockError),
}

impl fmt::Display for ClientHelloError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCipherSuiteLimit => "invalid ClientHello cipher-suite limit",
            Self::MissingFixedFields => "missing ClientHello fixed fields",
            Self::InvalidLegacyVersion => "invalid ClientHello legacy version",
            Self::LegacySessionIdTooLong => "ClientHello legacy session ID is too long",
            Self::TruncatedLegacySessionId => "truncated ClientHello legacy session ID",
            Self::MissingCipherSuiteLength => "missing ClientHello cipher-suite length",
            Self::EmptyCipherSuiteList => "empty ClientHello cipher-suite list",
            Self::OddCipherSuiteLength => "odd ClientHello cipher-suite length",
            Self::TooManyCipherSuites => "ClientHello cipher-suite list exceeds caller limit",
            Self::TruncatedCipherSuites => "truncated ClientHello cipher-suite list",
            Self::MissingCompressionMethodsLength => {
                "missing ClientHello compression-method length"
            }
            Self::TruncatedCompressionMethods => "truncated ClientHello compression methods",
            Self::InvalidCompressionMethods => "invalid ClientHello compression methods",
            Self::ExtensionVectorTooShort => "ClientHello extension vector is too short",
            Self::Extensions(_) => "invalid ClientHello extension vector",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClientHelloError {}

/// One exact borrowed, structurally decoded TLS 1.3 ClientHello body.
///
/// The value intentionally derives neither `Copy` nor `Clone`. Later offer and
/// extension policy can therefore remain attached to this exact parse.
pub struct ClientHelloMessage<'input> {
    body: &'input [u8],
    random: &'input [u8; CLIENT_RANDOM_LENGTH],
    legacy_session_id: &'input [u8],
    cipher_suite_bytes: &'input [u8],
    extensions: ExtensionBlock<'input>,
}

impl fmt::Debug for ClientHelloMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientHelloMessage")
            .field("body", &"[OPAQUE]")
            .field("random", &"[OPAQUE]")
            .field("legacy_session_id_length", &self.legacy_session_id.len())
            .field("cipher_suite_count", &self.cipher_suite_count())
            .field("extension_count", &self.extensions.len())
            .field(
                "extension_payload_length",
                &self.extensions.payload_length(),
            )
            .finish_non_exhaustive()
    }
}

impl<'input> ClientHelloMessage<'input> {
    /// Borrow the exact complete handshake body supplied to the decoder.
    #[must_use]
    pub const fn body(&self) -> &'input [u8] {
        self.body
    }

    /// Borrow the exact 32-byte ClientHello random.
    #[must_use]
    pub const fn random(&self) -> &'input [u8; CLIENT_RANDOM_LENGTH] {
        self.random
    }

    /// Borrow the exact zero-through-32-byte legacy session ID.
    #[must_use]
    pub const fn legacy_session_id(&self) -> &'input [u8] {
        self.legacy_session_id
    }

    /// Return the number of offered cipher suites.
    #[must_use]
    pub const fn cipher_suite_count(&self) -> usize {
        self.cipher_suite_bytes.len() / 2
    }

    /// Return one cipher-suite identifier by preference index.
    #[must_use]
    pub fn cipher_suite(&self, index: usize) -> Option<u16> {
        let start = index.checked_mul(2)?;
        let end = start.checked_add(2)?;
        let bytes = self.cipher_suite_bytes.get(start..end)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Iterate over offered cipher suites in wire preference order.
    #[must_use]
    pub const fn cipher_suites(&self) -> CipherSuiteIter<'input> {
        CipherSuiteIter {
            remaining: self.cipher_suite_bytes,
        }
    }

    /// Borrow the exact structurally decoded extension block.
    #[must_use]
    pub const fn extensions(&self) -> &ExtensionBlock<'input> {
        &self.extensions
    }
}

/// Exact-size iterator over borrowed two-byte cipher-suite identifiers.
#[derive(Clone, Copy)]
pub struct CipherSuiteIter<'input> {
    remaining: &'input [u8],
}

impl fmt::Debug for CipherSuiteIter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CipherSuiteIter")
            .field("remaining_suite_count", &self.len())
            .field("remaining", &"[OPAQUE]")
            .finish_non_exhaustive()
    }
}

impl Iterator for CipherSuiteIter<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let [first, second, tail @ ..] = self.remaining else {
            return None;
        };
        let suite = u16::from_be_bytes([*first, *second]);
        self.remaining = tail;
        Some(suite)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining.len() / 2;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for CipherSuiteIter<'_> {}
impl FusedIterator for CipherSuiteIter<'_> {}

/// Decode one exact RFC 8446 section 4.1.2 TLS 1.3 ClientHello body.
pub fn decode_client_hello(
    body: &[u8],
    limits: ClientHelloLimits,
) -> Result<ClientHelloMessage<'_>, ClientHelloError> {
    const FIXED_PREFIX_LENGTH: usize = 2 + CLIENT_RANDOM_LENGTH + 1;
    if body.len() < FIXED_PREFIX_LENGTH {
        return Err(ClientHelloError::MissingFixedFields);
    }
    if u16::from_be_bytes([body[0], body[1]]) != CLIENT_HELLO_LEGACY_VERSION {
        return Err(ClientHelloError::InvalidLegacyVersion);
    }

    let random = <&[u8; CLIENT_RANDOM_LENGTH]>::try_from(&body[2..34])
        .map_err(|_| ClientHelloError::MissingFixedFields)?;
    let session_id_length = body[34] as usize;
    if session_id_length > MAX_LEGACY_SESSION_ID_LENGTH {
        return Err(ClientHelloError::LegacySessionIdTooLong);
    }

    let session_start = FIXED_PREFIX_LENGTH;
    let session_end = session_start
        .checked_add(session_id_length)
        .ok_or(ClientHelloError::TruncatedLegacySessionId)?;
    let legacy_session_id = body
        .get(session_start..session_end)
        .ok_or(ClientHelloError::TruncatedLegacySessionId)?;

    let suite_length_end = session_end
        .checked_add(2)
        .ok_or(ClientHelloError::MissingCipherSuiteLength)?;
    let suite_length_bytes = body
        .get(session_end..suite_length_end)
        .ok_or(ClientHelloError::MissingCipherSuiteLength)?;
    let suite_length = u16::from_be_bytes([suite_length_bytes[0], suite_length_bytes[1]]) as usize;
    if suite_length == 0 {
        return Err(ClientHelloError::EmptyCipherSuiteList);
    }
    if suite_length & 1 != 0 {
        return Err(ClientHelloError::OddCipherSuiteLength);
    }
    if suite_length / 2 > limits.max_cipher_suite_count {
        return Err(ClientHelloError::TooManyCipherSuites);
    }
    let suites_end = suite_length_end
        .checked_add(suite_length)
        .ok_or(ClientHelloError::TruncatedCipherSuites)?;
    let cipher_suite_bytes = body
        .get(suite_length_end..suites_end)
        .ok_or(ClientHelloError::TruncatedCipherSuites)?;

    let compression_length = *body
        .get(suites_end)
        .ok_or(ClientHelloError::MissingCompressionMethodsLength)?
        as usize;
    let compression_start = suites_end + 1;
    let compression_end = compression_start
        .checked_add(compression_length)
        .ok_or(ClientHelloError::TruncatedCompressionMethods)?;
    let compression_methods = body
        .get(compression_start..compression_end)
        .ok_or(ClientHelloError::TruncatedCompressionMethods)?;
    if compression_methods != [0] {
        return Err(ClientHelloError::InvalidCompressionMethods);
    }

    let extensions = decode_extension_block(&body[compression_end..], limits.extension_limits)
        .map_err(ClientHelloError::Extensions)?;
    if extensions.payload_length() < MIN_CLIENT_HELLO_EXTENSION_PAYLOAD_LENGTH {
        return Err(ClientHelloError::ExtensionVectorTooShort);
    }

    Ok(ClientHelloMessage {
        body,
        random,
        legacy_session_id,
        cipher_suite_bytes,
        extensions,
    })
}
