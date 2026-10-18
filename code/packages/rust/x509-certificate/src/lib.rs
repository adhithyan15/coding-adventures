//! # Structural RFC 5280 certificate decoding
//!
//! This crate composes the complete `Certificate` and `TBSCertificate` shapes
//! from repository-owned DER and X.509 field primitives. It owns no algorithm
//! registry, signature verification, path policy, trust, clock, TLS, or network
//! authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_bit_string, decode_implicit_bit_string, Asn1Decoder, Asn1Element, Asn1Error,
    Asn1ErrorKind, Asn1Limits, DerBitString,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierError,
    X509AlgorithmIdentifierErrorKind,
};
use x509_certificate_version::{
    decode_explicit_x509_certificate_version, X509CertificateVersion,
    X509CertificateVersionErrorKind,
};
use x509_extensions::{decode_x509_extensions, X509Extensions, X509ExtensionsError};
use x509_name::{decode_x509_name, X509Name, X509NameErrorKind};
use x509_serial_number::{decode_x509_serial_number, X509SerialNumber, X509SerialNumberErrorKind};
use x509_subject_public_key_info::{
    decode_x509_subject_public_key_info, X509SubjectPublicKeyInfo,
    X509SubjectPublicKeyInfoErrorKind,
};
use x509_validity::{decode_x509_validity, X509Validity, X509ValidityErrorKind};

/// Stable TBSCertificate failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509TbsCertificateErrorKind {
    Structure(Asn1ErrorKind),
    InvalidVersion(X509CertificateVersionErrorKind),
    MissingSerialNumber,
    InvalidSerialNumber(X509SerialNumberErrorKind),
    MissingSignatureAlgorithm,
    InvalidSignatureAlgorithm(X509AlgorithmIdentifierErrorKind),
    MissingIssuer,
    InvalidIssuer(X509NameErrorKind),
    EmptyIssuer,
    MissingValidity,
    InvalidValidity(X509ValidityErrorKind),
    MissingSubject,
    InvalidSubject(X509NameErrorKind),
    MissingSubjectPublicKeyInfo,
    InvalidSubjectPublicKeyInfo(X509SubjectPublicKeyInfoErrorKind),
    InvalidIssuerUniqueId(Asn1ErrorKind),
    InvalidSubjectUniqueId(Asn1ErrorKind),
    InvalidExtensions(x509_extensions::X509ExtensionsErrorKind),
    UnexpectedOptionalField,
    VersionDoesNotPermitIssuerUniqueId,
    VersionDoesNotPermitSubjectUniqueId,
    VersionDoesNotPermitExtensions,
}

/// Stable Certificate failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateErrorKind {
    Structure(Asn1ErrorKind),
    MissingTbsCertificate,
    InvalidTbsCertificate(X509TbsCertificateErrorKind),
    MissingSignatureAlgorithm,
    InvalidSignatureAlgorithm(X509AlgorithmIdentifierErrorKind),
    SignatureAlgorithmMismatch,
    MissingSignatureValue,
    InvalidSignatureValue(Asn1ErrorKind),
    TrailingElement,
}

/// A redacted certificate failure at an offset in the complete DER document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateError {
    kind: X509CertificateErrorKind,
    offset: usize,
}

impl X509CertificateError {
    const fn new(kind: X509CertificateErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> X509CertificateErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509CertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509CertificateError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct X509TbsCertificateError {
    kind: X509TbsCertificateErrorKind,
    offset: usize,
}

impl X509TbsCertificateError {
    const fn new(kind: X509TbsCertificateErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509TbsCertificateErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509TbsCertificateErrorKind::Structure(error.kind()), offset)
    }
}

/// One complete borrowed RFC 5280 `TBSCertificate`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X509TbsCertificate<'a> {
    encoded: &'a [u8],
    version: X509CertificateVersion,
    serial_number: X509SerialNumber<'a>,
    signature_algorithm: X509AlgorithmIdentifier<'a>,
    signature_algorithm_encoded: &'a [u8],
    issuer: X509Name<'a>,
    validity: X509Validity,
    subject: X509Name<'a>,
    subject_public_key_info: X509SubjectPublicKeyInfo<'a>,
    issuer_unique_id: Option<DerBitString<'a>>,
    subject_unique_id: Option<DerBitString<'a>>,
    extensions: Option<X509Extensions<'a>>,
}

