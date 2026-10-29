//! # Bounded RSA-PSS encoded-message layout
//!
//! RFC 8017 EMSA-PSS verification first interprets the complete recovered
//! encoded message as `maskedDB || H || 0xbc`. This crate validates only that
//! public outer layout using caller-selected hash and salt lengths.
//!
//! It does not unmask `DB`, run MGF1, hash a message, compare `H`, perform RSA
//! arithmetic, or establish a signature verdict.

#![deny(unsafe_code)]

use core::fmt;

/// RFC 8017 EMSA-PSS's required final octet.
pub const RSA_PSS_TRAILER_FIELD: u8 = 0xbc;

/// Stable invalid-policy categories that retain no caller-supplied values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssEncodedMessageLayoutPolicyErrorKind {
    /// A selected hash must produce at least one octet.
    ZeroHashLength,
    /// A zero ceiling admits no encoded message.
    ZeroMaximumEncodedMessageBytes,
    /// The selected lengths cannot be represented by `usize`.
    LengthOverflow,
    /// The ceiling cannot hold the selected hash, salt, delimiter, and trailer.
    MaximumEncodedMessageTooSmall,
}

/// A redacted policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssEncodedMessageLayoutPolicyError {
    kind: RsaPssEncodedMessageLayoutPolicyErrorKind,
}

impl RsaPssEncodedMessageLayoutPolicyError {
    const fn new(kind: RsaPssEncodedMessageLayoutPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssEncodedMessageLayoutPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssEncodedMessageLayoutPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssEncodedMessageLayoutPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssEncodedMessageLayoutPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS layout policy error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssEncodedMessageLayoutPolicyError {}

/// Validated public sizing policy for one selected RSA-PSS algorithm profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssEncodedMessageLayoutPolicy {
    hash_length: usize,
    salt_length: usize,
    maximum_encoded_message_bytes: usize,
    minimum_encoded_message_bytes: usize,
}

impl RsaPssEncodedMessageLayoutPolicy {
    /// Validate selected hash/salt lengths and an encoded-message work ceiling.
    ///
    /// A zero-length salt is valid. The minimum follows RFC 8017's
    /// `emLen >= hLen + sLen + 2` requirement.
    pub const fn new(
        hash_length: usize,
        salt_length: usize,
        maximum_encoded_message_bytes: usize,
    ) -> Result<Self, RsaPssEncodedMessageLayoutPolicyError> {
        if hash_length == 0 {
            return Err(RsaPssEncodedMessageLayoutPolicyError::new(
                RsaPssEncodedMessageLayoutPolicyErrorKind::ZeroHashLength,
            ));
        }
        if maximum_encoded_message_bytes == 0 {
            return Err(RsaPssEncodedMessageLayoutPolicyError::new(
                RsaPssEncodedMessageLayoutPolicyErrorKind::ZeroMaximumEncodedMessageBytes,
            ));
        }
        let hash_and_salt = match hash_length.checked_add(salt_length) {
            Some(length) => length,
            None => {
                return Err(RsaPssEncodedMessageLayoutPolicyError::new(
                    RsaPssEncodedMessageLayoutPolicyErrorKind::LengthOverflow,
                ))
            }
        };
        let minimum_encoded_message_bytes = match hash_and_salt.checked_add(2) {
            Some(length) => length,
            None => {
                return Err(RsaPssEncodedMessageLayoutPolicyError::new(
                    RsaPssEncodedMessageLayoutPolicyErrorKind::LengthOverflow,
                ))
            }
        };
        if maximum_encoded_message_bytes < minimum_encoded_message_bytes {
            return Err(RsaPssEncodedMessageLayoutPolicyError::new(
                RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall,
            ));
        }
        Ok(Self {
            hash_length,
            salt_length,
            maximum_encoded_message_bytes,
            minimum_encoded_message_bytes,
        })
    }

    /// Return the selected hash output length.
    pub const fn hash_length(self) -> usize {
        self.hash_length
    }

    /// Return the selected salt length.
    pub const fn salt_length(self) -> usize {
        self.salt_length
    }

    /// Return the caller-owned encoded-message work ceiling.
    pub const fn maximum_encoded_message_bytes(self) -> usize {
        self.maximum_encoded_message_bytes
    }

    /// Return the minimum encoded width allowed by the selected parameters.
    pub const fn minimum_encoded_message_bytes(self) -> usize {
        self.minimum_encoded_message_bytes
    }
}

/// Stable layout-failure categories that retain no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssEncodedMessageLayoutErrorKind {
    /// The encoded input exceeds the caller-owned work ceiling.
    EncodedMessageTooLongForPolicy,
    /// RFC 8017 requires a positive encoded-message bit length.
    ZeroEncodedMessageBits,
    /// The recovered octet count differs from `ceil(emBits / 8)`.
    EncodedMessageLengthMismatch,
    /// The exact width cannot hold the selected hash and salt parameters.
    EncodedMessageTooShortForParameters,
    /// The final octet is not RFC 8017's `0xbc` trailer field.
    InvalidTrailerField,
    /// Bits above the declared `emBits` domain are nonzero in `maskedDB`.
    NonzeroUnusedHighBits,
}

