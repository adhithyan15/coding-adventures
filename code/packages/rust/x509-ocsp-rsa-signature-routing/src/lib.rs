//! # Policy-bound OCSP RSA signature routing
//!
//! Fixed signature verifiers deliberately do not choose algorithms. This
//! crate supplies the narrow composition step between caller-owned,
//! response-bound algorithm/key policy and the repository-owned OCSP RSA
//! PKCS #1 v1.5 SHA-256 and SHA-512 verifiers.
//!
//! The routing table is public policy data keyed only by the selected profile
//! index. Verification consumes the non-copyable policy evidence and derives
//! the exact response and key from it, preventing detached response or key
//! substitution. Missing and duplicate routes fail before cryptography. A
//! route that names the wrong fixed verifier fails its verifier's exact
//! algorithm-identifier and parameter checks.
//!
//! This evidence does not authorize or match a responder, bind `CertID` or
//! status to a certificate or path, decide freshness or revocation, establish
//! trust, perform TLS or HTTP, authorize OAuth transport, or grant network
//! authority.

#![deny(unsafe_code)]

use core::fmt;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_ocsp_rsa_algorithm_policy::ValidatedX509OcspRsaAlgorithmPolicy;
use x509_ocsp_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature, VerifiedX509OcspRsaSha256Signature,
    X509OcspRsaSha256SignatureErrorKind,
};
use x509_ocsp_rsa_pkcs1_v1_5_sha512_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature, VerifiedX509OcspRsaSha512Signature,
    X509OcspRsaSha512SignatureErrorKind,
};

/// One reviewed fixed verifier available to a caller-owned routing table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaSignatureVerifier {
    RsaPkcs1V15Sha256,
    RsaPkcs1V15Sha512,
}

/// Bind one algorithm-policy profile index to one fixed verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaSignatureRoute {
    profile_index: usize,
    verifier: X509OcspRsaSignatureVerifier,
}

impl X509OcspRsaSignatureRoute {
    pub const fn new(profile_index: usize, verifier: X509OcspRsaSignatureVerifier) -> Self {
        Self {
            profile_index,
            verifier,
        }
    }

    pub const fn profile_index(self) -> usize {
        self.profile_index
    }

    pub const fn verifier(self) -> X509OcspRsaSignatureVerifier {
        self.verifier
    }
}

/// Stable payload-free routing and verification failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaSignatureRoutingErrorKind {
    UnsupportedProfileRoute,
    AmbiguousProfileRoute,
    Sha256(X509OcspRsaSha256SignatureErrorKind),
    Sha512(X509OcspRsaSha512SignatureErrorKind),
}

/// A redacted routing or fixed-verifier failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaSignatureRoutingError {
    kind: X509OcspRsaSignatureRoutingErrorKind,
}

impl X509OcspRsaSignatureRoutingError {
    const fn new(kind: X509OcspRsaSignatureRoutingErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509OcspRsaSignatureRoutingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRsaSignatureRoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP RSA signature routing error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRsaSignatureRoutingError {}

/// Policy and SHA-256 signature evidence for the same exact response and key.
pub struct VerifiedX509OcspRsaSha256PolicySignature<'response, 'encoded, 'key> {
    algorithm_policy: ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    signature: VerifiedX509OcspRsaSha256Signature<'response, 'encoded, 'key>,
}

impl<'response, 'encoded, 'key>
    VerifiedX509OcspRsaSha256PolicySignature<'response, 'encoded, 'key>
{
    pub const fn algorithm_policy(
        &self,
    ) -> &ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key> {
        &self.algorithm_policy
    }

    pub const fn signature(
        &self,
    ) -> &VerifiedX509OcspRsaSha256Signature<'response, 'encoded, 'key> {
        &self.signature
    }
}

impl fmt::Debug for VerifiedX509OcspRsaSha256PolicySignature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509OcspRsaSha256PolicySignature")
    }
}

/// Policy and SHA-512 signature evidence for the same exact response and key.
pub struct VerifiedX509OcspRsaSha512PolicySignature<'response, 'encoded, 'key> {
    algorithm_policy: ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    signature: VerifiedX509OcspRsaSha512Signature<'response, 'encoded, 'key>,
}

