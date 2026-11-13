//! # Constructed-path RSA signature routing
//!
//! This crate binds one exact adjacency projected from a constructed X.509
//! path to certificate-bound child algorithm policy, certificate-wide issuer
//! role policy, and certificate-bound issuer RSA algorithm/key policy. Only
//! after all exact-object and issuing-CA checks pass does caller-owned routing
//! invoke one repository-owned fixed RSA certificate-signature verifier.
//!
//! It also composes reviewed construction, ordered intermediate path-length
//! evidence, the exact depth-zero target signature with complete server policy,
//! the reviewed snapshot's supported anchor DNS constraints against the exact
//! target and retained non-self-issued intermediate SANs, and every retained
//! intermediate's supported DNS/IP constraints against the exact target and
//! target-nearer non-self-issued intermediate SANs. This does not constrain
//! subordinate distinguished names, validate a complete path, establish trust,
//! check revocation, perform TLS or HTTP, or grant OAuth or network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate_path_construction::{
    ConstructedX509CertificatePath, ConstructedX509CertificatePathAdjacentLink,
};
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthError, X509CertificatePathLengthErrorKind,
    X509CertificatePathLengthState,
};
use x509_certificate_path_reviewed_anchor_binding::{
    InitializedNamePolicyReviewedConstructedX509CertificatePathLength,
    InitializedReviewedConstructedX509CertificatePathLength,
};
use x509_certificate_policy::{
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateRolePolicy, X509CertificateRole,
};
use x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy;
use x509_name_constraints::{
    decode_name_constraints, decode_name_constraints_contents, evaluate_dns_and_ip_names,
    evaluate_dns_name_subtrees, evaluate_dns_names, ConstrainedDnsAndIpNames, ConstrainedDnsNames,
    DnsConstraintErrorKind, IpConstraintErrorKind, NameConstraintEvaluationErrorKind,
    NameConstraintsErrorKind, NAME_CONSTRAINTS_OID,
};
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureRoutingErrorKind,
};
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;
use x509_subject_alt_name::{
    decode_subject_alt_name, SubjectAltNameErrorKind, SUBJECT_ALT_NAME_OID,
};
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

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

/// Stable, payload-free routed intermediate-policy binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathIntermediatePolicyErrorKind {
    /// The authenticated adjacency terminates at the construction's root.
    TerminalIssuer,
    /// The routed issuer and intermediate policy retain different objects.
    EvidenceMismatch,
}

/// A redacted routed intermediate-policy binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathIntermediatePolicyError {
    kind: X509RsaConstructedPathIntermediatePolicyErrorKind,
}

impl X509RsaConstructedPathIntermediatePolicyError {
    const fn new(kind: X509RsaConstructedPathIntermediatePolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathIntermediatePolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathIntermediatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path intermediate policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathIntermediatePolicyError {}

/// Stable, payload-free reviewed-terminal binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathReviewedAnchorErrorKind {
    /// The authenticated adjacency does not terminate at the construction's root.
    NonTerminalIssuer,
    /// The routed terminal issuer and reviewed-root evidence retain different objects.
    EvidenceMismatch,
}

/// A redacted routed reviewed-terminal binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathReviewedAnchorError {
    kind: X509RsaConstructedPathReviewedAnchorErrorKind,
}

impl X509RsaConstructedPathReviewedAnchorError {
    const fn new(kind: X509RsaConstructedPathReviewedAnchorErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathReviewedAnchorErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathReviewedAnchorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path reviewed-anchor error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathReviewedAnchorError {}

/// Stable, payload-free construction-initialized path-length failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind {
    /// The initializer and terminal proof retain different path objects.
    EvidenceMismatch,
    /// The retained reviewed construction contains no intermediate certificate.
    InvalidPathShape,
    /// Complete intermediate policy is not for the terminal proof's exact child.
    IntermediatePolicyMismatch,
    /// The authenticated intermediate exhausted or violated path-length policy.
    PathLength(X509CertificatePathLengthErrorKind),
}

/// A redacted construction-initialized path-length failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathInitializedIntermediatePathLengthError {
    kind: X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind,
}

impl X509RsaConstructedPathInitializedIntermediatePathLengthError {
    const fn new(kind: X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathInitializedIntermediatePathLengthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path initialized intermediate path-length error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathInitializedIntermediatePathLengthError {}

/// Stable, payload-free next ordered intermediate path-length failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathNextIntermediatePathLengthErrorKind {
    /// The retained steps describe different path objects or disconnected issuers.
    EvidenceMismatch,
    /// The routed proof is not the immediately target-nearer intermediate link.
    InvalidPathShape,
    /// Complete intermediate policy is not for the routed proof's exact child.
    IntermediatePolicyMismatch,
    /// The authenticated intermediate exhausted or violated path-length policy.
    PathLength(X509CertificatePathLengthErrorKind),
}

/// A redacted next ordered intermediate path-length failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathNextIntermediatePathLengthError {
    kind: X509RsaConstructedPathNextIntermediatePathLengthErrorKind,
}

impl X509RsaConstructedPathNextIntermediatePathLengthError {
    const fn new(kind: X509RsaConstructedPathNextIntermediatePathLengthErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathNextIntermediatePathLengthErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathNextIntermediatePathLengthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path next intermediate path-length error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathNextIntermediatePathLengthError {}

/// Stable, payload-free ordered intermediate accumulation failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind {
    /// The caller supplied a zero intermediate work ceiling.
    InvalidIntermediateLimit,
    /// The constructed path contains more intermediates than caller policy permits.
    IntermediateLimitExceeded,
    /// The evidence slices do not contain exactly one entry for every required step.
    EvidenceCountMismatch,
    /// Retained evidence describes another construction or a disconnected issuer.
    EvidenceMismatch,
    /// The construction has no intermediate or a proof is not the next required depth.
    InvalidPathShape,
    /// Complete intermediate policy is not for the authenticated child at that step.
    IntermediatePolicyMismatch,
    /// An authenticated intermediate exhausted or violated path-length policy.
    PathLength(X509CertificatePathLengthErrorKind),
}

/// A redacted ordered intermediate accumulation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathIntermediatePathLengthAccumulationError {
    kind: X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind,
}

impl X509RsaConstructedPathIntermediatePathLengthAccumulationError {
    const fn new(kind: X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathIntermediatePathLengthAccumulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path intermediate path-length accumulation error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathIntermediatePathLengthAccumulationError {}

/// Stable, payload-free accumulated target-signature closure failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathTargetSignatureClosureErrorKind {
    /// Retained evidence describes another construction or disconnected issuer.
    EvidenceMismatch,
    /// The routed proof is not the construction's exact depth-zero target link.
    InvalidPathShape,
    /// Complete server policy is not for the construction's exact target object.
    TargetPolicyMismatch,
}

/// A redacted accumulated target-signature closure failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathTargetSignatureClosureError {
    kind: X509RsaConstructedPathTargetSignatureClosureErrorKind,
}

impl X509RsaConstructedPathTargetSignatureClosureError {
    const fn new(kind: X509RsaConstructedPathTargetSignatureClosureErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathTargetSignatureClosureErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathTargetSignatureClosureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path target signature closure error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathTargetSignatureClosureError {}

/// Stable, payload-free reviewed two-link path binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathReviewedIntermediatePathErrorKind {
    /// The two proofs retain different constructed-path objects or certificates.
    EvidenceMismatch,
    /// The retained construction is not exactly target -> intermediate -> root.
    InvalidPathShape,
    /// The construction-derived path-length transition failed.
    PathLength(X509CertificatePathLengthErrorKind),
}

/// A redacted reviewed two-link path binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathReviewedIntermediatePathError {
    kind: X509RsaConstructedPathReviewedIntermediatePathErrorKind,
}

impl X509RsaConstructedPathReviewedIntermediatePathError {
    const fn new(kind: X509RsaConstructedPathReviewedIntermediatePathErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathReviewedIntermediatePathErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathReviewedIntermediatePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path reviewed intermediate path error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathReviewedIntermediatePathError {}

/// Stable, payload-free reviewed server-path binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaConstructedPathReviewedServerPolicyErrorKind {
    /// The server policy and constructed path retain different target objects.
    EvidenceMismatch,
    /// The selected reviewed anchor's constraint bytes were malformed or unsupported.
    InvalidAnchorNameConstraints(NameConstraintsErrorKind),
    /// The constrained target did not contain Subject Alternative Name.
    MissingTargetSubjectAlternativeName,
    /// The target's Subject Alternative Name extension was malformed.
    InvalidTargetSubjectAlternativeName(SubjectAltNameErrorKind),
    /// At least one target dNSName was invalid or outside the permitted subtrees.
    TargetDnsNameConstraints(DnsConstraintErrorKind),
    /// The exact intermediate's NameConstraints extension was not critical.
    NonCriticalIntermediateNameConstraints,
    /// The exact intermediate's NameConstraints value was malformed or unsupported.
    InvalidIntermediateNameConstraints(NameConstraintsErrorKind),
    /// A target dNSName violated the exact intermediate's DNS constraints.
    TargetIntermediateDnsNameConstraints(DnsConstraintErrorKind),
    /// A target iPAddress violated the exact intermediate's IP constraints.
    TargetIntermediateIpNameConstraints(IpConstraintErrorKind),
    /// A subordinate intermediate's present Subject Alternative Name was malformed.
    InvalidSubordinateIntermediateSubjectAlternativeName(SubjectAltNameErrorKind),
    /// A subordinate intermediate dNSName violated an ancestor intermediate's constraints.
    SubordinateIntermediateDnsNameConstraints(DnsConstraintErrorKind),
    /// A subordinate intermediate iPAddress violated an ancestor intermediate's constraints.
    SubordinateIntermediateIpNameConstraints(IpConstraintErrorKind),
    /// Bounded repeated-evaluation counts could not be represented by `usize`.
    SubordinateNameEvaluationCountOverflow,
    /// A subordinate intermediate dNSName violated the reviewed anchor's constraints.
    SubordinateIntermediateAnchorDnsNameConstraints(DnsConstraintErrorKind),
    /// Bounded reviewed-anchor subordinate-name counts could not be represented by `usize`.
    SubordinateAnchorNameEvaluationCountOverflow,
}

/// A redacted reviewed server-path binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaConstructedPathReviewedServerPolicyError {
    kind: X509RsaConstructedPathReviewedServerPolicyErrorKind,
}

impl X509RsaConstructedPathReviewedServerPolicyError {
    const fn new(kind: X509RsaConstructedPathReviewedServerPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaConstructedPathReviewedServerPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaConstructedPathReviewedServerPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA constructed-path reviewed server policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaConstructedPathReviewedServerPolicyError {}

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

/// Non-copyable evidence for one complete routed two-link reviewed path.
///
/// The intermediate input authenticates the target-to-intermediate adjacency
/// and retains complete intermediate policy. The fixed-shape binder derives
/// the RFC 5280 initial path-length capacity from the exact two-issuer
/// construction, advances it across that intermediate, and retains the result.
/// The terminal input authenticates the immediately following intermediate-to-
/// root adjacency and binds that exact root to reviewed Mozilla snapshot
/// membership. Both must retain the same exact construction. This is not
/// complete Web PKI path validation and does not apply name constraints or
/// target identity.
pub struct BoundX509RsaConstructedReviewedIntermediatePath<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_intermediate: ValidatedX509RsaConstructedPathIntermediatePathLength<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    BoundX509RsaConstructedReviewedIntermediatePath<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the depth-zero authenticated intermediate path-length transition.
    #[must_use]
    pub const fn authenticated_intermediate(
        &self,
    ) -> &ValidatedX509RsaConstructedPathIntermediatePathLength<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.authenticated_intermediate
    }

    /// Return the depth-one terminal authentication and reviewed-root handoff.
    #[must_use]
    pub const fn reviewed_terminal(
        &self,
    ) -> &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_terminal
    }
}

impl fmt::Debug for BoundX509RsaConstructedReviewedIntermediatePath<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaConstructedReviewedIntermediatePath")
    }
}

/// Non-copyable server policy bound to one routed reviewed two-link path.
///
/// The reviewed path retains both authenticated adjacencies, complete
/// intermediate policy, advanced path-length state, and reviewed-root
/// selection. The server policy retains complete target critical-extension,
/// algorithm, end-entity role, SAN-only identity, explicit-time validity, and
/// purpose policy for the construction's exact decoded target. This does not
/// apply anchor or intermediate constraints, check revocation, or authorize
/// TLS, HTTP, OAuth, or network use.
pub struct BoundX509RsaConstructedReviewedIntermediateServerPath<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    server_policy: ValidatedX509ServerCertificatePolicy<'server_certificate, 'target_encoded>,
    reviewed_path: BoundX509RsaConstructedReviewedIntermediatePath<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
}

