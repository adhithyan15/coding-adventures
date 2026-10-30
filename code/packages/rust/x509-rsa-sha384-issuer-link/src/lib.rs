//! # Bounded RSA SHA-384 X.509 issuer links
//!
//! This crate composes one narrow certificate-link boundary. It applies
//! caller-owned distinguished-name policy in the child-to-issuer direction,
//! requires the candidate issuer's algorithm-independent CA role, validates
//! that same certificate's RSA public key under caller-owned strength policy,
//! and verifies the child's exact RSA PKCS #1 v1.5 SHA-384 signature.
//!
//! Success authenticates only this exact pair. It does not route algorithms,
//! validate a complete path, establish reviewed trust, check revocation, or
//! provide TLS, HTTP, OAuth transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::X509Certificate;
use x509_certificate_policy::{
    validate_certificate_issuer_link, validate_certificate_role_extensions,
    ValidatedCertificateRolePolicy, X509CertificateIssuerLinkErrorKind, X509CertificateRole,
    X509CertificateRoleErrorKind,
};
use x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy;
use x509_name::X509Name;
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_pkcs1_v1_5_sha384_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha384_signature, X509RsaSha384SignatureErrorKind,
};
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

/// Caller-owned limits and RSA strength policy for one issuer link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384IssuerLinkPolicy<'policy> {
    issuer_role_limits: Asn1Limits,
    issuer_key_policy: RsaPublicKeyPolicy<'policy>,
    issuer_key_limits: Asn1Limits,
    public_operation_limits: RsaPublicOperationLimits,
}

impl<'policy> X509RsaSha384IssuerLinkPolicy<'policy> {
    #[must_use]
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

    #[must_use]
    pub const fn issuer_role_limits(self) -> Asn1Limits {
        self.issuer_role_limits
    }

    #[must_use]
    pub const fn issuer_key_policy(self) -> RsaPublicKeyPolicy<'policy> {
        self.issuer_key_policy
    }

    #[must_use]
    pub const fn issuer_key_limits(self) -> Asn1Limits {
        self.issuer_key_limits
    }

    #[must_use]
    pub const fn public_operation_limits(self) -> RsaPublicOperationLimits {
        self.public_operation_limits
    }
}

/// Stable issuer-link failure categories that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384IssuerLinkErrorKind {
    IssuerName(X509CertificateIssuerLinkErrorKind),
    IssuerRole(X509CertificateRoleErrorKind),
    IssuerPublicKey(RsaPublicKeyErrorKind),
    ChildSignature(X509RsaSha384SignatureErrorKind),
}

/// A redacted authenticated issuer-link failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384IssuerLinkError {
    kind: X509RsaSha384IssuerLinkErrorKind,
}

impl X509RsaSha384IssuerLinkError {
    const fn new(kind: X509RsaSha384IssuerLinkErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> X509RsaSha384IssuerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384IssuerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 issuer-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384IssuerLinkError {}

/// Stable policy-link failures that retain no certificate or key material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384IssuerPolicyLinkErrorKind {
    EvidenceMismatch,
    IssuerName(X509CertificateIssuerLinkErrorKind),
    ChildSignature(X509RsaSha384SignatureErrorKind),
}

/// A redacted failure while composing prevalidated issuer policy evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384IssuerPolicyLinkError {
    kind: X509RsaSha384IssuerPolicyLinkErrorKind,
}

impl X509RsaSha384IssuerPolicyLinkError {
    const fn new(kind: X509RsaSha384IssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha384IssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384IssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384IssuerPolicyLinkError {}

/// Stable complete-intermediate-policy binding failures with no payload data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384IntermediateIssuerPolicyLinkErrorKind {
    /// The complete intermediate policy is not for the authenticated issuer.
    EvidenceMismatch,
}

/// A redacted complete-intermediate-policy binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384IntermediateIssuerPolicyLinkError {
    kind: X509RsaSha384IntermediateIssuerPolicyLinkErrorKind,
}

impl X509RsaSha384IntermediateIssuerPolicyLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha384IntermediateIssuerPolicyLinkErrorKind::EvidenceMismatch,
        }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha384IntermediateIssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384IntermediateIssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 intermediate issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384IntermediateIssuerPolicyLinkError {}

