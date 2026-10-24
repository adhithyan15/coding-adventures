//! # Bounded RSA SHA-256 X.509 issuer links
//!
//! This crate composes one deliberately narrow certificate-link boundary. It
//! applies caller-owned distinguished-name policy in the child-to-issuer
//! direction, requires the candidate issuer's algorithm-independent CA role,
//! validates that certificate's RSA public key under caller-owned strength
//! policy, and verifies the child's exact RSA PKCS #1 v1.5 SHA-256 signature.
//!
//! The narrow result binds the exact child and candidate issuer. Stronger
//! entry points additionally retain caller-owned critical-extension and
//! issuing-CA role-policy evidence, then certificate-bound data-driven
//! algorithm-profile evidence, for that exact selected issuer. A final binding
//! can retain complete intermediate policy, including validity at one explicit
//! observation, for the identical issuer object. A construction-bound entry
//! point can instead consume one exact adjacent-link projection and retain its
//! path order while authenticating only that projected pair. A stronger
//! construction-bound entry point also retains complete intermediate policy
//! for the identical projected non-terminal issuer. It rejects the terminal
//! adjacency before RSA work so terminal trust-anchor semantics cannot be
//! represented as intermediate-policy evidence. A bounded follow-up can apply
//! the existing path-length transition while retaining either the selection-
//! bound or construction-bound authenticated intermediate, but each result
//! still proves only one caller-supplied state transition. Recognition is not
//! semantic validation of other critical extensions, and an algorithm profile
//! neither chooses nor enables this crate's fixed verifier. No result
//! establishes a complete path-length traversal, name constraints, revocation,
//! trust anchors, complete path validation, TLS, HTTP, or network effects.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::X509Certificate;
use x509_certificate_issuer_candidate_selection::SelectedX509IssuerCandidate;
use x509_certificate_path_construction::ConstructedX509CertificatePathAdjacentLink;
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthError, X509CertificatePathLengthState,
};
use x509_certificate_policy::{
    validate_certificate_issuer_link, validate_certificate_role_extensions,
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateRolePolicy,
    X509CertificateIssuerLinkErrorKind, X509CertificateRole, X509CertificateRoleErrorKind,
};
use x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy;
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

/// Stable constructed-path link failures that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ConstructedPathAdjacentLinkErrorKind {
    /// The projection did not expose exactly one of its private child forms.
    InvalidAdjacentLink,
    /// The exact projected child and issuer failed existing link checks.
    IssuerLink(X509RsaSha256IssuerLinkErrorKind),
}

/// A redacted failure while authenticating one constructed-path adjacency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256ConstructedPathAdjacentLinkError {
    kind: X509RsaSha256ConstructedPathAdjacentLinkErrorKind,
}

impl X509RsaSha256ConstructedPathAdjacentLinkError {
    const fn new(kind: X509RsaSha256ConstructedPathAdjacentLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256ConstructedPathAdjacentLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256ConstructedPathAdjacentLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 constructed-path adjacent-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256ConstructedPathAdjacentLinkError {}

/// Stable construction-and-intermediate-policy failures with no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind {
    /// The projected issuer is the construction's terminal certificate.
    TerminalIssuer,
    /// The complete intermediate policy is not for the exact projected issuer.
    EvidenceMismatch,
    /// The exact construction-bound pair failed existing link authentication.
    AdjacentLink(X509RsaSha256ConstructedPathAdjacentLinkErrorKind),
}

/// A redacted failure while binding constructed adjacency to complete issuer policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError {
    kind: X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind,
}

impl X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError {
    const fn new(kind: X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 constructed-path intermediate issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError {}

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

/// Stable full-policy composition failures that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256IssuerPolicyLinkErrorKind {
    /// Role or algorithm evidence is not for the exact selected issuing CA.
    EvidenceMismatch,
    /// The role-bound composition failed its existing issuer-link checks.
    IssuerRolePolicyLink(X509RsaSha256IssuerRolePolicyLinkErrorKind),
}

/// A redacted failure while binding selection, role, algorithm, and link evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256IssuerPolicyLinkError {
    kind: X509RsaSha256IssuerPolicyLinkErrorKind,
}

