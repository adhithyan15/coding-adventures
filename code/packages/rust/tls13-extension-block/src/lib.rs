//! Bounded allocation-free TLS 1.3 extension-block decoding.
//!
//! TLS handshake messages carry vectors of `Extension` values. Each vector is
//! encoded as one two-byte payload length followed by zero or more entries,
//! where every entry contains a two-byte type, a two-byte data length, and the
//! exact opaque data bytes.
//!
//! [`decode_extension_block`] validates that complete framing under explicit
//! caller-owned byte and count limits, rejects duplicate extension types, and
//! retains unknown types and data as borrowed values in wire order. It embeds
//! no extension registry and applies no handshake-message-specific policy.
//!
//! This crate does not decode extension contents, authorize an extension in a
//! protocol context, process a handshake, invoke cryptography, perform I/O, or
//! provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest extension count retained by this package.
pub const MAX_EXTENSIONS: usize = 64;

/// Largest payload representable by TLS's two-byte vector length.
pub const MAX_EXTENSION_BLOCK_PAYLOAD_LENGTH: usize = u16::MAX as usize;

/// Stable payload-free failures for limits and extension-block framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionBlockError {
    /// A caller-owned byte or count limit was outside reviewed bounds.
    InvalidLimits,
    /// The two-byte extension-vector length was absent or incomplete.
    MissingBlockLength,
    /// The declared vector length did not equal the complete remaining input.
    BlockLengthMismatch,
    /// The exact vector payload exceeded the caller-owned byte ceiling.
    BlockTooLong,
    /// An extension's four-byte type-and-length header was incomplete.
    TruncatedExtensionHeader,
    /// An extension's declared opaque data was incomplete.
    TruncatedExtensionData,
    /// The vector contained more entries than the caller-owned count ceiling.
    TooManyExtensions,
    /// One extension type appeared more than once in the same vector.
    DuplicateExtension,
}

impl fmt::Display for ExtensionBlockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid TLS extension-block limits",
            Self::MissingBlockLength => "missing TLS extension-block length",
            Self::BlockLengthMismatch => "TLS extension-block length mismatch",
            Self::BlockTooLong => "TLS extension block exceeds byte limit",
            Self::TruncatedExtensionHeader => "truncated TLS extension header",
            Self::TruncatedExtensionData => "truncated TLS extension data",
            Self::TooManyExtensions => "TLS extension block exceeds count limit",
            Self::DuplicateExtension => "duplicate TLS extension type",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExtensionBlockError {}

/// Caller-owned resource limits for one encoded extension block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionBlockLimits {
    max_payload_length: u16,
    max_extensions: u8,
}

impl ExtensionBlockLimits {
    /// Validate one payload-byte ceiling and one nonzero entry-count ceiling.
    pub fn new(
        max_payload_length: usize,
        max_extensions: usize,
    ) -> Result<Self, ExtensionBlockError> {
        if max_payload_length > MAX_EXTENSION_BLOCK_PAYLOAD_LENGTH
            || !(1..=MAX_EXTENSIONS).contains(&max_extensions)
        {
            return Err(ExtensionBlockError::InvalidLimits);
        }

        Ok(Self {
            max_payload_length: max_payload_length as u16,
            max_extensions: max_extensions as u8,
        })
    }

    /// Return the maximum accepted extension-vector payload length.
    #[must_use]
    pub const fn max_payload_length(self) -> usize {
        self.max_payload_length as usize
    }

    /// Return the maximum accepted number of extension entries.
    #[must_use]
    pub const fn max_extensions(self) -> usize {
        self.max_extensions as usize
    }
}

/// One exact two-byte TLS extension type.
///
/// The value is syntax only. It does not prove that the type is registered,
/// supported, or authorized in a particular handshake message.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ExtensionType(u16);

impl ExtensionType {
    /// Retain one exact extension-type wire value without applying a registry.
    #[must_use]
    pub const fn from_wire_value(value: u16) -> Self {
        Self(value)
    }

    /// Return the exact retained extension-type wire value.
    #[must_use]
    pub const fn wire_value(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for ExtensionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ExtensionType")
            .field(&self.0)
            .finish()
    }
}

/// One structurally decoded TLS extension in wire order.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Extension<'input> {
    extension_type: ExtensionType,
    data: &'input [u8],
}

impl<'input> Extension<'input> {
    /// Return this entry's exact unclassified extension type.
    #[must_use]
    pub const fn extension_type(self) -> ExtensionType {
        self.extension_type
    }

    /// Borrow this entry's complete opaque extension data.
    #[must_use]
    pub const fn data(self) -> &'input [u8] {
        self.data
    }
}

