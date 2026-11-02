//! # Required RFC 8954 OCSP response-nonce policy
//!
//! An OCSP nonce has one deliberately narrow job: bind an authenticated
//! response to the request that supplied the unpredictable nonce. Structural
//! `ResponseData` decoding cannot make that claim because it retains outer
//! extensions generically. This crate adds the strict response-echo policy
//! boundary while keeping request construction, randomness, authentication,
//! certificate binding, freshness, and revocation decisions separate.

#![deny(unsafe_code)]

use ct_compare::ct_eq;
use der_asn1::{decode_octet_string, Asn1Decoder, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;
use x509_ocsp_response_data::X509OcspResponseData;

/// RFC 8954 `id-pkix-ocsp-nonce` (`1.3.6.1.5.5.7.48.1.2`).
pub const ID_PKIX_OCSP_NONCE_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 48, 1, 2];

/// RFC 8954 requires newer clients to send a 32-octet nonce.
pub const REQUIRED_CLIENT_NONCE_LENGTH: usize = 32;

/// RFC 8954 permits nonce values from one through 32 octets.
pub const MAX_OCSP_NONCE_LENGTH: usize = 32;

/// Stable fail-closed categories that retain no nonce or response bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspResponseNoncePolicyErrorKind {
    MissingResponseExtensions,
    UnexpectedResponseExtensionCount,
    UnexpectedResponseExtension,
    CriticalNonceExtension,
    InvalidNonceEncoding(Asn1ErrorKind),
    InvalidNonceLength,
    NonceMismatch,
}

/// A redacted failure at an offset local to the embedded nonce encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspResponseNoncePolicyError {
    kind: X509OcspResponseNoncePolicyErrorKind,
    offset: usize,
}

impl X509OcspResponseNoncePolicyError {
    const fn new(kind: X509OcspResponseNoncePolicyErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn nonce_encoding(error: Asn1Error) -> Self {
        Self::new(
            X509OcspResponseNoncePolicyErrorKind::InvalidNonceEncoding(error.kind()),
            error.offset(),
        )
    }

    /// Return the stable, payload-free failure category.
    pub const fn kind(self) -> X509OcspResponseNoncePolicyErrorKind {
        self.kind
    }

    /// Return the offset within the extension's embedded nonce document.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspResponseNoncePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP response nonce policy error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspResponseNoncePolicyError {}

/// Non-copyable evidence that one exact response echoed one exact request nonce.
///
/// The evidence retains both inputs by identity. It does not authenticate the
/// response or turn structural OCSP status into a revocation verdict.
pub struct ValidatedX509OcspResponseNonce<'proof, 'response, 'nonce> {
    response_data: &'proof X509OcspResponseData<'response>,
    requested_nonce: &'nonce [u8; REQUIRED_CLIENT_NONCE_LENGTH],
}

impl<'proof, 'response, 'nonce> ValidatedX509OcspResponseNonce<'proof, 'response, 'nonce> {
    /// Return the exact decoded response object retained by this proof.
    pub const fn response_data(&self) -> &'proof X509OcspResponseData<'response> {
        self.response_data
    }

    /// Return the exact caller-owned request nonce retained by this proof.
    pub const fn requested_nonce(&self) -> &'nonce [u8; REQUIRED_CLIENT_NONCE_LENGTH] {
        self.requested_nonce
    }
}

/// Require one exact RFC 8954 response echo for a modern 32-octet client nonce.
///
/// The supplied decoder owns the semantic extension's depth and element
/// budget. Embedded decoding uses the generic extension's opaque container
/// context, so the nonce document cannot restart at depth zero. The response
/// must carry exactly one outer extension and that extension must be the
/// non-critical OCSP nonce. The embedded `Nonce` OCTET STRING must have an RFC
/// 8954 length and must equal the complete request nonce under the repository's
/// full-scan comparison primitive.
///
/// This operation allocates nothing and performs no I/O, randomness, clock,
/// credential, signing, verification, trust, TLS, HTTP, or network effect.
pub fn validate_required_x509_ocsp_response_nonce<'proof, 'response, 'nonce>(
    decoder: &mut Asn1Decoder,
    response_data: &'proof X509OcspResponseData<'response>,
    requested_nonce: &'nonce [u8; REQUIRED_CLIENT_NONCE_LENGTH],
) -> Result<
    ValidatedX509OcspResponseNonce<'proof, 'response, 'nonce>,
    X509OcspResponseNoncePolicyError,
> {
    let extensions = response_data.response_extensions().ok_or_else(|| {
        X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::MissingResponseExtensions,
            0,
        )
    })?;
    if extensions.len() != 1 {
        return Err(X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::UnexpectedResponseExtensionCount,
            0,
        ));
    }

    let extension = extensions
        .get(0)
        .expect("a one-entry X509Extensions value must retain its first extension");
    if !extension.extension_id().equals(ID_PKIX_OCSP_NONCE_OID) {
        return Err(X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::UnexpectedResponseExtension,
            0,
        ));
    }
    if extension.critical() {
        return Err(X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::CriticalNonceExtension,
            0,
        ));
    }

    let nonce_element = decoder
        .decode_exact_embedded_context(
            extension.extension_value_context(),
            extension.extension_value(),
        )
        .map_err(X509OcspResponseNoncePolicyError::nonce_encoding)?;
    let response_nonce = decode_octet_string(nonce_element)
        .map_err(X509OcspResponseNoncePolicyError::nonce_encoding)?;
    if response_nonce.is_empty() || response_nonce.len() > MAX_OCSP_NONCE_LENGTH {
        return Err(X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::InvalidNonceLength,
            0,
        ));
    }
    if !ct_eq(response_nonce, requested_nonce) {
        return Err(X509OcspResponseNoncePolicyError::new(
            X509OcspResponseNoncePolicyErrorKind::NonceMismatch,
            0,
        ));
    }

    Ok(ValidatedX509OcspResponseNonce {
        response_data,
        requested_nonce,
    })
}
