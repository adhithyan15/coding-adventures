//! # Exact PKCS #1 v1.5 SHA-256 encoded messages
//!
//! RSA signature verification has two independent jobs. The public-key
//! operation recovers a modulus-width byte string. EMSA-PKCS1-v1_5 then proves
//! that the whole string is the one canonical encoding of the expected message
//! digest. This crate owns only that second, byte-level job.
//!
//! For SHA-256, RFC 8017 permits exactly this shape:
//!
//! ```text
//! 00 01 ff ... ff 00 30 31 30 0d 06 09 60 86 48 01 65 03 04 02 01
//!                     05 00 04 20 <32-byte digest>
//! ```
//!
//! The padding contains at least eight `ff` bytes. The trailer begins at the
//! position implied by the total input length, so the validator never searches
//! for a convenient separator that could hide malformed padding.

use core::fmt;

/// SHA-256 produces exactly 32 octets.
pub const SHA256_DIGEST_LENGTH: usize = 32;

/// Canonical SHA-256 `DigestInfo` is a 19-octet DER prefix plus the digest.
pub const SHA256_DIGEST_INFO_LENGTH: usize = 51;

/// RFC 8017 requires at least eight `ff` padding octets.
pub const MINIMUM_PADDING_LENGTH: usize = 8;

/// Two header octets, padding, one separator, and the complete `DigestInfo`.
pub const MINIMUM_ENCODED_MESSAGE_LENGTH: usize =
    2 + MINIMUM_PADDING_LENGTH + 1 + SHA256_DIGEST_INFO_LENGTH;

// The SHA-256 DigestInfo prefix is deliberately a byte constant rather than a
// generic DER decoder. Accepting exactly one reviewed encoding prevents BER
// alternatives, omitted NULL parameters, or algorithm substitution from
// becoming accidental policy.
const SHA256_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x31, // DigestInfo SEQUENCE, 49 content octets
    0x30, 0x0d, // AlgorithmIdentifier SEQUENCE, 13 content octets
    0x06, 0x09, // OBJECT IDENTIFIER, 9 content octets
    0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, // id-sha256
    0x05, 0x00, // canonical present NULL parameters
    0x04, 0x20, // OCTET STRING, 32 digest octets
];

/// Stable failure categories that retain no caller-provided bytes or offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sha256EncodedMessageErrorKind {
    /// The input cannot contain the RFC-mandated minimum padding and trailer.
    EncodedMessageTooShort,
    /// The encoded message did not begin with `00 01`.
    InvalidBlockHeader,
    /// At least one byte in the deterministic padding range was not `ff`.
    InvalidPadding,
    /// The byte immediately before `DigestInfo` was not the zero separator.
    InvalidSeparator,
    /// The canonical SHA-256 `DigestInfo` prefix did not match exactly.
    InvalidDigestInfo,
    /// The encoded digest differed from the caller's complete expected digest.
    DigestMismatch,
}

/// A redacted encoded-message validation failure.
///
/// The error exposes only a stable category. In particular, it carries no
/// encoded-message bytes, digest bytes, mismatch index, or padding length.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Sha256EncodedMessageError {
    kind: Sha256EncodedMessageErrorKind,
}

impl Sha256EncodedMessageError {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Sha256EncodedMessageErrorKind {
        self.kind
    }

    const fn new(kind: Sha256EncodedMessageErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Sha256EncodedMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sha256EncodedMessageError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Sha256EncodedMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "PKCS #1 v1.5 SHA-256 encoded-message error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for Sha256EncodedMessageError {}

/// Opaque evidence that one complete encoded message matched one digest.
///
/// This zero-sized value retains no input. It proves neither that RSA produced
/// the supplied bytes nor that any key, certificate, path, or peer is trusted.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ValidatedSha256EncodedMessage {
    _private: (),
}

impl fmt::Debug for ValidatedSha256EncodedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedSha256EncodedMessage")
    }
}

/// Validate one exact RFC 8017 EMSA-PKCS1-v1_5 SHA-256 encoded message.
///
/// `encoded_message` must be the complete fixed-width output of a separate RSA
/// public operation. `expected_digest` must be computed over the intended
/// signed bytes by a separate repository-owned SHA-256 boundary. This function
/// does not perform either operation and therefore cannot establish signature
/// or certificate trust by itself.
pub fn validate_sha256_encoded_message(
    encoded_message: &[u8],
    expected_digest: &[u8; SHA256_DIGEST_LENGTH],
) -> Result<ValidatedSha256EncodedMessage, Sha256EncodedMessageError> {
    if encoded_message.len() < MINIMUM_ENCODED_MESSAGE_LENGTH {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::EncodedMessageTooShort,
        ));
    }

    if encoded_message[0] != 0x00 || encoded_message[1] != 0x01 {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::InvalidBlockHeader,
        ));
    }

    // The fixed-size trailer determines every boundary. Checked subtraction is
    // already justified by the minimum-length guard above.
    let digest_info_start = encoded_message.len() - SHA256_DIGEST_INFO_LENGTH;
    let separator_index = digest_info_start - 1;

    if encoded_message[2..separator_index]
        .iter()
        .any(|octet| *octet != 0xff)
    {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::InvalidPadding,
        ));
    }

    if encoded_message[separator_index] != 0x00 {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::InvalidSeparator,
        ));
    }

    let prefix_end = digest_info_start + SHA256_DIGEST_INFO_PREFIX.len();
    if encoded_message[digest_info_start..prefix_end] != SHA256_DIGEST_INFO_PREFIX {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::InvalidDigestInfo,
        ));
    }

    // SHA-256 digests are public verification data, but a complete comparison
    // is still preferable to exposing the first mismatch position. The fold
    // examines every digest byte before making one decision.
    let encoded_digest = &encoded_message[prefix_end..];
    let digest_difference = encoded_digest
        .iter()
        .zip(expected_digest)
        .fold(0u8, |difference, (encoded, expected)| {
            difference | (encoded ^ expected)
        });
    if digest_difference != 0 {
        return Err(Sha256EncodedMessageError::new(
            Sha256EncodedMessageErrorKind::DigestMismatch,
        ));
    }

    Ok(ValidatedSha256EncodedMessage { _private: () })
}
