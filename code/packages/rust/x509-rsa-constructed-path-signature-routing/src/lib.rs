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
use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate_path_construction::ConstructedX509CertificatePathAdjacentLink;
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthError, X509CertificatePathLengthErrorKind,
    X509CertificatePathLengthState,
};
use x509_certificate_path_reviewed_anchor_binding::InitializedReviewedConstructedX509CertificatePathLength;
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
use x509_subject_alt_name::{decode_subject_alt_name, SubjectAltNameErrorKind};
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

    let path_length_state = X509CertificatePathLengthState::new(
        initialized_path_length
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
    );
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(&intermediate_policy)
        .map_err(|error| {
            X509RsaConstructedPathInitializedIntermediatePathLengthError::new(
                X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::PathLength(
                    error.kind(),
                ),
            )
        })?;

    Ok(
        AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength {
            initialized_path_length,
            reviewed_terminal,
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
    let previous_terminal_link = previous_step
        .reviewed_terminal()
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
        previous_step.intermediate_policy().certificate(),
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

    let path_length_state = X509CertificatePathLengthState::new(
        previous_step
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
    );
    let path_length_state = path_length_state
        .advance_validated_intermediate_ca(&intermediate_policy)
        .map_err(|error| {
            X509RsaConstructedPathNextIntermediatePathLengthError::new(
                X509RsaConstructedPathNextIntermediatePathLengthErrorKind::PathLength(error.kind()),
            )
        })?;

    Ok(
        AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength {
            previous_step,
            next_signature,
            intermediate_policy,
            path_length_state,
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