impl fmt::Debug for Extension<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Extension")
            .field("extension_type", &self.extension_type)
            .field("data_length", &self.data.len())
            .field("data", &"[OPAQUE]")
            .finish()
    }
}

const EMPTY_EXTENSION: Extension<'static> = Extension {
    extension_type: ExtensionType(0),
    data: &[],
};

/// One exact decoded TLS extension vector with fixed-capacity borrowed entries.
pub struct ExtensionBlock<'input> {
    entries: [Extension<'input>; MAX_EXTENSIONS],
    count: u8,
    payload_length: u16,
}

impl<'input> ExtensionBlock<'input> {
    /// Return the number of retained extension entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.count as usize
    }

    /// Return whether this syntactically valid extension vector is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the exact declared payload length, excluding its two-byte prefix.
    #[must_use]
    pub const fn payload_length(&self) -> usize {
        self.payload_length as usize
    }

    /// Borrow the decoded entries in their original wire order.
    #[must_use]
    pub fn entries(&self) -> &[Extension<'input>] {
        &self.entries[..self.len()]
    }

    /// Return the unique entry with one exact type, if present.
    #[must_use]
    pub fn find(&self, extension_type: ExtensionType) -> Option<Extension<'input>> {
        self.entries()
            .iter()
            .copied()
            .find(|entry| entry.extension_type == extension_type)
    }
}

impl fmt::Debug for ExtensionBlock<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtensionBlock")
            .field("entry_count", &self.len())
            .field("payload_length", &self.payload_length())
            .field("entry_data", &"[OPAQUE]")
            .finish_non_exhaustive()
    }
}

