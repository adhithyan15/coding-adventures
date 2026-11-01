//! # Exact PKCS #1 v1.5 SHA-384 encoded messages
//!
//! RSA signature verification has two independent jobs. The public-key
//! operation recovers one modulus-width byte string. EMSA-PKCS1-v1_5 then
//! proves that the whole string is the canonical encoding of the expected
//! message digest. This crate owns only that second, byte-level job.
//!
//! For SHA-384, RFC 8017 permits exactly this shape:
//!
//! ```text
//! 00 01 ff ... ff 00 30 41 30 0d 06 09 60 86 48 01 65 03 04 02 02
//!                     05 00 04 30 <48-byte digest>
//! ```
//!
//! The padding contains at least eight `ff` bytes. The fixed trailer determines
//! every boundary, so validation never searches for a convenient separator
//! that could hide malformed padding.

#![deny(unsafe_code)]

use core::fmt;

/// SHA-384 produces exactly 48 octets.
pub const SHA384_DIGEST_LENGTH: usize = 48;

/// Canonical SHA-384 `DigestInfo` is a 19-octet DER prefix plus the digest.
pub const SHA384_DIGEST_INFO_LENGTH: usize = 67;

/// RFC 8017 requires at least eight `ff` padding octets.
pub const MINIMUM_PADDING_LENGTH: usize = 8;

/// Two header octets, padding, one separator, and the complete `DigestInfo`.
pub const MINIMUM_ENCODED_MESSAGE_LENGTH: usize =
    2 + MINIMUM_PADDING_LENGTH + 1 + SHA384_DIGEST_INFO_LENGTH;

// SHA-384 DigestInfo is a fixed byte constant rather than a generic DER
// decoder. Exact matching rejects BER alternatives, omitted NULL parameters,
// non-canonical lengths, and algorithm substitution at this policy boundary.
const SHA384_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x41, // DigestInfo SEQUENCE, 65 content octets
    0x30, 0x0d, // AlgorithmIdentifier SEQUENCE, 13 content octets
    0x06, 0x09, // OBJECT IDENTIFIER, 9 content octets
    0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02, // id-sha384
    0x05, 0x00, // canonical present NULL parameters
    0x04, 0x30, // OCTET STRING, 48 digest octets
];

/// Stable failure categories that retain no caller-provided bytes or offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sha384EncodedMessageErrorKind {
    /// The input cannot contain the RFC-mandated minimum padding and trailer.
    EncodedMessageTooShort,
    /// The encoded message did not begin with `00 01`.
    InvalidBlockHeader,
    /// At least one byte in the deterministic padding range was not `ff`.
    InvalidPadding,
    /// The byte immediately before `DigestInfo` was not the zero separator.
    InvalidSeparator,
    /// The canonical SHA-384 `DigestInfo` prefix did not match exactly.
    InvalidDigestInfo,
    /// The encoded digest differed from the caller's complete expected digest.
    DigestMismatch,
}

/// A redacted encoded-message validation failure.
///
/// The error exposes only a stable category. It retains no encoded-message
/// bytes, digest bytes, mismatch index, or padding length.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Sha384EncodedMessageError {
    kind: Sha384EncodedMessageErrorKind,
}

impl Sha384EncodedMessageError {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Sha384EncodedMessageErrorKind {
        self.kind
    }

    const fn new(kind: Sha384EncodedMessageErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Sha384EncodedMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sha384EncodedMessageError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Sha384EncodedMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "PKCS #1 v1.5 SHA-384 encoded-message error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for Sha384EncodedMessageError {}

/// Opaque evidence that one complete encoded message matched one digest.
///
/// This zero-sized value retains no input. It proves neither that RSA produced
/// the supplied bytes nor that any key, certificate, path, or peer is trusted.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ValidatedSha384EncodedMessage {
    _private: (),
}

impl fmt::Debug for ValidatedSha384EncodedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedSha384EncodedMessage")
    }
}

/// Validate one exact RFC 8017 EMSA-PKCS1-v1_5 SHA-384 encoded message.
///
/// `encoded_message` must be the complete fixed-width output of a separate RSA
/// public operation. `expected_digest` must be computed over the intended
/// signed bytes by a separate repository-owned SHA-384 boundary. This function
/// performs neither operation and cannot establish signature or certificate
/// trust by itself.
pub fn validate_sha384_encoded_message(
    encoded_message: &[u8],
    expected_digest: &[u8; SHA384_DIGEST_LENGTH],
) -> Result<ValidatedSha384EncodedMessage, Sha384EncodedMessageError> {
    if encoded_message.len() < MINIMUM_ENCODED_MESSAGE_LENGTH {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::EncodedMessageTooShort,
        ));
    }

    if encoded_message[0] != 0x00 || encoded_message[1] != 0x01 {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::InvalidBlockHeader,
        ));
    }

    // The minimum-length guard proves both subtractions. Deriving the
    // separator from the fixed trailer rejects earlier embedded zero bytes and
    // avoids scanning untrusted padding for a convenient delimiter.
    let digest_info_start = encoded_message.len() - SHA384_DIGEST_INFO_LENGTH;
    let separator_index = digest_info_start - 1;

    if encoded_message[2..separator_index]
        .iter()
        .any(|octet| *octet != 0xff)
    {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::InvalidPadding,
        ));
    }

    if encoded_message[separator_index] != 0x00 {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::InvalidSeparator,
        ));
    }

    let prefix_end = digest_info_start + SHA384_DIGEST_INFO_PREFIX.len();
    if encoded_message[digest_info_start..prefix_end] != SHA384_DIGEST_INFO_PREFIX {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::InvalidDigestInfo,
        ));
    }

    // Digests are public verification data. Still, reduce all mismatch bits
    // before returning so no mismatch position becomes part of this API.
    let encoded_digest = &encoded_message[prefix_end..];
    let digest_difference = encoded_digest
        .iter()
        .zip(expected_digest)
        .fold(0u8, |difference, (encoded, expected)| {
            difference | (encoded ^ expected)
        });
    if digest_difference != 0 {
        return Err(Sha384EncodedMessageError::new(
            Sha384EncodedMessageErrorKind::DigestMismatch,
        ));
    }

    Ok(ValidatedSha384EncodedMessage { _private: () })
}
