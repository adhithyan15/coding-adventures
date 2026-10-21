//! # Bounded RSA SHA-256 X.509 issuer links
//!
//! This crate composes one deliberately narrow certificate-link boundary. It
//! applies caller-owned distinguished-name policy in the child-to-issuer
//! direction, requires the candidate issuer's algorithm-independent CA role,
//! validates that certificate's RSA public key under caller-owned strength
//! policy, and verifies the child's exact RSA PKCS #1 v1.5 SHA-256 signature.
//!
//! The narrow result binds the exact child and candidate issuer. A stronger
//! entry point additionally retains caller-owned critical-extension
//! recognition and issuing-CA role-policy evidence for that exact selected
//! issuer. Recognition is not semantic validation of other critical
//! extensions. Neither result establishes validity, path length, name
//! constraints, revocation, trust anchors, path construction, TLS, HTTP, or
//! network effects.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::X509Certificate;
use x509_certificate_issuer_candidate_selection::SelectedX509IssuerCandidate;
use x509_certificate_policy::{
    validate_certificate_issuer_link, validate_certificate_role_extensions,
    ValidatedCertificateRolePolicy, X509CertificateIssuerLinkErrorKind, X509CertificateRole,
    X509CertificateRoleErrorKind,
};
use x509_name::X509Name;
use x509_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha256_signature, X509RsaSha256SignatureErrorKind,
};
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

/// Caller-owned limits and RSA strength policy for one issuer link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256IssuerLinkPolicy<'policy> {
    issuer_role_limits: Asn1Limits,
    issuer_key_policy: RsaPublicKeyPolicy<'policy>,
    issuer_key_limits: Asn1Limits,
    public_operation_limits: RsaPublicOperationLimits,
}

impl<'policy> X509RsaSha256IssuerLinkPolicy<'policy> {
    pub const fn new(
        issuer_role_limits: Asn1Limits,
        issuer_key_policy: RsaPublicKeyPolicy<'policy>,
        issuer_key_limits: Asn1Limits,
        public_operation_limits: RsaPublicOperationLimits,
    ) -> Self {
        Self {
            issuer_role_limits,
            issuer_key_policy,
            issuer_key_limits,
            public_operation_limits,
        }
    }

    pub const fn issuer_role_limits(self) -> Asn1Limits {
        self.issuer_role_limits
    }

    pub const fn issuer_key_policy(self) -> RsaPublicKeyPolicy<'policy> {
        self.issuer_key_policy
    }

    pub const fn issuer_key_limits(self) -> Asn1Limits {
        self.issuer_key_limits
    }

    pub const fn public_operation_limits(self) -> RsaPublicOperationLimits {
        self.public_operation_limits
    }
}

/// Stable issuer-link failure categories that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256IssuerLinkErrorKind {
    IssuerName(X509CertificateIssuerLinkErrorKind),
    IssuerRole(X509CertificateRoleErrorKind),
    IssuerPublicKey(RsaPublicKeyErrorKind),
    ChildSignature(X509RsaSha256SignatureErrorKind),
}

/// A redacted authenticated issuer-link failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256IssuerLinkError {
    kind: X509RsaSha256IssuerLinkErrorKind,
}

impl X509RsaSha256IssuerLinkError {
    const fn new(kind: X509RsaSha256IssuerLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256IssuerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256IssuerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 issuer-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256IssuerLinkError {}

/// Stable stronger-composition failure categories that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256IssuerRolePolicyLinkErrorKind {
    /// The role-policy evidence is not for the exact selected issuing CA.
    EvidenceMismatch,
    /// The selected certificate pair failed the existing issuer-link checks.
    IssuerLink(X509RsaSha256IssuerLinkErrorKind),
}

/// A redacted failure while composing selection, role-policy, and issuer-link evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256IssuerRolePolicyLinkError {
    kind: X509RsaSha256IssuerRolePolicyLinkErrorKind,
}

impl X509RsaSha256IssuerRolePolicyLinkError {
    const fn new(kind: X509RsaSha256IssuerRolePolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256IssuerRolePolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256IssuerRolePolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 issuer role-policy link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256IssuerRolePolicyLinkError {}

/// Non-copyable evidence for one exact child-to-candidate-issuer link.
///
/// This evidence proves only the directed Name-policy result, issuing-CA role,
/// RSA key policy, and exact child signature checks performed by
/// [`verify_x509_rsa_sha256_issuer_link`]. It does not establish a valid or
/// trusted certification path.
pub struct VerifiedX509RsaSha256IssuerLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    issuer_public_key: ValidatedRsaPublicKey<'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    pub const fn child(&self) -> &'child_certificate X509Certificate<'child_encoded> {
        self.child
    }

    pub const fn candidate_issuer(&self) -> &'issuer_certificate X509Certificate<'issuer_encoded> {
        self.candidate_issuer
    }

    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'issuer_encoded> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha256IssuerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256IssuerLink")
    }
}

