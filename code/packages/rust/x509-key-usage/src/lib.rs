//! # Bounded RFC 5280 Key Usage values
//!
//! This crate recognizes and decodes only the Key Usage extension. It
//! deliberately grants no certificate-role, certificate-path, trust-root, or
//! cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{decode_bit_string, Asn1Decoder, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;
use x509_extension::X509Extension;

/// RFC 5280 `id-ce-keyUsage` (`2.5.29.15`).
pub const KEY_USAGE_OID: &[u64] = &[2, 5, 29, 15];

const DEFINED_BIT_COUNT: usize = 9;

/// One defined RFC 5280 Key Usage bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum KeyUsageBit {
    DigitalSignature = 1 << 0,
    ContentCommitment = 1 << 1,
    KeyEncipherment = 1 << 2,
    DataEncipherment = 1 << 3,
    KeyAgreement = 1 << 4,
    KeyCertSign = 1 << 5,
    CrlSign = 1 << 6,
    EncipherOnly = 1 << 7,
    DecipherOnly = 1 << 8,
}

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyUsageErrorKind {
    WrongExtensionId,
    Structure(Asn1ErrorKind),
    InvalidBitString(Asn1ErrorKind),
    Empty,
    UndefinedBit,
    NonMinimalNamedBitString,
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyUsageError {
    kind: KeyUsageErrorKind,
    offset: usize,
}

impl KeyUsageError {
    const fn new(kind: KeyUsageErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(KeyUsageErrorKind::Structure(error.kind()), error.offset())
    }

    fn bit_string(error: Asn1Error) -> Self {
        Self::new(
            KeyUsageErrorKind::InvalidBitString(error.kind()),
            error.offset(),
        )
    }

    pub const fn kind(self) -> KeyUsageErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for KeyUsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 key usage error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for KeyUsageError {}

/// One validated RFC 5280 Key Usage value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyUsage {
    bits: u16,
}

impl KeyUsage {
    /// Return whether one defined usage is asserted.
    pub const fn contains(self, usage: KeyUsageBit) -> bool {
        self.bits & usage as u16 != 0
    }

    /// Return the normalized nine-bit mask, with RFC bit zero at bit zero.
    pub const fn bits(self) -> u16 {
        self.bits
    }
}

/// Decode one generic extension as exact RFC 5280 Key Usage.
pub fn decode_key_usage(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'_>,
) -> Result<KeyUsage, KeyUsageError> {
    if !extension.extension_id().equals(KEY_USAGE_OID) {
        return Err(KeyUsageError::new(KeyUsageErrorKind::WrongExtensionId, 0));
    }

    let element = decoder
        .decode_exact(extension.extension_value())
        .map_err(KeyUsageError::structure)?;
    let value_offset = element.header().len();
    let bit_string = decode_bit_string(element).map_err(KeyUsageError::bit_string)?;
    let bytes = bit_string.bytes();

    if bit_string.bit_len() == 0 {
        return Err(KeyUsageError::new(KeyUsageErrorKind::Empty, value_offset));
    }

    for bit in DEFINED_BIT_COUNT..bit_string.bit_len() {
        if named_bit_is_set(bytes, bit) {
            return Err(KeyUsageError::new(
                KeyUsageErrorKind::UndefinedBit,
                value_offset + 1 + bit / 8,
            ));
        }
    }

    let last_bit = bit_string.bit_len() - 1;
    if !named_bit_is_set(bytes, last_bit) {
        return Err(KeyUsageError::new(
            KeyUsageErrorKind::NonMinimalNamedBitString,
            value_offset + 1 + last_bit / 8,
        ));
    }

    let mut bits = 0u16;
    for bit in 0..bit_string.bit_len().min(DEFINED_BIT_COUNT) {
        if named_bit_is_set(bytes, bit) {
            bits |= 1u16 << bit;
        }
    }

    Ok(KeyUsage { bits })
}

fn named_bit_is_set(bytes: &[u8], bit: usize) -> bool {
    bytes[bit / 8] & (0x80 >> (bit % 8)) != 0
}