impl<'path, 'server_certificate, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    BoundX509RsaConstructedReviewedIntermediateServerPath<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return complete server policy for the construction's exact target.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'server_certificate, 'target_encoded> {
        &self.server_policy
    }

    /// Return the authenticated reviewed two-link construction.
    #[must_use]
    pub const fn reviewed_path(
        &self,
    ) -> &BoundX509RsaConstructedReviewedIntermediatePath<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_path
    }
}

impl fmt::Debug for BoundX509RsaConstructedReviewedIntermediateServerPath<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaConstructedReviewedIntermediateServerPath")
    }
}

/// Non-copyable routed server path after reviewed-anchor DNS policy.
///
/// The exact target is derived from complete server policy retained by the
/// routed proof. Constraint bytes are derived from its exact selected reviewed
/// terminal. Success proves either that the snapshot entry has no constraints
/// or that every target dNSName satisfies its supported permitted DNS
/// subtrees. This does not apply intermediate constraints, excluded or non-DNS
/// anchor constraints, revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    reviewed_server_path: BoundX509RsaConstructedReviewedIntermediateServerPath<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<'path, 'server_certificate, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed complete routed server-path evidence.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &BoundX509RsaConstructedReviewedIntermediateServerPath<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_server_path
    }

    /// Return DNS evidence when the reviewed snapshot entry carried constraints.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl fmt::Debug
    for ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<'_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy")
    }
}

/// Non-copyable routed server path after exact intermediate name constraints.
///
/// Success retains the complete construction-bound anchor-policy proof and
/// proves either that its exact intermediate has no NameConstraints extension
/// or that its supported critical permitted-and-excluded DNS and IP subsets
/// were applied together to every retained dNSName and iPAddress in the exact
/// target. It is not generalized path validation, hostname verification,
/// revocation, TLS, HTTP, OAuth transport, or network authority.
pub struct ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    anchor_policy: ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
    constrained_names: Option<ConstrainedDnsAndIpNames>,
}

impl<'path, 'server_certificate, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed complete construction-bound anchor-policy proof.
    #[must_use]
    pub const fn anchor_policy(
        &self,
    ) -> &ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.anchor_policy
    }

    /// Return DNS evidence when the exact intermediate carried supported
    /// critical NameConstraints.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }

    /// Return combined DNS/IP evidence when the exact intermediate carried
    /// supported critical NameConstraints.
    #[must_use]
    pub const fn constrained_names(&self) -> Option<ConstrainedDnsAndIpNames> {
        self.constrained_names
    }
}

impl fmt::Debug
    for ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy")
    }
}

/// Non-copyable routed terminal authentication bound to a reviewed root.
///
/// The routed proof authenticates the construction's final child-to-issuer
/// adjacency and retains certificate-wide issuing-CA policy plus data-driven
/// RSA algorithm/key policy for that issuer. Reviewed-root selection must
/// retain that same exact decoded terminal certificate. This is not complete
/// path validation and does not apply trust-anchor constraints.
pub struct BoundX509RsaConstructedPathTerminalToReviewedAnchor<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
    'anchor_certificate,
    'anchor_encoded,
> {
    verified_terminal_signature: VerifiedX509RsaConstructedPathPolicySignature<
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
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
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
        'anchor_certificate,
        'anchor_encoded,
    >
    BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >
{
    /// Return the consumed routed proof for the terminal adjacency.
    #[must_use]
    pub const fn verified_terminal_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
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
        &self.verified_terminal_signature
    }

    /// Return reviewed-root selection for the exact authenticated terminal.
    #[must_use]
    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug
    for BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaConstructedPathTerminalToReviewedAnchor")
    }
}

/// Non-copyable routed authentication bound to complete intermediate policy.
///
/// The authenticated adjacency supplies construction order and data-driven RSA
/// signature evidence. The complete policy supplies certificate-wide critical-
/// extension recognition, issuing-CA role, algorithm policy, self-issued
/// classification, and explicit-time validity for that exact non-terminal
/// issuer. This evidence does not advance path length or validate a path.
pub struct VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
    verified_signature: VerifiedX509RsaConstructedPathPolicySignature<
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
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded>,
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
    VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
    /// Return the consumed construction-bound routed signature proof.
    #[must_use]
    pub const fn verified_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
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
        &self.verified_signature
    }

    /// Return complete policy for the exact authenticated intermediate.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'issuer_certificate, 'issuer_encoded> {
        &self.intermediate_policy
    }
}

impl fmt::Debug
    for VerifiedX509RsaConstructedPathIntermediatePolicySignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaConstructedPathIntermediatePolicySignature")
    }
}

/// Non-copyable evidence for one routed constructed-path path-length step.
///
/// The retained authenticated intermediate preserves construction order,
/// routed signature evidence, and complete policy. The retained state is the
/// result of applying the repository-owned path-length transition to only that
/// policy. The caller supplies the starting state, so this does not prove prior
/// traversal or complete path validation.
pub struct ValidatedX509RsaConstructedPathIntermediatePathLength<
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
    authenticated_intermediate: VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
    path_length_state: X509CertificatePathLengthState,
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
    ValidatedX509RsaConstructedPathIntermediatePathLength<
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
    /// Return the consumed routed authenticated-intermediate proof.
    #[must_use]
    pub const fn authenticated_intermediate(
        &self,
    ) -> &VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
        &self.authenticated_intermediate
    }

    /// Return the advanced path-length state.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for ValidatedX509RsaConstructedPathIntermediatePathLength<'_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// First Name-policy-backed anchor-to-target path-length transition.
///
/// The terminal proof authenticates the anchor-nearest intermediate as the
/// exact child of the reviewed trust anchor. Complete intermediate policy for
/// that same child supplies role, self-issued, and path-length inputs. The
/// borrowed initializer retains the identical policy-backed construction, its
/// exact Name-value policy, reviewed anchor, issuing-CA role policy, and
/// derived initial capacity. This proves only the first ordered transition,
/// not subsequent traversal or complete path validity.
pub struct AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    initialized_path_length:
        &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>,
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the borrowed Name-policy-backed construction and capacity proof.
    #[must_use]
    pub const fn initialized_path_length(
        &self,
    ) -> &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        self.initialized_path_length
    }

    /// Return authentication of the anchor-nearest intermediate by the root.
    #[must_use]
    pub const fn reviewed_terminal(
        &self,
    ) -> &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_terminal
    }

    /// Return complete policy for the authenticated anchor-nearest child.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }

    /// Return path-length state after the first ordered transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength",
        )
    }
}

/// Second Name-policy-backed anchor-to-target path-length transition.
///
/// The previous stronger step proves the root-authenticated anchor-nearest
/// intermediate while retaining the exact Name policy. The next routed proof
/// must be the immediately target-nearer adjacency from that same construction,
/// and its issuer must be the already processed intermediate. Complete policy
/// for its exact candidate child supplies the second transition inputs. This
/// proves exactly two ordered intermediate steps, not arbitrary traversal or
/// complete path validity.
pub struct AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    previous_step:
        AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    next_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>,
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the retained policy-backed first ordered transition.
    #[must_use]
    pub const fn previous_step(
        &self,
    ) -> &AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.previous_step
    }

    /// Return authentication of the immediately target-nearer intermediate.
    #[must_use]
    pub const fn next_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.next_signature
    }

    /// Return complete policy for the authenticated second candidate child.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }

    /// Return path-length state after the second ordered transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// First anchor-to-target path-length transition from reviewed construction.
///
/// The terminal proof authenticates the anchor-nearest intermediate as the
/// exact child of the reviewed trust anchor. Complete intermediate policy for
/// that same child supplies role, self-issued, and path-length inputs. The
/// borrowed initializer retains the identical reviewed construction and its
/// derived initial capacity. This proves only the first ordered transition,
/// not subsequent traversal or complete path validity.
pub struct AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    initialized_path_length: &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>,
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the borrowed reviewed construction and initial capacity proof.
    #[must_use]
    pub const fn initialized_path_length(
        &self,
    ) -> &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        self.initialized_path_length
    }

    /// Return authentication of the anchor-nearest intermediate by the root.
    #[must_use]
    pub const fn reviewed_terminal(
        &self,
    ) -> &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_terminal
    }

    /// Return complete policy for the authenticated anchor-nearest child.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }

    /// Return path-length state after the first ordered transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// Second anchor-to-target path-length transition from reviewed construction.
///
/// The previous step proves the root-authenticated anchor-nearest intermediate.
/// The next routed proof must be the immediately target-nearer adjacency from
/// the same exact construction, and its issuer must be that already processed
/// intermediate. Complete policy for the routed proof's exact candidate child
/// supplies the next transition inputs. This proves exactly two ordered
/// intermediate steps, not arbitrary traversal or complete path validity.
pub struct AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    previous_step: AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    next_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy:
        ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>,
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the retained root-authenticated first ordered transition.
    #[must_use]
    pub const fn previous_step(
        &self,
    ) -> &AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.previous_step
    }

    /// Return authentication of the immediately target-nearer intermediate.
    #[must_use]
    pub const fn next_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.next_signature
    }

    /// Return complete policy for the authenticated second candidate child.
    #[must_use]
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded> {
        &self.intermediate_policy
    }

    /// Return path-length state after the second ordered transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// Borrowed evidence that every constructed intermediate advanced path length.
///
/// The reviewed terminal authenticates the anchor-nearest intermediate. The
/// follow-on signatures and complete policies are retained in anchor-to-target
/// order and cover exactly every remaining intermediate in the construction.
/// The caller-selected ceiling bounds both evidence slices before traversal.
/// This stops before the target signature and is not complete path validity.
pub struct AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    initialized_path_length: &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    ordered_signatures: &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >],
    intermediate_policies:
        &'evidence [ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>],
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the borrowed reviewed construction and initial capacity proof.
    #[must_use]
    pub const fn initialized_path_length(
        &self,
    ) -> &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        self.initialized_path_length
    }

    /// Return the root-authenticated terminal adjacency and reviewed anchor.
    #[must_use]
    pub const fn reviewed_terminal(
        &self,
    ) -> &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        self.reviewed_terminal
    }

    /// Return all routed follow-on proofs in anchor-to-target order.
    #[must_use]
    pub const fn ordered_signatures(
        &self,
    ) -> &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >] {
        self.ordered_signatures
    }

    /// Return complete policy for every intermediate in anchor-to-target order.
    #[must_use]
    pub const fn intermediate_policies(
        &self,
    ) -> &'evidence [ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>]
    {
        self.intermediate_policies
    }

    /// Return the exact number of processed intermediate certificates.
    #[must_use]
    pub const fn processed_intermediate_count(&self) -> usize {
        self.intermediate_policies.len()
    }

    /// Return path-length state after every intermediate transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// Name-policy-backed evidence for every intermediate path-length transition.
///
/// This is the stronger counterpart to the generic accumulator. It retains the
/// exact Name-value policy, reviewed anchor, and certificate-wide anchor-role
/// policy through the policy-backed initializer while borrowing the same
/// complete ordered evidence slices. This stops before target authentication
/// and is not complete path or Name validity.
pub struct AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    initialized_path_length:
        &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    reviewed_terminal: &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    ordered_signatures: &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >],
    intermediate_policies:
        &'evidence [ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>],
    path_length_state: X509CertificatePathLengthState,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the exact policy-backed construction and initial capacity proof.
    #[must_use]
    pub const fn initialized_path_length(
        &self,
    ) -> &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        self.initialized_path_length
    }

    /// Return the root-authenticated terminal adjacency and reviewed anchor.
    #[must_use]
    pub const fn reviewed_terminal(
        &self,
    ) -> &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    > {
        self.reviewed_terminal
    }

    /// Return all routed follow-on proofs in anchor-to-target order.
    #[must_use]
    pub const fn ordered_signatures(
        &self,
    ) -> &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >] {
        self.ordered_signatures
    }

    /// Return complete policy for every intermediate in anchor-to-target order.
    #[must_use]
    pub const fn intermediate_policies(
        &self,
    ) -> &'evidence [ValidatedX509IntermediateCertificatePolicy<'candidates, 'candidate_encoded>]
    {
        self.intermediate_policies
    }

    /// Return the exact number of processed intermediate certificates.
    #[must_use]
    pub const fn processed_intermediate_count(&self) -> usize {
        self.intermediate_policies.len()
    }

    /// Return path-length state after every intermediate transition.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength")
    }
}