/// Non-copyable evidence that retains both an authenticated selected link and
/// certificate-wide issuing-CA role-policy evidence for its exact issuer.
///
/// This is intentionally distinct from [`VerifiedX509RsaSha256IssuerLink`]. It
/// additionally proves that caller-owned critical-extension recognition and
/// the issuing-CA role gate were applied to the exact selected issuer. It does
/// not semantically validate recognized non-role extensions or establish a
/// complete or trusted certification path.
pub struct VerifiedX509RsaSha256IssuerRolePolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    verified_link: VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha256IssuerRolePolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    pub const fn verified_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.verified_link
    }

    pub const fn issuer_role_policy(
        &self,
    ) -> &ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.issuer_role_policy
    }
}

impl fmt::Debug for VerifiedX509RsaSha256IssuerRolePolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256IssuerRolePolicyLink")
    }
}

/// Verify one directed RSA PKCS #1 v1.5 SHA-256 issuer link.
///
/// `same_name` is called exactly once with the child issuer Name followed by
/// the candidate issuer subject Name. All checks use the same exact candidate
/// certificate before non-copyable evidence is returned.
pub fn verify_x509_rsa_sha256_issuer_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'child_encoded>, &X509Name<'issuer_encoded>) -> bool,
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IssuerLinkError,
> {
    validate_certificate_issuer_link(child, candidate_issuer, same_name).map_err(|error| {
        X509RsaSha256IssuerLinkError::new(X509RsaSha256IssuerLinkErrorKind::IssuerName(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_sha256_issuer_link_after_name(child, candidate_issuer, policy)
}

/// Authenticate the exact child and issuer already bound by unique selection.
///
/// This consuming entry point performs no second Name comparison. It derives
/// both certificate references from `selected_candidate`, then applies the
/// issuing-CA role, caller-owned RSA key policy, and exact child-signature
/// checks to that same selected candidate. Selection evidence is not by itself
/// authentication, and this result still does not establish a complete or
/// trusted certification path.
pub fn verify_selected_x509_rsa_sha256_issuer_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    selected_candidate: SelectedX509IssuerCandidate<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IssuerLinkError,
> {
    verify_x509_rsa_sha256_issuer_link_after_name(
        selected_candidate.child_certificate(),
        selected_candidate.candidate_certificate(),
        policy,
    )
}

/// Authenticate one selected issuer while retaining certificate-wide role-policy evidence.
///
/// Before any issuer role, key, or signature work, this consuming entry point
/// requires `issuer_role_policy` to retain the exact selected candidate object
/// and to have been produced for the issuing-CA role. The existing bounded
/// issuer-link checks then run unchanged, including role validation under the
/// supplied policy limits. The distinct result retains both non-copyable proof
/// objects and grants no path, trust, time, identity, revocation, TLS, HTTP, or
/// network authority.
pub fn verify_selected_x509_rsa_sha256_issuer_link_with_role_policy<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    selected_candidate: SelectedX509IssuerCandidate<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256IssuerRolePolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IssuerRolePolicyLinkError,
> {
    if !std::ptr::eq(
        selected_candidate.candidate_certificate(),
        issuer_role_policy.certificate(),
    ) || issuer_role_policy.role_extensions().role()
        != X509CertificateRole::IssuingCertificateAuthority
    {
        return Err(X509RsaSha256IssuerRolePolicyLinkError::new(
            X509RsaSha256IssuerRolePolicyLinkErrorKind::EvidenceMismatch,
        ));
    }

    let verified_link = verify_selected_x509_rsa_sha256_issuer_link(selected_candidate, policy)
        .map_err(|error| {
            X509RsaSha256IssuerRolePolicyLinkError::new(
                X509RsaSha256IssuerRolePolicyLinkErrorKind::IssuerLink(error.kind()),
            )
        })?;

    Ok(VerifiedX509RsaSha256IssuerRolePolicyLink {
        verified_link,
        issuer_role_policy,
    })
}

fn verify_x509_rsa_sha256_issuer_link_after_name<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IssuerLinkError,
> {
    validate_certificate_role_extensions(
        candidate_issuer,
        X509CertificateRole::IssuingCertificateAuthority,
        policy.issuer_role_limits,
    )
    .map_err(|error| {
        X509RsaSha256IssuerLinkError::new(X509RsaSha256IssuerLinkErrorKind::IssuerRole(
            error.kind(),
        ))
    })?;

    let issuer_public_key = validate_rsa_public_key(
        candidate_issuer.tbs_certificate().subject_public_key_info(),
        policy.issuer_key_policy,
        policy.issuer_key_limits,
    )
    .map_err(|error| {
        X509RsaSha256IssuerLinkError::new(X509RsaSha256IssuerLinkErrorKind::IssuerPublicKey(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_pkcs1_v1_5_sha256_signature(
        child,
        issuer_public_key,
        policy.public_operation_limits,
    )
    .map_err(|error| {
        X509RsaSha256IssuerLinkError::new(X509RsaSha256IssuerLinkErrorKind::ChildSignature(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509RsaSha256IssuerLink {
        child,
        candidate_issuer,
        issuer_public_key,
    })
}
