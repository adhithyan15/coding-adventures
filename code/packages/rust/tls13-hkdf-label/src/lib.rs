//! Bounded, allocation-free TLS 1.3 `HkdfLabel` framing.
//!
//! RFC 8446 section 7.1 domain-separates every TLS 1.3 HKDF expansion by
//! serializing a public output length, the fixed `"tls13 "` namespace plus a
//! caller label, and a public context. This crate owns only that serialization:
//! it accepts no secret and invokes no hash or HKDF implementation.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