/// Accumulated intermediate evidence closed by the exact target signature.
///
/// The retained routed target proof is the depth-zero adjacency from the same
/// construction as the accumulated anchor-to-target intermediate evidence.
/// Its issuer is the exact target-nearest intermediate already processed by
/// that accumulator, and complete server policy retains the construction's
/// exact target object. This is signature and role-policy closure only; it
/// does not establish complete path, hostname, revocation, TLS, HTTP, OAuth,
/// or network validity.
pub struct BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    accumulated_intermediates: AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    target_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'target, 'target_encoded>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return all retained anchor-to-target intermediate evidence.
    #[must_use]
    pub const fn accumulated_intermediates(
        &self,
    ) -> &AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.accumulated_intermediates
    }

    /// Return routed authentication of the exact construction target.
    #[must_use]
    pub const fn target_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.target_signature
    }

    /// Return complete server policy for the exact construction target.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'target, 'target_encoded> {
        &self.server_policy
    }
}

impl fmt::Debug
    for BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature")
    }
}

/// Name-policy-backed accumulated evidence closed by the exact target signature.
///
/// This stronger closure retains the exact Name-value policy, reviewed anchor,
/// certificate-wide anchor-role policy, ordered intermediate signature and
/// path-length evidence, depth-zero target signature, and complete server
/// policy in one value. It does not establish complete Name or path validity,
/// revocation, TLS, HTTP, OAuth, or network authority.
pub struct BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    accumulated_intermediates:
        AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    target_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'target, 'target_encoded>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return all retained Name-policy-backed intermediate evidence.
    #[must_use]
    pub const fn accumulated_intermediates(
        &self,
    ) -> &AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.accumulated_intermediates
    }

    /// Return routed authentication of the exact construction target.
    #[must_use]
    pub const fn target_signature(
        &self,
    ) -> &VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.target_signature
    }

    /// Return complete server policy for the exact construction target.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'target, 'target_encoded> {
        &self.server_policy
    }
}

impl fmt::Debug
    for BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature")
    }
}

/// Arbitrary-depth routed closure after reviewed-anchor DNS policy.
///
/// The exact target and its validated Subject Alternative Name index are
/// derived from the retained server policy. Optional constraint bytes are
/// derived from the reviewed anchor retained by the accumulator's terminal
/// proof. Success proves either that the snapshot entry has no constraints or
/// that every target dNSName satisfies its supported permitted DNS subtrees.
/// This does not apply intermediate constraints, excluded or non-DNS anchor
/// constraints, revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    target_signature_closure: BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the complete consumed target-signature closure.
    #[must_use]
    pub const fn target_signature_closure(
        &self,
    ) -> &BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.target_signature_closure
    }

    /// Return DNS evidence when the reviewed snapshot entry carried constraints.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl fmt::Debug
    for ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy")
    }
}

/// Name-policy-backed routed closure after reviewed-anchor DNS policy.
///
/// This stronger result retains the exact Name-value policy and all reviewed
/// construction, anchor-role, ordered intermediate, target-signature, and
/// complete server-policy evidence while applying the same supported reviewed-
/// anchor permitted-DNS subset as the generic result. It does not establish
/// complete Name or path validity, revocation, TLS, HTTP, OAuth, or network
/// authority.
pub struct ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    target_signature_closure:
        BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the complete consumed Name-policy-backed target closure.
    #[must_use]
    pub const fn target_signature_closure(
        &self,
    ) -> &BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.target_signature_closure
    }

    /// Return DNS evidence when the reviewed snapshot entry carried constraints.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl fmt::Debug
    for ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy",
        )
    }
}

/// Name-policy-backed closure after every retained intermediate constraint.
///
/// Complete intermediate policies are visited only in the anchor-to-target
/// order retained by the strong accumulator. Every present NameConstraints
/// extension must be critical, decode as the supported DNS/IP subset, and
/// admit every exact target identity. This retains the complete stronger
/// reviewed-anchor policy without establishing complete Name or path validity,
/// revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    anchor_policy: ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constrained_intermediate_count: usize,
    constrained_dns_names: Option<ConstrainedDnsNames>,
    constrained_names: Option<ConstrainedDnsAndIpNames>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the consumed strong reviewed-anchor policy and complete closure.
    #[must_use]
    pub const fn anchor_policy(
        &self,
    ) -> &ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.anchor_policy
    }

    /// Return how many retained intermediates carried supported constraints.
    #[must_use]
    pub const fn constrained_intermediate_count(&self) -> usize {
        self.constrained_intermediate_count
    }

    /// Return target DNS-count evidence when any intermediate constrained it.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }

    /// Return target DNS/IP counts when any intermediate constrained them.
    #[must_use]
    pub const fn constrained_names(&self) -> Option<ConstrainedDnsAndIpNames> {
        self.constrained_names
    }
}

impl fmt::Debug
    for ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy",
        )
    }
}

/// Arbitrary-depth closure after every retained intermediate name policy.
///
/// Complete intermediate policies are visited only in the anchor-to-target
/// order already retained by the accumulator. Every present NameConstraints
/// extension must be critical, decode as the supported DNS/IP subset, and
/// admit every exact target identity. This does not constrain subordinate CA
/// names, support every GeneralName, establish complete path validity, check
/// revocation, perform TLS or HTTP, or authorize OAuth or network activity.
pub struct ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    anchor_policy: ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constrained_intermediate_count: usize,
    constrained_dns_names: Option<ConstrainedDnsNames>,
    constrained_names: Option<ConstrainedDnsAndIpNames>,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the consumed reviewed-anchor policy and complete closure.
    #[must_use]
    pub const fn anchor_policy(
        &self,
    ) -> &ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.anchor_policy
    }

    /// Return how many retained intermediates carried supported constraints.
    #[must_use]
    pub const fn constrained_intermediate_count(&self) -> usize {
        self.constrained_intermediate_count
    }

    /// Return target DNS-count evidence when any intermediate constrained it.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }

    /// Return target DNS/IP counts when any intermediate constrained them.
    #[must_use]
    pub const fn constrained_names(&self) -> Option<ConstrainedDnsAndIpNames> {
        self.constrained_names
    }
}

impl fmt::Debug
    for ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy",
        )
    }
}

/// Arbitrary-depth closure after supported subordinate-CA SAN constraints.
///
/// Each retained intermediate's supported DNS/IP constraints have been
/// applied to every target-nearer, non-self-issued intermediate certificate's
/// optional Subject Alternative Name. The consumed target policy separately
/// proves the same constraints against the final target. This does not
/// constrain subordinate distinguished names, apply reviewed-anchor
/// constraints to subordinate CAs, support every GeneralName, establish
/// complete path validity, check revocation, or authorize transport.
pub struct ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    target_policy:
        ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constraint_application_count: usize,
    dns_name_evaluation_count: usize,
    ip_address_evaluation_count: usize,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the consumed exact-target intermediate-constraints policy.
    #[must_use]
    pub const fn target_policy(
        &self,
    ) -> &ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >{
        &self.target_policy
    }

    /// Return successful ancestor-constraint to subordinate-CA applications.
    #[must_use]
    pub const fn constraint_application_count(&self) -> usize {
        self.constraint_application_count
    }

    /// Return total repeated subordinate dNSName evaluations.
    #[must_use]
    pub const fn dns_name_evaluation_count(&self) -> usize {
        self.dns_name_evaluation_count
    }

    /// Return total repeated subordinate iPAddress evaluations.
    #[must_use]
    pub const fn ip_address_evaluation_count(&self) -> usize {
        self.ip_address_evaluation_count
    }
}

impl fmt::Debug
    for ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy",
        )
    }
}

/// Arbitrary-depth closure after reviewed-anchor subordinate-CA DNS policy.
///
/// The reviewed anchor's supported permitted-DNS constraint has been applied
/// to every retained non-self-issued intermediate certificate's optional
/// Subject Alternative Name. The consumed policy already proves the anchor
/// constraint against the final target and supported intermediate constraints
/// across the path. This does not constrain subordinate distinguished names,
/// support every GeneralName, establish complete path validity, check
/// revocation, or authorize transport.
pub struct ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    subordinate_policy:
        ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constraint_application_count: usize,
    dns_name_evaluation_count: usize,
}

impl<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the consumed subordinate-intermediate constraints policy.
    #[must_use]
    pub const fn subordinate_policy(
        &self,
    ) -> &ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >{
        &self.subordinate_policy
    }

    /// Return reviewed-anchor to subordinate-CA constraint applications.
    #[must_use]
    pub const fn constraint_application_count(&self) -> usize {
        self.constraint_application_count
    }

    /// Return total reviewed-anchor subordinate dNSName evaluations.
    #[must_use]
    pub const fn dns_name_evaluation_count(&self) -> usize {
        self.dns_name_evaluation_count
    }
}

impl fmt::Debug
    for ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy",
        )
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

/// Bind routed constructed-path authentication to complete intermediate policy.
///
/// The routed proof must describe a non-terminal issuer, and the exact issuer
/// retained by its construction projection must be the same decoded object
/// retained by `intermediate_policy`. Byte-equal independent decodes are not
/// interchangeable. Success consumes both proofs so later path-length work
/// cannot detach policy from the authenticated construction order.
pub fn bind_x509_rsa_constructed_path_intermediate_policy<
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
    verified_signature: VerifiedX509RsaConstructedPathPolicySignature<
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
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
    X509RsaConstructedPathIntermediatePolicyError,
> {
    let adjacent_link = verified_signature.adjacent_link();
    if adjacent_link.issuer_depth() + 1 == adjacent_link.constructed_path().issuer_count() {
        return Err(X509RsaConstructedPathIntermediatePolicyError::new(
            X509RsaConstructedPathIntermediatePolicyErrorKind::TerminalIssuer,
        ));
    }
    if !same_exact_object(
        adjacent_link.issuer_certificate(),
        intermediate_policy.certificate(),
    ) {
        return Err(X509RsaConstructedPathIntermediatePolicyError::new(
            X509RsaConstructedPathIntermediatePolicyErrorKind::EvidenceMismatch,
        ));
    }

    Ok(VerifiedX509RsaConstructedPathIntermediatePolicySignature {
        verified_signature,
        intermediate_policy,
    })
}

/// Bind routed terminal authentication to exact reviewed-root selection.
///
/// The authenticated adjacency must be the construction's final selected
/// issuer, and that issuer must be the same decoded certificate object retained
/// by `reviewed_anchor`. Byte-equal independent decodes are not interchangeable.
/// Success consumes both proofs so the reviewed terminal cannot be substituted
/// after data-driven signature verification.
pub fn bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
    'anchor_certificate,
    'anchor_encoded,
>(
    verified_terminal_signature: VerifiedX509RsaConstructedPathPolicySignature<
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
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
) -> Result<
    BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    X509RsaConstructedPathReviewedAnchorError,
> {
    let adjacent_link = verified_terminal_signature.adjacent_link();
    if adjacent_link.issuer_depth() + 1 != adjacent_link.constructed_path().issuer_count() {
        return Err(X509RsaConstructedPathReviewedAnchorError::new(
            X509RsaConstructedPathReviewedAnchorErrorKind::NonTerminalIssuer,
        ));
    }
    if !same_exact_object(
        adjacent_link.issuer_certificate(),
        reviewed_anchor.candidate_certificate(),
    ) {
        return Err(X509RsaConstructedPathReviewedAnchorError::new(
            X509RsaConstructedPathReviewedAnchorErrorKind::EvidenceMismatch,
        ));
    }

    Ok(BoundX509RsaConstructedPathTerminalToReviewedAnchor {
        verified_terminal_signature,
        reviewed_anchor,
    })
}

/// Advance path length from one routed construction-bound intermediate proof.
///
/// Role, self-issued classification, and `pathLenConstraint` are derived only
/// from the complete policy retained by `authenticated_intermediate`. Success
/// consumes and retains that ordered proof with the advanced state. This does
/// not bind the caller-owned starting state to a prior construction depth.
pub fn advance_x509_rsa_constructed_path_intermediate_path_length<
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
    path_length_state: X509CertificatePathLengthState,
    authenticated_intermediate: VerifiedX509RsaConstructedPathIntermediatePolicySignature<
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
) -> Result<
    ValidatedX509RsaConstructedPathIntermediatePathLength<
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
    X509CertificatePathLengthError,
> {
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(authenticated_intermediate.intermediate_policy())?;

    Ok(ValidatedX509RsaConstructedPathIntermediatePathLength {
        authenticated_intermediate,
        path_length_state,
    })
}

fn advance_initialized_reviewed_intermediate_path_length_state<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    initialized_path: &ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    initialized_path_length_state: &X509CertificatePathLengthState,
    reviewed_terminal: &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: &ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    X509CertificatePathLengthState,
    X509RsaConstructedPathInitializedIntermediatePathLengthError,
