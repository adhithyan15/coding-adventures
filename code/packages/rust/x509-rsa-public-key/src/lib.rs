//! # Bounded X.509 RSA public keys
//!
//! A generic `SubjectPublicKeyInfo` says only that an algorithm identifier and
//! a BIT STRING were well formed. RSA needs a second, deliberately narrower
//! boundary: the identifier must be `rsaEncryption` with NULL parameters, and
//! the BIT STRING must contain exactly one canonical PKCS #1 `RSAPublicKey`.
//! This crate performs that interpretation without allocating or acquiring any
//! signing, verification, trust, TLS, clock, or network authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_integer, decode_null, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, Asn1Limits,
    DerInteger,
};
use std::error::Error;
use std::fmt;
use x509_subject_public_key_info::X509SubjectPublicKeyInfo;

/// RFC 3279 `rsaEncryption` (`1.2.840.113549.1.1.1`).
///
/// This identifies only the PKCS #1 public-key representation. It does not
/// select a certificate-signature scheme or enable an RSA implementation.
pub const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113549, 1, 1, 1];

/// Stable caller-policy construction failures. No category retains policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RsaPublicKeyPolicyDefinitionErrorKind {
    ZeroMinimumModulusBits,
    InvertedModulusBitRange,
    EmptyAllowedPublicExponents,
    InvalidAllowedPublicExponent,
    DuplicateAllowedPublicExponent,
}

/// A redacted invalid-policy error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RsaPublicKeyPolicyDefinitionError {
    kind: RsaPublicKeyPolicyDefinitionErrorKind,
}

impl RsaPublicKeyPolicyDefinitionError {
    const fn new(kind: RsaPublicKeyPolicyDefinitionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> RsaPublicKeyPolicyDefinitionErrorKind {
        self.kind
    }
}

impl fmt::Display for RsaPublicKeyPolicyDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA public-key policy definition error {:?}",
            self.kind
        )
    }
}

impl Error for RsaPublicKeyPolicyDefinitionError {}

/// One caller-supplied, validated RSA public-key strength policy.
///
/// The exponent allowlist is borrowed policy data. The constructor validates
/// it once so key processing has one closed membership decision and cannot be
/// made ambiguous by duplicate entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RsaPublicKeyPolicy<'a> {
    minimum_modulus_bits: usize,
    maximum_modulus_bits: usize,
    allowed_public_exponents: &'a [u64],
}

impl<'a> RsaPublicKeyPolicy<'a> {
    pub fn new(
        minimum_modulus_bits: usize,
        maximum_modulus_bits: usize,
        allowed_public_exponents: &'a [u64],
    ) -> Result<Self, RsaPublicKeyPolicyDefinitionError> {
        if minimum_modulus_bits == 0 {
            return Err(RsaPublicKeyPolicyDefinitionError::new(
                RsaPublicKeyPolicyDefinitionErrorKind::ZeroMinimumModulusBits,
            ));
        }
        if minimum_modulus_bits > maximum_modulus_bits {
            return Err(RsaPublicKeyPolicyDefinitionError::new(
                RsaPublicKeyPolicyDefinitionErrorKind::InvertedModulusBitRange,
            ));
        }
        if allowed_public_exponents.is_empty() {
            return Err(RsaPublicKeyPolicyDefinitionError::new(
                RsaPublicKeyPolicyDefinitionErrorKind::EmptyAllowedPublicExponents,
            ));
        }
        for (index, exponent) in allowed_public_exponents.iter().copied().enumerate() {
            if exponent < 3 || exponent % 2 == 0 {
                return Err(RsaPublicKeyPolicyDefinitionError::new(
                    RsaPublicKeyPolicyDefinitionErrorKind::InvalidAllowedPublicExponent,
                ));
            }
            if allowed_public_exponents[..index].contains(&exponent) {
                return Err(RsaPublicKeyPolicyDefinitionError::new(
                    RsaPublicKeyPolicyDefinitionErrorKind::DuplicateAllowedPublicExponent,
                ));
            }
        }
        Ok(Self {
            minimum_modulus_bits,
            maximum_modulus_bits,
            allowed_public_exponents,
        })
    }

    pub const fn minimum_modulus_bits(self) -> usize {
        self.minimum_modulus_bits
    }

