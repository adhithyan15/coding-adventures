//! # Bounded RSA SHA-512 X.509 issuer links
//!
//! This crate composes one narrow certificate-link boundary. It applies
//! caller-owned distinguished-name policy in the child-to-issuer direction,
//! requires the candidate issuer's algorithm-independent CA role, validates
//! that same certificate's RSA public key under caller-owned strength policy,
//! and verifies the child's exact RSA PKCS #1 v1.5 SHA-512 signature.
//!
//! Success authenticates only this exact pair. It does not route algorithms,
//! validate a complete path, establish reviewed trust, check revocation, or
//! provide TLS, HTTP, OAuth transport, or network authority.
//! A construction-bound entry point instead derives both endpoints from one
//! exact adjacent-link projection and retains that projection with the
//! validated issuer key. It authenticates no other construction depth and
//! grants no complete-path authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::X509Certificate;
use x509_certificate_path_construction::ConstructedX509CertificatePathAdjacentLink;
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthError, X509CertificatePathLengthState,
};
use x509_certificate_policy::{
    validate_certificate_issuer_link, validate_certificate_role_extensions,
    ValidatedCertificateRolePolicy, X509CertificateIssuerLinkErrorKind, X509CertificateRole,
    X509CertificateRoleErrorKind,
};
use x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy;
use x509_name::X509Name;
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_pkcs1_v1_5_sha512_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha512_signature, X509RsaSha512SignatureErrorKind,
};
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

/// Caller-owned limits and RSA strength policy for one issuer link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512IssuerLinkPolicy<'policy> {
    issuer_role_limits: Asn1Limits,
    issuer_key_policy: RsaPublicKeyPolicy<'policy>,
    issuer_key_limits: Asn1Limits,
    public_operation_limits: RsaPublicOperationLimits,
}

impl<'policy> X509RsaSha512IssuerLinkPolicy<'policy> {
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
pub enum X509RsaSha512IssuerLinkErrorKind {
    IssuerName(X509CertificateIssuerLinkErrorKind),
    IssuerRole(X509CertificateRoleErrorKind),
    IssuerPublicKey(RsaPublicKeyErrorKind),
    ChildSignature(X509RsaSha512SignatureErrorKind),
}

/// A redacted authenticated issuer-link failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512IssuerLinkError {
    kind: X509RsaSha512IssuerLinkErrorKind,
}

impl X509RsaSha512IssuerLinkError {
    const fn new(kind: X509RsaSha512IssuerLinkErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> X509RsaSha512IssuerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512IssuerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 issuer-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512IssuerLinkError {}

/// Stable constructed-path link failures that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512ConstructedPathAdjacentLinkErrorKind {
    /// The projection did not expose exactly one of its private child forms.
    InvalidAdjacentLink,
    /// The exact projected child and issuer failed existing link checks.
    IssuerLink(X509RsaSha512IssuerLinkErrorKind),
}

/// A redacted failure while authenticating one constructed-path adjacency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512ConstructedPathAdjacentLinkError {
    kind: X509RsaSha512ConstructedPathAdjacentLinkErrorKind,
}

impl X509RsaSha512ConstructedPathAdjacentLinkError {
    const fn new(kind: X509RsaSha512ConstructedPathAdjacentLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512ConstructedPathAdjacentLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512ConstructedPathAdjacentLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 constructed-path adjacent-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512ConstructedPathAdjacentLinkError {}

/// Stable construction-and-intermediate-policy failures with no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind {
    /// The projected issuer is the construction's terminal certificate.
    TerminalIssuer,
    /// The complete intermediate policy is not for the exact projected issuer.
    EvidenceMismatch,
    /// The exact construction-bound pair failed existing link authentication.
    AdjacentLink(X509RsaSha512ConstructedPathAdjacentLinkErrorKind),
}

/// A redacted failure while binding constructed adjacency to complete issuer policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError {
    kind: X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind,
}

impl X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError {
    const fn new(kind: X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 constructed-path intermediate issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError {}

/// Stable policy-link failures that retain no certificate or key material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512IssuerPolicyLinkErrorKind {
    EvidenceMismatch,
    IssuerName(X509CertificateIssuerLinkErrorKind),
    ChildSignature(X509RsaSha512SignatureErrorKind),
}

/// A redacted failure while composing prevalidated issuer policy evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512IssuerPolicyLinkError {
    kind: X509RsaSha512IssuerPolicyLinkErrorKind,
}