> {
    let terminal_link = reviewed_terminal
        .verified_terminal_signature()
        .adjacent_link();
    if !same_exact_object(initialized_path, terminal_link.constructed_path()) {
        return Err(
            X509RsaConstructedPathInitializedIntermediatePathLengthError::new(
                X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::EvidenceMismatch,
            ),
        );
    }

    let Some(anchor_nearest_intermediate) = terminal_link.child_candidate_certificate() else {
        return Err(
            X509RsaConstructedPathInitializedIntermediatePathLengthError::new(
                X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::InvalidPathShape,
            ),
        );
    };
    if !same_exact_object(
        anchor_nearest_intermediate,
        intermediate_policy.certificate(),
    ) {
        return Err(
            X509RsaConstructedPathInitializedIntermediatePathLengthError::new(
                X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::IntermediatePolicyMismatch,
            ),
        );
    }

    X509CertificatePathLengthState::new(
        initialized_path_length_state.remaining_non_self_issued_intermediate_cas(),
    )
    .advance_validated_intermediate_ca(intermediate_policy)
    .map_err(|error| {
        X509RsaConstructedPathInitializedIntermediatePathLengthError::new(
            X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::PathLength(
                error.kind(),
            ),
        )
    })
}

fn advance_next_reviewed_intermediate_path_length_state<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    reviewed_terminal: &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    previous_intermediate_policy: &ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
    previous_path_length_state: &X509CertificatePathLengthState,
    next_signature: &VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: &ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<X509CertificatePathLengthState, X509RsaConstructedPathNextIntermediatePathLengthError> {
    let previous_terminal_link = reviewed_terminal
        .verified_terminal_signature()
        .adjacent_link();
    let next_link = next_signature.adjacent_link();
    if !same_exact_object(
        previous_terminal_link.constructed_path(),
        next_link.constructed_path(),
    ) {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::EvidenceMismatch,
        ));
    }

    let Some(expected_next_depth) = previous_terminal_link.issuer_depth().checked_sub(1) else {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::InvalidPathShape,
        ));
    };
    if next_link.issuer_depth() != expected_next_depth {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::InvalidPathShape,
        ));
    }
    if !same_exact_object(
        next_link.issuer_certificate(),
        previous_intermediate_policy.certificate(),
    ) {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::EvidenceMismatch,
        ));
    }

    let Some(next_intermediate) = next_link.child_candidate_certificate() else {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::InvalidPathShape,
        ));
    };
    if !same_exact_object(next_intermediate, intermediate_policy.certificate()) {
        return Err(X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::IntermediatePolicyMismatch,
        ));
    }

    X509CertificatePathLengthState::new(
        previous_path_length_state.remaining_non_self_issued_intermediate_cas(),
    )
    .advance_validated_intermediate_ca(intermediate_policy)
    .map_err(|error| {
        X509RsaConstructedPathNextIntermediatePathLengthError::new(
            X509RsaConstructedPathNextIntermediatePathLengthErrorKind::PathLength(error.kind()),
        )
    })
}

/// Advance the policy-backed root-authenticated first intermediate.
///
/// RFC 5280 path processing proceeds from the trust anchor toward the target.
/// The terminal routed proof therefore authenticates the anchor-nearest
/// intermediate as its child using the reviewed root's key. The terminal proof
/// and stronger initializer must retain the same exact constructed-path object,
/// and complete intermediate policy must retain that exact terminal child.
/// Only after those checks pass is a private working state recreated from the
/// construction-derived capacity and advanced with that child policy.
///
/// The result retains the complete Name-policy-backed initializer, terminal
/// authentication, child policy, and advanced state. It does not process later
/// intermediates or establish complete path, Name, TLS, HTTP, or OAuth validity.
pub fn advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    initialized_path_length: &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathInitializedIntermediatePathLengthError,
> {
    let initialized_path = initialized_path_length
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    let path_length_state = advance_initialized_reviewed_intermediate_path_length_state(
        initialized_path,
        initialized_path_length.path_length_state(),
        &reviewed_terminal,
        &intermediate_policy,
    )?;

    Ok(
        AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength {
            initialized_path_length,
            reviewed_terminal,
            intermediate_policy,
            path_length_state,
        },
    )
}

/// Advance the root-authenticated first intermediate from initialized state.
///
/// RFC 5280 path processing proceeds from the trust anchor toward the target.
/// The terminal routed proof therefore authenticates the anchor-nearest
/// intermediate as its child using the reviewed root's key. The terminal proof
/// and initializer must retain the same exact constructed-path object, and
/// complete intermediate policy must retain that exact terminal child. Only
/// after those checks pass is a private working state recreated from the
/// construction-derived capacity and advanced with that child policy.
///
/// The initializer is borrowed because it owns the path that the terminal
/// proof itself borrows. The result retains that reference, the consumed
/// terminal authentication, complete child policy, and advanced state. This is
/// one ordered transition only; it does not process remaining intermediates or
/// establish complete path, TLS, HTTP, or OAuth validity.
pub fn advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    initialized_path_length: &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathInitializedIntermediatePathLengthError,
> {
    let initialized_path = initialized_path_length
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let path_length_state = advance_initialized_reviewed_intermediate_path_length_state(
        initialized_path,
        initialized_path_length.path_length_state(),
        &reviewed_terminal,
        &intermediate_policy,
    )?;

    Ok(
        AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength {
            initialized_path_length,
            reviewed_terminal,
            intermediate_policy,
            path_length_state,
        },
    )
}

/// Advance the policy-backed immediately target-nearer intermediate.
///
/// The previous stronger step and `next_signature` must retain the same exact
/// construction. The routed proof's issuer depth must immediately precede the
/// reviewed terminal depth, and its issuer must be the exact intermediate
/// already processed by the previous step. Its child must be another selected
/// candidate, and `intermediate_policy` must retain that exact child. Only then
/// is the prior retained path-length state recreated privately and advanced.
///
/// Success retains the exact Name-policy-backed first step, the second routed
/// proof, its child policy, and the twice-advanced state. It does not iterate
/// over later depths or establish complete path, TLS, HTTP, OAuth, or network
/// validity.
pub fn advance_next_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    previous_step:
        AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    next_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathNextIntermediatePathLengthError,
> {
    let path_length_state = advance_next_reviewed_intermediate_path_length_state(
        previous_step.reviewed_terminal(),
        previous_step.intermediate_policy(),
        previous_step.path_length_state(),
        &next_signature,
        &intermediate_policy,
    )?;

    Ok(
        AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength {
            previous_step,
            next_signature,
            intermediate_policy,
            path_length_state,
        },
    )
}

/// Advance the immediately target-nearer authenticated intermediate.
///
/// The previous ordered step and `next_signature` must retain the same exact
/// construction. The routed proof's issuer depth must immediately precede the
/// reviewed terminal depth, and its issuer must be the exact intermediate
/// already processed by the previous step. Its child must be another selected
/// candidate, and `intermediate_policy` must retain that exact child. Only then
/// is the prior retained path-length state recreated privately and advanced.
///
/// Success consumes and retains both ordered proofs and the second child policy.
/// It does not iterate over later depths or establish complete path, TLS, HTTP,
/// OAuth, or network validity.
pub fn advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length<
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    previous_step: AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    next_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength<
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathNextIntermediatePathLengthError,
> {
    let path_length_state = advance_next_reviewed_intermediate_path_length_state(
        previous_step.reviewed_terminal(),
        previous_step.intermediate_policy(),
        previous_step.path_length_state(),
        &next_signature,
        &intermediate_policy,
    )?;

    Ok(
        AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength {
            previous_step,
            next_signature,
            intermediate_policy,
            path_length_state,
        },
    )
}

/// Accumulate every policy-backed intermediate path-length transition.
///
/// This stronger entry point accepts only the Name-policy-backed reviewed
/// initializer. It applies the same bounded, exact-construction ordered
/// traversal as the generic accumulator while retaining the exact Name-value
/// policy, reviewed anchor, and anchor-role policy through its result. It stops
/// before target authentication and does not establish complete path, Name,
/// TLS, HTTP, OAuth, or network validity.
pub fn accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    initialized_path_length: &'initialized InitializedNamePolicyReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    ordered_signatures: &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >],
    intermediate_policies: &'evidence [ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >],
    max_intermediates: usize,
) -> Result<
    AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathIntermediatePathLengthAccumulationError,
> {
    let initialized_path = initialized_path_length
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    let path_length_state = accumulate_reviewed_intermediate_path_length_state(
        initialized_path,
        initialized_path_length.path_length_state(),
        reviewed_terminal,
        ordered_signatures,
        intermediate_policies,
        max_intermediates,
    )?;

    Ok(
        AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength {
            initialized_path_length,
            reviewed_terminal,
            ordered_signatures,
            intermediate_policies,
            path_length_state,
        },
    )
}

/// Accumulate every intermediate path-length transition in construction order.
///
/// `intermediate_policies` must contain exactly one complete policy per
/// intermediate, ordered from the reviewed anchor toward the target.
/// `ordered_signatures` must contain the corresponding already routed proof for
/// every intermediate after the root-authenticated first child. The exact
/// construction determines both counts; `max_intermediates` is an additional
/// caller-selected nonzero work ceiling.
///
/// Each follow-on proof must consume the immediately target-nearer issuer depth,
/// use the previously processed intermediate as its exact issuer object, and
/// authenticate another selected candidate whose exact object is retained by
/// the next policy. Success borrows all complete inputs and the final advanced
/// state. It deliberately stops before the target signature and does not
/// establish complete path, TLS, HTTP, OAuth, or network validity.
pub fn accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    initialized_path_length: &'initialized InitializedReviewedConstructedX509CertificatePathLength<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    reviewed_terminal: &'evidence BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    ordered_signatures: &'evidence [VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >],
    intermediate_policies: &'evidence [ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >],
    max_intermediates: usize,
) -> Result<
    AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathIntermediatePathLengthAccumulationError,
> {
    let initialized_path = initialized_path_length
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let path_length_state = accumulate_reviewed_intermediate_path_length_state(
        initialized_path,
        initialized_path_length.path_length_state(),
        reviewed_terminal,
        ordered_signatures,
        intermediate_policies,
        max_intermediates,
    )?;

    Ok(
        AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength {
            initialized_path_length,
            reviewed_terminal,
            ordered_signatures,
            intermediate_policies,
            path_length_state,
        },
    )
}

fn accumulate_reviewed_intermediate_path_length_state<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    initialized_path: &ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    initial_path_length_state: &X509CertificatePathLengthState,
    reviewed_terminal: &BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
    ordered_signatures: &[VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >],
    intermediate_policies: &[ValidatedX509IntermediateCertificatePolicy<
        'candidates,
        'candidate_encoded,
    >],
    max_intermediates: usize,
) -> Result<
    X509CertificatePathLengthState,
    X509RsaConstructedPathIntermediatePathLengthAccumulationError,
> {
    if max_intermediates == 0 {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidIntermediateLimit,
            ),
        );
    }

    let terminal_link = reviewed_terminal
        .verified_terminal_signature()
        .adjacent_link();
    if !same_exact_object(initialized_path, terminal_link.constructed_path()) {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceMismatch,
            ),
        );
    }

    let Some(intermediate_count) = initialized_path.issuer_count().checked_sub(1) else {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
            ),
        );
    };
    if intermediate_count == 0 {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
            ),
        );
    }
    if intermediate_count > max_intermediates {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::IntermediateLimitExceeded,
            ),
        );
    }
    if intermediate_policies.len() != intermediate_count
        || ordered_signatures.len().checked_add(1) != Some(intermediate_count)
    {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceCountMismatch,
            ),
        );
    }

    let Some(first_intermediate) = terminal_link.child_candidate_certificate() else {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
            ),
        );
    };
    let first_policy = &intermediate_policies[0];
    if !same_exact_object(first_intermediate, first_policy.certificate()) {
        return Err(
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::IntermediatePolicyMismatch,
            ),
        );
    }

    let mut path_length_state = X509CertificatePathLengthState::new(
        initial_path_length_state.remaining_non_self_issued_intermediate_cas(),
    );
    path_length_state = path_length_state
        .advance_validated_intermediate_ca(first_policy)
        .map_err(|error| {
            X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::PathLength(
                    error.kind(),
                ),
            )
        })?;

    let mut expected_depth = terminal_link.issuer_depth();
    let mut previous_intermediate = first_policy.certificate();
    for (signature, policy) in ordered_signatures
        .iter()
        .zip(intermediate_policies[1..].iter())
    {
        let Some(next_depth) = expected_depth.checked_sub(1) else {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
                ),
            );
        };
        let link = signature.adjacent_link();
        if !same_exact_object(initialized_path, link.constructed_path()) {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceMismatch,
                ),
            );
        }
        if link.issuer_depth() != next_depth {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
                ),
            );
        }
        if !same_exact_object(link.issuer_certificate(), previous_intermediate) {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceMismatch,
                ),
            );
        }
        let Some(next_intermediate) = link.child_candidate_certificate() else {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape,
                ),
            );
        };
        if !same_exact_object(next_intermediate, policy.certificate()) {
            return Err(
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::IntermediatePolicyMismatch,
                ),
            );
        }

        path_length_state = path_length_state
            .advance_validated_intermediate_ca(policy)
            .map_err(|error| {
                X509RsaConstructedPathIntermediatePathLengthAccumulationError::new(
                    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::PathLength(
                        error.kind(),
                    ),
                )
            })?;
        expected_depth = next_depth;
        previous_intermediate = policy.certificate();
    }

    Ok(path_length_state)
}