    pub const fn maximum_modulus_bits(self) -> usize {
        self.maximum_modulus_bits
    }

    pub const fn allowed_public_exponents(self) -> &'a [u64] {
        self.allowed_public_exponents
    }

    fn permits_modulus_bits(self, bits: usize) -> bool {
        self.minimum_modulus_bits <= bits && bits <= self.maximum_modulus_bits
    }

    fn permits_public_exponent(self, exponent: u64) -> bool {
        self.allowed_public_exponents.contains(&exponent)
    }
}

/// Stable key failure categories. No category retains source or policy bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RsaPublicKeyErrorKind {
    WrongAlgorithm,
    MissingAlgorithmParameters,
    InvalidAlgorithmParameters(Asn1ErrorKind),
    EmptySubjectPublicKey,
    NonByteAlignedSubjectPublicKey,
    Structure(Asn1ErrorKind),
    MissingModulus,
    InvalidModulus(Asn1ErrorKind),
    NonPositiveModulus,
    EvenModulus,
    ModulusBitLengthOverflow,
    ModulusBitLengthOutOfRange,
    MissingPublicExponent,
    InvalidPublicExponent(Asn1ErrorKind),
    PublicExponentTooSmall,
    EvenPublicExponent,
    PublicExponentNotLessThanModulus,
    PublicExponentNotAllowed,
    TrailingElement,
}

/// A redacted failure at an offset local to the nested PKCS #1 payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RsaPublicKeyError {
    kind: RsaPublicKeyErrorKind,
    offset: usize,
}

impl RsaPublicKeyError {
    const fn new(kind: RsaPublicKeyErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            RsaPublicKeyErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(RsaPublicKeyErrorKind::Structure(error.kind()), offset)
    }

    fn modulus(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            RsaPublicKeyErrorKind::InvalidModulus(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn public_exponent(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            RsaPublicKeyErrorKind::InvalidPublicExponent(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> RsaPublicKeyErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for RsaPublicKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA public-key error {:?} at payload byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for RsaPublicKeyError {}

/// One borrowed RSA public key that passed structural and caller policy checks.
///
/// RSA public keys are public data, but construction remains private so this
/// value is evidence that the complete shape and policy gates ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedRsaPublicKey<'a> {
    modulus: &'a [u8],
    modulus_bits: usize,
    public_exponent: u64,
}

impl<'a> ValidatedRsaPublicKey<'a> {
    /// Normalized unsigned, big-endian modulus magnitude.
    pub const fn modulus(self) -> &'a [u8] {
        self.modulus
    }

    pub const fn modulus_bits(self) -> usize {
        self.modulus_bits
    }

    pub const fn public_exponent(self) -> u64 {
        self.public_exponent
    }
}

/// Validate one decoded SubjectPublicKeyInfo as an RSA public key.
///
/// A new decoder owns the nested PKCS #1 budget because the generic SPKI was
/// already decoded by a different schema walk. The nested decoder never sees
/// the containing certificate or receives authority from it.
pub fn validate_rsa_public_key<'key, 'policy>(
    subject_public_key_info: X509SubjectPublicKeyInfo<'key>,
    policy: RsaPublicKeyPolicy<'policy>,
    limits: Asn1Limits,
) -> Result<ValidatedRsaPublicKey<'key>, RsaPublicKeyError> {
    let algorithm = subject_public_key_info.algorithm();
    if !algorithm.algorithm().equals(RSA_ENCRYPTION_OID) {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::WrongAlgorithm,
            0,
        ));
    }
    let parameters = algorithm.parameters().ok_or_else(|| {
        RsaPublicKeyError::new(RsaPublicKeyErrorKind::MissingAlgorithmParameters, 0)
    })?;
    decode_null(parameters).map_err(|error| {
        RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::InvalidAlgorithmParameters(error.kind()),
            0,
        )
    })?;

    let subject_public_key = subject_public_key_info.subject_public_key();
    if subject_public_key.bytes().is_empty() {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::EmptySubjectPublicKey,
            0,
        ));
    }
    if subject_public_key.unused_bits() != 0 {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::NonByteAlignedSubjectPublicKey,
            0,
        ));
    }

    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(subject_public_key.bytes())
        .map_err(RsaPublicKeyError::structure)?;
    decode_pkcs1_public_key(&mut decoder, root, policy)
}

