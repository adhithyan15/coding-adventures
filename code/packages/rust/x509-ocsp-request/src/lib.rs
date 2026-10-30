//! # Minimal unsigned RFC 6960 OCSP request framing
//!
//! This crate validates one caller-supplied canonical `CertID` and wraps those
//! exact bytes in the minimal unsigned, single-request `OCSPRequest` shape. It
//! owns no hash selection, certificate binding, nonce, transport, or network
//! authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_ocsp_cert_id::{decode_x509_ocsp_cert_id, X509OcspCertId, X509OcspCertIdErrorKind};

/// Stable request-framing failures retaining no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRequestErrorKind {
    /// The complete caller input is not one canonical DER element.
    CertIdFraming(Asn1ErrorKind),
    /// The canonical element is not a structurally valid OCSP `CertID`.
    InvalidCertId(X509OcspCertIdErrorKind),
    /// A nested encoded length cannot be represented by this platform.
    LengthOverflow,
    /// Caller-owned output cannot hold the complete request.
    OutputTooSmall,
}

/// A redacted OCSP request-framing failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRequestError {
    kind: X509OcspRequestErrorKind,
}

impl X509OcspRequestError {
    const fn new(kind: X509OcspRequestErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509OcspRequestErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP request framing error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRequestError {}

#[derive(Clone, Copy)]
struct RequestLengths {
    request: usize,
    request_list: usize,
    tbs_request: usize,
    ocsp_request: usize,
}

impl RequestLengths {
    fn for_cert_id(cert_id_length: usize) -> Option<Self> {
        let request = sequence_encoded_len(cert_id_length)?;
        let request_list = sequence_encoded_len(request)?;
        let tbs_request = sequence_encoded_len(request_list)?;
        let ocsp_request = sequence_encoded_len(tbs_request)?;
        Some(Self {
            request,
            request_list,
            tbs_request,
            ocsp_request,
        })
    }
}

/// One exact canonical `CertID` admitted for minimal OCSP request framing.
///
/// The value is deliberately non-copyable and retains both the structural
/// decode and exact caller bytes. It proves no hash policy or certificate
/// binding.
pub struct ValidatedX509OcspRequestCertId<'encoded> {
    encoded_cert_id: &'encoded [u8],
    cert_id: X509OcspCertId<'encoded>,
    lengths: RequestLengths,
}

impl<'encoded> ValidatedX509OcspRequestCertId<'encoded> {
    /// Return the exact canonical DER supplied by the caller.
    #[must_use]
    pub const fn encoded_cert_id(&self) -> &'encoded [u8] {
        self.encoded_cert_id
    }

    /// Return the structurally decoded identifier.
    #[must_use]
    pub const fn cert_id(&self) -> X509OcspCertId<'encoded> {
        self.cert_id
    }

    /// Return the exact output length required by the minimal request.
    #[must_use]
    pub const fn encoded_request_len(&self) -> usize {
        self.lengths.ocsp_request
    }
}

impl fmt::Debug for ValidatedX509OcspRequestCertId<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspRequestCertId")
    }
}

/// Validate one complete canonical OCSP `CertID` for request framing.
///
/// Caller-owned ASN.1 limits bound DER length, depth, element, and OID work.
pub fn validate_x509_ocsp_request_cert_id(
    encoded_cert_id: &[u8],
    limits: Asn1Limits,
) -> Result<ValidatedX509OcspRequestCertId<'_>, X509OcspRequestError> {
    let mut decoder = Asn1Decoder::new(limits);
    let element = decoder.decode_exact(encoded_cert_id).map_err(|error| {
        X509OcspRequestError::new(X509OcspRequestErrorKind::CertIdFraming(error.kind()))
    })?;
    let cert_id = decode_x509_ocsp_cert_id(&mut decoder, element).map_err(|error| {
        X509OcspRequestError::new(X509OcspRequestErrorKind::InvalidCertId(error.kind()))
    })?;
    let lengths = RequestLengths::for_cert_id(encoded_cert_id.len())
        .ok_or_else(|| X509OcspRequestError::new(X509OcspRequestErrorKind::LengthOverflow))?;

    Ok(ValidatedX509OcspRequestCertId {
        encoded_cert_id,
        cert_id,
        lengths,
    })
}

/// One encoded minimal unsigned OCSP request bound to its exact identifier.
pub struct EncodedX509OcspRequest<'request, 'cert_id, 'output> {
    cert_id: &'request ValidatedX509OcspRequestCertId<'cert_id>,
    encoded: &'output [u8],
}

impl<'request, 'cert_id, 'output> EncodedX509OcspRequest<'request, 'cert_id, 'output> {
    /// Return the exact validated identifier wrapped by this request.
    #[must_use]
    pub const fn cert_id(&self) -> &'request ValidatedX509OcspRequestCertId<'cert_id> {
        self.cert_id
    }

    /// Return the complete encoded request.
    #[must_use]
    pub const fn encoded(&self) -> &'output [u8] {
        self.encoded
    }
}

impl fmt::Debug for EncodedX509OcspRequest<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncodedX509OcspRequest")
    }
}

/// Encode the minimal unsigned RFC 6960 request containing one exact `CertID`.
///
/// The default version, requestor name, all extensions, and optional signature
/// are omitted. All nested lengths and complete output capacity are validated
/// before caller-owned output changes. Extra output capacity remains untouched.
pub fn encode_unsigned_x509_ocsp_request<'request, 'cert_id, 'output>(
    cert_id: &'request ValidatedX509OcspRequestCertId<'cert_id>,
    output: &'output mut [u8],
) -> Result<EncodedX509OcspRequest<'request, 'cert_id, 'output>, X509OcspRequestError> {
    if output.len() < cert_id.lengths.ocsp_request {
        return Err(X509OcspRequestError::new(
            X509OcspRequestErrorKind::OutputTooSmall,
        ));
    }

    let mut offset = 0;
    write_sequence_header(output, &mut offset, cert_id.lengths.tbs_request);
    write_sequence_header(output, &mut offset, cert_id.lengths.request_list);
    write_sequence_header(output, &mut offset, cert_id.lengths.request);
    write_sequence_header(output, &mut offset, cert_id.encoded_cert_id.len());
    let end = offset + cert_id.encoded_cert_id.len();
    output[offset..end].copy_from_slice(cert_id.encoded_cert_id);
    debug_assert_eq!(end, cert_id.lengths.ocsp_request);

    Ok(EncodedX509OcspRequest {
        cert_id,
        encoded: &output[..end],
    })
}

fn sequence_encoded_len(contents_length: usize) -> Option<usize> {
    1usize
        .checked_add(der_length_encoded_len(contents_length))?
        .checked_add(contents_length)
}

fn der_length_encoded_len(length: usize) -> usize {
    if length <= 127 {
        1
    } else {
        1 + ((usize::BITS - length.leading_zeros()).div_ceil(8) as usize)
    }
}

fn write_sequence_header(output: &mut [u8], offset: &mut usize, contents_length: usize) {
    output[*offset] = 0x30;
    *offset += 1;
    write_der_length(output, offset, contents_length);
}

fn write_der_length(output: &mut [u8], offset: &mut usize, length: usize) {
    if length <= 127 {
        output[*offset] = length as u8;
        *offset += 1;
        return;
    }

    let bytes = length.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .expect("long-form DER length is nonzero");
    let significant = &bytes[first..];
    output[*offset] = 0x80 | significant.len() as u8;
    *offset += 1;
    let end = *offset + significant.len();
    output[*offset..end].copy_from_slice(significant);
    *offset = end;
}