fn validate_accumulated_target_signature_closure(
    constructed_path: &ConstructedX509CertificatePath<'_, '_, '_, '_>,
    intermediate_policies: &[ValidatedX509IntermediateCertificatePolicy<'_, '_>],
    target_signature: &VerifiedX509RsaConstructedPathPolicySignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    server_policy: &ValidatedX509ServerCertificatePolicy<'_, '_>,
) -> Result<(), X509RsaConstructedPathTargetSignatureClosureError> {
    let target_link = target_signature.adjacent_link();
    if !same_exact_object(constructed_path, target_link.constructed_path()) {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::EvidenceMismatch,
        ));
    }
    if target_link.issuer_depth() != 0 {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::InvalidPathShape,
        ));
    }
    let Some(target_certificate) = target_link.child_target_certificate() else {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::InvalidPathShape,
        ));
    };
    if !same_exact_object(target_certificate, constructed_path.target_certificate()) {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::EvidenceMismatch,
        ));
    }
    let Some(target_nearest_policy) = intermediate_policies.last() else {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::InvalidPathShape,
        ));
    };
    if !same_exact_object(
        target_link.issuer_certificate(),
        target_nearest_policy.certificate(),
    ) {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::EvidenceMismatch,
        ));
    }
    if !same_exact_object(server_policy.certificate(), target_certificate) {
        return Err(X509RsaConstructedPathTargetSignatureClosureError::new(
            X509RsaConstructedPathTargetSignatureClosureErrorKind::TargetPolicyMismatch,
        ));
    }

    Ok(())
}

/// Close accumulated intermediate evidence with the exact routed target link.
///
/// The target proof must be the depth-zero adjacency from the same exact
/// constructed path retained by the accumulator. Its child must be that
/// construction's target and its issuer must be the exact target-nearest
/// intermediate already retained by the final accumulated policy. Complete
/// server policy must retain the same exact target object. Success consumes
/// and keeps all three proofs without granting complete path, hostname,
/// revocation, TLS, HTTP, OAuth, or network authority.
pub fn bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    accumulated_intermediates: AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    target_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'target, 'target_encoded>,
) -> Result<
    BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathTargetSignatureClosureError,
> {
    let constructed_path = accumulated_intermediates
        .initialized_path_length()
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    validate_accumulated_target_signature_closure(
        constructed_path,
        accumulated_intermediates.intermediate_policies(),
        &target_signature,
        &server_policy,
    )?;

    Ok(
        BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature {
            accumulated_intermediates,
            target_signature,
            server_policy,
        },
    )
}

/// Close Name-policy-backed accumulated evidence with the exact target link.
///
/// This applies the same exact-construction, depth-zero target, target-nearest
/// issuer, and exact server-policy checks as the generic closure while only
/// accepting the stronger Name-policy-backed accumulator. Success preserves
/// that accumulator and all of its provenance in the returned closure.
pub fn bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    accumulated_intermediates:
        AccumulatedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    target_signature: VerifiedX509RsaConstructedPathPolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'target, 'target_encoded>,
) -> Result<
    BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathTargetSignatureClosureError,
> {
    let constructed_path = accumulated_intermediates
        .initialized_path_length()
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    validate_accumulated_target_signature_closure(
        constructed_path,
        accumulated_intermediates.intermediate_policies(),
        &target_signature,
        &server_policy,
    )?;

    Ok(
        BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature {
            accumulated_intermediates,
            target_signature,
            server_policy,
        },
    )
}

/// Apply selected reviewed-anchor DNS policy to one arbitrary-depth closure.
///
/// The target certificate and validated SAN extension index are derived only
/// from the closure's complete server policy. Optional constraint bytes are
/// derived only from the reviewed anchor retained through the accumulator's
/// terminal proof. Separate caller limits bound both repository-owned
/// decoders, and success retains the complete closure.
pub fn validate_accumulated_reviewed_x509_rsa_constructed_path_target_anchor_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    target_signature_closure: BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let certificate = target_signature_closure.server_policy().certificate();
    let subject_alt_name_extension_index = target_signature_closure
        .server_policy()
        .identity()
        .extension_index();
    let name_constraints = target_signature_closure
        .accumulated_intermediates()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        subject_alt_name_extension_index,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy {
            target_signature_closure,
            constrained_dns_names,
        },
    )
}

/// Apply reviewed-anchor DNS policy to a Name-policy-backed target closure.
///
/// The exact target and validated SAN index are derived only from the complete
/// server policy retained by the stronger closure. Optional constraint bytes
/// are derived only from its reviewed anchor. The same repository-owned
/// supported permitted-DNS evaluator and caller-owned limits used by the
/// generic result apply here, while success retains the stronger closure.
pub fn validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    target_signature_closure:
        BoundAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetSignature<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let certificate = target_signature_closure.server_policy().certificate();
    let subject_alt_name_extension_index = target_signature_closure
        .server_policy()
        .identity()
        .extension_index();
    let name_constraints = target_signature_closure
        .accumulated_intermediates()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        subject_alt_name_extension_index,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy {
            target_signature_closure,
            constrained_dns_names,
        },
    )
}

/// Apply every retained intermediate constraint to the strong target proof.
///
/// The target and validated SAN index are derived only from the complete
/// server policy retained by `anchor_policy`. Intermediate certificates come
/// only from the strong accumulator and are visited in its proved anchor-to-
/// target order. The same private bounded evaluator used by the generic entry
/// point supplies identical supported constraint and failure semantics.
pub fn validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    anchor_policy: ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
>{
    let target_signature_closure = anchor_policy.target_signature_closure();
    let target_certificate = target_signature_closure.server_policy().certificate();
    let subject_alt_name_extension_index = target_signature_closure
        .server_policy()
        .identity()
        .extension_index();
    let intermediate_certificates = target_signature_closure
        .accumulated_intermediates()
        .intermediate_policies()
        .iter()
        .map(ValidatedX509IntermediateCertificatePolicy::certificate);
    let (constrained_intermediate_count, constrained_dns_names, constrained_names) =
        evaluate_ordered_intermediate_name_constraints_policy(
            target_certificate,
            subject_alt_name_extension_index,
            intermediate_certificates,
            constraint_limits,
            subject_alt_name_limits,
        )?;

    Ok(
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy {
            anchor_policy,
            constrained_intermediate_count,
            constrained_dns_names,
            constrained_names,
        },
    )
}

/// Apply every retained intermediate's supported critical name constraints.
///
/// The target and validated SAN index are derived from the complete server
/// policy retained by `anchor_policy`. Intermediate certificates are derived
/// from the accumulator's complete policy slice and visited only in its
/// anchor-to-target order. The earlier accumulator ceiling bounds visits;
/// separate caller limits bound every constraints and SAN decode.
pub fn validate_accumulated_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    anchor_policy: ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let target_signature_closure = anchor_policy.target_signature_closure();
    let target_certificate = target_signature_closure.server_policy().certificate();
    let subject_alt_name_extension_index = target_signature_closure
        .server_policy()
        .identity()
        .extension_index();
    let intermediate_certificates = target_signature_closure
        .accumulated_intermediates()
        .intermediate_policies()
        .iter()
        .map(ValidatedX509IntermediateCertificatePolicy::certificate);
    let (constrained_intermediate_count, constrained_dns_names, constrained_names) =
        evaluate_ordered_intermediate_name_constraints_policy(
            target_certificate,
            subject_alt_name_extension_index,
            intermediate_certificates,
            constraint_limits,
            subject_alt_name_limits,
        )?;

    Ok(
        ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy {
            anchor_policy,
            constrained_intermediate_count,
            constrained_dns_names,
            constrained_names,
        },
    )
}

/// Apply each intermediate's supported constraints to subordinate CA SANs.
///
/// All certificates, their anchor-to-target order, and their self-issued
/// classifications come only from the consumed proof. A constraining
/// intermediate is never applied to itself. Target-nearer self-issued
/// intermediates are skipped as required for non-final self-issued
/// certificates; the consumed target policy already covers the final target.
pub fn validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    target_policy:
        ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let intermediate_policies = target_policy
        .anchor_policy()
        .target_signature_closure()
        .accumulated_intermediates()
        .intermediate_policies();
    let (constraint_application_count, dns_name_evaluation_count, ip_address_evaluation_count) =
        evaluate_ordered_subordinate_intermediate_name_constraints_policy(
            intermediate_policies.len(),
            |index| intermediate_policies[index].certificate(),
            |index| {
                intermediate_policies[index]
                    .self_issued_classification()
                    .is_self_issued()
            },
            constraint_limits,
            subject_alt_name_limits,
        )?;

    Ok(
        ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy {
            target_policy,
            constraint_application_count,
            dns_name_evaluation_count,
            ip_address_evaluation_count,
        },
    )
}

/// Apply the reviewed anchor's supported DNS constraint to subordinate CAs.
///
/// Constraint bytes, certificates, anchor-to-target order, and self-issued
/// classifications come only from the consumed proof. An unconstrained
/// reviewed snapshot entry succeeds without reading subordinate SANs.
/// Non-final self-issued intermediates are skipped; the consumed target-anchor
/// policy already covers the final target.
pub fn validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    subordinate_policy:
        ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let accumulated_intermediates = subordinate_policy
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .accumulated_intermediates();
    let intermediate_policies = accumulated_intermediates.intermediate_policies();
    let name_constraints = accumulated_intermediates
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let (constraint_application_count, dns_name_evaluation_count) =
        evaluate_selected_anchor_subordinate_intermediate_dns_policy(
            intermediate_policies.len(),
            |index| intermediate_policies[index].certificate(),
            |index| {
                intermediate_policies[index]
                    .self_issued_classification()
                    .is_self_issued()
            },
            name_constraints,
            constraint_limits,
            subject_alt_name_limits,
        )?;

    Ok(
        ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy {
            subordinate_policy,
            constraint_application_count,
            dns_name_evaluation_count,
        },
    )
}

/// Bind one exact routed target -> intermediate -> reviewed-root construction.
///
/// The authenticated intermediate must be the depth-zero projection and the
/// reviewed terminal must be the depth-one projection from the identical
/// constructed-path object. The construction must contain exactly those two
/// issuers, and the terminal child must be the exact intermediate retained by
/// complete policy. Success derives the initial path-length capacity from that
/// exact shape, advances it across the retained intermediate, and keeps the
/// resulting state with both authenticated links, their data-driven
/// algorithm/key policy, complete intermediate policy, and reviewed-root
/// handoff. No caller-supplied path-length state enters this fixed-shape proof.
pub fn bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    authenticated_intermediate: VerifiedX509RsaConstructedPathIntermediatePolicySignature<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    reviewed_terminal: BoundX509RsaConstructedPathTerminalToReviewedAnchor<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    BoundX509RsaConstructedReviewedIntermediatePath<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaConstructedPathReviewedIntermediatePathError,
> {
    let intermediate_link = authenticated_intermediate
        .verified_signature()
        .adjacent_link();
    let terminal_link = reviewed_terminal
        .verified_terminal_signature()
        .adjacent_link();
    let constructed_path = intermediate_link.constructed_path();

    if !same_exact_object(constructed_path, terminal_link.constructed_path()) {
        return Err(X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::EvidenceMismatch,
        ));
    }
    if constructed_path.issuer_count() != 2
        || intermediate_link.issuer_depth() != 0
        || terminal_link.issuer_depth() != 1
    {
        return Err(X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::InvalidPathShape,
        ));
    }

    let Some(terminal_child) = terminal_link.child_candidate_certificate() else {
        return Err(X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::InvalidPathShape,
        ));
    };
    let policy_intermediate = authenticated_intermediate
        .intermediate_policy()
        .certificate();
    if !same_exact_object(terminal_child, intermediate_link.issuer_certificate())
        || !same_exact_object(terminal_child, policy_intermediate)
    {
        return Err(X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::EvidenceMismatch,
        ));
    }

    // RFC 5280 initializes the path-length state to the number of
    // intermediate certificates in the prospective path. The exact fixed
    // shape above proves two selected issuers: one intermediate and one trust
    // anchor. Deriving this value here prevents callers from weakening or
    // spuriously exhausting the retained construction with detached state.
    let Some(intermediate_count) = constructed_path.issuer_count().checked_sub(1) else {
        return Err(X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::InvalidPathShape,
        ));
    };
    let intermediate_count = u64::try_from(intermediate_count).map_err(|_| {
        X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::InvalidPathShape,
        )
    })?;
    let authenticated_intermediate = advance_x509_rsa_constructed_path_intermediate_path_length(
        X509CertificatePathLengthState::new(intermediate_count),
        authenticated_intermediate,
    )
    .map_err(|error| {
        X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::PathLength(error.kind()),
        )
    })?;

    Ok(BoundX509RsaConstructedReviewedIntermediatePath {
        authenticated_intermediate,
        reviewed_terminal,
    })
}