impl<'response, 'encoded, 'key>
    VerifiedX509OcspRsaSha512PolicySignature<'response, 'encoded, 'key>
{
    pub const fn algorithm_policy(
        &self,
    ) -> &ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key> {
        &self.algorithm_policy
    }

    pub const fn signature(
        &self,
    ) -> &VerifiedX509OcspRsaSha512Signature<'response, 'encoded, 'key> {
        &self.signature
    }
}

impl fmt::Debug for VerifiedX509OcspRsaSha512PolicySignature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509OcspRsaSha512PolicySignature")
    }
}

/// Non-copyable policy-bound signature evidence from one fixed verifier.
pub enum VerifiedX509OcspRsaPolicySignature<'response, 'encoded, 'key> {
    Sha256(VerifiedX509OcspRsaSha256PolicySignature<'response, 'encoded, 'key>),
    Sha512(VerifiedX509OcspRsaSha512PolicySignature<'response, 'encoded, 'key>),
}

impl VerifiedX509OcspRsaPolicySignature<'_, '_, '_> {
    pub const fn verifier(&self) -> X509OcspRsaSignatureVerifier {
        match self {
            Self::Sha256(_) => X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256,
            Self::Sha512(_) => X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha512,
        }
    }
}

impl fmt::Debug for VerifiedX509OcspRsaPolicySignature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sha256(_) => "VerifiedX509OcspRsaPolicySignature::Sha256",
            Self::Sha512(_) => "VerifiedX509OcspRsaPolicySignature::Sha512",
        })
    }
}

/// Select exactly one profile-index route and invoke its fixed verifier.
pub fn verify_x509_ocsp_rsa_policy_signature<'response, 'encoded, 'key>(
    algorithm_policy: ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    routes: &[X509OcspRsaSignatureRoute],
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509OcspRsaPolicySignature<'response, 'encoded, 'key>,
    X509OcspRsaSignatureRoutingError,
> {
    let profile_index = algorithm_policy.profile_index();
    let mut selected = None;
    for route in routes {
        if route.profile_index == profile_index {
            if selected.is_some() {
                return Err(X509OcspRsaSignatureRoutingError::new(
                    X509OcspRsaSignatureRoutingErrorKind::AmbiguousProfileRoute,
                ));
            }
            selected = Some(route.verifier);
        }
    }
    let verifier = selected.ok_or_else(|| {
        X509OcspRsaSignatureRoutingError::new(
            X509OcspRsaSignatureRoutingErrorKind::UnsupportedProfileRoute,
        )
    })?;

    use X509OcspRsaSignatureVerifier::{RsaPkcs1V15Sha256, RsaPkcs1V15Sha512};
    match verifier {
        RsaPkcs1V15Sha256 => sha256(algorithm_policy, operation_limits),
        RsaPkcs1V15Sha512 => sha512(algorithm_policy, operation_limits),
    }
}

fn sha256<'response, 'encoded, 'key>(
    algorithm_policy: ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509OcspRsaPolicySignature<'response, 'encoded, 'key>,
    X509OcspRsaSignatureRoutingError,
> {
    let signature = verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(
        algorithm_policy.response(),
        algorithm_policy.responder_public_key(),
        operation_limits,
    )
    .map_err(|error| {
        X509OcspRsaSignatureRoutingError::new(X509OcspRsaSignatureRoutingErrorKind::Sha256(
            error.kind(),
        ))
    })?;
    Ok(VerifiedX509OcspRsaPolicySignature::Sha256(
        VerifiedX509OcspRsaSha256PolicySignature {
            algorithm_policy,
            signature,
        },
    ))
}

fn sha512<'response, 'encoded, 'key>(
    algorithm_policy: ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509OcspRsaPolicySignature<'response, 'encoded, 'key>,
    X509OcspRsaSignatureRoutingError,
> {
    let signature = verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(
        algorithm_policy.response(),
        algorithm_policy.responder_public_key(),
        operation_limits,
    )
    .map_err(|error| {
        X509OcspRsaSignatureRoutingError::new(X509OcspRsaSignatureRoutingErrorKind::Sha512(
            error.kind(),
        ))
    })?;
    Ok(VerifiedX509OcspRsaPolicySignature::Sha512(
        VerifiedX509OcspRsaSha512PolicySignature {
            algorithm_policy,
            signature,
        },
    ))
}
