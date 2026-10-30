//! # Policy-bound X.509 RSA signature routing
//!
//! Fixed certificate-signature verifiers deliberately do not choose
//! algorithms. This crate supplies the narrow composition step between exact
//! child-certificate algorithm policy, exact issuer-certificate RSA key
//! policy, and the fixed repository-owned PKCS #1 v1.5 SHA-256, SHA-384, and
//! SHA-512 verifiers.
//!
//! The caller owns the profile-index routing table. Missing and duplicate
//! routes fail before cryptography. A mismatched route fails the selected
//! fixed verifier's exact AlgorithmIdentifier checks. Success retains both
//! non-copyable policy inputs and exact verification evidence.
//!
//! This does not decide issuer linkage or role, validate a path or trust,
//! check target policy or revocation, perform TLS or HTTP, or grant OAuth or
//! network authority.

#![deny(unsafe_code)]

use core::fmt;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate_policy::ValidatedCertificateAlgorithmPolicy;
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha256_signature, VerifiedX509RsaSha256Signature,
    X509RsaSha256SignatureErrorKind,
};
use x509_rsa_pkcs1_v1_5_sha384_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha384_signature, VerifiedX509RsaSha384Signature,
    X509RsaSha384SignatureErrorKind,
};
use x509_rsa_pkcs1_v1_5_sha512_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha512_signature, VerifiedX509RsaSha512Signature,
    X509RsaSha512SignatureErrorKind,
};

/// One reviewed fixed verifier available to caller-owned route policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSignatureVerifier {
    RsaPkcs1V15Sha256,
    RsaPkcs1V15Sha384,
    RsaPkcs1V15Sha512,
}

/// Bind one selected child algorithm-profile index to one fixed verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSignatureRoute {
    profile_index: usize,
    verifier: X509RsaSignatureVerifier,
}

impl X509RsaSignatureRoute {
    #[must_use]
    pub const fn new(profile_index: usize, verifier: X509RsaSignatureVerifier) -> Self {
        Self {
            profile_index,
            verifier,
        }
    }

    #[must_use]
    pub const fn profile_index(self) -> usize {
        self.profile_index
    }

    #[must_use]
    pub const fn verifier(self) -> X509RsaSignatureVerifier {
        self.verifier
    }
}

/// Stable payload-free routing and verification failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSignatureRoutingErrorKind {
    UnsupportedProfileRoute,
    AmbiguousProfileRoute,
    Sha256(X509RsaSha256SignatureErrorKind),
    Sha384(X509RsaSha384SignatureErrorKind),
    Sha512(X509RsaSha512SignatureErrorKind),
}

/// A redacted routing or fixed-verifier failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSignatureRoutingError {
    kind: X509RsaSignatureRoutingErrorKind,
}

impl X509RsaSignatureRoutingError {
    const fn new(kind: X509RsaSignatureRoutingErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> X509RsaSignatureRoutingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSignatureRoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA signature routing error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSignatureRoutingError {}

macro_rules! policy_signature {
    ($name:ident, $signature:ident, $debug_name:literal) => {
        pub struct $name<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded> {
            child_algorithm_policy:
                ValidatedCertificateAlgorithmPolicy<'child_certificate, 'child_encoded>,
            issuer_rsa_algorithm_policy:
                ValidatedX509RsaAlgorithmPolicy<'issuer_certificate, 'issuer_encoded>,
            signature: $signature<'child_certificate, 'child_encoded, 'issuer_encoded>,
        }

        impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
            $name<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
        {
            #[must_use]
            pub const fn child_algorithm_policy(
                &self,
            ) -> &ValidatedCertificateAlgorithmPolicy<'child_certificate, 'child_encoded> {
                &self.child_algorithm_policy
            }

            #[must_use]
            pub const fn issuer_rsa_algorithm_policy(
                &self,
            ) -> &ValidatedX509RsaAlgorithmPolicy<'issuer_certificate, 'issuer_encoded> {
                &self.issuer_rsa_algorithm_policy
            }

            #[must_use]
            pub const fn signature(
                &self,
            ) -> &$signature<'child_certificate, 'child_encoded, 'issuer_encoded> {
                &self.signature
            }
        }

        impl fmt::Debug for $name<'_, '_, '_, '_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($debug_name)
            }
        }
    };
}

policy_signature!(
    VerifiedX509RsaSha256PolicySignature,
    VerifiedX509RsaSha256Signature,
    "VerifiedX509RsaSha256PolicySignature"
);
policy_signature!(
    VerifiedX509RsaSha384PolicySignature,
    VerifiedX509RsaSha384Signature,
    "VerifiedX509RsaSha384PolicySignature"
);
policy_signature!(
    VerifiedX509RsaSha512PolicySignature,
    VerifiedX509RsaSha512Signature,
    "VerifiedX509RsaSha512PolicySignature"
);