impl X509RsaSha512IssuerPolicyLinkError {
    const fn new(kind: X509RsaSha512IssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512IssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512IssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512IssuerPolicyLinkError {}

/// Stable complete-intermediate-policy binding failures with no payload data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512IntermediateIssuerPolicyLinkErrorKind {
    /// The complete intermediate policy is not for the authenticated issuer.
    EvidenceMismatch,
}

/// A redacted complete-intermediate-policy binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512IntermediateIssuerPolicyLinkError {
    kind: X509RsaSha512IntermediateIssuerPolicyLinkErrorKind,
}

impl X509RsaSha512IntermediateIssuerPolicyLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha512IntermediateIssuerPolicyLinkErrorKind::EvidenceMismatch,
        }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512IntermediateIssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512IntermediateIssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 intermediate issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512IntermediateIssuerPolicyLinkError {}

/// Non-copyable evidence for one exact child-to-candidate-issuer link.
///
/// This proves only the directed Name-policy result, issuing-CA role, RSA key
/// policy, and exact child signature checks performed by
/// [`verify_x509_rsa_sha512_issuer_link`]. It does not establish a complete or
/// trusted certification path.
pub struct VerifiedX509RsaSha512IssuerLink<
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
    VerifiedX509RsaSha512IssuerLink<
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

impl fmt::Debug for VerifiedX509RsaSha512IssuerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha512IssuerLink")
    }
}

/// Non-copyable authentication evidence for one exact constructed-path adjacency.
///
/// The retained projection supplies both endpoints and their order. The
/// validated key proves that the existing fixed RSA PKCS #1 v1.5 SHA-512
/// verifier authenticated that exact projected child with that exact selected
/// issuer. This evidence covers no other path link and grants no complete-path
/// or trust authority.
pub struct VerifiedX509RsaSha512ConstructedPathAdjacentLink<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    adjacent_link: ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    issuer_public_key: ValidatedRsaPublicKey<'candidate_encoded>,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    VerifiedX509RsaSha512ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the exact construction-bound pair that was authenticated.
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

    /// Return the validated public key of the projected issuer.
    #[must_use]
    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'candidate_encoded> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha512ConstructedPathAdjacentLink<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha512ConstructedPathAdjacentLink")
    }
}

/// Non-copyable evidence binding non-terminal constructed-path authentication to complete policy.
///
/// The retained complete intermediate policy applies caller-owned critical-
/// extension recognition, issuing-CA role, data-driven algorithm,
/// self-issued, and explicit-time validity policy to the exact projected
/// non-terminal issuer that authenticated the retained child. It neither
/// advances path-length state nor authenticates any other path adjacency.
pub struct VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    verified_link: VerifiedX509RsaSha512ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed construction-bound authenticated link.
    #[must_use]
    pub const fn verified_link(
        &self,
    ) -> &VerifiedX509RsaSha512ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.verified_link
    }

    /// Return complete policy for the exact projected non-terminal issuer.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }
}

impl fmt::Debug
    for VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<'_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink")
    }
}

/// Non-copyable evidence for one construction-bound intermediate path-length step.
///
/// The retained authenticated-intermediate proof binds complete policy to one
/// exact non-terminal adjacency and keeps its construction order available.
/// The retained state is the result of applying the existing path-length
/// transition from that policy. The caller supplies the starting state; this
/// result does not prove that it represents every earlier construction depth,
/// authenticate another adjacency, or establish complete validation or trust.
pub struct ValidatedX509RsaSha512ConstructedPathIntermediatePathLength<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_intermediate: VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    path_length_state: X509CertificatePathLengthState,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ValidatedX509RsaSha512ConstructedPathIntermediatePathLength<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed construction-bound authenticated intermediate.
    #[must_use]
    pub const fn authenticated_intermediate(
        &self,
    ) -> &VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.authenticated_intermediate
    }

    /// Return the advanced path-length state.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for ValidatedX509RsaSha512ConstructedPathIntermediatePathLength<'_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha512ConstructedPathIntermediatePathLength")
    }
}