/// Bind complete target-server policy to one routed reviewed construction.
///
/// The policy certificate must be the exact decoded target object retained by
/// the constructed path. Matching encoded bytes from an independent decode are
/// insufficient. Success consumes and retains both complete inputs. This is
/// pure evidence composition and adds no constraints, revocation, transport,
/// OAuth, or network authority.
pub fn bind_x509_rsa_constructed_reviewed_intermediate_server_path<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    server_policy: ValidatedX509ServerCertificatePolicy<'server_certificate, 'target_encoded>,
    reviewed_path: BoundX509RsaConstructedReviewedIntermediatePath<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    BoundX509RsaConstructedReviewedIntermediateServerPath<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let constructed_target = reviewed_path
        .authenticated_intermediate()
        .authenticated_intermediate()
        .verified_signature()
        .adjacent_link()
        .constructed_path()
        .target_certificate();
    if !same_exact_object(server_policy.certificate(), constructed_target) {
        return Err(X509RsaConstructedPathReviewedServerPolicyError::new(
            X509RsaConstructedPathReviewedServerPolicyErrorKind::EvidenceMismatch,
        ));
    }

    Ok(BoundX509RsaConstructedReviewedIntermediateServerPath {
        server_policy,
        reviewed_path,
    })
}

/// Apply selected reviewed-anchor DNS policy to one exact routed server path.
///
/// Both the target certificate and optional constraint bytes are derived only
/// through the consumed proof. Separate caller limits bound the constraint and
/// Subject Alternative Name decoders. The complete proof remains retained.
pub fn validate_x509_rsa_constructed_reviewed_intermediate_server_anchor_policy<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    reviewed_server_path: BoundX509RsaConstructedReviewedIntermediateServerPath<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let certificate = reviewed_server_path.server_policy().certificate();
    let subject_alt_name_extension_index = reviewed_server_path
        .server_policy()
        .identity()
        .extension_index();
    let name_constraints = reviewed_server_path
        .reviewed_path()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        subject_alt_name_extension_index,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy {
            reviewed_server_path,
            constrained_dns_names,
        },
    )
}

/// Apply the exact routed intermediate's supported critical name constraints.
///
/// This consumes the complete construction-bound anchor-policy proof. The
/// target, its already-validated SAN extension index, and intermediate are
/// derived only through that retained proof. An absent extension succeeds
/// without new evidence; a present extension must be critical and must decode
/// as the supported permitted-and-excluded DNS and IP subsets under
/// caller-owned limits. Every retained target DNS-ID and IP-ID is evaluated in
/// one fail-closed combined operation.
pub fn validate_x509_rsa_constructed_reviewed_intermediate_server_name_constraints_policy<
    'path,
    'server_certificate,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    anchor_policy: ValidatedX509RsaConstructedReviewedIntermediateServerAnchorPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let target_certificate = anchor_policy
        .reviewed_server_path()
        .server_policy()
        .certificate();
    let subject_alt_name_extension_index = anchor_policy
        .reviewed_server_path()
        .server_policy()
        .identity()
        .extension_index();
    let intermediate_certificate = anchor_policy
        .reviewed_server_path()
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let constrained_names = evaluate_intermediate_name_constraints_policy(
        target_certificate,
        subject_alt_name_extension_index,
        intermediate_certificate,
        constraint_limits,
        subject_alt_name_limits,
    )?;
    let (constrained_dns_names, constrained_names) = match constrained_names {
        Some((dns, combined)) => (Some(dns), Some(combined)),
        None => (None, None),
    };

    Ok(
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy {
            anchor_policy,
            constrained_dns_names,
            constrained_names,
        },
    )
}

fn evaluate_selected_anchor_dns_policy(
    certificate: &x509_certificate::X509Certificate<'_>,
    subject_alt_name_extension_index: usize,
    name_constraints: Option<&[u8]>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<Option<ConstrainedDnsNames>, X509RsaConstructedPathReviewedServerPolicyError> {
    let Some(encoded_constraints) = name_constraints else {
        return Ok(None);
    };

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints =
        decode_name_constraints_contents(&mut constraint_decoder, encoded_constraints).map_err(
            |error| {
                X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidAnchorNameConstraints(
                    error.kind(),
                ),
            )
            },
        )?;
    let extension = certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| extensions.get(subject_alt_name_extension_index))
        .ok_or_else(|| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::MissingTargetSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name = decode_subject_alt_name(&mut subject_alt_name_decoder, extension)
        .map_err(|error| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidTargetSubjectAlternativeName(
                    error.kind(),
                ),
            )
        })?;
    let constrained_dns_names = evaluate_dns_names(&constraints, subject_alt_name.dns_names())
        .map_err(|error| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetDnsNameConstraints(
                    error.kind(),
                ),
            )
        })?;

    Ok(Some(constrained_dns_names))
}

fn evaluate_intermediate_name_constraints_policy(
    target_certificate: &x509_certificate::X509Certificate<'_>,
    subject_alt_name_extension_index: usize,
    intermediate_certificate: &x509_certificate::X509Certificate<'_>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    Option<(ConstrainedDnsNames, ConstrainedDnsAndIpNames)>,
    X509RsaConstructedPathReviewedServerPolicyError,
> {
    let Some(extension) = intermediate_certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(NAME_CONSTRAINTS_OID))
        })
    else {
        return Ok(None);
    };
    if !extension.critical() {
        return Err(X509RsaConstructedPathReviewedServerPolicyError::new(
            X509RsaConstructedPathReviewedServerPolicyErrorKind::NonCriticalIntermediateNameConstraints,
        ));
    }

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints = decode_name_constraints(&mut constraint_decoder, extension.extension_value())
        .map_err(|error| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidIntermediateNameConstraints(
                    error.kind(),
                ),
            )
        })?;
    let subject_alt_name_extension = target_certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| extensions.get(subject_alt_name_extension_index))
        .ok_or_else(|| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::MissingTargetSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name =
        decode_subject_alt_name(&mut subject_alt_name_decoder, subject_alt_name_extension)
            .map_err(|error| {
                X509RsaConstructedPathReviewedServerPolicyError::new(
                    X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidTargetSubjectAlternativeName(
                        error.kind(),
                    ),
                )
            })?;
    let constrained_names = evaluate_dns_and_ip_names(
        &constraints,
        subject_alt_name.dns_names(),
        subject_alt_name.ip_addresses(),
    )
    .map_err(|error| {
        let kind = match error.kind() {
            NameConstraintEvaluationErrorKind::Dns(kind) =>
                X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateDnsNameConstraints(kind),
            NameConstraintEvaluationErrorKind::Ip(kind) =>
                X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateIpNameConstraints(kind),
        };
        X509RsaConstructedPathReviewedServerPolicyError::new(kind)
    })?;
    let constrained_dns_names = evaluate_dns_name_subtrees(
        constraints.permitted_dns_subtrees(),
        constraints.excluded_dns_subtrees(),
        subject_alt_name.dns_names(),
    )
    .map_err(|error| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateDnsNameConstraints(
                    error.kind(),
                ),
            )
        })?;

    Ok(Some((constrained_dns_names, constrained_names)))
}

