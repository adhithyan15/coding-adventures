//! # Bounded X.509 RSASSA-PSS parameter decoding
//!
//! This crate interprets one generic signature AlgorithmIdentifier as the
//! RFC 4055 / RFC 8017 RSASSA-PSS parameter structure. It preserves the
//! explicit message hash, MGF1 hash, salt length, and trailer field while
//! applying RFC 4055 signature-validation compatibility rules.
//!
//! It owns no algorithm-allowlist, key policy, cryptographic operation,
//! signature verdict, trust decision, or transport authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{decode_integer, decode_null, Asn1Decoder, Asn1Error, Asn1ErrorKind};
use der_tlv::TagClass;
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierErrorKind,
};

/// `id-RSASSA-PSS` (`1.2.840.113549.1.1.10`).
pub const RSASSA_PSS_OID: &[u64] = &[1, 2, 840, 113549, 1, 1, 10];
/// `id-mgf1` (`1.2.840.113549.1.1.8`).
pub const MGF1_OID: &[u64] = &[1, 2, 840, 113549, 1, 1, 8];
const SHA1_OID: &[u64] = &[1, 3, 14, 3, 2, 26];
const SHA224_OID: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 4];
const SHA256_OID: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 1];
const SHA384_OID: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 2];
const SHA512_OID: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 3];

/// RFC 4055 digest identifiers structurally permitted in PSS parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsassaPssDigestAlgorithm {
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

/// One decoded, public RSASSA-PSS parameter set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsassaPssParameters {
    message_hash: X509RsassaPssDigestAlgorithm,
    mgf1_hash: X509RsassaPssDigestAlgorithm,
    salt_length: u64,
    trailer_field: u64,
}

impl X509RsassaPssParameters {
    pub const fn message_hash(self) -> X509RsassaPssDigestAlgorithm {
        self.message_hash
    }

    pub const fn mgf1_hash(self) -> X509RsassaPssDigestAlgorithm {
        self.mgf1_hash
    }

    pub const fn salt_length(self) -> u64 {
        self.salt_length
    }

    pub const fn trailer_field(self) -> u64 {
        self.trailer_field
    }
}

/// Stable failures. No category retains source bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsassaPssParametersErrorKind {
    WrongAlgorithm,
    MissingParameters,
    Structure(Asn1ErrorKind),
    UnexpectedParameterTag,
    DuplicateParameter,
    OutOfOrderParameter,
    InvalidHashAlgorithmIdentifier(X509AlgorithmIdentifierErrorKind),
    UnsupportedHashAlgorithm,
    InvalidHashAlgorithmParameters(Asn1ErrorKind),
    InvalidMaskGenAlgorithmIdentifier(X509AlgorithmIdentifierErrorKind),
    UnsupportedMaskGenAlgorithm,
    MissingMaskGenHashAlgorithm,
    InvalidMaskGenHashAlgorithmIdentifier(X509AlgorithmIdentifierErrorKind),
    UnsupportedMaskGenHashAlgorithm,
    InvalidMaskGenHashParameters(Asn1ErrorKind),
    InvalidSaltLength(Asn1ErrorKind),
    ExplicitDefaultSaltLength,
    InvalidTrailerField(Asn1ErrorKind),
}

/// A redacted failure at an offset local to the encoded PSS parameters value.
///
/// Offset zero is the parameters SEQUENCE tag. Nested failures are rebased
/// through every explicit wrapper and AlgorithmIdentifier container.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct X509RsassaPssParametersError {
    kind: X509RsassaPssParametersErrorKind,
    offset: usize,
}

impl X509RsassaPssParametersError {
    const fn new(kind: X509RsassaPssParametersErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> X509RsassaPssParametersErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Debug for X509RsassaPssParametersError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509RsassaPssParametersError")
            .field("kind", &self.kind)
            .field("offset", &self.offset)
            .finish()
    }
}

impl fmt::Display for X509RsassaPssParametersError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSASSA-PSS parameters error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl std::error::Error for X509RsassaPssParametersError {}