/// Non-copyable evidence for one exact policy-bound RSA SHA-512 issuer link.
///
/// This composes certificate-wide critical-extension and issuing-CA role
/// evidence with certificate-bound, data-driven RSA algorithm/key evidence,
/// caller-owned Name policy, and the child's exact SHA-512 signature. It does
/// not establish a complete path, trust, identity, revocation, TLS, or network
/// authority.
pub struct VerifiedX509RsaSha512IssuerPolicyLink<
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
    VerifiedX509RsaSha512IssuerPolicyLink<
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

impl fmt::Debug for VerifiedX509RsaSha512IssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha512IssuerPolicyLink")
    }
}

/// Non-copyable evidence binding one authenticated SHA-512 issuer link to
/// complete intermediate policy for that exact decoded issuer object.
///
/// The retained intermediate policy covers certificate-wide critical
/// extensions and issuing-CA role, data-driven algorithm policy, self-issued
/// classification, and explicit-time validity. This wrapper does not advance
/// path length, evaluate NameConstraints, authenticate another link, establish
/// trust, check revocation, or grant TLS or network authority.
pub struct VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    issuer_policy_link: VerifiedX509RsaSha512IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
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
    ) -> &VerifiedX509RsaSha512IssuerPolicyLink<
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

impl fmt::Debug for VerifiedX509RsaSha512IntermediateIssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha512IntermediateIssuerPolicyLink")
    }
}

/// Non-copyable evidence for one authenticated intermediate path-length step.
///
/// The retained authenticated-intermediate proof binds one exact child link to
/// complete policy for the exact issuing intermediate. The retained state is
/// the result of applying the existing path-length transition from that same
/// intermediate policy. This proves neither the intermediate's parent link nor
/// path ordering, adjacency to another step, complete validation, or trust.
pub struct ValidatedX509RsaSha512AuthenticatedIntermediatePathLength<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    authenticated_intermediate: VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    path_length_state: X509CertificatePathLengthState,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    ValidatedX509RsaSha512AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed authenticated intermediate proof.
    #[must_use]
    pub const fn authenticated_intermediate(
        &self,
    ) -> &VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.authenticated_intermediate
    }

    /// Return the advanced path-length state.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug for ValidatedX509RsaSha512AuthenticatedIntermediatePathLength<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha512AuthenticatedIntermediatePathLength")
    }
}

/// Verify one directed RSA PKCS #1 v1.5 SHA-512 issuer link.
///
/// `same_name` is called exactly once with the child issuer Name followed by
/// the candidate issuer subject Name. Every later check uses that same exact
/// candidate certificate before non-copyable evidence is returned.
pub fn verify_x509_rsa_sha512_issuer_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'child_encoded>, &X509Name<'issuer_encoded>) -> bool,
    policy: X509RsaSha512IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha512IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512IssuerLinkError,
