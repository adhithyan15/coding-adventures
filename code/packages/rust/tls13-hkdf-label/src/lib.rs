//! Bounded, allocation-free TLS 1.3 `HKDF-Expand-Label` and `Derive-Secret`
//! composition.
//!
//! RFC 8446 section 7.1 domain-separates every TLS 1.3 HKDF expansion by
//! serializing a public output length, the fixed `"tls13 "` namespace plus a
//! caller label, and a public context. This crate owns that serialization and
//! composes it with the repository-owned fixed-memory HKDF boundary. The same
//! PRK-bound algorithm policy also enforces the exact transcript-hash and
//! output lengths for one `Derive-Secret` operation. It does not select a
//! cipher suite or compose a complete TLS key schedule.
//!
//! ```
//! use tls13_hkdf_label::HkdfLabel;
//!
//! let label = HkdfLabel::new(16, b"key", b"").unwrap();
//! let mut storage = [0u8; 13];
//! let encoded = label.encode(&mut storage).unwrap();
//!
//! assert_eq!(encoded, b"\x00\x10\x09tls13 key\x00");
//! ```

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{hkdf_expand_fixed, HkdfError, HkdfPrk};
use core::fmt;

/// The namespace prepended to every caller label by RFC 8446 section 7.1.
pub const TLS13_LABEL_PREFIX: &[u8; 6] = b"tls13 ";

/// The shortest caller label that can produce the RFC-required seven-byte
/// prefixed label.
pub const MIN_LABEL_LENGTH: usize = 1;

/// The longest caller label after reserving six of the one-byte length's 255
/// values for [`TLS13_LABEL_PREFIX`].
pub const MAX_LABEL_LENGTH: usize = u8::MAX as usize - TLS13_LABEL_PREFIX.len();

/// The context is encoded behind a one-byte length.
pub const MAX_CONTEXT_LENGTH: usize = u8::MAX as usize;

/// `uint16 length`, two one-byte vector lengths, and the largest label and
/// context payloads.
pub const MAX_ENCODED_LENGTH: usize = 2 + 1 + u8::MAX as usize + 1 + u8::MAX as usize;

/// Stable, payload-free failures for TLS 1.3 HKDF label framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HkdfLabelError {
    /// The RFC structure requires the prefixed label to contain at least seven
    /// bytes, so a caller label cannot be empty.
    EmptyLabel,
    /// Prefix plus caller label would not fit behind the one-byte length.
    LabelTooLong,
    /// Context would not fit behind the one-byte length.
    ContextTooLong,
    /// Caller-owned output cannot hold the complete encoding.
    OutputTooSmall,
}

impl fmt::Display for HkdfLabelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::EmptyLabel => "TLS 1.3 HKDF label is empty",
            Self::LabelTooLong => "TLS 1.3 HKDF label exceeds its bound",
            Self::ContextTooLong => "TLS 1.3 HKDF context exceeds its bound",
            Self::OutputTooSmall => "TLS 1.3 HKDF label output is too small",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for HkdfLabelError {}

/// Stable failures for fixed-memory TLS 1.3 `HKDF-Expand-Label`.
#[derive(Debug, PartialEq, Eq)]
pub enum HkdfExpandLabelError {
    /// Caller output cannot be represented by the RFC's two-byte length.
    OutputLengthTooLarge,
    /// Public label or context framing failed.
    Framing(HkdfLabelError),
    /// Repository-owned fixed-memory HKDF expansion failed.
    Expand(HkdfError),
}

impl fmt::Display for HkdfExpandLabelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputLengthTooLarge => {
                formatter.write_str("TLS 1.3 HKDF output exceeds its framing bound")
            }
            Self::Framing(error) => write!(formatter, "TLS 1.3 HKDF framing failed: {error}"),
            Self::Expand(error) => write!(formatter, "TLS 1.3 HKDF expansion failed: {error}"),
        }
    }
}

impl std::error::Error for HkdfExpandLabelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutputLengthTooLarge => None,
            Self::Framing(error) => Some(error),
            Self::Expand(error) => Some(error),
        }
    }
}

