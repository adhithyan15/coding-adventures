//! # Exact OCSP `ResponderID` certificate binding
//!
//! This crate binds one exact decoded `BasicOCSPResponse` to one exact decoded
//! candidate responder certificate. It deliberately grants no responder
//! authorization, signature-verification, path, trust, freshness, revocation,
//! TLS, HTTP, OAuth-transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use ct_compare::ct_eq_fixed;
use sha1::sum1;
use x509_certificate::X509Certificate;
use x509_name::X509Name;
use x509_ocsp_basic_response::X509OcspBasicResponse;
use x509_ocsp_response_data::X509OcspResponderId;

/// RFC 6960 `KeyHash` length for the SHA-1 digest of `subjectPublicKey` bits.
pub const X509_OCSP_RESPONDER_KEY_HASH_LENGTH: usize = 20;

/// The `ResponderID` alternative matched by successful binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspResponderIdKind {
    ByName,
    ByKey,
}

/// Stable failure categories that retain no response or certificate payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspResponderIdBindingErrorKind {
    NameMismatch,
    InvalidKeyHashLength,
    KeyHashMismatch,
}

/// A redacted responder-identifier binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspResponderIdBindingError {
    kind: X509OcspResponderIdBindingErrorKind,
}

impl X509OcspResponderIdBindingError {
    const fn new(kind: X509OcspResponderIdBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspResponderIdBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspResponderIdBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP responder ID binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspResponderIdBindingError {}

/// Non-copyable evidence retaining the exact matched response and certificate.
///
/// This value proves only that the response's structural `ResponderID` matched
/// the candidate certificate under the selected identifier rule. It is not
/// response-signature verification or responder authorization.
pub struct BoundX509OcspResponderCertificate<
    'response,
    'response_encoded,
    'certificate,
    'certificate_encoded,
> {
    response: &'response X509OcspBasicResponse<'response_encoded>,
    certificate: &'certificate X509Certificate<'certificate_encoded>,
    kind: X509OcspResponderIdKind,
}

impl<'response, 'response_encoded, 'certificate, 'certificate_encoded>
    BoundX509OcspResponderCertificate<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >
{
    /// Return the exact response whose `ResponderID` matched.
    pub const fn response(&self) -> &'response X509OcspBasicResponse<'response_encoded> {
        self.response
    }

    /// Return the exact certificate matched to the response.
    pub const fn certificate(&self) -> &'certificate X509Certificate<'certificate_encoded> {
        self.certificate
    }

    /// Return the matched `ResponderID` alternative.
    pub const fn kind(&self) -> X509OcspResponderIdKind {
        self.kind
    }
}

impl fmt::Debug for BoundX509OcspResponderCertificate<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspResponderCertificate")
    }
}

/// Bind one exact OCSP response's `ResponderID` to one candidate certificate.
///
/// `byName` invokes `names_equal` exactly once with the responder Name first
/// and candidate subject second. `byKey` never invokes Name policy: it requires
/// a 20-byte value, hashes the candidate's `subjectPublicKey` BIT STRING
/// contents (excluding the DER tag, length, and unused-bits count octet) with
/// repository-owned SHA-1, and compares the complete digest in constant time.
///
/// Success does not verify that the certificate's key signed the response or
/// authorize that certificate as an OCSP responder. Those are separate policy
/// and cryptographic composition boundaries.
pub fn bind_x509_ocsp_responder_id_to_certificate<
    'response,
    'response_encoded,
    'certificate,
    'certificate_encoded,
    NamesEqual,
>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    certificate: &'certificate X509Certificate<'certificate_encoded>,
    mut names_equal: NamesEqual,
) -> Result<
    BoundX509OcspResponderCertificate<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >,
    X509OcspResponderIdBindingError,
>
where
    NamesEqual: FnMut(&X509Name<'response_encoded>, &X509Name<'certificate_encoded>) -> bool,
{
    let kind = match response.response_data().responder_id() {
        X509OcspResponderId::ByName(responder_name) => {
            if !names_equal(responder_name, certificate.tbs_certificate().subject()) {
                return Err(X509OcspResponderIdBindingError::new(
                    X509OcspResponderIdBindingErrorKind::NameMismatch,
                ));
            }
            X509OcspResponderIdKind::ByName
        }
        X509OcspResponderId::ByKey(responder_key_hash) => {
            let responder_key_hash: &[u8; X509_OCSP_RESPONDER_KEY_HASH_LENGTH] =
                (*responder_key_hash).try_into().map_err(|_| {
                    X509OcspResponderIdBindingError::new(
                        X509OcspResponderIdBindingErrorKind::InvalidKeyHashLength,
                    )
                })?;
            let subject_public_key = certificate
                .tbs_certificate()
                .subject_public_key_info()
                .subject_public_key();
            let computed_key_hash = sum1(subject_public_key.bytes());
            if !ct_eq_fixed(&computed_key_hash, responder_key_hash) {
                return Err(X509OcspResponderIdBindingError::new(
                    X509OcspResponderIdBindingErrorKind::KeyHashMismatch,
                ));
            }
            X509OcspResponderIdKind::ByKey
        }
    };

    Ok(BoundX509OcspResponderCertificate {
        response,
        certificate,
        kind,
    })
}