impl X509RsaSha256IssuerPolicyLinkError {
    const fn new(kind: X509RsaSha256IssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256IssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256IssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 issuer policy link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256IssuerPolicyLinkError {}

/// Stable authenticated-intermediate binding failures with no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256IntermediateIssuerPolicyLinkErrorKind {
    /// The complete intermediate policy is not for the authenticated issuer.
    EvidenceMismatch,
}

/// A redacted failure while binding an authenticated link to intermediate policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256IntermediateIssuerPolicyLinkError {
    kind: X509RsaSha256IntermediateIssuerPolicyLinkErrorKind,
}

impl X509RsaSha256IntermediateIssuerPolicyLinkError {
    const fn new(kind: X509RsaSha256IntermediateIssuerPolicyLinkErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RsaSha256IntermediateIssuerPolicyLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256IntermediateIssuerPolicyLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 intermediate issuer policy-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256IntermediateIssuerPolicyLinkError {}

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

/// Non-copyable authentication evidence for one exact constructed-path adjacency.
///
/// The retained projection supplies both endpoints and their order. The
/// validated key proves that the existing fixed RSA PKCS #1 v1.5 SHA-256
/// verifier authenticated that exact projected child with that exact selected
/// issuer. This evidence covers no other path link and grants no complete-path
/// or trust authority.
pub struct VerifiedX509RsaSha256ConstructedPathAdjacentLink<
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
    VerifiedX509RsaSha256ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the exact construction-bound pair that was authenticated.
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
    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'candidate_encoded> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha256ConstructedPathAdjacentLink<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256ConstructedPathAdjacentLink")
    }
}

/// Non-copyable evidence binding non-terminal constructed-path authentication to complete policy.
///
/// The retained complete intermediate policy applies caller-owned critical-
/// extension recognition, issuing-CA role, data-driven algorithm,
/// self-issued, and explicit-time validity policy to the exact projected
/// non-terminal issuer that authenticated the retained child. It neither
/// advances path-length state nor authenticates any other path adjacency.
pub struct VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    verified_link: VerifiedX509RsaSha256ConstructedPathAdjacentLink<
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
    VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed construction-bound authenticated link.
    pub const fn verified_link(
        &self,
    ) -> &VerifiedX509RsaSha256ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.verified_link
    }

    /// Return complete policy for the exact projected non-terminal issuer.
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }
}

impl fmt::Debug
    for VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<'_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink")
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
pub struct ValidatedX509RsaSha256ConstructedPathIntermediatePathLength<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_intermediate: VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    path_length_state: X509CertificatePathLengthState,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ValidatedX509RsaSha256ConstructedPathIntermediatePathLength<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed construction-bound authenticated intermediate.
    pub const fn authenticated_intermediate(
        &self,
    ) -> &VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.authenticated_intermediate
    }

    /// Return the advanced path-length state.
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for ValidatedX509RsaSha256ConstructedPathIntermediatePathLength<'_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha256ConstructedPathIntermediatePathLength")
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

/// Non-copyable evidence retaining authenticated link, role, and algorithm policy.
///
/// The algorithm evidence is certificate-bound caller policy. It does not
/// route to or enable the fixed RSA SHA-256 verifier, and its encoded key bound
/// is not a replacement for the issuer-link's RSA modulus-strength policy.
pub struct VerifiedX509RsaSha256IssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    role_policy_link: VerifiedX509RsaSha256IssuerRolePolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    issuer_algorithm_policy:
        ValidatedCertificateAlgorithmPolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    pub const fn role_policy_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerRolePolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.role_policy_link
    }

    pub const fn issuer_algorithm_policy(
        &self,
    ) -> &ValidatedCertificateAlgorithmPolicy<'issuer_certificate, 'issuer_encoded> {
        &self.issuer_algorithm_policy
    }
}

impl fmt::Debug for VerifiedX509RsaSha256IssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256IssuerPolicyLink")
    }
}