/// Decode one generic signature AlgorithmIdentifier as RSASSA-PSS parameters.
///
/// The parameters field is required for an X.509 signature algorithm. Omitted
/// component fields receive their RFC defaults. RFC 4055 requires signature
/// validators to recognize explicitly present SHA-1, MGF1-SHA-1, and
/// trailer-field-1 defaults as equivalent to omission. An explicit default
/// salt length is rejected because DER requires that DEFAULT omitted and RFC
/// 4055 provides no corresponding validator exception.
pub fn decode_x509_rsassa_pss_parameters(
    decoder: &mut Asn1Decoder,
    algorithm: X509AlgorithmIdentifier<'_>,
) -> Result<X509RsassaPssParameters, X509RsassaPssParametersError> {
    if !algorithm.algorithm().equals(RSASSA_PSS_OID) {
        return Err(X509RsassaPssParametersError::new(
            X509RsassaPssParametersErrorKind::WrongAlgorithm,
            0,
        ));
    }
    let parameters = algorithm.parameters().ok_or_else(|| {
        X509RsassaPssParametersError::new(X509RsassaPssParametersErrorKind::MissingParameters, 0)
    })?;
    let value_offset = parameters.header().len();
    let value_length = parameters.value().len();
    let mut fields = decoder.sequence(parameters).map_err(|error| {
        X509RsassaPssParametersError::new(
            X509RsassaPssParametersErrorKind::Structure(error.kind()),
            error.offset(),
        )
    })?;

    let mut result = X509RsassaPssParameters {
        message_hash: X509RsassaPssDigestAlgorithm::Sha1,
        mgf1_hash: X509RsassaPssDigestAlgorithm::Sha1,
        salt_length: 20,
        trailer_field: 1,
    };
    let mut last_tag = None;
    while !fields.remaining().is_empty() {
        let offset = child_offset(value_offset, value_length, fields.remaining().len());
        let field = fields
            .read(decoder)
            .map_err(|error| structure(error, value_offset, offset))?
            .expect("a non-empty DER cursor must yield one element");
        let tag = field.tag();
        if tag.class != TagClass::ContextSpecific || !tag.constructed || tag.number > 3 {
            return Err(X509RsassaPssParametersError::new(
                X509RsassaPssParametersErrorKind::UnexpectedParameterTag,
                offset,
            ));
        }
        if let Some(previous) = last_tag {
            if tag.number == previous {
                return Err(X509RsassaPssParametersError::new(
                    X509RsassaPssParametersErrorKind::DuplicateParameter,
                    offset,
                ));
            }
            if tag.number < previous {
                return Err(X509RsassaPssParametersError::new(
                    X509RsassaPssParametersErrorKind::OutOfOrderParameter,
                    offset,
                ));
            }
        }
        last_tag = Some(tag.number);
        let wrapper_header_length = field.header().len();
        let explicit = decoder
            .explicit(field, tag.number)
            .map_err(|error| explicit_structure(error, offset, wrapper_header_length))?;
        let explicit_offset = offset + wrapper_header_length;

        match tag.number {
            0 => {
                let identifier =
                    decode_x509_algorithm_identifier(decoder, explicit).map_err(|error| {
                        X509RsassaPssParametersError::new(
                            X509RsassaPssParametersErrorKind::InvalidHashAlgorithmIdentifier(
                                error.kind(),
                            ),
                            explicit_offset + error.offset(),
                        )
                    })?;
                let digest = decode_digest_identifier(identifier)
                    .map_err(|kind| X509RsassaPssParametersError::new(kind, explicit_offset))?;
                result.message_hash = digest;
            }
            1 => {
                let identifier =
                    decode_x509_algorithm_identifier(decoder, explicit).map_err(|error| {
                        X509RsassaPssParametersError::new(
                            X509RsassaPssParametersErrorKind::InvalidMaskGenAlgorithmIdentifier(
                                error.kind(),
                            ),
                            explicit_offset + error.offset(),
                        )
                    })?;
                if !identifier.algorithm().equals(MGF1_OID) {
                    return Err(X509RsassaPssParametersError::new(
                        X509RsassaPssParametersErrorKind::UnsupportedMaskGenAlgorithm,
                        explicit_offset,
                    ));
                }
                let hash_element = identifier.parameters().ok_or_else(|| {
                    X509RsassaPssParametersError::new(
                        X509RsassaPssParametersErrorKind::MissingMaskGenHashAlgorithm,
                        explicit_offset,
                    )
                })?;
                let hash_element_offset =
                    explicit_offset + subslice_offset(explicit.encoded(), hash_element.encoded());
                let hash_identifier = decode_x509_algorithm_identifier(decoder, hash_element)
                    .map_err(|error| {
                        X509RsassaPssParametersError::new(
                            X509RsassaPssParametersErrorKind::InvalidMaskGenHashAlgorithmIdentifier(
                                error.kind(),
                            ),
                            hash_element_offset + error.offset(),
                        )
                    })?;
                let digest = decode_mgf_digest_identifier(hash_identifier)
                    .map_err(|kind| X509RsassaPssParametersError::new(kind, hash_element_offset))?;
                result.mgf1_hash = digest;
            }
            2 => {
                let salt_length = decode_integer(explicit)
                    .and_then(|integer| integer.to_u64())
                    .map_err(|error| {
                        X509RsassaPssParametersError::new(
                            X509RsassaPssParametersErrorKind::InvalidSaltLength(error.kind()),
                            explicit_offset + error.offset(),
                        )
                    })?;
                if salt_length == 20 {
                    return Err(X509RsassaPssParametersError::new(
                        X509RsassaPssParametersErrorKind::ExplicitDefaultSaltLength,
                        offset,
                    ));
                }
                result.salt_length = salt_length;
            }
            3 => {
                let trailer_field = decode_integer(explicit)
                    .and_then(|integer| integer.to_u64())
                    .map_err(|error| {
                        X509RsassaPssParametersError::new(
                            X509RsassaPssParametersErrorKind::InvalidTrailerField(error.kind()),
                            explicit_offset + error.offset(),
                        )
                    })?;
                result.trailer_field = trailer_field;
            }
            _ => unreachable!("tag range was checked"),
        }
    }
    Ok(result)
}