/// Non-copyable policy-bound signature evidence from one fixed verifier.
pub enum VerifiedX509RsaPolicySignature<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    Sha256(
        VerifiedX509RsaSha256PolicySignature<
            'child_certificate,
            'child_encoded,
            'issuer_certificate,
            'issuer_encoded,
        >,
    ),
    Sha384(
        VerifiedX509RsaSha384PolicySignature<
            'child_certificate,
            'child_encoded,
            'issuer_certificate,
            'issuer_encoded,
        >,
    ),
    Sha512(
        VerifiedX509RsaSha512PolicySignature<
            'child_certificate,
            'child_encoded,
            'issuer_certificate,
            'issuer_encoded,
        >,
    ),
}

impl VerifiedX509RsaPolicySignature<'_, '_, '_, '_> {
    #[must_use]
    pub const fn verifier(&self) -> X509RsaSignatureVerifier {
        match self {
            Self::Sha256(_) => X509RsaSignatureVerifier::RsaPkcs1V15Sha256,
            Self::Sha384(_) => X509RsaSignatureVerifier::RsaPkcs1V15Sha384,
            Self::Sha512(_) => X509RsaSignatureVerifier::RsaPkcs1V15Sha512,
        }
    }
}

impl fmt::Debug for VerifiedX509RsaPolicySignature<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sha256(_) => "VerifiedX509RsaPolicySignature::Sha256",
            Self::Sha384(_) => "VerifiedX509RsaPolicySignature::Sha384",
            Self::Sha512(_) => "VerifiedX509RsaPolicySignature::Sha512",
        })
    }
}

/// Select exactly one child-profile route and invoke its fixed verifier.
pub fn verify_x509_rsa_policy_signature<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child_algorithm_policy: ValidatedCertificateAlgorithmPolicy<'child_certificate, 'child_encoded>,
    issuer_rsa_algorithm_policy: ValidatedX509RsaAlgorithmPolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
    routes: &[X509RsaSignatureRoute],
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509RsaPolicySignature<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSignatureRoutingError,
> {
    let profile_index = child_algorithm_policy.algorithm_profile().profile_index();
    let mut selected = None;
    for route in routes {
        if route.profile_index == profile_index {
            if selected.is_some() {
                return Err(X509RsaSignatureRoutingError::new(
                    X509RsaSignatureRoutingErrorKind::AmbiguousProfileRoute,
                ));
            }
            selected = Some(route.verifier);
        }
    }
    let verifier = selected.ok_or_else(|| {
        X509RsaSignatureRoutingError::new(X509RsaSignatureRoutingErrorKind::UnsupportedProfileRoute)
    })?;

    match verifier {
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256 => {
            let signature = verify_x509_rsa_pkcs1_v1_5_sha256_signature(
                child_algorithm_policy.certificate(),
                issuer_rsa_algorithm_policy.rsa_public_key(),
                operation_limits,
            )
            .map_err(|error| {
                X509RsaSignatureRoutingError::new(X509RsaSignatureRoutingErrorKind::Sha256(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedX509RsaPolicySignature::Sha256(
                VerifiedX509RsaSha256PolicySignature {
                    child_algorithm_policy,
                    issuer_rsa_algorithm_policy,
                    signature,
                },
            ))
        }
        X509RsaSignatureVerifier::RsaPkcs1V15Sha384 => {
            let signature = verify_x509_rsa_pkcs1_v1_5_sha384_signature(
                child_algorithm_policy.certificate(),
                issuer_rsa_algorithm_policy.rsa_public_key(),
                operation_limits,
            )
            .map_err(|error| {
                X509RsaSignatureRoutingError::new(X509RsaSignatureRoutingErrorKind::Sha384(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedX509RsaPolicySignature::Sha384(
                VerifiedX509RsaSha384PolicySignature {
                    child_algorithm_policy,
                    issuer_rsa_algorithm_policy,
                    signature,
                },
            ))
        }
        X509RsaSignatureVerifier::RsaPkcs1V15Sha512 => {
            let signature = verify_x509_rsa_pkcs1_v1_5_sha512_signature(
                child_algorithm_policy.certificate(),
                issuer_rsa_algorithm_policy.rsa_public_key(),
                operation_limits,
            )
            .map_err(|error| {
                X509RsaSignatureRoutingError::new(X509RsaSignatureRoutingErrorKind::Sha512(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedX509RsaPolicySignature::Sha512(
                VerifiedX509RsaSha512PolicySignature {
                    child_algorithm_policy,
                    issuer_rsa_algorithm_policy,
                    signature,
                },
            ))
        }
    }
}