/// Non-copyable evidence binding one authenticated link to complete issuer policy.
///
/// The retained intermediate policy applies critical-extension recognition,
/// issuing-CA role, data-driven algorithm, self-issued classification, and
/// explicit-time validity policy to the exact candidate issuer that verified
/// the retained child's signature. This does not authenticate the
/// intermediate certificate's own issuer, advance path-length state, or
/// establish a complete or trusted certification path.
pub struct VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    issuer_policy_link: VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    pub const fn issuer_policy_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.issuer_policy_link
    }

    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.intermediate_policy
    }
}

impl fmt::Debug for VerifiedX509RsaSha256IntermediateIssuerPolicyLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256IntermediateIssuerPolicyLink")
    }
}

/// Non-copyable evidence for one authenticated intermediate path-length step.
///
/// The retained authenticated-intermediate proof binds one exact child link to
/// complete policy for the exact issuing intermediate. The retained state is
/// the result of applying the existing path-length transition from that same
/// intermediate policy. This proves neither the intermediate's parent link nor
/// path ordering, adjacency to another step, complete validation, or trust.
pub struct ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    authenticated_intermediate: VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    path_length_state: X509CertificatePathLengthState,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed authenticated intermediate proof.
    pub const fn authenticated_intermediate(
        &self,
    ) -> &VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.authenticated_intermediate
    }

    /// Return the advanced path-length state.
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug for ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha256AuthenticatedIntermediatePathLength")
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