impl<'a> X509TbsCertificate<'a> {
    /// Return the exact canonical DER bytes covered by the certificate signature.
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }

    pub const fn version(&self) -> X509CertificateVersion {
        self.version
    }

    pub const fn serial_number(&self) -> X509SerialNumber<'a> {
        self.serial_number
    }

    pub const fn signature_algorithm(&self) -> X509AlgorithmIdentifier<'a> {
        self.signature_algorithm
    }

    pub const fn issuer(&self) -> &X509Name<'a> {
        &self.issuer
    }

    pub const fn validity(&self) -> X509Validity {
        self.validity
    }

    pub const fn subject(&self) -> &X509Name<'a> {
        &self.subject
    }

    pub const fn subject_public_key_info(&self) -> X509SubjectPublicKeyInfo<'a> {
        self.subject_public_key_info
    }

    pub const fn issuer_unique_id(&self) -> Option<DerBitString<'a>> {
        self.issuer_unique_id
    }

    pub const fn subject_unique_id(&self) -> Option<DerBitString<'a>> {
        self.subject_unique_id
    }

    pub const fn extensions(&self) -> Option<&X509Extensions<'a>> {
        self.extensions.as_ref()
    }
}

/// One complete borrowed RFC 5280 `Certificate`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X509Certificate<'a> {
    encoded: &'a [u8],
    tbs_certificate: X509TbsCertificate<'a>,
    signature_algorithm: X509AlgorithmIdentifier<'a>,
    signature_value: DerBitString<'a>,
}

impl<'a> X509Certificate<'a> {
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }

    pub const fn tbs_certificate(&self) -> &X509TbsCertificate<'a> {
        &self.tbs_certificate
    }

    pub const fn signature_algorithm(&self) -> X509AlgorithmIdentifier<'a> {
        self.signature_algorithm
    }

    pub const fn signature_value(&self) -> DerBitString<'a> {
        self.signature_value
    }
}