/// A redacted RSA-PSS encoded-message layout failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssEncodedMessageLayoutError {
    kind: RsaPssEncodedMessageLayoutErrorKind,
}

impl RsaPssEncodedMessageLayoutError {
    const fn new(kind: RsaPssEncodedMessageLayoutErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssEncodedMessageLayoutErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssEncodedMessageLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssEncodedMessageLayoutError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssEncodedMessageLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS encoded-message layout error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssEncodedMessageLayoutError {}

/// Borrowed evidence that one public encoded message has an accepted layout.
///
/// This type is deliberately neither `Copy` nor `Clone`: the evidence retains
/// the exact complete input and its field slices for the next verification
/// stage. It proves no PSS padding, hash, RSA signature, or trust verdict.
pub struct ValidatedRsaPssEncodedMessageLayout<'encoded> {
    encoded_message: &'encoded [u8],
    masked_data_block: &'encoded [u8],
    hash: &'encoded [u8],
    encoded_message_bits: usize,
    salt_length: usize,
    unused_high_bits: u8,
}

impl<'encoded> ValidatedRsaPssEncodedMessageLayout<'encoded> {
    /// Return the complete exact recovered encoded message.
    pub const fn encoded_message(&self) -> &'encoded [u8] {
        self.encoded_message
    }

    /// Return the exact borrowed `maskedDB` field.
    pub const fn masked_data_block(&self) -> &'encoded [u8] {
        self.masked_data_block
    }

    /// Return the exact borrowed `H` field.
    pub const fn hash(&self) -> &'encoded [u8] {
        self.hash
    }

    /// Return the caller-declared public `emBits`.
    pub const fn encoded_message_bits(&self) -> usize {
        self.encoded_message_bits
    }

    /// Return the policy-selected salt length for the later DB validator.
    pub const fn salt_length(&self) -> usize {
        self.salt_length
    }

    /// Return the number of forbidden high bits already proven zero.
    pub const fn unused_high_bits(&self) -> u8 {
        self.unused_high_bits
    }
}

impl fmt::Debug for ValidatedRsaPssEncodedMessageLayout<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssEncodedMessageLayout")
    }
}

/// Validate the public outer layout of one exact RFC 8017 EMSA-PSS value.
///
/// `encoded_message_bits` is `emBits`, normally one less than the validated RSA
/// modulus bit length. This function validates only `maskedDB || H || 0xbc`;
/// it does not run MGF1, interpret the unmasked data block, hash, or verify RSA.
pub fn decode_rsa_pss_encoded_message_layout<'encoded>(
    encoded_message: &'encoded [u8],
    encoded_message_bits: usize,
    policy: RsaPssEncodedMessageLayoutPolicy,
) -> Result<ValidatedRsaPssEncodedMessageLayout<'encoded>, RsaPssEncodedMessageLayoutError> {
    if encoded_message.len() > policy.maximum_encoded_message_bytes {
        return Err(RsaPssEncodedMessageLayoutError::new(
            RsaPssEncodedMessageLayoutErrorKind::EncodedMessageTooLongForPolicy,
        ));
    }
    if encoded_message_bits == 0 {
        return Err(RsaPssEncodedMessageLayoutError::new(
            RsaPssEncodedMessageLayoutErrorKind::ZeroEncodedMessageBits,
        ));
    }

    let encoded_message_length =
        encoded_message_bits / 8 + usize::from(!encoded_message_bits.is_multiple_of(8));
    if encoded_message.len() != encoded_message_length {
        return Err(RsaPssEncodedMessageLayoutError::new(
            RsaPssEncodedMessageLayoutErrorKind::EncodedMessageLengthMismatch,
        ));
    }
    if encoded_message.len() < policy.minimum_encoded_message_bytes {
        return Err(RsaPssEncodedMessageLayoutError::new(
            RsaPssEncodedMessageLayoutErrorKind::EncodedMessageTooShortForParameters,
        ));
    }
    if encoded_message.last() != Some(&RSA_PSS_TRAILER_FIELD) {
        return Err(RsaPssEncodedMessageLayoutError::new(
            RsaPssEncodedMessageLayoutErrorKind::InvalidTrailerField,
        ));
    }

    let hash_start = encoded_message.len() - policy.hash_length - 1;
    let masked_data_block = &encoded_message[..hash_start];
    let hash = &encoded_message[hash_start..encoded_message.len() - 1];
    let remainder_bits = (encoded_message_bits % 8) as u8;
    let unused_high_bits = if remainder_bits == 0 {
        0
    } else {
        8 - remainder_bits
    };
    if unused_high_bits != 0 {
        let forbidden_high_bits = u8::MAX << (8 - unused_high_bits);
        if masked_data_block[0] & forbidden_high_bits != 0 {
            return Err(RsaPssEncodedMessageLayoutError::new(
                RsaPssEncodedMessageLayoutErrorKind::NonzeroUnusedHighBits,
            ));
        }
    }

    Ok(ValidatedRsaPssEncodedMessageLayout {
        encoded_message,
        masked_data_block,
        hash,
        encoded_message_bits,
        salt_length: policy.salt_length,
        unused_high_bits,
    })
}