/// Authenticate one exact adjacent pair projected from a constructed path.
///
/// This consuming entry point does not rerun Name policy. It accepts neither
/// endpoint separately: the child and issuer are derived only from the
/// projection's private depth and retained construction. Depth zero verifies
/// the target child; later depths verify the preceding selected candidate.
/// Success retains the entire projection beside the validated issuer key so
/// downstream traversal cannot silently discard path order.
///
/// Only one fixed RSA PKCS #1 v1.5 SHA-256 link is authenticated. This does not
/// route algorithms, retain complete issuer policy, authenticate another link,
/// validate a path, recognize trust, check revocation, or provide TLS or HTTP.
pub fn verify_constructed_x509_rsa_sha256_issuer_link<
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
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256ConstructedPathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaSha256ConstructedPathAdjacentLinkError,
> {
    let issuer = adjacent_link.issuer_certificate();
    let issuer_public_key = match (
        adjacent_link.child_target_certificate(),
        adjacent_link.child_candidate_certificate(),
    ) {
        (Some(child), None) => verify_x509_rsa_sha256_issuer_link_after_name(child, issuer, policy)
            .map(|verified| verified.issuer_public_key()),
        (None, Some(child)) => verify_x509_rsa_sha256_issuer_link_after_name(child, issuer, policy)
            .map(|verified| verified.issuer_public_key()),
        _ => {
            return Err(X509RsaSha256ConstructedPathAdjacentLinkError::new(
                X509RsaSha256ConstructedPathAdjacentLinkErrorKind::InvalidAdjacentLink,
            ));
        }
    }
    .map_err(|error| {
        X509RsaSha256ConstructedPathAdjacentLinkError::new(
            X509RsaSha256ConstructedPathAdjacentLinkErrorKind::IssuerLink(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaSha256ConstructedPathAdjacentLink {
        adjacent_link,
        issuer_public_key,
    })
}

/// Authenticate one non-terminal constructed adjacency with complete policy for its issuer.
///
/// The projected issuer must not be the construction's terminal certificate,
/// and the intermediate-policy certificate must be the identical decoded
/// object projected as issuer by `adjacent_link`. Both checks happen before the
/// existing RSA key and signature work, so terminal policy or policy for a
/// detached or byte-equal re-decode cannot authorize cryptography for the
/// retained path. Success retains both complete proof objects without advancing
/// path length, authenticating another adjacency, establishing trust, or
/// granting transport authority.
pub fn verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link<
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
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError,
> {
    if adjacent_link.issuer_depth() + 1 == adjacent_link.constructed_path().issuer_count() {
        return Err(
            X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind::TerminalIssuer,
            ),
        );
    }

    if !std::ptr::eq(
        adjacent_link.issuer_certificate(),
        intermediate_policy.certificate(),
    ) {
        return Err(
            X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind::EvidenceMismatch,
            ),
        );
    }

    let verified_link = verify_constructed_x509_rsa_sha256_issuer_link(adjacent_link, policy)
        .map_err(|error| {
            X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkError::new(
                X509RsaSha256ConstructedPathIntermediateIssuerPolicyLinkErrorKind::AdjacentLink(
                    error.kind(),
                ),
            )
        })?;

    Ok(
        VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink {
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
pub fn advance_constructed_x509_rsa_sha256_intermediate_path_length<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    path_length_state: X509CertificatePathLengthState,
    authenticated_intermediate: VerifiedX509RsaSha256ConstructedPathIntermediateIssuerPolicyLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    ValidatedX509RsaSha256ConstructedPathIntermediatePathLength<
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
        ValidatedX509RsaSha256ConstructedPathIntermediatePathLength {
            authenticated_intermediate,
            path_length_state,
        },
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

/// Authenticate one selected issuer while retaining role and algorithm policy.
///
/// All three evidence inputs must retain the same exact selected candidate,
/// and the role evidence must classify it as an issuing CA. Those bindings are
/// checked before the existing role, RSA key, or signature work begins. The
/// algorithm profile remains caller policy data and does not select this
/// function's fixed RSA PKCS #1 v1.5 SHA-256 implementation.
pub fn verify_selected_x509_rsa_sha256_issuer_link_with_policies<
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
    issuer_algorithm_policy: ValidatedCertificateAlgorithmPolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
    policy: X509RsaSha256IssuerLinkPolicy<'_>,
) -> Result<
    VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IssuerPolicyLinkError,
> {
    let selected_issuer = selected_candidate.candidate_certificate();
    if !std::ptr::eq(selected_issuer, issuer_role_policy.certificate())
        || issuer_role_policy.role_extensions().role()
            != X509CertificateRole::IssuingCertificateAuthority
        || !std::ptr::eq(selected_issuer, issuer_algorithm_policy.certificate())
    {
        return Err(X509RsaSha256IssuerPolicyLinkError::new(
            X509RsaSha256IssuerPolicyLinkErrorKind::EvidenceMismatch,
        ));
    }

    let role_policy_link = verify_selected_x509_rsa_sha256_issuer_link_with_role_policy(
        selected_candidate,
        issuer_role_policy,
        policy,
    )
    .map_err(|error| {
        X509RsaSha256IssuerPolicyLinkError::new(
            X509RsaSha256IssuerPolicyLinkErrorKind::IssuerRolePolicyLink(error.kind()),
        )
    })?;

    Ok(VerifiedX509RsaSha256IssuerPolicyLink {
        role_policy_link,
        issuer_algorithm_policy,
    })
}

/// Bind one authenticated selected issuer to its complete intermediate policy.
///
/// Object identity is checked before either proof is retained. Byte-equal
/// certificates decoded into separate objects are intentionally distinct: a
/// path traversal must not apply self-issued, validity, algorithm, or
/// critical-extension policy from one object to a link authenticated with
/// another. Success retains both inputs but grants no additional path, trust,
/// revocation, transport, or network authority.
pub fn bind_x509_rsa_sha256_intermediate_issuer_policy_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    issuer_policy_link: VerifiedX509RsaSha256IssuerPolicyLink<
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
    VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256IntermediateIssuerPolicyLinkError,
> {
    let candidate_issuer = issuer_policy_link
        .role_policy_link()
        .verified_link()
        .candidate_issuer();
    if !std::ptr::eq(candidate_issuer, intermediate_policy.certificate()) {
        return Err(X509RsaSha256IntermediateIssuerPolicyLinkError::new(
            X509RsaSha256IntermediateIssuerPolicyLinkErrorKind::EvidenceMismatch,
        ));
    }

    Ok(VerifiedX509RsaSha256IntermediateIssuerPolicyLink {
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
pub fn advance_x509_rsa_sha256_authenticated_intermediate_path_length<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    path_length_state: X509CertificatePathLengthState,
    authenticated_intermediate: VerifiedX509RsaSha256IntermediateIssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509CertificatePathLengthError,
> {
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(authenticated_intermediate.intermediate_policy())?;

    Ok(ValidatedX509RsaSha256AuthenticatedIntermediatePathLength {
        authenticated_intermediate,
        path_length_state,
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