/// Stable failures for one fixed-memory TLS 1.3 `Derive-Secret` operation.
#[derive(Debug, PartialEq, Eq)]
pub enum DeriveSecretError {
    /// The supplied transcript hash is not exactly Hash.length bytes.
    TranscriptHashLengthMismatch { expected: usize, actual: usize },
    /// Caller-owned secret output is not exactly Hash.length bytes.
    OutputLengthMismatch { expected: usize, actual: usize },
    /// The underlying fixed-memory `HKDF-Expand-Label` operation failed.
    Expand(HkdfExpandLabelError),
}

impl fmt::Display for DeriveSecretError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TranscriptHashLengthMismatch { expected, actual } => write!(
                formatter,
                "TLS 1.3 transcript hash length {actual} does not match selected hash length {expected}"
            ),
            Self::OutputLengthMismatch { expected, actual } => write!(
                formatter,
                "TLS 1.3 derived secret length {actual} does not match selected hash length {expected}"
            ),
            Self::Expand(error) => write!(formatter, "TLS 1.3 secret derivation failed: {error}"),
        }
    }
}

impl std::error::Error for DeriveSecretError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TranscriptHashLengthMismatch { .. } | Self::OutputLengthMismatch { .. } => None,
            Self::Expand(error) => Some(error),
        }
    }
}

/// Validated public inputs for one RFC 8446 `HkdfLabel` encoding.
///
/// Private fields make the length bounds an invariant rather than a convention.
/// The borrowed label and context are public protocol metadata, never secret
/// key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HkdfLabel<'a> {
    output_length: u16,
    label: &'a [u8],
    context: &'a [u8],
}

impl<'a> HkdfLabel<'a> {
    /// Validate the complete set of public framing inputs.
    pub fn new(
        output_length: u16,
        label: &'a [u8],
        context: &'a [u8],
    ) -> Result<Self, HkdfLabelError> {
        if label.len() < MIN_LABEL_LENGTH {
            return Err(HkdfLabelError::EmptyLabel);
        }
        if label.len() > MAX_LABEL_LENGTH {
            return Err(HkdfLabelError::LabelTooLong);
        }
        if context.len() > MAX_CONTEXT_LENGTH {
            return Err(HkdfLabelError::ContextTooLong);
        }

        Ok(Self {
            output_length,
            label,
            context,
        })
    }

    /// Return the public output length encoded in the first two bytes.
    #[must_use]
    pub const fn output_length(&self) -> u16 {
        self.output_length
    }

    /// Return the caller label without the fixed TLS namespace prefix.
    #[must_use]
    pub const fn label(&self) -> &'a [u8] {
        self.label
    }

    /// Return the public context bytes.
    #[must_use]
    pub const fn context(&self) -> &'a [u8] {
        self.context
    }

    /// Return the exact caller-owned storage required by [`Self::encode`].
    #[must_use]
    pub const fn encoded_len(&self) -> usize {
        2 + 1 + TLS13_LABEL_PREFIX.len() + self.label.len() + 1 + self.context.len()
    }

    /// Encode the RFC 8446 structure into caller-owned storage.
    ///
    /// Capacity is checked before the first write. On failure the complete
    /// output slice is unchanged. On success only the returned prefix is
    /// modified; any excess caller capacity is left untouched.
    pub fn encode<'output>(
        &self,
        output: &'output mut [u8],
    ) -> Result<&'output [u8], HkdfLabelError> {
        let encoded_len = self.encoded_len();
        if output.len() < encoded_len {
            return Err(HkdfLabelError::OutputTooSmall);
        }

        let prefixed_label_len = TLS13_LABEL_PREFIX.len() + self.label.len();
        let mut cursor = 0;

        output[cursor..cursor + 2].copy_from_slice(&self.output_length.to_be_bytes());
        cursor += 2;

        output[cursor] = prefixed_label_len as u8;
        cursor += 1;
        output[cursor..cursor + TLS13_LABEL_PREFIX.len()].copy_from_slice(TLS13_LABEL_PREFIX);
        cursor += TLS13_LABEL_PREFIX.len();
        output[cursor..cursor + self.label.len()].copy_from_slice(self.label);
        cursor += self.label.len();

        output[cursor] = self.context.len() as u8;
        cursor += 1;
        output[cursor..cursor + self.context.len()].copy_from_slice(self.context);

        Ok(&output[..encoded_len])
    }
}