/// Decode one exact RFC 5280 certificate document under explicit limits.
pub fn decode_x509_certificate(
    input: &[u8],
    limits: Asn1Limits,
) -> Result<X509Certificate<'_>, X509CertificateError> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(input).map_err(|error| {
        X509CertificateError::new(
            X509CertificateErrorKind::Structure(error.kind()),
            error.offset(),
        )
    })?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut fields = decoder.sequence(root).map_err(|error| {
        X509CertificateError::new(
            X509CertificateErrorKind::Structure(error.kind()),
            error.offset(),
        )
    })?;

    let tbs_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let tbs_element = read_required(
        &mut decoder,
        &mut fields,
        value_offset,
        tbs_offset,
        X509CertificateErrorKind::MissingTbsCertificate,
    )?;
    let tbs_certificate = decode_tbs_certificate(&mut decoder, tbs_element).map_err(|error| {
        X509CertificateError::new(
            X509CertificateErrorKind::InvalidTbsCertificate(error.kind),
            tbs_offset + error.offset,
        )
    })?;

    let signature_algorithm_offset =
        child_offset(value_offset, value_length, fields.remaining().len());
    let signature_algorithm_element = read_required(
        &mut decoder,
        &mut fields,
        value_offset,
        signature_algorithm_offset,
        X509CertificateErrorKind::MissingSignatureAlgorithm,
    )?;
    let signature_algorithm =
        decode_x509_algorithm_identifier(&mut decoder, signature_algorithm_element)
            .map_err(|error| algorithm_error(error, signature_algorithm_offset))?;

    if tbs_certificate.signature_algorithm_encoded != signature_algorithm_element.encoded() {
        return Err(X509CertificateError::new(
            X509CertificateErrorKind::SignatureAlgorithmMismatch,
            signature_algorithm_offset,
        ));
    }

    let signature_value_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let signature_value_element = read_required(
        &mut decoder,
        &mut fields,
        value_offset,
        signature_value_offset,
        X509CertificateErrorKind::MissingSignatureValue,
    )?;
    let signature_value = decode_bit_string(signature_value_element).map_err(|error| {
        X509CertificateError::new(
            X509CertificateErrorKind::InvalidSignatureValue(error.kind()),
            signature_value_offset + error.offset(),
        )
    })?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(&mut decoder)
        .map_err(|error| outer_child_structure(error, value_offset, trailing_offset))?
        .is_some()
    {
        return Err(X509CertificateError::new(
            X509CertificateErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509Certificate {
        encoded: input,
        tbs_certificate,
        signature_algorithm,
        signature_value,
    })
}

fn decode_tbs_certificate<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509TbsCertificate<'a>, X509TbsCertificateError> {
    let encoded = element.encoded();
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509TbsCertificateError::structure)?;

    let first_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let first = fields
        .read(decoder)
        .map_err(|error| {
            X509TbsCertificateError::child_structure(error, value_offset, first_offset)
        })?
        .ok_or_else(|| {
            X509TbsCertificateError::new(
                X509TbsCertificateErrorKind::MissingSerialNumber,
                first_offset,
            )
        })?;
    let (version, serial_element, serial_offset) = if is_context_number(first, 0) {
        let version =
            decode_explicit_x509_certificate_version(decoder, first).map_err(|error| {
                X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::InvalidVersion(error.kind()),
                    first_offset + error.offset(),
                )
            })?;
        let serial_offset = child_offset(value_offset, value_length, fields.remaining().len());
        let serial = read_tbs_required(
            decoder,
            &mut fields,
            value_offset,
            serial_offset,
            X509TbsCertificateErrorKind::MissingSerialNumber,
        )?;
        (version, serial, serial_offset)
    } else {
        (X509CertificateVersion::V1, first, first_offset)
    };
    let serial_number = decode_x509_serial_number(serial_element).map_err(|error| {
        X509TbsCertificateError::new(
            X509TbsCertificateErrorKind::InvalidSerialNumber(error.kind()),
            serial_offset + error.offset(),
        )
    })?;

    let signature_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let signature_element = read_tbs_required(
        decoder,
        &mut fields,
        value_offset,
        signature_offset,
        X509TbsCertificateErrorKind::MissingSignatureAlgorithm,
    )?;
    let signature_algorithm = decode_x509_algorithm_identifier(decoder, signature_element)
        .map_err(|error| {
            X509TbsCertificateError::new(
                X509TbsCertificateErrorKind::InvalidSignatureAlgorithm(error.kind()),
                signature_offset + error.offset(),
            )
        })?;

    let issuer_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let issuer_element = read_tbs_required(
        decoder,
        &mut fields,
        value_offset,
        issuer_offset,
        X509TbsCertificateErrorKind::MissingIssuer,
    )?;
    let issuer = decode_x509_name(decoder, issuer_element).map_err(|error| {
        X509TbsCertificateError::new(
            X509TbsCertificateErrorKind::InvalidIssuer(error.kind()),
            issuer_offset + error.offset(),
        )
    })?;
    if issuer.is_empty() {
        return Err(X509TbsCertificateError::new(
            X509TbsCertificateErrorKind::EmptyIssuer,
            issuer_offset + issuer_element.header().len(),
        ));
    }

    let validity_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let validity_element = read_tbs_required(
        decoder,
        &mut fields,
        value_offset,
        validity_offset,
        X509TbsCertificateErrorKind::MissingValidity,
    )?;
    let validity = decode_x509_validity(decoder, validity_element).map_err(|error| {
        X509TbsCertificateError::new(
            X509TbsCertificateErrorKind::InvalidValidity(error.kind()),
            validity_offset + error.offset(),
        )
    })?;

    let subject_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let subject_element = read_tbs_required(
        decoder,
        &mut fields,
        value_offset,
        subject_offset,
        X509TbsCertificateErrorKind::MissingSubject,
    )?;
    let subject = decode_x509_name(decoder, subject_element).map_err(|error| {
        X509TbsCertificateError::new(
            X509TbsCertificateErrorKind::InvalidSubject(error.kind()),
            subject_offset + error.offset(),
        )
    })?;

    let subject_public_key_info_offset =
        child_offset(value_offset, value_length, fields.remaining().len());
    let subject_public_key_info_element = read_tbs_required(
        decoder,
        &mut fields,
        value_offset,
        subject_public_key_info_offset,
        X509TbsCertificateErrorKind::MissingSubjectPublicKeyInfo,
    )?;
    let subject_public_key_info =
        decode_x509_subject_public_key_info(decoder, subject_public_key_info_element).map_err(
            |error| {
                X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::InvalidSubjectPublicKeyInfo(error.kind()),
                    subject_public_key_info_offset + error.offset(),
                )
            },
        )?;

    let mut issuer_unique_id = None;
    let mut subject_unique_id = None;
    let mut extensions = None;
    while !fields.remaining().is_empty() {
        let optional_offset = child_offset(value_offset, value_length, fields.remaining().len());
        let optional = fields.read(decoder).map_err(|error| {
            X509TbsCertificateError::child_structure(error, value_offset, optional_offset)
        })?;
        let optional = optional.expect("a non-empty DER cursor must yield one element");

        if is_context_number(optional, 1)
            && issuer_unique_id.is_none()
            && subject_unique_id.is_none()
            && extensions.is_none()
        {
            if version == X509CertificateVersion::V1 {
                return Err(X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::VersionDoesNotPermitIssuerUniqueId,
                    optional_offset,
                ));
            }
            issuer_unique_id = Some(decode_implicit_bit_string(optional, 1).map_err(|error| {
                X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::InvalidIssuerUniqueId(error.kind()),
                    optional_offset + error.offset(),
                )
            })?);
        } else if is_context_number(optional, 2)
            && subject_unique_id.is_none()
            && extensions.is_none()
        {
            if version == X509CertificateVersion::V1 {
                return Err(X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::VersionDoesNotPermitSubjectUniqueId,
                    optional_offset,
                ));
            }
            subject_unique_id = Some(decode_implicit_bit_string(optional, 2).map_err(|error| {
                X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::InvalidSubjectUniqueId(error.kind()),
                    optional_offset + error.offset(),
                )
            })?);
        } else if is_context_number(optional, 3) && extensions.is_none() {
            if version != X509CertificateVersion::V3 {
                return Err(X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::VersionDoesNotPermitExtensions,
                    optional_offset,
                ));
            }
            let wrapper_header_len = optional.header().len();
            let extension_element = decoder.explicit(optional, 3).map_err(|error| {
                let local_offset = match error.kind() {
                    Asn1ErrorKind::Framing(_) => wrapper_header_len + error.offset(),
                    _ => error.offset(),
                };
                X509TbsCertificateError::new(
                    X509TbsCertificateErrorKind::Structure(error.kind()),
                    optional_offset + local_offset,
                )
            })?;
            extensions = Some(decode_x509_extensions(decoder, extension_element).map_err(
                |error: X509ExtensionsError| {
                    X509TbsCertificateError::new(
                        X509TbsCertificateErrorKind::InvalidExtensions(error.kind()),
                        optional_offset + wrapper_header_len + error.offset(),
                    )
                },
            )?);
        } else {
            return Err(X509TbsCertificateError::new(
                X509TbsCertificateErrorKind::UnexpectedOptionalField,
                optional_offset,
            ));
        }
    }

    Ok(X509TbsCertificate {
        encoded,
        version,
        serial_number,
        signature_algorithm,
        signature_algorithm_encoded: signature_element.encoded(),
        issuer,
        validity,
        subject,
        subject_public_key_info,
        issuer_unique_id,
        subject_unique_id,
        extensions,
    })
}