fn decode_digest_identifier(
    identifier: X509AlgorithmIdentifier<'_>,
) -> Result<X509RsassaPssDigestAlgorithm, X509RsassaPssParametersErrorKind> {
    let digest = digest_from_oid(identifier)
        .ok_or(X509RsassaPssParametersErrorKind::UnsupportedHashAlgorithm)?;
    validate_digest_parameters(identifier).map_err(|error| {
        X509RsassaPssParametersErrorKind::InvalidHashAlgorithmParameters(error.kind())
    })?;
    Ok(digest)
}

fn decode_mgf_digest_identifier(
    identifier: X509AlgorithmIdentifier<'_>,
) -> Result<X509RsassaPssDigestAlgorithm, X509RsassaPssParametersErrorKind> {
    let digest = digest_from_oid(identifier)
        .ok_or(X509RsassaPssParametersErrorKind::UnsupportedMaskGenHashAlgorithm)?;
    validate_digest_parameters(identifier).map_err(|error| {
        X509RsassaPssParametersErrorKind::InvalidMaskGenHashParameters(error.kind())
    })?;
    Ok(digest)
}

fn digest_from_oid(
    identifier: X509AlgorithmIdentifier<'_>,
) -> Option<X509RsassaPssDigestAlgorithm> {
    let oid = identifier.algorithm();
    if oid.equals(SHA1_OID) {
        Some(X509RsassaPssDigestAlgorithm::Sha1)
    } else if oid.equals(SHA224_OID) {
        Some(X509RsassaPssDigestAlgorithm::Sha224)
    } else if oid.equals(SHA256_OID) {
        Some(X509RsassaPssDigestAlgorithm::Sha256)
    } else if oid.equals(SHA384_OID) {
        Some(X509RsassaPssDigestAlgorithm::Sha384)
    } else if oid.equals(SHA512_OID) {
        Some(X509RsassaPssDigestAlgorithm::Sha512)
    } else {
        None
    }
}

fn validate_digest_parameters(identifier: X509AlgorithmIdentifier<'_>) -> Result<(), Asn1Error> {
    match identifier.parameters() {
        Some(parameters) => decode_null(parameters),
        None => Ok(()),
    }
}

fn structure(
    error: Asn1Error,
    value_offset: usize,
    child_offset: usize,
) -> X509RsassaPssParametersError {
    let offset = match error.kind() {
        Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
        _ => child_offset + error.offset(),
    };
    X509RsassaPssParametersError::new(
        X509RsassaPssParametersErrorKind::Structure(error.kind()),
        offset,
    )
}

fn explicit_structure(
    error: Asn1Error,
    wrapper_offset: usize,
    wrapper_header_length: usize,
) -> X509RsassaPssParametersError {
    let local_offset = match error.kind() {
        Asn1ErrorKind::Framing(_) => wrapper_header_length + error.offset(),
        _ => error.offset(),
    };
    X509RsassaPssParametersError::new(
        X509RsassaPssParametersErrorKind::Structure(error.kind()),
        wrapper_offset + local_offset,
    )
}

fn subslice_offset(parent: &[u8], child: &[u8]) -> usize {
    parent
        .windows(child.len())
        .position(|window| core::ptr::eq(window.as_ptr(), child.as_ptr()))
        .expect("decoded child must borrow from its enclosing element")
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