/// Derive caller-owned output with TLS 1.3 `HKDF-Expand-Label`.
///
/// The public RFC 8446 `HkdfLabel` information is encoded into one fixed-size
/// stack buffer, then passed to repository-owned [`hkdf_expand_fixed`]. The
/// algorithm is carried by `prk`, so framing cannot choose a hash that
/// disagrees with Extract. The function performs no heap allocation.
///
/// Public length, label, and context failures occur before `output` changes.
/// The underlying fixed-memory HKDF boundary preserves that property for its
/// public failures and wipes caller output if an unexpected PRF failure occurs.
/// Successful output remains caller-owned.
///
/// This function does not select a cipher suite, compose a TLS key schedule,
/// process handshake or record state, or provide certificate or transport
/// authority.
///
/// # Example
///
/// ```
/// use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};
/// use tls13_hkdf_label::hkdf_expand_label_fixed;
///
/// let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
/// let mut output = [0u8; 16];
/// hkdf_expand_label_fixed(&prk, b"key", b"transcript hash", &mut output).unwrap();
/// ```
pub fn hkdf_expand_label_fixed(
    prk: &HkdfPrk,
    label: &[u8],
    context: &[u8],
    output: &mut [u8],
) -> Result<(), HkdfExpandLabelError> {
    let output_length =
        u16::try_from(output.len()).map_err(|_| HkdfExpandLabelError::OutputLengthTooLarge)?;
    let label =
        HkdfLabel::new(output_length, label, context).map_err(HkdfExpandLabelError::Framing)?;
    let mut info_storage = [0u8; MAX_ENCODED_LENGTH];
    let info = label
        .encode(&mut info_storage)
        .map_err(HkdfExpandLabelError::Framing)?;

    hkdf_expand_fixed(prk, info, output).map_err(HkdfExpandLabelError::Expand)
}