/// Non-copyable evidence for one exact child-to-candidate-issuer link.
///
/// This proves only the directed Name-policy result, issuing-CA role, RSA key
/// policy, and exact child signature checks performed by
/// [`verify_x509_rsa_sha384_issuer_link`]. It does not establish a complete or
/// trusted certification path.
pub struct VerifiedX509RsaSha384IssuerLink<
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
    VerifiedX509RsaSha384IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    #[must_use]
    pub const fn child(&self) -> &'child_certificate X509Certificate<'child_encoded> {
        self.child
    }

    #[must_use]
    pub const fn candidate_issuer(&self) -> &'issuer_certificate X509Certificate<'issuer_encoded> {
        self.candidate_issuer
    }

    #[must_use]
    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'issuer_encoded> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha384IssuerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha384IssuerLink")
    }
}

/// Non-copyable evidence for one exact policy-bound RSA SHA-384 issuer link.
///
/// This composes certificate-wide critical-extension and issuing-CA role
/// evidence with certificate-bound, data-driven RSA algorithm/key evidence,
/// caller-owned Name policy, and the child's exact SHA-384 signature. It does
/// not establish a complete path, trust, identity, revocation, TLS, or network
/// authority.
pub struct VerifiedX509RsaSha384IssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
    issuer_rsa_algorithm_policy:
        ValidatedX509RsaAlgorithmPolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the exact authenticated child certificate.
    #[must_use]
    pub const fn child(&self) -> &'child_certificate X509Certificate<'child_encoded> {
        self.child
    }

    /// Return the exact candidate issuer certificate.
    #[must_use]
    pub const fn candidate_issuer(&self) -> &'issuer_certificate X509Certificate<'issuer_encoded> {
        self.candidate_issuer
    }

    /// Return the consumed critical-extension and issuing-CA role evidence.
    #[must_use]
    pub const fn issuer_role_policy(
        &self,
    ) -> &ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.issuer_role_policy
    }

    /// Return the consumed certificate-bound RSA algorithm/key evidence.
    #[must_use]
    pub const fn issuer_rsa_algorithm_policy(
        &self,
    ) -> &ValidatedX509RsaAlgorithmPolicy<'issuer_certificate, 'issuer_encoded> {
        &self.issuer_rsa_algorithm_policy
    }
}

impl fmt::Debug for VerifiedX509RsaSha384IssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha384IssuerPolicyLink")
    }
}

/// Non-copyable evidence binding one authenticated SHA-384 issuer link to
/// complete intermediate policy for that exact decoded issuer object.
///
/// The retained intermediate policy covers certificate-wide critical
/// extensions and issuing-CA role, data-driven algorithm policy, self-issued
/// classification, and explicit-time validity. This wrapper does not advance
/// path length, evaluate NameConstraints, authenticate another link, establish
/// trust, check revocation, or grant TLS or network authority.
pub struct VerifiedX509RsaSha384IntermediateIssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    issuer_policy_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha384IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed authenticated policy-bound issuer link.
    #[must_use]
    pub const fn issuer_policy_link(
        &self,
    ) -> &VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.issuer_policy_link
    }

    /// Return complete policy for the exact authenticated intermediate.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.intermediate_policy
    }
}

impl fmt::Debug for VerifiedX509RsaSha384IntermediateIssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha384IntermediateIssuerPolicyLink")
    }
}

/// Verify one directed RSA PKCS #1 v1.5 SHA-384 issuer link.
///
/// `same_name` is called exactly once with the child issuer Name followed by
/// the candidate issuer subject Name. Every later check uses that same exact
/// candidate certificate before non-copyable evidence is returned.
pub fn verify_x509_rsa_sha384_issuer_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'child_encoded>, &X509Name<'issuer_encoded>) -> bool,
    policy: X509RsaSha384IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha384IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha384IssuerLinkError,