fn decode_pkcs1_public_key<'key>(
    decoder: &mut Asn1Decoder,
    root: Asn1Element<'key>,
    policy: RsaPublicKeyPolicy<'_>,
) -> Result<ValidatedRsaPublicKey<'key>, RsaPublicKeyError> {
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut fields = decoder
        .sequence(root)
        .map_err(RsaPublicKeyError::structure)?;

    let modulus_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let modulus_element = fields
        .read(decoder)
        .map_err(|error| RsaPublicKeyError::child_structure(error, value_offset, modulus_offset))?
        .ok_or_else(|| {
            RsaPublicKeyError::new(RsaPublicKeyErrorKind::MissingModulus, modulus_offset)
        })?;
    let modulus_integer = decode_integer(modulus_element)
        .map_err(|error| RsaPublicKeyError::modulus(error, modulus_offset))?;
    let modulus = positive_magnitude(modulus_integer).ok_or_else(|| {
        RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::NonPositiveModulus,
            modulus_offset + modulus_element.header().len(),
        )
    })?;
    if modulus.last().is_some_and(|byte| byte & 1 == 0) {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::EvenModulus,
            modulus_offset + modulus_element.encoded().len() - 1,
        ));
    }
    let modulus_bits = unsigned_bit_length(modulus).ok_or_else(|| {
        RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::ModulusBitLengthOverflow,
            modulus_offset,
        )
    })?;
    if !policy.permits_modulus_bits(modulus_bits) {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::ModulusBitLengthOutOfRange,
            modulus_offset,
        ));
    }

    let exponent_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let exponent_element = fields
        .read(decoder)
        .map_err(|error| RsaPublicKeyError::child_structure(error, value_offset, exponent_offset))?
        .ok_or_else(|| {
            RsaPublicKeyError::new(
                RsaPublicKeyErrorKind::MissingPublicExponent,
                exponent_offset,
            )
        })?;
    let exponent_integer = decode_integer(exponent_element)
        .map_err(|error| RsaPublicKeyError::public_exponent(error, exponent_offset))?;
    let public_exponent = exponent_integer
        .to_u64()
        .map_err(|error| RsaPublicKeyError::public_exponent(error, exponent_offset))?;
    if public_exponent < 3 {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::PublicExponentTooSmall,
            exponent_offset,
        ));
    }
    if public_exponent % 2 == 0 {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::EvenPublicExponent,
            exponent_offset,
        ));
    }
    if !exponent_is_less_than_modulus(public_exponent, modulus) {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::PublicExponentNotLessThanModulus,
            exponent_offset,
        ));
    }
    if !policy.permits_public_exponent(public_exponent) {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::PublicExponentNotAllowed,
            exponent_offset,
        ));
    }

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| RsaPublicKeyError::child_structure(error, value_offset, trailing_offset))?
        .is_some()
    {
        return Err(RsaPublicKeyError::new(
            RsaPublicKeyErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(ValidatedRsaPublicKey {
        modulus,
        modulus_bits,
        public_exponent,
    })
}

/// DER INTEGER is signed. A positive value whose high magnitude bit is one has
/// exactly one leading zero sign octet, which the generic decoder has already
/// proven non-redundant. No other normalization is permitted here.
fn positive_magnitude(integer: DerInteger<'_>) -> Option<&[u8]> {
    if integer.is_negative() {
        return None;
    }
    let signed = integer.signed_bytes();
    let magnitude = if signed[0] == 0 { &signed[1..] } else { signed };
    (!magnitude.is_empty() && magnitude.iter().any(|byte| *byte != 0)).then_some(magnitude)
}

fn unsigned_bit_length(magnitude: &[u8]) -> Option<usize> {
    let leading_bits = 8usize.checked_sub(magnitude[0].leading_zeros() as usize)?;
    magnitude
        .len()
        .checked_sub(1)?
        .checked_mul(8)?
        .checked_add(leading_bits)
}

/// Compare without constructing a bignum. If the modulus needs more than eight
/// bytes it is necessarily larger than a `u64`; otherwise one bounded fold is
/// exact.
fn exponent_is_less_than_modulus(exponent: u64, modulus: &[u8]) -> bool {
    if modulus.len() > std::mem::size_of::<u64>() {
        return true;
    }
    let modulus_value = modulus
        .iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
    exponent < modulus_value
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