/// Decode one exact two-byte-length-prefixed TLS extension vector.
///
/// The complete caller input must contain exactly one vector: trailing bytes
/// are rejected by the same declared-length equality check as truncation.
/// Unknown extension types and opaque data are retained without interpretation.
/// Duplicate types fail closed before a second value is retained.
pub fn decode_extension_block(
    input: &[u8],
    limits: ExtensionBlockLimits,
) -> Result<ExtensionBlock<'_>, ExtensionBlockError> {
    if input.len() < 2 {
        return Err(ExtensionBlockError::MissingBlockLength);
    }

    let payload_length = u16::from_be_bytes([input[0], input[1]]) as usize;
    let payload = &input[2..];
    if payload.len() != payload_length {
        return Err(ExtensionBlockError::BlockLengthMismatch);
    }
    if payload_length > limits.max_payload_length() {
        return Err(ExtensionBlockError::BlockTooLong);
    }

    let mut block = ExtensionBlock {
        entries: [EMPTY_EXTENSION; MAX_EXTENSIONS],
        count: 0,
        payload_length: payload_length as u16,
    };
    let mut offset = 0;

    while offset < payload.len() {
        if block.len() >= limits.max_extensions() {
            return Err(ExtensionBlockError::TooManyExtensions);
        }
        if payload.len() - offset < 4 {
            return Err(ExtensionBlockError::TruncatedExtensionHeader);
        }

        let extension_type =
            ExtensionType(u16::from_be_bytes([payload[offset], payload[offset + 1]]));
        let data_length = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
        let data_start = offset + 4;
        let data_end = data_start + data_length;
        if data_end > payload.len() {
            return Err(ExtensionBlockError::TruncatedExtensionData);
        }
        if block
            .entries()
            .iter()
            .any(|entry| entry.extension_type == extension_type)
        {
            return Err(ExtensionBlockError::DuplicateExtension);
        }

        block.entries[block.len()] = Extension {
            extension_type,
            data: &payload[data_start..data_end],
        };
        block.count += 1;
        offset = data_end;
    }

    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(bytes: usize, count: usize) -> ExtensionBlockLimits {
        ExtensionBlockLimits::new(bytes, count).unwrap()
    }

    #[test]
    fn decodes_empty_known_and_unknown_entries_in_wire_order() {
        let empty = decode_extension_block(&[0, 0], limits(64, 8)).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.payload_length(), 0);

        let encoded = [
            0, 11, // complete payload length
            0, 43, 0, 2, 3, 4, // supported_versions syntax, still opaque here
            0xff, 0x01, 0, 1, 0xaa, // unknown private-use type
        ];
        let block = decode_extension_block(&encoded, limits(64, 8)).unwrap();

        assert_eq!(block.len(), 2);
        assert_eq!(block.payload_length(), 11);
        assert_eq!(block.entries()[0].extension_type().wire_value(), 43);
        assert_eq!(block.entries()[0].data(), &[3, 4]);
        assert_eq!(block.entries()[1].extension_type().wire_value(), 0xff01);
        assert_eq!(block.entries()[1].data(), &[0xaa]);
        assert_eq!(
            block
                .find(ExtensionType::from_wire_value(43))
                .unwrap()
                .data(),
            &[3, 4]
        );
        assert_eq!(block.find(ExtensionType::from_wire_value(44)), None);
    }

    #[test]
    fn rejects_length_prefix_and_entry_truncation() {
        assert_eq!(
            decode_extension_block(&[], limits(64, 8)).unwrap_err(),
            ExtensionBlockError::MissingBlockLength
        );
        assert_eq!(
            decode_extension_block(&[0], limits(64, 8)).unwrap_err(),
            ExtensionBlockError::MissingBlockLength
        );
        assert_eq!(
            decode_extension_block(&[0, 1], limits(64, 8)).unwrap_err(),
            ExtensionBlockError::BlockLengthMismatch
        );
        assert_eq!(
            decode_extension_block(&[0, 3, 0, 1, 0], limits(64, 8)).unwrap_err(),
            ExtensionBlockError::TruncatedExtensionHeader
        );
        assert_eq!(
            decode_extension_block(&[0, 6, 0, 1, 0, 3, 0xaa, 0xbb], limits(64, 8)).unwrap_err(),
            ExtensionBlockError::TruncatedExtensionData
        );
    }

    #[test]
    fn rejects_duplicate_types_and_limit_exhaustion() {
        let duplicate = [0, 8, 0, 1, 0, 0, 0, 1, 0, 0];
        assert_eq!(
            decode_extension_block(&duplicate, limits(64, 8)).unwrap_err(),
            ExtensionBlockError::DuplicateExtension
        );

        let two_entries = [0, 8, 0, 1, 0, 0, 0, 2, 0, 0];
        assert_eq!(
            decode_extension_block(&two_entries, limits(64, 1)).unwrap_err(),
            ExtensionBlockError::TooManyExtensions
        );
        assert_eq!(
            decode_extension_block(&two_entries, limits(7, 8)).unwrap_err(),
            ExtensionBlockError::BlockTooLong
        );
    }

    #[test]
    fn accepts_the_fixed_entry_capacity_without_allocation() {
        let payload_length = (MAX_EXTENSIONS * 4) as u16;
        let mut encoded = payload_length.to_be_bytes().to_vec();
        for value in 0..MAX_EXTENSIONS as u16 {
            encoded.extend_from_slice(&value.to_be_bytes());
            encoded.extend_from_slice(&0u16.to_be_bytes());
        }

        let block = decode_extension_block(
            &encoded,
            limits(MAX_EXTENSION_BLOCK_PAYLOAD_LENGTH, MAX_EXTENSIONS),
        )
        .unwrap();
        assert_eq!(block.len(), MAX_EXTENSIONS);
        assert_eq!(block.entries()[63].extension_type().wire_value(), 63);
    }

    #[test]
    fn validates_caller_owned_limits() {
        assert_eq!(
            ExtensionBlockLimits::new(MAX_EXTENSION_BLOCK_PAYLOAD_LENGTH + 1, 1).unwrap_err(),
            ExtensionBlockError::InvalidLimits
        );
        assert_eq!(
            ExtensionBlockLimits::new(0, 0).unwrap_err(),
            ExtensionBlockError::InvalidLimits
        );
        assert_eq!(
            ExtensionBlockLimits::new(0, MAX_EXTENSIONS + 1).unwrap_err(),
            ExtensionBlockError::InvalidLimits
        );
        assert_eq!(limits(0, 1).max_payload_length(), 0);
        assert_eq!(limits(0, 1).max_extensions(), 1);
    }

    #[test]
    fn diagnostics_are_stable_and_do_not_expose_opaque_data() {
        let encoded = [0, 8, 0, 1, 0, 4, 0xde, 0xad, 0xbe, 0xef];
        let block = decode_extension_block(&encoded, limits(8, 1)).unwrap();
        let block_debug = format!("{block:?}");
        let entry_debug = format!("{:?}", block.entries()[0]);
        assert!(block_debug.contains("[OPAQUE]"));
        assert!(entry_debug.contains("[OPAQUE]"));
        assert!(!block_debug.contains("deadbeef"));
        assert!(!entry_debug.contains("deadbeef"));

        let errors = [
            ExtensionBlockError::InvalidLimits,
            ExtensionBlockError::MissingBlockLength,
            ExtensionBlockError::BlockLengthMismatch,
            ExtensionBlockError::BlockTooLong,
            ExtensionBlockError::TruncatedExtensionHeader,
            ExtensionBlockError::TruncatedExtensionData,
            ExtensionBlockError::TooManyExtensions,
            ExtensionBlockError::DuplicateExtension,
        ];
        for error in errors {
            assert!(!error.to_string().contains("0x"));
            assert!(!format!("{error:?}").contains("0x"));
        }
    }
}