/// Derive one Hash.length TLS 1.3 secret from an already-computed transcript
/// hash.
///
/// RFC 8446 section 7.1 defines:
///
/// ```text
/// Derive-Secret(Secret, Label, Messages) =
///     HKDF-Expand-Label(Secret, Label, Transcript-Hash(Messages), Hash.length)
/// ```
///
/// This boundary deliberately accepts `transcript_hash`, not `Messages`. It
/// does not own transcript sequencing or hashing. The algorithm carried by the
/// wipe-on-drop `prk` determines the sole accepted transcript-hash and output
/// lengths: 32 bytes for SHA-256 or 48 bytes for SHA-384. Those public lengths
/// are checked before caller output changes, then the operation delegates to
/// [`hkdf_expand_label_fixed`] and inherits its stack-only framing and
/// unexpected-PRF output wipe.
///
/// This function is one key-schedule primitive. It does not label or retain
/// early, handshake, master, traffic, exporter, or resumption secret state and
/// does not provide handshake, record, certificate, transport, or HTTPS
/// authority.
///
/// # Example
///
/// ```
/// use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};
/// use tls13_hkdf_label::derive_secret_from_transcript_hash_fixed;
///
/// let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
/// let transcript_hash = [0x42; 32];
/// let mut secret = [0u8; 32];
/// derive_secret_from_transcript_hash_fixed(
///     &prk,
///     b"c hs traffic",
///     &transcript_hash,
///     &mut secret,
/// )
/// .unwrap();
/// ```
pub fn derive_secret_from_transcript_hash_fixed(
    prk: &HkdfPrk,
    label: &[u8],
    transcript_hash: &[u8],
    output: &mut [u8],
) -> Result<(), DeriveSecretError> {
    let expected = prk.algorithm().hash_len();
    if transcript_hash.len() != expected {
        return Err(DeriveSecretError::TranscriptHashLengthMismatch {
            expected,
            actual: transcript_hash.len(),
        });
    }
    if output.len() != expected {
        return Err(DeriveSecretError::OutputLengthMismatch {
            expected,
            actual: output.len(),
        });
    }

    hkdf_expand_label_fixed(prk, label, transcript_hash, output).map_err(DeriveSecretError::Expand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};

    fn decode_hex(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn encodes_traffic_key_and_iv_labels_exactly() {
        let key = HkdfLabel::new(16, b"key", b"").unwrap();
        let iv = HkdfLabel::new(12, b"iv", b"").unwrap();
        let mut key_output = [0u8; 13];
        let mut iv_output = [0u8; 12];

        assert_eq!(
            key.encode(&mut key_output).unwrap(),
            b"\x00\x10\x09tls13 key\x00"
        );
        assert_eq!(
            iv.encode(&mut iv_output).unwrap(),
            b"\x00\x0c\x08tls13 iv\x00"
        );
    }

    #[test]
    fn retains_public_inputs_and_all_uint16_output_lengths() {
        let context = [0xAA, 0x55];
        let zero = HkdfLabel::new(0, b"derived", &context).unwrap();
        let maximum = HkdfLabel::new(u16::MAX, b"derived", &context).unwrap();

        assert_eq!(zero.output_length(), 0);
        assert_eq!(maximum.output_length(), u16::MAX);
        assert_eq!(zero.label(), b"derived");
        assert_eq!(zero.context(), context);
    }

    #[test]
    fn accepts_maximum_label_and_context_bounds() {
        let label = [b'l'; MAX_LABEL_LENGTH];
        let context = [b'c'; MAX_CONTEXT_LENGTH];
        let framed = HkdfLabel::new(u16::MAX, &label, &context).unwrap();
        let mut output = [0u8; MAX_ENCODED_LENGTH];
        let encoded = framed.encode(&mut output).unwrap();

        assert_eq!(framed.encoded_len(), MAX_ENCODED_LENGTH);
        assert_eq!(encoded.len(), MAX_ENCODED_LENGTH);
        assert_eq!(&encoded[..3], b"\xff\xff\xff");
        assert_eq!(&encoded[3..9], TLS13_LABEL_PREFIX);
        assert_eq!(encoded[9 + MAX_LABEL_LENGTH], u8::MAX);
        assert_eq!(&encoded[10 + MAX_LABEL_LENGTH..], &context);
    }

    #[test]
    fn rejects_empty_and_oversized_inputs() {
        let long_label = [0u8; MAX_LABEL_LENGTH + 1];
        let long_context = [0u8; MAX_CONTEXT_LENGTH + 1];

        assert_eq!(
            HkdfLabel::new(16, b"", b"").unwrap_err(),
            HkdfLabelError::EmptyLabel
        );
        assert_eq!(
            HkdfLabel::new(16, &long_label, b"").unwrap_err(),
            HkdfLabelError::LabelTooLong
        );
        assert_eq!(
            HkdfLabel::new(16, b"key", &long_context).unwrap_err(),
            HkdfLabelError::ContextTooLong
        );
    }

    #[test]
    fn short_output_fails_before_mutation() {
        let framed = HkdfLabel::new(16, b"key", b"").unwrap();
        let mut output = [0xA5; 12];

        assert_eq!(
            framed.encode(&mut output).unwrap_err(),
            HkdfLabelError::OutputTooSmall
        );
        assert_eq!(output, [0xA5; 12]);
    }

    #[test]
    fn excess_output_is_untouched_after_returned_encoding() {
        let framed = HkdfLabel::new(12, b"iv", b"").unwrap();
        let mut output = [0xA5; 16];
        let encoded_len = {
            let encoded = framed.encode(&mut output).unwrap();
            assert_eq!(encoded, b"\x00\x0c\x08tls13 iv\x00");
            encoded.len()
        };

        assert_eq!(&output[encoded_len..], &[0xA5; 4]);
    }

    #[test]
    fn errors_are_stable_and_redacted() {
        assert_eq!(
            HkdfLabelError::EmptyLabel.to_string(),
            "TLS 1.3 HKDF label is empty"
        );
        assert_eq!(
            HkdfLabelError::LabelTooLong.to_string(),
            "TLS 1.3 HKDF label exceeds its bound"
        );
        assert_eq!(
            HkdfLabelError::ContextTooLong.to_string(),
            "TLS 1.3 HKDF context exceeds its bound"
        );
        assert_eq!(
            HkdfLabelError::OutputTooSmall.to_string(),
            "TLS 1.3 HKDF label output is too small"
        );
    }

    #[test]
    fn expands_rfc8448_sha256_derived_secret() {
        // RFC 8448 section 3 derives this value from the all-zero early
        // secret with the SHA-256 empty-string hash as context.
        let context =
            decode_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        let expected =
            decode_hex("6f2615a108c702c5678f54fc9dbab69716c076189c48250cebeac3576c3611ba");
        let zero_ikm = [0u8; 32];
        let prk = hkdf_extract_fixed(&[], &zero_ikm, FixedHkdfHash::Sha256).unwrap();
        let mut output = [0u8; 32];

        hkdf_expand_label_fixed(&prk, b"derived", &context, &mut output).unwrap();

        assert_eq!(output.as_slice(), expected);
    }

    #[test]
    fn derives_rfc8448_sha256_secret() {
        // RFC 8448 section 3 derives this value from the all-zero early
        // secret with the SHA-256 empty-string transcript hash as context.
        let transcript_hash =
            decode_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        let expected =
            decode_hex("6f2615a108c702c5678f54fc9dbab69716c076189c48250cebeac3576c3611ba");
        let zero_ikm = [0u8; 32];
        let prk = hkdf_extract_fixed(&[], &zero_ikm, FixedHkdfHash::Sha256).unwrap();
        let mut output = [0u8; 32];

        derive_secret_from_transcript_hash_fixed(&prk, b"derived", &transcript_hash, &mut output)
            .unwrap();

        assert_eq!(output.as_slice(), expected);
    }

    #[test]
    fn derives_independent_sha384_secret() {
        // Independently generated from RFC 8446 section 7.1 with Python's
        // standard-library SHA-384 and HMAC implementations.
        let transcript_hash = decode_hex(
            "37a8d6d4980e12984ba0607a9e84548acd518c5aa7c94890eae3854dd734ab2e\
             80e8643cf6cb10ae7b467fa9fa3c9034",
        );
        let expected = decode_hex(
            "78e3bfd2b19036f0ea2ec81b0c35ca3c66bf04dd54dbcb2ae37d642f8ac723e6\
             50e68bcdf6aa2a01fd0d43298efdaeb3",
        );
        let prk = hkdf_extract_fixed(
            b"derive-secret-salt",
            b"derive-secret-input",
            FixedHkdfHash::Sha384,
        )
        .unwrap();
        let mut output = [0u8; 48];

        derive_secret_from_transcript_hash_fixed(
            &prk,
            b"c hs traffic",
            &transcript_hash,
            &mut output,
        )
        .unwrap();

        assert_eq!(output.as_slice(), expected);
    }

    #[test]
    fn derive_secret_public_failures_leave_output_unchanged() {
        let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
        let transcript_hash = [0x42; 32];
        let short_transcript_hash = [0x42; 31];
        let oversized_label = [b'l'; MAX_LABEL_LENGTH + 1];
        let mut output = [0xA5; 32];
        let mut short_output = [0xA5; 31];

        assert_eq!(
            derive_secret_from_transcript_hash_fixed(
                &prk,
                b"derived",
                &short_transcript_hash,
                &mut output,
            ),
            Err(DeriveSecretError::TranscriptHashLengthMismatch {
                expected: 32,
                actual: 31,
            })
        );
        assert_eq!(output, [0xA5; 32]);

        assert_eq!(
            derive_secret_from_transcript_hash_fixed(
                &prk,
                b"derived",
                &transcript_hash,
                &mut short_output,
            ),
            Err(DeriveSecretError::OutputLengthMismatch {
                expected: 32,
                actual: 31,
            })
        );
        assert_eq!(short_output, [0xA5; 31]);

        assert_eq!(
            derive_secret_from_transcript_hash_fixed(
                &prk,
                &oversized_label,
                &transcript_hash,
                &mut output,
            ),
            Err(DeriveSecretError::Expand(HkdfExpandLabelError::Framing(
                HkdfLabelError::LabelTooLong
            )))
        );
        assert_eq!(output, [0xA5; 32]);
    }

    #[test]
    fn derive_secret_errors_are_stable_and_payload_free() {
        assert_eq!(
            DeriveSecretError::TranscriptHashLengthMismatch {
                expected: 48,
                actual: 32,
            }
            .to_string(),
            "TLS 1.3 transcript hash length 32 does not match selected hash length 48"
        );
        assert_eq!(
            DeriveSecretError::OutputLengthMismatch {
                expected: 32,
                actual: 31,
            }
            .to_string(),
            "TLS 1.3 derived secret length 31 does not match selected hash length 32"
        );
        assert_eq!(
            DeriveSecretError::Expand(HkdfExpandLabelError::Framing(
                HkdfLabelError::EmptyLabel,
            ))
            .to_string(),
            "TLS 1.3 secret derivation failed: TLS 1.3 HKDF framing failed: TLS 1.3 HKDF label is empty"
        );
    }

    #[test]
    fn expands_sha384_across_three_hmac_blocks() {
        // Independently generated from RFC 8446 section 7.1 with the Python
        // standard library's SHA-384 HMAC implementation.
        let expected = decode_hex(
            "29e2f37a0cf31153c6b9760f5518eb5159d0db25f774e360927894d5bb9d3d5e\
             83c4aa7756ecdd8db31c9b4dc79b128d1fa620f7748b7cc0eb984058e4619511\
             f264c7b6d50638431e7d63b245058b5f819a32416ac84269e4a448c5ccc56c1f1f",
        );
        let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha384).unwrap();
        let mut output = [0u8; 97];

        hkdf_expand_label_fixed(&prk, b"key", b"sha384 context", &mut output).unwrap();

        assert_eq!(output.as_slice(), expected);
    }

    #[test]
    fn public_failures_leave_output_unchanged() {
        let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
        let mut output = [0xA5; 16];
        let oversized_label = [b'l'; MAX_LABEL_LENGTH + 1];
        let oversized_context = [b'c'; MAX_CONTEXT_LENGTH + 1];

        assert_eq!(
            hkdf_expand_label_fixed(&prk, b"", b"", &mut output),
            Err(HkdfExpandLabelError::Framing(HkdfLabelError::EmptyLabel))
        );
        assert_eq!(output, [0xA5; 16]);
        assert_eq!(
            hkdf_expand_label_fixed(&prk, &oversized_label, b"", &mut output),
            Err(HkdfExpandLabelError::Framing(HkdfLabelError::LabelTooLong))
        );
        assert_eq!(output, [0xA5; 16]);
        assert_eq!(
            hkdf_expand_label_fixed(&prk, b"key", &oversized_context, &mut output),
            Err(HkdfExpandLabelError::Framing(
                HkdfLabelError::ContextTooLong
            ))
        );
        assert_eq!(output, [0xA5; 16]);
    }

    #[test]
    fn hkdf_length_failures_leave_output_unchanged() {
        let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
        let mut empty = [];
        assert_eq!(
            hkdf_expand_label_fixed(&prk, b"key", b"", &mut empty),
            Err(HkdfExpandLabelError::Expand(HkdfError::OutputTooShort))
        );

        let maximum = 255 * FixedHkdfHash::Sha256.hash_len();
        let mut oversized = vec![0xA5; maximum + 1];
        assert_eq!(
            hkdf_expand_label_fixed(&prk, b"key", b"", &mut oversized),
            Err(HkdfExpandLabelError::Expand(HkdfError::OutputTooLong {
                requested: maximum + 1,
                maximum,
            }))
        );
        assert!(oversized.iter().all(|byte| *byte == 0xA5));

        let mut unrepresentable = vec![0xA5; u16::MAX as usize + 1];
        assert_eq!(
            hkdf_expand_label_fixed(&prk, b"key", b"", &mut unrepresentable),
            Err(HkdfExpandLabelError::OutputLengthTooLarge)
        );
        assert!(unrepresentable.iter().all(|byte| *byte == 0xA5));
    }

    #[test]
    fn composition_errors_are_stable_and_redacted() {
        assert_eq!(
            HkdfExpandLabelError::OutputLengthTooLarge.to_string(),
            "TLS 1.3 HKDF output exceeds its framing bound"
        );
        assert_eq!(
            HkdfExpandLabelError::Framing(HkdfLabelError::EmptyLabel).to_string(),
            "TLS 1.3 HKDF framing failed: TLS 1.3 HKDF label is empty"
        );
        assert_eq!(
            HkdfExpandLabelError::Expand(HkdfError::OutputTooShort).to_string(),
            "TLS 1.3 HKDF expansion failed: HKDF output length must be positive"
        );
    }
}
