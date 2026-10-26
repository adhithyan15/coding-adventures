//! # Exact issuer binding for authenticated OCSP responder candidates
//!
//! An authenticated responder candidate and a valid certificate issuer link are
//! insufficient when their evidence can retain different responder certificate
//! objects. This crate consumes both proofs, rejects cross-certificate mixing,
//! and returns non-copyable evidence retaining the exact authenticated response,
//! responder certificate, and candidate issuer relationship.
//!
//! The result proves that one candidate issuer signed the policy-approved
//! responder certificate whose key signed one exact OCSP response. It does not
//! establish that the candidate issuer issued a particular target certificate,
//! bind a `CertID` or revocation decision, validate a responder path, establish
//! trust, check responder revocation, or grant TLS, HTTP, OAuth-transport, or
//! network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_ocsp_authenticated_responder::AuthenticatedX509OcspRsaSha256Responder;
use x509_rsa_sha256_issuer_link::VerifiedX509RsaSha256IssuerLink;

/// Stable failures that retain no response, certificate, key, or signature bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspIssuerBoundResponderErrorKind {
    /// Authentication and issuer-link evidence retain distinct responder objects.
    EvidenceMismatch,
}

/// A redacted failure while binding responder authentication to an issuer link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspIssuerBoundResponderError {
    kind: X509OcspIssuerBoundResponderErrorKind,
}

impl X509OcspIssuerBoundResponderError {
    const fn new(kind: X509OcspIssuerBoundResponderErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspIssuerBoundResponderErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspIssuerBoundResponderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP issuer-bound responder error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspIssuerBoundResponderError {}

/// Non-copyable evidence binding one authenticated responder to one issuer.
///
/// The retained responder proof covers exact `ResponderID`, certificate policy,
/// and response signature evidence. The retained issuer link proves that the
/// same responder certificate was signed by the exact candidate issuer, whose
/// issuing-CA role and RSA public key were validated by that link.
pub struct IssuerBoundX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
> {
    authenticated_responder: AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link:
        VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>,
}

impl<'response, 'response_encoded, 'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    IssuerBoundX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return authentication for the exact response and responder certificate.
    pub const fn authenticated_responder(
        &self,
    ) -> &AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    > {
        &self.authenticated_responder
    }

    /// Return the exact responder-certificate-to-candidate-issuer link.
    pub const fn responder_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    {
        &self.responder_issuer_link
    }
}

impl fmt::Debug for IssuerBoundX509OcspRsaSha256Responder<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IssuerBoundX509OcspRsaSha256Responder")
    }
}

/// Bind one exact authenticated responder to its exact candidate issuer link.
///
/// Object identity is checked before evidence is returned. Byte-equal but
/// separately decoded responder certificates are rejected so a valid issuer
/// link cannot be attached to authentication evidence for another object.
pub fn bind_x509_ocsp_authenticated_responder_to_issuer<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
>(
    authenticated_responder: AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link: VerifiedX509RsaSha256IssuerLink<
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    IssuerBoundX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspIssuerBoundResponderError,
> {
    if !std::ptr::eq(
        authenticated_responder
            .responder_certificate_policy()
            .certificate(),
        responder_issuer_link.child(),
    ) {
        return Err(X509OcspIssuerBoundResponderError::new(
            X509OcspIssuerBoundResponderErrorKind::EvidenceMismatch,
        ));
    }

    Ok(IssuerBoundX509OcspRsaSha256Responder {
        authenticated_responder,
        responder_issuer_link,
    })
}