fn read_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    child_offset: usize,
    missing: X509CertificateErrorKind,
) -> Result<Asn1Element<'a>, X509CertificateError> {
    fields
        .read(decoder)
        .map_err(|error| outer_child_structure(error, value_offset, child_offset))?
        .ok_or_else(|| X509CertificateError::new(missing, child_offset))
}

fn read_tbs_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    child_offset: usize,
    missing: X509TbsCertificateErrorKind,
) -> Result<Asn1Element<'a>, X509TbsCertificateError> {
    fields
        .read(decoder)
        .map_err(|error| {
            X509TbsCertificateError::child_structure(error, value_offset, child_offset)
        })?
        .ok_or_else(|| X509TbsCertificateError::new(missing, child_offset))
}

fn outer_child_structure(
    error: Asn1Error,
    value_offset: usize,
    child_offset: usize,
) -> X509CertificateError {
    let offset = match error.kind() {
        Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
        _ => child_offset + error.offset(),
    };
    X509CertificateError::new(X509CertificateErrorKind::Structure(error.kind()), offset)
}

fn algorithm_error(
    error: X509AlgorithmIdentifierError,
    child_offset: usize,
) -> X509CertificateError {
    X509CertificateError::new(
        X509CertificateErrorKind::InvalidSignatureAlgorithm(error.kind()),
        child_offset + error.offset(),
    )
}

fn is_context_number(element: Asn1Element<'_>, number: u32) -> bool {
    let tag = element.tag();
    tag.class == TagClass::ContextSpecific && tag.number == number
}

fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