> {
    validate_certificate_issuer_link(child, candidate_issuer, same_name).map_err(|error| {
        X509RsaSha384IssuerLinkError::new(X509RsaSha384IssuerLinkErrorKind::IssuerName(
            error.kind(),
        ))
    })?;

    validate_certificate_role_extensions(
        candidate_issuer,
        X509CertificateRole::IssuingCertificateAuthority,
        policy.issuer_role_limits,
    )
    .map_err(|error| {
        X509RsaSha384IssuerLinkError::new(X509RsaSha384IssuerLinkErrorKind::IssuerRole(
            error.kind(),
        ))
    })?;

    let issuer_public_key = validate_rsa_public_key(
        candidate_issuer.tbs_certificate().subject_public_key_info(),
        policy.issuer_key_policy,
        policy.issuer_key_limits,
    )
    .map_err(|error| {
        X509RsaSha384IssuerLinkError::new(X509RsaSha384IssuerLinkErrorKind::IssuerPublicKey(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_pkcs1_v1_5_sha384_signature(
        child,
        issuer_public_key,
        policy.public_operation_limits,
    )
    .map_err(|error| {
        X509RsaSha384IssuerLinkError::new(X509RsaSha384IssuerLinkErrorKind::ChildSignature(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509RsaSha384IssuerLink {
        child,
        candidate_issuer,
        issuer_public_key,
    })
}

/// Verify one policy-bound RSA PKCS #1 v1.5 SHA-384 issuer link.
///
/// Both policy values must retain the exact `candidate_issuer` object. The
/// role policy must also retain the issuing-CA role. Those binding checks run
/// before `same_name` or any RSA work. The fixed SHA-384 verifier is selected
/// by this API; algorithm-policy evidence never routes a verifier.
pub fn verify_x509_rsa_sha384_issuer_link_with_policies<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'child_encoded>, &X509Name<'issuer_encoded>) -> bool,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer_certificate, 'issuer_encoded>,
    issuer_rsa_algorithm_policy: ValidatedX509RsaAlgorithmPolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
    public_operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha384IssuerPolicyLinkError,
> {
    if !core::ptr::eq(candidate_issuer, issuer_role_policy.certificate())
        || issuer_role_policy.role_extensions().role()
            != X509CertificateRole::IssuingCertificateAuthority
        || !core::ptr::eq(candidate_issuer, issuer_rsa_algorithm_policy.certificate())
    {
        return Err(X509RsaSha384IssuerPolicyLinkError::new(
            X509RsaSha384IssuerPolicyLinkErrorKind::EvidenceMismatch,
        ));
    }

    validate_certificate_issuer_link(child, candidate_issuer, same_name).map_err(|error| {
        X509RsaSha384IssuerPolicyLinkError::new(X509RsaSha384IssuerPolicyLinkErrorKind::IssuerName(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_pkcs1_v1_5_sha384_signature(
        child,
        issuer_rsa_algorithm_policy.rsa_public_key(),
        public_operation_limits,
    )
    .map_err(|error| {
        X509RsaSha384IssuerPolicyLinkError::new(
            X509RsaSha384IssuerPolicyLinkErrorKind::ChildSignature(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaSha384IssuerPolicyLink {
        child,
        candidate_issuer,
        issuer_role_policy,
        issuer_rsa_algorithm_policy,
    })
}

/// Bind one authenticated SHA-384 issuer link to complete policy for its exact
/// intermediate certificate object.
///
/// Pointer identity is required before either proof is retained. Encoded-byte
/// equality cannot attach policy from an independently decoded certificate to
/// the issuer that authenticated the child. Success grants no additional path,
/// trust, revocation, transport, or network authority.
pub fn bind_x509_rsa_sha384_intermediate_issuer_policy_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    issuer_policy_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    VerifiedX509RsaSha384IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha384IntermediateIssuerPolicyLinkError,
> {
    if !std::ptr::eq(
        issuer_policy_link.candidate_issuer(),
        intermediate_policy.certificate(),
    ) {
        return Err(X509RsaSha384IntermediateIssuerPolicyLinkError::evidence_mismatch());
    }

    Ok(VerifiedX509RsaSha384IntermediateIssuerPolicyLink {
        issuer_policy_link,
        intermediate_policy,
    })
}