fn evaluate_ordered_intermediate_name_constraints_policy<'certificate, 'encoded, I>(
    target_certificate: &x509_certificate::X509Certificate<'_>,
    subject_alt_name_extension_index: usize,
    intermediate_certificates: I,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    (
        usize,
        Option<ConstrainedDnsNames>,
        Option<ConstrainedDnsAndIpNames>,
    ),
    X509RsaConstructedPathReviewedServerPolicyError,
>
where
    'encoded: 'certificate,
    I: IntoIterator<Item = &'certificate x509_certificate::X509Certificate<'encoded>>,
{
    let mut constrained_intermediate_count = 0;
    let mut constrained_dns_names = None;
    let mut constrained_names = None;

    for intermediate_certificate in intermediate_certificates {
        let Some((next_dns_names, next_names)) = evaluate_intermediate_name_constraints_policy(
            target_certificate,
            subject_alt_name_extension_index,
            intermediate_certificate,
            constraint_limits,
            subject_alt_name_limits,
        )?
        else {
            continue;
        };
        constrained_intermediate_count += 1;
        constrained_dns_names = Some(next_dns_names);
        constrained_names = Some(next_names);
    }

    Ok((
        constrained_intermediate_count,
        constrained_dns_names,
        constrained_names,
    ))
}

fn evaluate_ordered_subordinate_intermediate_name_constraints_policy<
    'certificate,
    'encoded,
    CertificateAt,
    IsSelfIssuedAt,
>(
    intermediate_count: usize,
    certificate_at: CertificateAt,
    is_self_issued_at: IsSelfIssuedAt,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<(usize, usize, usize), X509RsaConstructedPathReviewedServerPolicyError>
where
    'encoded: 'certificate,
    CertificateAt: Fn(usize) -> &'certificate x509_certificate::X509Certificate<'encoded>,
    IsSelfIssuedAt: Fn(usize) -> bool,
{
    let mut constraint_application_count = 0usize;
    let mut dns_name_evaluation_count = 0usize;
    let mut ip_address_evaluation_count = 0usize;

    for constraining_index in 0..intermediate_count {
        let constraining_certificate = certificate_at(constraining_index);
        let Some(extension) = constraining_certificate
            .tbs_certificate()
            .extensions()
            .and_then(|extensions| {
                extensions
                    .iter()
                    .find(|extension| extension.extension_id().equals(NAME_CONSTRAINTS_OID))
            })
        else {
            continue;
        };
        if !extension.critical() {
            return Err(X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::NonCriticalIntermediateNameConstraints,
            ));
        }
        let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
        let constraints =
            decode_name_constraints(&mut constraint_decoder, extension.extension_value())
                .map_err(|error| {
                    X509RsaConstructedPathReviewedServerPolicyError::new(
                        X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidIntermediateNameConstraints(
                            error.kind(),
                        ),
                    )
                })?;

        for subordinate_index in (constraining_index + 1)..intermediate_count {
            if is_self_issued_at(subordinate_index) {
                continue;
            }
            let subordinate_certificate = certificate_at(subordinate_index);
            let subject_alt_name_extension = subordinate_certificate
                .tbs_certificate()
                .extensions()
                .and_then(|extensions| {
                    extensions
                        .iter()
                        .find(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
                });
            let constrained_names = match subject_alt_name_extension {
                None => evaluate_dns_and_ip_names(&constraints, &[], &[]),
                Some(extension) => {
                    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
                    let subject_alt_name =
                        decode_subject_alt_name(&mut subject_alt_name_decoder, extension).map_err(
                            |error| {
                                X509RsaConstructedPathReviewedServerPolicyError::new(
                                    X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidSubordinateIntermediateSubjectAlternativeName(
                                        error.kind(),
                                    ),
                                )
                            },
                        )?;
                    evaluate_dns_and_ip_names(
                        &constraints,
                        subject_alt_name.dns_names(),
                        subject_alt_name.ip_addresses(),
                    )
                }
            }
            .map_err(|error| {
                let kind = match error.kind() {
                    NameConstraintEvaluationErrorKind::Dns(kind) => {
                        X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateIntermediateDnsNameConstraints(kind)
                    }
                    NameConstraintEvaluationErrorKind::Ip(kind) => {
                        X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateIntermediateIpNameConstraints(kind)
                    }
                };
                X509RsaConstructedPathReviewedServerPolicyError::new(kind)
            })?;

            constraint_application_count = constraint_application_count
                .checked_add(1)
                .ok_or_else(subordinate_name_evaluation_count_overflow)?;
            dns_name_evaluation_count = dns_name_evaluation_count
                .checked_add(constrained_names.dns_count())
                .ok_or_else(subordinate_name_evaluation_count_overflow)?;
            ip_address_evaluation_count = ip_address_evaluation_count
                .checked_add(constrained_names.ip_count())
                .ok_or_else(subordinate_name_evaluation_count_overflow)?;
        }
    }

    Ok((
        constraint_application_count,
        dns_name_evaluation_count,
        ip_address_evaluation_count,
    ))
}

fn evaluate_selected_anchor_subordinate_intermediate_dns_policy<
    'certificate,
    'encoded,
    CertificateAt,
    IsSelfIssuedAt,
>(
    intermediate_count: usize,
    certificate_at: CertificateAt,
    is_self_issued_at: IsSelfIssuedAt,
    name_constraints: Option<&[u8]>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<(usize, usize), X509RsaConstructedPathReviewedServerPolicyError>
where
    'encoded: 'certificate,
    CertificateAt: Fn(usize) -> &'certificate x509_certificate::X509Certificate<'encoded>,
    IsSelfIssuedAt: Fn(usize) -> bool,
{
    let Some(encoded_constraints) = name_constraints else {
        return Ok((0, 0));
    };

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints =
        decode_name_constraints_contents(&mut constraint_decoder, encoded_constraints).map_err(
            |error| {
                X509RsaConstructedPathReviewedServerPolicyError::new(
            X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidAnchorNameConstraints(
                error.kind(),
            ),
        )
            },
        )?;
    let mut constraint_application_count = 0usize;
    let mut dns_name_evaluation_count = 0usize;

    for intermediate_index in 0..intermediate_count {
        if is_self_issued_at(intermediate_index) {
            continue;
        }
        let intermediate_certificate = certificate_at(intermediate_index);
        let subject_alt_name_extension = intermediate_certificate
            .tbs_certificate()
            .extensions()
            .and_then(|extensions| {
                extensions
                    .iter()
                    .find(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
            });
        let constrained_dns_names = match subject_alt_name_extension {
            None => evaluate_dns_names(&constraints, &[]),
            Some(extension) => {
                let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
                let subject_alt_name =
                    decode_subject_alt_name(&mut subject_alt_name_decoder, extension).map_err(
                        |error| {
                            X509RsaConstructedPathReviewedServerPolicyError::new(
                                X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidSubordinateIntermediateSubjectAlternativeName(
                                    error.kind(),
                                ),
                            )
                        },
                    )?;
                evaluate_dns_names(&constraints, subject_alt_name.dns_names())
            }
        }
        .map_err(|error| {
            X509RsaConstructedPathReviewedServerPolicyError::new(
                X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateIntermediateAnchorDnsNameConstraints(
                    error.kind(),
                ),
            )
        })?;

        constraint_application_count = constraint_application_count
            .checked_add(1)
            .ok_or_else(subordinate_anchor_name_evaluation_count_overflow)?;
        dns_name_evaluation_count = dns_name_evaluation_count
            .checked_add(constrained_dns_names.count())
            .ok_or_else(subordinate_anchor_name_evaluation_count_overflow)?;
    }

    Ok((constraint_application_count, dns_name_evaluation_count))
}

fn subordinate_name_evaluation_count_overflow() -> X509RsaConstructedPathReviewedServerPolicyError {
    X509RsaConstructedPathReviewedServerPolicyError::new(
        X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateNameEvaluationCountOverflow,
    )
}

fn subordinate_anchor_name_evaluation_count_overflow(
) -> X509RsaConstructedPathReviewedServerPolicyError {
    X509RsaConstructedPathReviewedServerPolicyError::new(
        X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateAnchorNameEvaluationCountOverflow,
    )
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    core::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use x509_certificate::decode_x509_certificate;
    use x509_subject_alt_name::SUBJECT_ALT_NAME_OID;

    const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];
    const NAME_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x1e];
    const NULL_DER: &[u8] = &[0x05, 0x00];

    fn der(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(value.len() + 4);
        encoded.push(tag);
        match value.len() {
            length @ 0..=127 => encoded.push(length as u8),
            length @ 128..=255 => encoded.extend([0x81, length as u8]),
            length @ 256..=65_535 => {
                encoded.extend([0x82, (length >> 8) as u8, length as u8]);
            }
            _ => panic!("test value too large"),
        }
        encoded.extend(value);
        encoded
    }

    fn algorithm(oid: &[u8]) -> Vec<u8> {
        let mut fields = der(0x06, oid);
        fields.extend(NULL_DER);
        der(0x30, &fields)
    }

    fn name(value: &[u8]) -> Vec<u8> {
        let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
        attribute.extend(der(0x0c, value));
        der(0x30, &der(0x31, &der(0x30, &attribute)))
    }

    fn extension(oid: &[u8], value: &[u8]) -> Vec<u8> {
        let mut fields = der(0x06, oid);
        fields.extend(der(0x04, value));
        der(0x30, &fields)
    }

    fn critical_extension(oid: &[u8], value: &[u8]) -> Vec<u8> {
        let mut fields = der(0x06, oid);
        fields.extend([0x01, 0x01, 0xff]);
        fields.extend(der(0x04, value));
        der(0x30, &fields)
    }

    fn subject_alt_name(names: &[Vec<u8>]) -> Vec<u8> {
        extension(SUBJECT_ALT_NAME_OID_DER, &der(0x30, &names.concat()))
    }

    fn dns_name(value: &str) -> Vec<u8> {
        der(0x82, value.as_bytes())
    }

    fn ip_address(value: &[u8]) -> Vec<u8> {
        der(0x87, value)
    }

    fn constraint_subtrees(tag: u8, dns_subtrees: &[&[u8]], ip_subtrees: &[&[u8]]) -> Vec<u8> {
        let mut contents = Vec::new();
        for subtree in dns_subtrees {
            contents.extend(der(0x30, &der(0x82, subtree)));
        }
        for subtree in ip_subtrees {
            contents.extend(der(0x30, &der(0x87, subtree)));
        }
        der(tag, &contents)
    }

    fn permitted_dns_subtrees(subtrees: &[&[u8]]) -> Vec<u8> {
        constraint_subtrees(0xa0, subtrees, &[])
    }

    fn name_constraints(permitted: &[&[u8]], excluded: &[&[u8]], critical: bool) -> Vec<u8> {
        mixed_name_constraints(permitted, &[], excluded, &[], critical)
    }

    fn mixed_name_constraints(
        permitted_dns: &[&[u8]],
        permitted_ip: &[&[u8]],
        excluded_dns: &[&[u8]],
        excluded_ip: &[&[u8]],
        critical: bool,
    ) -> Vec<u8> {
        let mut contents = Vec::new();
        if !permitted_dns.is_empty() || !permitted_ip.is_empty() {
            contents.extend(constraint_subtrees(0xa0, permitted_dns, permitted_ip));
        }
        if !excluded_dns.is_empty() || !excluded_ip.is_empty() {
            contents.extend(constraint_subtrees(0xa1, excluded_dns, excluded_ip));
        }
        let encoded = der(0x30, &contents);
        if critical {
            critical_extension(NAME_CONSTRAINTS_OID_DER, &encoded)
        } else {
            extension(NAME_CONSTRAINTS_OID_DER, &encoded)
        }
    }

    fn certificate(extensions: &[Vec<u8>]) -> Vec<u8> {
        let mut validity = der(0x17, b"260101000000Z");
        validity.extend(der(0x17, b"270101000000Z"));
        let mut subject_public_key_info = algorithm(KEY_OID_DER);
        subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));
        let mut tbs = der(0xa0, &der(0x02, &[0x02]));
        tbs.extend(der(0x02, &[0x01]));
        tbs.extend(algorithm(SIGNATURE_OID_DER));
        tbs.extend(name(b"issuer.example"));
        tbs.extend(der(0x30, &validity));
        tbs.extend(name(b"api.example.test"));
        tbs.extend(der(0x30, &subject_public_key_info));
        if !extensions.is_empty() {
            tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
        }
        let mut fields = der(0x30, &tbs);
        fields.extend(algorithm(SIGNATURE_OID_DER));
        fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
        der(0x30, &fields)
    }

    fn evaluate(
        encoded: &[u8],
        constraints: Option<&[u8]>,
    ) -> Result<Option<ConstrainedDnsNames>, X509RsaConstructedPathReviewedServerPolicyError> {
        let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
        evaluate_selected_anchor_dns_policy(
            &certificate,
            certificate
                .tbs_certificate()
                .extensions()
                .and_then(|extensions| {
                    extensions
                        .iter()
                        .position(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
                })
                .unwrap_or(usize::MAX),
            constraints,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_intermediate(
        target_encoded: &[u8],
        intermediate_encoded: &[u8],
    ) -> Result<
        Option<(ConstrainedDnsNames, ConstrainedDnsAndIpNames)>,
        X509RsaConstructedPathReviewedServerPolicyError,
    > {
        let target = decode_x509_certificate(target_encoded, Asn1Limits::default()).unwrap();
        let intermediate =
            decode_x509_certificate(intermediate_encoded, Asn1Limits::default()).unwrap();
        let subject_alt_name_extension_index = target
            .tbs_certificate()
            .extensions()
            .and_then(|extensions| {
                extensions
                    .iter()
                    .position(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
            })
            .unwrap_or(usize::MAX);
        evaluate_intermediate_name_constraints_policy(
            &target,
            subject_alt_name_extension_index,
            &intermediate,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_ordered_intermediates(
        target_encoded: &[u8],
        intermediate_encoded: &[Vec<u8>],
    ) -> Result<
        (
            usize,
            Option<ConstrainedDnsNames>,
            Option<ConstrainedDnsAndIpNames>,
        ),
        X509RsaConstructedPathReviewedServerPolicyError,
    > {
        let target = decode_x509_certificate(target_encoded, Asn1Limits::default()).unwrap();
        let intermediates = intermediate_encoded
            .iter()
            .map(|encoded| decode_x509_certificate(encoded, Asn1Limits::default()).unwrap())
            .collect::<Vec<_>>();
        let subject_alt_name_extension_index = target
            .tbs_certificate()
            .extensions()
            .and_then(|extensions| {
                extensions
                    .iter()
                    .position(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
            })
            .unwrap_or(usize::MAX);
        evaluate_ordered_intermediate_name_constraints_policy(
            &target,
            subject_alt_name_extension_index,
            intermediates.iter(),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_ordered_subordinate_intermediates(
        intermediate_encoded: &[Vec<u8>],
        self_issued: &[bool],
    ) -> Result<(usize, usize, usize), X509RsaConstructedPathReviewedServerPolicyError> {
        assert_eq!(intermediate_encoded.len(), self_issued.len());
        let intermediates = intermediate_encoded
            .iter()
            .map(|encoded| decode_x509_certificate(encoded, Asn1Limits::default()).unwrap())
            .collect::<Vec<_>>();
        evaluate_ordered_subordinate_intermediate_name_constraints_policy(
            intermediates.len(),
            |index| &intermediates[index],
            |index| self_issued[index],
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_anchor_subordinate_intermediates(
        intermediate_encoded: &[Vec<u8>],
        self_issued: &[bool],
        name_constraints: Option<&[u8]>,
    ) -> Result<(usize, usize), X509RsaConstructedPathReviewedServerPolicyError> {
        assert_eq!(intermediate_encoded.len(), self_issued.len());
        let intermediates = intermediate_encoded
            .iter()
            .map(|encoded| decode_x509_certificate(encoded, Asn1Limits::default()).unwrap())
            .collect::<Vec<_>>();
        evaluate_selected_anchor_subordinate_intermediate_dns_policy(
            intermediates.len(),
            |index| &intermediates[index],
            |index| self_issued[index],
            name_constraints,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

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

    #[test]
    fn construction_derived_path_length_error_is_stable_and_payload_free() {
        let error = X509RsaConstructedPathReviewedIntermediatePathError::new(
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::PathLength(
                X509CertificatePathLengthErrorKind::MaximumExceeded,
            ),
        );

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedIntermediatePathErrorKind::PathLength(
                X509CertificatePathLengthErrorKind::MaximumExceeded,
            )
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA constructed-path reviewed intermediate path error PathLength(MaximumExceeded)"
        );
    }

    #[test]
    fn reviewed_anchor_constraints_apply_to_every_target_dns_name() {
        let encoded = certificate(&[subject_alt_name(&[
            dns_name("api.tr"),
            dns_name("*.service.tr"),
        ])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let constrained = evaluate(&encoded, Some(&constraints)).unwrap().unwrap();

        assert_eq!(constrained.count(), 2);
    }

    #[test]
    fn critical_intermediate_constraints_apply_to_target_dns_and_ip_names_together() {
        let target = certificate(&[subject_alt_name(&[
            dns_name("api.example.test"),
            ip_address(&[192, 0, 2, 9]),
        ])]);
        let permitted_ip = [192, 0, 2, 0, 255, 255, 255, 0];
        let intermediate = certificate(&[mixed_name_constraints(
            &[b".example.test"],
            &[&permitted_ip],
            &[],
            &[],
            true,
        )]);

        let constrained = evaluate_intermediate(&target, &intermediate)
            .unwrap()
            .unwrap();

        assert_eq!(constrained.0.count(), 1);
        assert_eq!(constrained.1.dns_count(), 1);
        assert_eq!(constrained.1.ip_count(), 1);
        assert_eq!(constrained.1.total_count(), 2);
    }

    #[test]
    fn ordered_intermediate_constraints_all_apply_to_target_dns_and_ip_names() {
        let target = certificate(&[subject_alt_name(&[
            dns_name("api.example.test"),
            ip_address(&[192, 0, 2, 9]),
        ])]);
        let broad_ip = [192, 0, 0, 0, 255, 255, 0, 0];
        let narrow_ip = [192, 0, 2, 0, 255, 255, 255, 0];
        let intermediates = [
            certificate(&[mixed_name_constraints(
                &[b".test"],
                &[&broad_ip],
                &[],
                &[],
                true,
            )]),
            certificate(&[mixed_name_constraints(
                &[b".example.test"],
                &[&narrow_ip],
                &[],
                &[],
                true,
            )]),
        ];

        let (constrained_intermediate_count, constrained_dns, constrained_names) =
            evaluate_ordered_intermediates(&target, &intermediates).unwrap();

        assert_eq!(constrained_intermediate_count, 2);
        assert_eq!(constrained_dns.unwrap().count(), 1);
        assert_eq!(constrained_names.unwrap().dns_count(), 1);
        assert_eq!(constrained_names.unwrap().ip_count(), 1);
    }

    #[test]
    fn later_ordered_intermediate_constraint_violation_fails_closed() {
        let target = certificate(&[subject_alt_name(&[dns_name("api.example.test")])]);
        let intermediates = [
            certificate(&[name_constraints(&[b".test"], &[], true)]),
            certificate(&[name_constraints(
                &[b".example.test"],
                &[b".example.test"],
                true,
            )]),
        ];

        let error = evaluate_ordered_intermediates(&target, &intermediates).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameExcluded,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("api.example.test"));
    }

    #[test]
    fn ordered_constraints_apply_to_target_nearer_non_self_issued_ca_sans() {
        let broad_ip = [192, 0, 0, 0, 255, 255, 0, 0];
        let narrow_ip = [192, 0, 2, 0, 255, 255, 255, 0];
        let intermediates = [
            certificate(&[mixed_name_constraints(
                &[b".test"],
                &[&broad_ip],
                &[],
                &[],
                true,
            )]),
            certificate(&[
                subject_alt_name(&[dns_name("ca.example.test"), ip_address(&[192, 0, 2, 1])]),
                mixed_name_constraints(&[b".example.test"], &[&narrow_ip], &[], &[], true),
            ]),
            certificate(&[subject_alt_name(&[
                dns_name("issuing.example.test"),
                ip_address(&[192, 0, 2, 2]),
            ])]),
        ];

        let (applications, dns_evaluations, ip_evaluations) =
            evaluate_ordered_subordinate_intermediates(&intermediates, &[false, false, false])
                .unwrap();

        assert_eq!(applications, 3);
        assert_eq!(dns_evaluations, 3);
        assert_eq!(ip_evaluations, 3);
    }

    #[test]
    fn self_issued_subordinate_ca_is_skipped_but_later_ca_is_constrained() {
        let intermediates = [
            certificate(&[name_constraints(&[b".test"], &[], true)]),
            certificate(&[subject_alt_name(&[dns_name("outside.invalid")])]),
            certificate(&[subject_alt_name(&[dns_name("issuing.example.test")])]),
        ];

        let (applications, dns_evaluations, ip_evaluations) =
            evaluate_ordered_subordinate_intermediates(&intermediates, &[false, true, false])
                .unwrap();

        assert_eq!(applications, 1);
        assert_eq!(dns_evaluations, 1);
        assert_eq!(ip_evaluations, 0);
    }

    #[test]
    fn absent_subordinate_ca_san_is_an_empty_supported_name_set() {
        let intermediates = [
            certificate(&[name_constraints(&[b".test"], &[], true)]),
            certificate(&[]),
        ];

        let (applications, dns_evaluations, ip_evaluations) =
            evaluate_ordered_subordinate_intermediates(&intermediates, &[false, false]).unwrap();

        assert_eq!(applications, 1);
        assert_eq!(dns_evaluations, 0);
        assert_eq!(ip_evaluations, 0);
    }

    #[test]
    fn subordinate_ca_constraint_violation_fails_closed_and_redacted() {
        let intermediates = [
            certificate(&[name_constraints(&[b".test"], &[], true)]),
            certificate(&[subject_alt_name(&[dns_name("secret.invalid")])]),
        ];

        let error = evaluate_ordered_subordinate_intermediates(&intermediates, &[false, false])
            .unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateIntermediateDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("secret.invalid"));
    }

    #[test]
    fn malformed_subordinate_ca_san_fails_closed_and_redacted() {
        let malformed_subject_alt_name = [0x30, 0x01, 0x82];
        let intermediates = [
            certificate(&[name_constraints(&[b".test"], &[], true)]),
            certificate(&[extension(
                SUBJECT_ALT_NAME_OID_DER,
                &malformed_subject_alt_name,
            )]),
        ];

        let error = evaluate_ordered_subordinate_intermediates(&intermediates, &[false, false])
            .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidSubordinateIntermediateSubjectAlternativeName(_)
        ));
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("30 01 82"));
    }

    #[test]
    fn reviewed_anchor_constraints_apply_to_non_self_issued_subordinate_ca_sans() {
        let intermediates = [
            certificate(&[subject_alt_name(&[dns_name("policy.example.test")])]),
            certificate(&[subject_alt_name(&[dns_name("issuing.example.test")])]),
        ];
        let constraints = permitted_dns_subtrees(&[b".test"]);

        let (applications, dns_evaluations) = evaluate_anchor_subordinate_intermediates(
            &intermediates,
            &[false, false],
            Some(&constraints),
        )
        .unwrap();

        assert_eq!(applications, 2);
        assert_eq!(dns_evaluations, 2);
    }

    #[test]
    fn reviewed_anchor_constraints_skip_self_issued_subordinate_ca() {
        let intermediates = [
            certificate(&[subject_alt_name(&[dns_name("outside.invalid")])]),
            certificate(&[subject_alt_name(&[dns_name("issuing.example.test")])]),
        ];
        let constraints = permitted_dns_subtrees(&[b".test"]);

        let (applications, dns_evaluations) = evaluate_anchor_subordinate_intermediates(
            &intermediates,
            &[true, false],
            Some(&constraints),
        )
        .unwrap();

        assert_eq!(applications, 1);
        assert_eq!(dns_evaluations, 1);
    }

    #[test]
    fn reviewed_anchor_constraints_treat_absent_subordinate_san_as_empty() {
        let intermediates = [certificate(&[])];
        let constraints = permitted_dns_subtrees(&[b".test"]);

        let (applications, dns_evaluations) =
            evaluate_anchor_subordinate_intermediates(&intermediates, &[false], Some(&constraints))
                .unwrap();

        assert_eq!(applications, 1);
        assert_eq!(dns_evaluations, 0);
    }

    #[test]
    fn unconstrained_reviewed_anchor_does_not_read_subordinate_sans() {
        let malformed_subject_alt_name = [0x30, 0x01, 0x82];
        let intermediates = [certificate(&[extension(
            SUBJECT_ALT_NAME_OID_DER,
            &malformed_subject_alt_name,
        )])];

        let counts =
            evaluate_anchor_subordinate_intermediates(&intermediates, &[false], None).unwrap();

        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn malformed_reviewed_anchor_subordinate_san_fails_closed_and_redacted() {
        let malformed_subject_alt_name = [0x30, 0x01, 0x82];
        let intermediates = [certificate(&[extension(
            SUBJECT_ALT_NAME_OID_DER,
            &malformed_subject_alt_name,
        )])];
        let constraints = permitted_dns_subtrees(&[b".test"]);

        let error =
            evaluate_anchor_subordinate_intermediates(&intermediates, &[false], Some(&constraints))
                .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidSubordinateIntermediateSubjectAlternativeName(_)
        ));
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("30 01 82"));
    }

    #[test]
    fn malformed_reviewed_anchor_subordinate_constraint_fails_closed_and_redacted() {
        let malformed_name_constraints = [0xa0, 0x01, 0x30];
        let intermediates = [certificate(&[])];

        let error = evaluate_anchor_subordinate_intermediates(
            &intermediates,
            &[false],
            Some(&malformed_name_constraints),
        )
        .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::InvalidAnchorNameConstraints(_)
        ));
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("a0 01 30"));
    }

    #[test]
    fn reviewed_anchor_subordinate_constraint_violation_fails_closed_and_redacted() {
        let intermediates = [certificate(&[subject_alt_name(&[dns_name(
            "secret.invalid",
        )])])];
        let constraints = permitted_dns_subtrees(&[b".test"]);

        let error =
            evaluate_anchor_subordinate_intermediates(&intermediates, &[false], Some(&constraints))
                .unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::SubordinateIntermediateAnchorDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("secret.invalid"));
    }

    #[test]
    fn reviewed_anchor_constraint_violation_is_redacted_and_stable() {
        let encoded = certificate(&[subject_alt_name(&[
            dns_name("api.tr"),
            dns_name("secret.internal"),
        ])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let error = evaluate(&encoded, Some(&constraints)).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("api.tr"));
        assert!(!diagnostic.contains("secret.internal"));
    }

    #[test]
    fn constrained_target_requires_subject_alt_name() {
        let encoded = certificate(&[]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let error = evaluate(&encoded, Some(&constraints)).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::MissingTargetSubjectAlternativeName
        );
    }

    #[test]
    fn critical_intermediate_constraints_apply_to_every_target_dns_name() {
        let target = certificate(&[subject_alt_name(&[
            dns_name("api.example.test"),
            dns_name("*.service.example.test"),
        ])]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], true)]);

        let constrained = evaluate_intermediate(&target, &intermediate)
            .unwrap()
            .unwrap();

        assert_eq!(constrained.0.count(), 2);
        assert_eq!(constrained.1.dns_count(), 2);
        assert_eq!(constrained.1.ip_count(), 0);
    }

    #[test]
    fn absent_intermediate_constraints_retain_no_dns_evidence() {
        let target = certificate(&[subject_alt_name(&[dns_name("api.example.test")])]);
        let intermediate = certificate(&[]);

        assert_eq!(evaluate_intermediate(&target, &intermediate).unwrap(), None);
    }

    #[test]
    fn intermediate_excluded_subtree_violation_is_redacted_and_stable() {
        let target = certificate(&[subject_alt_name(&[
            dns_name("api.example.test"),
            dns_name("secret.internal.example.test"),
        ])]);
        let intermediate = certificate(&[name_constraints(
            &[b".example.test"],
            &[b".internal.example.test"],
            true,
        )]);

        let error = evaluate_intermediate(&target, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameExcluded,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("api.example.test"));
        assert!(!diagnostic.contains("secret.internal.example.test"));
    }

    #[test]
    fn intermediate_ip_constraint_violation_is_redacted_and_stable() {
        let target = certificate(&[subject_alt_name(&[
            dns_name("api.example.test"),
            ip_address(&[198, 51, 100, 9]),
        ])]);
        let permitted_ip = [192, 0, 2, 0, 255, 255, 255, 0];
        let intermediate = certificate(&[mixed_name_constraints(
            &[b".example.test"],
            &[&permitted_ip],
            &[],
            &[],
            true,
        )]);

        let error = evaluate_intermediate(&target, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::TargetIntermediateIpNameConstraints(
                IpConstraintErrorKind::PresentedIpAddressNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("198.51.100.9"));
        assert!(!diagnostic.contains("api.example.test"));
    }

    #[test]
    fn present_intermediate_name_constraints_must_be_critical() {
        let target = certificate(&[subject_alt_name(&[dns_name("api.example.test")])]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], false)]);

        let error = evaluate_intermediate(&target, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaConstructedPathReviewedServerPolicyErrorKind::NonCriticalIntermediateNameConstraints
        );
    }
}
