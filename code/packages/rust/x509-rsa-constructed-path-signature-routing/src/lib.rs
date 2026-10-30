//! # Constructed-path RSA signature routing
//!
//! This crate binds one exact adjacency projected from a constructed X.509
//! path to certificate-bound child algorithm policy, certificate-wide issuer
//! role policy, and certificate-bound issuer RSA algorithm/key policy. Only
//! after all exact-object and issuing-CA checks pass does caller-owned routing
//! invoke one repository-owned fixed RSA certificate-signature verifier.
//!
//! It authenticates one projected link only. It does not validate a complete
//! path, establish trust, check revocation, perform TLS or HTTP, or grant OAuth
//! or network authority.

#![deny(unsafe_code)]

use core::fmt;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate_path_construction::ConstructedX509CertificatePathAdjacentLink;
use x509_certificate_policy::{
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateRolePolicy, X509CertificateRole,
};
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureRoutingErrorKind,
};

/// Stable, payload-free constructed-path RSA routing failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathSignatureRoutingErrorKind {
    /// The projected child and child algorithm policy retain different objects.
    ChildEvidenceMismatch,
    /// The projected issuer and issuer role policy retain different objects.
    IssuerRoleEvidenceMismatch,
    /// The projected issuer is not authorized as an issuing CA.
    IssuerNotCertificateAuthority,
    /// The projected issuer and RSA algorithm/key policy retain different objects.
    IssuerAlgorithmEvidenceMismatch,
    /// Data-driven routing or the selected fixed verifier failed.
    Signature(X509RsaSignatureRoutingErrorKind),
}

/// A redacted constructed-path RSA routing failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathSignatureRoutingError {
    kind: X509RsaConstructedPathSignatureRoutingErrorKind,
}

impl X509RsaConstructedPathSignatureRoutingError {
    const fn new(kind: X509RsaConstructedPathSignatureRoutingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathSignatureRoutingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathSignatureRoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path signature routing error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathSignatureRoutingError {}

/// Non-copyable evidence authenticating one exact constructed-path adjacency.
///
/// The retained projection preserves path order. The retained issuer role
/// policy proves certificate-wide critical-extension recognition and issuing-
/// CA role for the projected issuer. Routed signature evidence retains the
/// exact child algorithm policy, issuer RSA algorithm/key policy, and selected
/// fixed-verifier result. This is not complete path validation or trust.
pub struct VerifiedX509RsaConstructedPathPolicySignature<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    adjacent_link: ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
    signature: VerifiedX509RsaPolicySignature<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
}

impl<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
    VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the exact construction-bound adjacent pair that was authenticated.
    #[must_use]
    pub const fn adjacent_link(
        &self,
    ) -> &ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.adjacent_link
    }

    /// Return the complete issuing-CA role policy for the projected issuer.
    #[must_use]
    pub const fn issuer_role_policy(
        &self,
    ) -> &ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.issuer_role_policy
    }

    /// Return the routed exact-certificate signature evidence.
    #[must_use]
    pub const fn signature(
        &self,
    ) -> &VerifiedX509RsaPolicySignature<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.signature
    }
}

impl fmt::Debug
    for VerifiedX509RsaConstructedPathPolicySignature<'_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaConstructedPathPolicySignature")
    }
}

/// Authenticate one exact adjacency projected from a constructed path.
///
/// Object identity, not DER equality, binds every policy input to the
/// projected endpoints. All identity and issuing-CA checks complete before
/// data-driven routing invokes a fixed RSA verifier. Success retains every
/// consumed proof so no endpoint or policy can be substituted afterward.
pub fn verify_x509_rsa_constructed_path_policy_signature<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    adjacent_link: ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    child_algorithm_policy: ValidatedCertificateAlgorithmPolicy<'child_certificate, 'child_encoded>,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
    issuer_rsa_algorithm_policy: ValidatedX509RsaAlgorithmPolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
    routes: &[X509RsaSignatureRoute],
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaConstructedPathSignatureRoutingError,
> {
    let child_matches = adjacent_link
        .child_target_certificate()
        .is_some_and(|child| same_exact_object(child, child_algorithm_policy.certificate()))
        || adjacent_link
            .child_candidate_certificate()
            .is_some_and(|child| same_exact_object(child, child_algorithm_policy.certificate()));
    if !child_matches {
        return Err(X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::ChildEvidenceMismatch,
        ));
    }

    if !same_exact_object(
        adjacent_link.issuer_certificate(),
        issuer_role_policy.certificate(),
    ) {
        return Err(X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::IssuerRoleEvidenceMismatch,
        ));
    }
    if issuer_role_policy.role_extensions().role()
        != X509CertificateRole::IssuingCertificateAuthority
    {
        return Err(X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::IssuerNotCertificateAuthority,
        ));
    }
    if !same_exact_object(
        adjacent_link.issuer_certificate(),
        issuer_rsa_algorithm_policy.certificate(),
    ) {
        return Err(X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::IssuerAlgorithmEvidenceMismatch,
        ));
    }

    let signature = verify_x509_rsa_policy_signature(
        child_algorithm_policy,
        issuer_rsa_algorithm_policy,
        routes,
        operation_limits,
    )
    .map_err(|error| {
        X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::Signature(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaConstructedPathPolicySignature {
        adjacent_link,
        issuer_role_policy,
        signature,
    })
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    core::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_object_identity_rejects_byte_equal_distinct_values() {
        let left = [1_u8, 2, 3];
        let right = [1_u8, 2, 3];

        assert!(same_exact_object(&left, &left));
        assert!(!same_exact_object(&left, &right));
    }

    #[test]
    fn mismatch_error_is_stable_and_payload_free() {
        let error = X509RsaConstructedPathSignatureRoutingError::new(
            X509RsaConstructedPathSignatureRoutingErrorKind::ChildEvidenceMismatch,
        );

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathSignatureRoutingErrorKind::ChildEvidenceMismatch
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA constructed-path signature routing error ChildEvidenceMismatch"
        );
    }
}