> {
    validate_certificate_issuer_link(child, candidate_issuer, same_name).map_err(|error| {
        X509RsaSha512IssuerLinkError::new(X509RsaSha512IssuerLinkErrorKind::IssuerName(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_sha512_issuer_link_after_name(child, candidate_issuer, policy)
}

/// Authenticate one exact adjacent pair projected from a constructed path.
///
/// This consuming entry point does not rerun Name policy. It accepts neither
/// endpoint separately: the child and issuer are derived only from the
/// projection's private depth and retained construction. Depth zero verifies
/// the target child; later depths verify the preceding selected candidate.
/// Success retains the entire projection beside the validated issuer key so
/// downstream traversal cannot silently discard path order.
///
/// Only one fixed RSA PKCS #1 v1.5 SHA-512 link is authenticated. This does not
/// route algorithms, retain complete issuer policy, authenticate another link,
/// validate a path, recognize trust, check revocation, or provide TLS or HTTP.
pub fn verify_constructed_x509_rsa_sha512_issuer_link<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    adjacent_link: ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    policy: X509RsaSha512IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha512ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaSha512ConstructedPathAdjacentLinkError,
> {
    let issuer = adjacent_link.issuer_certificate();
    let issuer_public_key = match (
        adjacent_link.child_target_certificate(),
        adjacent_link.child_candidate_certificate(),
    ) {
        (Some(child), None) => verify_x509_rsa_sha512_issuer_link_after_name(child, issuer, policy)
            .map(|verified| verified.issuer_public_key()),
        (None, Some(child)) => verify_x509_rsa_sha512_issuer_link_after_name(child, issuer, policy)
            .map(|verified| verified.issuer_public_key()),
        _ => {
            return Err(X509RsaSha512ConstructedPathAdjacentLinkError::new(
                X509RsaSha512ConstructedPathAdjacentLinkErrorKind::InvalidAdjacentLink,
            ));
        }
    }
    .map_err(|error| {
        X509RsaSha512ConstructedPathAdjacentLinkError::new(
            X509RsaSha512ConstructedPathAdjacentLinkErrorKind::IssuerLink(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaSha512ConstructedPathAdjacentLink {
        adjacent_link,
        issuer_public_key,
    })
}

/// Authenticate one constructed non-terminal issuer with its complete policy.
///
/// Terminal rejection and exact decoded-object binding run before the existing
/// RSA key and signature work, so terminal policy or policy for a detached or
/// byte-equal re-decode cannot authorize cryptography for the retained path.
/// Success retains both complete proof objects without advancing path length,
/// authenticating another adjacency, establishing trust, or granting transport
/// authority.
pub fn verify_constructed_x509_rsa_sha512_intermediate_issuer_policy_link<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    adjacent_link: ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
    policy: X509RsaSha512IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError,
> {
    if adjacent_link.issuer_depth() + 1 == adjacent_link.constructed_path().issuer_count() {
        return Err(
            X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind::TerminalIssuer,
            ),
        );
    }

    if !std::ptr::eq(
        adjacent_link.issuer_certificate(),
        intermediate_policy.certificate(),
    ) {
        return Err(
            X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind::EvidenceMismatch,
            ),
        );
    }

    let verified_link = verify_constructed_x509_rsa_sha512_issuer_link(adjacent_link, policy)
        .map_err(|error| {
            X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha512ConstructedPathIntermediateIssuerPolicyLinkErrorKind::AdjacentLink(
                    error.kind(),
                ),
            )
        })?;

    Ok(
        VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink {
            verified_link,
            intermediate_policy,
        },
    )
}

/// Advance path-length state from one construction-bound intermediate proof.
///
/// The transition derives role, self-issued classification, and any decoded
/// `pathLenConstraint` only from the complete intermediate policy retained by
/// `authenticated_intermediate`. Success consumes and retains the ordered proof
/// with the advanced state, so this boundary cannot discard path order or apply
/// detached policy. The caller-supplied starting state is not itself bound to a
/// construction depth, and this function does not authenticate another link,
/// establish trust, or validate a complete path.
pub fn advance_constructed_x509_rsa_sha512_intermediate_path_length<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    path_length_state: X509CertificatePathLengthState,
    authenticated_intermediate: VerifiedX509RsaSha512ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    ValidatedX509RsaSha512ConstructedPathIntermediatePathLength<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509CertificatePathLengthError,
> {
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(authenticated_intermediate.intermediate_policy())?;

    Ok(
        ValidatedX509RsaSha512ConstructedPathIntermediatePathLength {
            authenticated_intermediate,
            path_length_state,
        },
    )
}

fn verify_x509_rsa_sha512_issuer_link_after_name<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    candidate_issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    policy: X509RsaSha512IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha512IssuerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512IssuerLinkError,
> {
    validate_certificate_role_extensions(
        candidate_issuer,
        X509CertificateRole::IssuingCertificateAuthority,
        policy.issuer_role_limits,
    )
    .map_err(|error| {
        X509RsaSha512IssuerLinkError::new(X509RsaSha512IssuerLinkErrorKind::IssuerRole(
            error.kind(),
        ))
    })?;

    let issuer_public_key = validate_rsa_public_key(
        candidate_issuer.tbs_certificate().subject_public_key_info(),
        policy.issuer_key_policy,
        policy.issuer_key_limits,
    )
    .map_err(|error| {
        X509RsaSha512IssuerLinkError::new(X509RsaSha512IssuerLinkErrorKind::IssuerPublicKey(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_pkcs1_v1_5_sha512_signature(
        child,
        issuer_public_key,
        policy.public_operation_limits,
    )
    .map_err(|error| {
        X509RsaSha512IssuerLinkError::new(X509RsaSha512IssuerLinkErrorKind::ChildSignature(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509RsaSha512IssuerLink {
        child,
        candidate_issuer,
        issuer_public_key,
    })
}

/// Verify one policy-bound RSA PKCS #1 v1.5 SHA-512 issuer link.
///
/// Both policy values must retain the exact `candidate_issuer` object. The
/// role policy must also retain the issuing-CA role. Those binding checks run
/// before `same_name` or any RSA work. The fixed SHA-512 verifier is selected
/// by this API; algorithm-policy evidence never routes a verifier.
pub fn verify_x509_rsa_sha512_issuer_link_with_policies<
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
    VerifiedX509RsaSha512IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512IssuerPolicyLinkError,
> {
    if !core::ptr::eq(candidate_issuer, issuer_role_policy.certificate())
        || issuer_role_policy.role_extensions().role()
            != X509CertificateRole::IssuingCertificateAuthority
        || !core::ptr::eq(candidate_issuer, issuer_rsa_algorithm_policy.certificate())
    {
        return Err(X509RsaSha512IssuerPolicyLinkError::new(
            X509RsaSha512IssuerPolicyLinkErrorKind::EvidenceMismatch,
        ));
    }

    validate_certificate_issuer_link(child, candidate_issuer, same_name).map_err(|error| {
        X509RsaSha512IssuerPolicyLinkError::new(X509RsaSha512IssuerPolicyLinkErrorKind::IssuerName(
            error.kind(),
        ))
    })?;

    verify_x509_rsa_pkcs1_v1_5_sha512_signature(
        child,
        issuer_rsa_algorithm_policy.rsa_public_key(),
        public_operation_limits,
    )
    .map_err(|error| {
        X509RsaSha512IssuerPolicyLinkError::new(
            X509RsaSha512IssuerPolicyLinkErrorKind::ChildSignature(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaSha512IssuerPolicyLink {
        child,
        candidate_issuer,
        issuer_role_policy,
        issuer_rsa_algorithm_policy,
    })
}

/// Bind one authenticated SHA-512 issuer link to complete policy for its exact
/// intermediate certificate object.
///
/// Pointer identity is required before either proof is retained. Encoded-byte
/// equality cannot attach policy from an independently decoded certificate to
/// the issuer that authenticated the child. Success grants no additional path,
/// trust, revocation, transport, or network authority.
pub fn bind_x509_rsa_sha512_intermediate_issuer_policy_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    issuer_policy_link: VerifiedX509RsaSha512IssuerPolicyLink<
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
    VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512IntermediateIssuerPolicyLinkError,
> {
    if !std::ptr::eq(
        issuer_policy_link.candidate_issuer(),
        intermediate_policy.certificate(),
    ) {
        return Err(X509RsaSha512IntermediateIssuerPolicyLinkError::evidence_mismatch());
    }

    Ok(VerifiedX509RsaSha512IntermediateIssuerPolicyLink {
        issuer_policy_link,
        intermediate_policy,
    })
}

/// Advance path-length state from one authenticated intermediate proof.
///
/// The transition derives role, self-issued classification, and any decoded
/// `pathLenConstraint` only from the complete intermediate policy retained by
/// `authenticated_intermediate`. Success consumes and retains that proof with
/// the advanced state so this composition boundary cannot silently discard the
/// authenticated adjacent link. It does not authenticate the intermediate's
/// own issuer, establish ordering, or validate a complete path.
pub fn advance_x509_rsa_sha512_authenticated_intermediate_path_length<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    path_length_state: X509CertificatePathLengthState,
    authenticated_intermediate: VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    ValidatedX509RsaSha512AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509CertificatePathLengthError,
> {
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(authenticated_intermediate.intermediate_policy())?;

    Ok(ValidatedX509RsaSha512AuthenticatedIntermediatePathLength {
        authenticated_intermediate,
        path_length_state,
    })
}
