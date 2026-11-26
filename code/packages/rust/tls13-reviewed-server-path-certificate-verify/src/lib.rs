//! TLS 1.3 server `CertificateVerify` bound to a reviewed RSA path leaf.
//!
//! The path-only composition accepts selected-transcript hash evidence and the
//! strongest accumulated reviewed RSA server-path proof. Stronger HTTPS entry
//! points accept that path only through the existing exact DNS destination
//! binding, with or without retained anchor self-signature evidence. Direct,
//! nested, double-nested, recursive-responder, and alternative intermediate-
//! status-responder revocation entry points accept the leaf only through their
//! exact destination-bound complete fixed-path proofs. All entry points
//! authorize the server role, derive and validate the exact retained target
//! certificate's RSA key, and delegate signature verification to repository-
//! owned code. A stronger transition binds only the five complete revocation
//! variants to the no-PSK server-flight position that expects
//! `CertificateVerify`, then advances that state to expect `Finished`. A final
//! bounded transition admits one server-role `Finished` verdict for the same
//! selected suite and completes that exact flight. A separate consuming entry
//! point derives the RSA key only through the complete reviewed path and proves
//! exact transcript continuity from authenticated server `CertificateVerify`
//! through a caller-supplied Finished key. It checks that key's role and suite
//! but does not establish its key-schedule or traffic-secret provenance. It
//! can also join the reviewed destination/path verdict to the exact ordered
//! retry `CertificateVerify` callback only when both retain the same verdict
//! object. A sealed receipt now carries the preceding authenticated retry
//! Certificate leaf join across callback lifetimes and completes only against
//! the same reviewed-authentication object, single-use server record-state
//! issuer, decoded leaf, and signature verdict. That completed receipt can in
//! turn consume the exact authenticated ordered retry server `Finished` proof
//! only when both retain the same lifetime-bearing record owner. The stronger
//! join consumes the committed Finished record evidence, also retaining the
//! exact post-Finished transcript. Only that stronger result can derive
//! application traffic secrets from the retained handshake's Master Secret
//! and committed snapshot. This does not install keys, authenticate later
//! record epochs, or provide TLS, HTTP, OAuth, or network readiness.

#![forbid(unsafe_code)]

use core::fmt;

use der_asn1::Asn1Limits;
use https_reviewed_server_path_binding::{
    BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedDoubleNestedNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedNestedNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedRecursiveResponderNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedServerPath,
    BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature, ReviewedHttpsServerPath,
};
use tls13_application_traffic_secrets::{
    derive_application_traffic_secrets, ApplicationTrafficSecretError,
    ApplicationTrafficSecretStorage, ApplicationTrafficSecrets,
};
use tls13_certificate_verify_input::CertificateVerifyRole;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    verify_and_advance_owned_certificate_verify_transcript,
    verify_finished_after_owned_certificate_verify, verify_transcript_bound_certificate_verify,
    CertificateVerifyFinishedContinuityErrorKind, CertificateVerifyTranscriptAdvanceErrorKind,
    HashedTranscriptBoundCertificateVerifyInput, OwnedVerifiedCertificateVerifyFinishedContinuity,
    VerifiedTranscriptBoundCertificateVerify,
};
use tls13_cipher_suite_policy::SelectedCipherSuite;
use tls13_finished::{FinishedKey, FinishedRole, VerifiedFinished};
use tls13_handshake::Tls13HandshakeType;
use tls13_no_psk_server_flight::{NoPskServerFlight, ServerFlightError};
use tls13_retry_handshake_traffic_key_material_binding::{
    BoundRetryServerCertificateLeaf, CommittedOrderedRetryServerFinished,
    OrderedRetryServerFlightIdentity, VerifiedBoundRetryServerCertificateVerifySignature,
    VerifiedOrderedRetryServerFinished,
};
use tls13_retry_handshake_traffic_secrets_binding::ExactRetryBaseSecrets;
use tls13_rsa_pss_certificate_verify::Tls13RsaPssCertificateVerifyErrorKind;
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use tls13_signature_scheme_policy::AuthorizedSignatureScheme;
use tls13_transcript::SelectedTls13Transcript;
use x509_certificate::X509Certificate;
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

mod direct_revocation_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
    }
}

mod nested_revocation_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
    }
}

mod double_nested_revocation_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
    }
}

mod recursive_revocation_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
    }
}

mod alternative_revocation_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
    }
}

mod retry_reviewed_server_authentication_evidence_sealed {
    use x509_certificate::X509Certificate;

    pub trait Sealed<'encoded> {
        fn server_certificate(&self) -> &X509Certificate<'encoded>;
        fn certificate_verify_verdict_identity(&self) -> *const ();
    }
}

/// Repository-sealed HTTPS destination, reviewed path, and server-signature
/// evidence that may be joined to an authenticated retry Certificate leaf.
pub trait RetryReviewedServerAuthenticationEvidence<'encoded>:
    retry_reviewed_server_authentication_evidence_sealed::Sealed<'encoded>
{
}

mod ordered_retry_server_certificate_verify_evidence_sealed {
    use tls13_retry_handshake_traffic_key_material_binding::OrderedRetryServerFlightIdentity;

    pub trait Sealed<'issuer> {
        fn ordered_server_flight_identity(&self) -> OrderedRetryServerFlightIdentity<'issuer>;
        fn certificate_verify_verdict_identity(&self) -> *const ();
    }
}

/// Repository-sealed authenticated ordered retry `CertificateVerify` evidence.
pub trait OrderedRetryServerCertificateVerifyEvidence<'issuer>:
    ordered_retry_server_certificate_verify_evidence_sealed::Sealed<'issuer>
{
}

mod complete_revocation_certificate_verify_sealed {
    use tls13_cipher_suite_policy::SelectedCipherSuite;

    pub trait Sealed {
        fn selected_suite(&self) -> SelectedCipherSuite;
    }
}

impl<'server_name, 'host, 'target, 'target_encoded, Proof, IntermediateRevocation>
    direct_revocation_evidence_sealed::Sealed<'target_encoded>
    for BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.destination().certificate()
    }
}

impl<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    > nested_revocation_evidence_sealed::Sealed<'target_encoded>
    for BoundHttpsDnsDestinationToReviewedNestedNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.destination().certificate()
    }
}

impl<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
    > alternative_revocation_evidence_sealed::Sealed<'target_encoded>
    for BoundHttpsDnsDestinationToReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.destination().certificate()
    }
}

impl<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
    > recursive_revocation_evidence_sealed::Sealed<'target_encoded>
    for BoundHttpsDnsDestinationToReviewedRecursiveResponderNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.destination().certificate()
    }
}

impl<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        OriginalResponderRevocation,
        IntermediateRevocation,
        AddedResponderRevocation,
    > double_nested_revocation_evidence_sealed::Sealed<'target_encoded>
    for BoundHttpsDnsDestinationToReviewedDoubleNestedNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        OriginalResponderRevocation,
        IntermediateRevocation,
        AddedResponderRevocation,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.destination().certificate()
    }
}

/// Stable, payload-free failures from reviewed-leaf verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedServerPathCertificateVerifyErrorKind {
    /// The retained RFC 8446 signed input selected the client role.
    WrongEndpointRole,
    /// The reviewed leaf's exact SubjectPublicKeyInfo failed RSA policy.
    PublicKey(RsaPublicKeyErrorKind),
    /// The exact message failed repository-owned RSA-PSS verification.
    Verification(Tls13RsaPssCertificateVerifyErrorKind),
}

/// One redacted reviewed-leaf `CertificateVerify` failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ReviewedServerPathCertificateVerifyError {
    kind: ReviewedServerPathCertificateVerifyErrorKind,
}

impl ReviewedServerPathCertificateVerifyError {
    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> ReviewedServerPathCertificateVerifyErrorKind {
        self.kind
    }
}

impl fmt::Debug for ReviewedServerPathCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReviewedServerPathCertificateVerifyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for ReviewedServerPathCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reviewed server-path CertificateVerify error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReviewedServerPathCertificateVerifyError {}

/// Stable, payload-free failures from reviewed-path CertificateVerify-to-
/// Finished continuity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind {
    /// The reviewed target's exact SubjectPublicKeyInfo failed RSA policy.
    PublicKey(RsaPublicKeyErrorKind),
    /// Exact CertificateVerify authentication or transcript advancement failed.
    CertificateVerify(CertificateVerifyTranscriptAdvanceErrorKind),
    /// Exact post-CertificateVerify Finished authentication failed.
    Finished(CertificateVerifyFinishedContinuityErrorKind),
}

/// One redacted consuming reviewed-path continuity failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ReviewedServerPathFinishedKeyTranscriptContinuityError {
    kind: ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind,
}

impl ReviewedServerPathFinishedKeyTranscriptContinuityError {
    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind {
        self.kind
    }
}

impl fmt::Debug for ReviewedServerPathFinishedKeyTranscriptContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReviewedServerPathFinishedKeyTranscriptContinuityError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for ReviewedServerPathFinishedKeyTranscriptContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reviewed server-path CertificateVerify-to-Finished-key transcript continuity error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReviewedServerPathFinishedKeyTranscriptContinuityError {}

/// Non-copyable evidence retaining the complete reviewed path and exact
/// selected-transcript server signature verdict.
pub struct VerifiedReviewedServerPathCertificateVerify<
    'reviewed,
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
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
> {
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
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
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'reviewed,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
    VerifiedReviewedServerPathCertificateVerify<
        'reviewed,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
    /// Return the exact complete reviewed server-path proof.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &'reviewed ReviewedHttpsServerPath<
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
        self.reviewed_server_path
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl fmt::Debug
    for VerifiedReviewedServerPathCertificateVerify<
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
        formatter.write_str("VerifiedReviewedServerPathCertificateVerify")
    }
}

/// Non-copyable evidence retaining the complete reviewed path and exact
/// consuming CertificateVerify-to-Finished-key transcript continuity graph.
pub struct VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity<
    'reviewed,
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
    'body,
> {
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
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
    continuity: OwnedVerifiedCertificateVerifyFinishedContinuity<'body, 'target_encoded>,
}

impl<
        'reviewed,
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
        'body,
    >
    VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity<
        'reviewed,
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
        'body,
    >
{
    /// Return the exact complete reviewed server-path proof.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &'reviewed ReviewedHttpsServerPath<
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
        self.reviewed_server_path
    }

    /// Return the exact consuming CertificateVerify-to-Finished proof.
    #[must_use]
    pub const fn continuity(
        &self,
    ) -> &OwnedVerifiedCertificateVerifyFinishedContinuity<'body, 'target_encoded> {
        &self.continuity
    }
}

impl fmt::Debug
    for VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity<
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
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity")
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, complete
/// direct no-check fixed-path revocation proof, and selected-transcript server
/// signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    Evidence: ?Sized,
> {
    https_destination_complete_revocation: &'destination Evidence,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence: ?Sized,
    >
    VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >
{
    /// Return the exact destination and complete fixed-path revocation proof.
    #[must_use]
    pub const fn https_destination_complete_revocation(&self) -> &'destination Evidence {
        self.https_destination_complete_revocation
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl<Evidence: ?Sized> fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, complete
/// alternative intermediate-status-responder no-check fixed-path revocation
/// proof, and selected-transcript server signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    Evidence: ?Sized,
> {
    https_destination_complete_revocation: &'destination Evidence,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence: ?Sized,
    >
    VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >
{
    /// Return the exact destination and alternative complete revocation proof.
    #[must_use]
    pub const fn https_destination_complete_revocation(&self) -> &'destination Evidence {
        self.https_destination_complete_revocation
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl<Evidence: ?Sized> fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, complete
/// recursive-responder no-check fixed-path revocation proof, and selected-
/// transcript server signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    Evidence: ?Sized,
> {
    https_destination_complete_revocation: &'destination Evidence,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence: ?Sized,
    >
    VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >
{
    /// Return the exact destination and recursive complete revocation proof.
    #[must_use]
    pub const fn https_destination_complete_revocation(&self) -> &'destination Evidence {
        self.https_destination_complete_revocation
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl<Evidence: ?Sized> fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, complete
/// nested no-check fixed-path revocation proof, and selected-transcript server
/// signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    Evidence: ?Sized,
> {
    https_destination_complete_revocation: &'destination Evidence,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence: ?Sized,
    >
    VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >
{
    /// Return the exact destination and nested complete revocation proof.
    #[must_use]
    pub const fn https_destination_complete_revocation(&self) -> &'destination Evidence {
        self.https_destination_complete_revocation
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl<Evidence: ?Sized> fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, complete
/// double-nested no-check fixed-path revocation proof, and selected-transcript
/// server signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    Evidence: ?Sized,
> {
    https_destination_complete_revocation: &'destination Evidence,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence: ?Sized,
    >
    VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >
{
    /// Return the exact destination and double-nested complete revocation proof.
    #[must_use]
    pub const fn https_destination_complete_revocation(&self) -> &'destination Evidence {
        self.https_destination_complete_revocation
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl<Evidence: ?Sized> fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the exact HTTPS DNS destination, reviewed
/// path, anchor self-signature, and selected-transcript server signature
/// verdict.
pub struct VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify<
    'destination,
    'server_name,
    'host,
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
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
> {
    https_destination_reviewed_server_path_anchor_self_signature:
        &'destination BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
            'server_name,
            'host,
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
            'signature_certificate,
            'signature_encoded,
            'signature_issuer_certificate,
            'signature_issuer_encoded,
            'signature_anchor_certificate,
            'signature_anchor_encoded,
        >,
    verification: VerifiedReviewedServerPathCertificateVerify<
        'destination,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
}

impl<
        'destination,
        'server_name,
        'host,
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
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
    VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
    /// Return the exact destination, reviewed path, and anchor signature proof.
    #[must_use]
    pub const fn https_destination_reviewed_server_path_anchor_self_signature(
        &self,
    ) -> &'destination BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        'server_name,
        'host,
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
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    > {
        self.https_destination_reviewed_server_path_anchor_self_signature
    }

    /// Return the exact reviewed-path `CertificateVerify` verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedReviewedServerPathCertificateVerify<
        'destination,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    > {
        &self.verification
    }
}

impl fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify<
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
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify",
        )
    }
}

/// Non-copyable evidence retaining the complete exact HTTPS DNS destination,
/// reviewed path, and selected-transcript server signature verdict.
pub struct VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
    'destination,
    'server_name,
    'host,
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
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
> {
    https_destination_reviewed_server_path:
        &'destination BoundHttpsDnsDestinationToReviewedServerPath<
            'server_name,
            'host,
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
    verification: VerifiedReviewedServerPathCertificateVerify<
        'destination,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
}

impl<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
    VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
    /// Return the exact HTTPS DNS destination and reviewed path proof.
    #[must_use]
    pub const fn https_destination_reviewed_server_path(
        &self,
    ) -> &'destination BoundHttpsDnsDestinationToReviewedServerPath<
        'server_name,
        'host,
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
        self.https_destination_reviewed_server_path
    }

    /// Return the exact reviewed-path `CertificateVerify` verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedReviewedServerPathCertificateVerify<
        'destination,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    > {
        &self.verification
    }
}

impl fmt::Debug
    for VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
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
        formatter.write_str("VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify")
    }
}

impl<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    > retry_reviewed_server_authentication_evidence_sealed::Sealed<'target_encoded>
    for VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
    fn server_certificate(&self) -> &X509Certificate<'target_encoded> {
        self.https_destination_reviewed_server_path()
            .destination()
            .certificate()
    }

    fn certificate_verify_verdict_identity(&self) -> *const () {
        core::ptr::from_ref(self.verification().verification()).cast()
    }
}

impl<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    > RetryReviewedServerAuthenticationEvidence<'target_encoded>
    for VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
}

impl<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    > ordered_retry_server_certificate_verify_evidence_sealed::Sealed<'issuer>
    for VerifiedBoundRetryServerCertificateVerifySignature<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >
{
    fn ordered_server_flight_identity(&self) -> OrderedRetryServerFlightIdentity<'issuer> {
        self.binding().pending().ordered_server_flight_identity()
    }

    fn certificate_verify_verdict_identity(&self) -> *const () {
        core::ptr::from_ref(self.verification()).cast()
    }
}

impl<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    > OrderedRetryServerCertificateVerifyEvidence<'issuer>
    for VerifiedBoundRetryServerCertificateVerifySignature<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >
{
}

/// Stable payload-free failures while joining the authenticated retry leaf to
/// complete HTTPS destination, reviewed-path, and signature evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryReviewedServerAuthenticationBindingError {
    /// The evidence graphs retain different decoded leaf objects.
    LeafCertificateMismatch,
}

impl fmt::Display for RetryReviewedServerAuthenticationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("authenticated retry leaf and reviewed server authentication leaf differ")
    }
}

impl std::error::Error for RetryReviewedServerAuthenticationBindingError {}

/// Authenticated retry Certificate leaf joined to reviewed server authentication.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// complete ordered retry leaf proof beside sealed HTTPS destination, reviewed
/// path, path-derived key, and server-signature evidence after exact decoded-
/// leaf object identity succeeds.
pub struct BoundRetryServerCertificateLeafToReviewedServerAuthentication<
    'retry_leaf,
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'certificate,
    'encoded,
    'authentication,
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
> {
    retry_leaf: &'retry_leaf BoundRetryServerCertificateLeaf<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
    >,
    authentication: &'authentication Authentication,
}

impl<
        'retry_leaf,
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
        'authentication,
        Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
    >
    BoundRetryServerCertificateLeafToReviewedServerAuthentication<
        'retry_leaf,
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
        'authentication,
        Authentication,
    >
{
    /// Borrow the complete authenticated ordered retry leaf proof.
    #[must_use]
    pub const fn retry_leaf(
        &self,
    ) -> &'retry_leaf BoundRetryServerCertificateLeaf<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
    > {
        self.retry_leaf
    }

    /// Borrow the complete destination, reviewed-path, and signature evidence.
    #[must_use]
    pub const fn authentication(&self) -> &'authentication Authentication {
        self.authentication
    }
}

impl<'encoded, Authentication> fmt::Debug
    for BoundRetryServerCertificateLeafToReviewedServerAuthentication<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        'encoded,
        '_,
        Authentication,
    >
where
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerCertificateLeafToReviewedServerAuthentication")
            .field("retry_leaf", &"[REDACTED]")
            .field("authentication", &"[REDACTED]")
            .finish()
    }
}

/// Join an authenticated ordered retry Certificate leaf to sealed HTTPS DNS
/// destination, reviewed server-path, path-derived key, and signature evidence.
pub fn bind_retry_server_certificate_leaf_to_reviewed_server_authentication<
    'retry_leaf,
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'certificate,
    'encoded,
    'authentication,
    Authentication,
>(
    retry_leaf: &'retry_leaf BoundRetryServerCertificateLeaf<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
    >,
    authentication: &'authentication Authentication,
) -> Result<
    BoundRetryServerCertificateLeafToReviewedServerAuthentication<
        'retry_leaf,
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
        'authentication,
        Authentication,
    >,
    RetryReviewedServerAuthenticationBindingError,
>
where
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
{
    let authentication_leaf =
        retry_reviewed_server_authentication_evidence_sealed::Sealed::server_certificate(
            authentication,
        );
    if !core::ptr::eq(retry_leaf.leaf_certificate(), authentication_leaf) {
        return Err(RetryReviewedServerAuthenticationBindingError::LeafCertificateMismatch);
    }
    Ok(
        BoundRetryServerCertificateLeafToReviewedServerAuthentication {
            retry_leaf,
            authentication,
        },
    )
}

/// Stable payload-free failures while binding reviewed server authentication
/// to the exact ordered retry `CertificateVerify` callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedRetryReviewedServerAuthenticationBindingError {
    /// The complete evidence graphs retain different signature-verdict objects.
    CertificateVerifyVerdictMismatch,
}

impl fmt::Display for OrderedRetryReviewedServerAuthenticationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ordered retry CertificateVerify and reviewed server authentication verdicts differ",
        )
    }
}

impl std::error::Error for OrderedRetryReviewedServerAuthenticationBindingError {}

/// Reviewed server authentication joined to its ordered retry callback.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// complete HTTPS destination, reviewed path, path-derived key, and signature
/// graph beside the complete ordered-message proof only after exact retained-
/// verdict object identity succeeds. Value equality is not sufficient.
pub struct BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify<
    'authentication,
    'ordered,
    Authentication: ?Sized,
    Ordered: ?Sized,
> {
    authentication: &'authentication Authentication,
    ordered_certificate_verify: &'ordered Ordered,
}

impl<'authentication, 'ordered, Authentication: ?Sized, Ordered: ?Sized>
    BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify<
        'authentication,
        'ordered,
        Authentication,
        Ordered,
    >
{
    /// Borrow the complete destination, reviewed-path, and signature graph.
    #[must_use]
    pub const fn authentication(&self) -> &'authentication Authentication {
        self.authentication
    }

    /// Borrow the exact authenticated ordered retry signature proof.
    #[must_use]
    pub const fn ordered_certificate_verify(&self) -> &'ordered Ordered {
        self.ordered_certificate_verify
    }
}

impl<Authentication: ?Sized, Ordered: ?Sized> fmt::Debug
    for BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify<
        '_,
        '_,
        Authentication,
        Ordered,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify")
            .field("authentication", &"[REDACTED]")
            .field("ordered_certificate_verify", &"[REDACTED]")
            .finish()
    }
}

/// Bind reviewed destination/path authentication to the exact ordered retry
/// `CertificateVerify` proof that retained the same signature verdict object.
pub fn bind_reviewed_server_authentication_to_ordered_retry_certificate_verify<
    'authentication,
    'ordered,
    'issuer,
    'encoded,
    Authentication,
    Ordered,
>(
    authentication: &'authentication Authentication,
    ordered_certificate_verify: &'ordered Ordered,
) -> Result<
    BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify<
        'authentication,
        'ordered,
        Authentication,
        Ordered,
    >,
    OrderedRetryReviewedServerAuthenticationBindingError,
>
where
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
    Ordered: OrderedRetryServerCertificateVerifyEvidence<'issuer> + ?Sized,
{
    let authentication_verdict = retry_reviewed_server_authentication_evidence_sealed::Sealed::certificate_verify_verdict_identity(authentication);
    let ordered_verdict = ordered_retry_server_certificate_verify_evidence_sealed::Sealed::certificate_verify_verdict_identity(ordered_certificate_verify);
    if authentication_verdict != ordered_verdict {
        return Err(
            OrderedRetryReviewedServerAuthenticationBindingError::CertificateVerifyVerdictMismatch,
        );
    }

    Ok(
        BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify {
            authentication,
            ordered_certificate_verify,
        },
    )
}

/// Stable payload-free failures while completing ordered retry Certificate
/// continuity through the reviewed server-authentication graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedRetryReviewedServerCertificateContinuityError {
    /// The later callback retained a different reviewed-authentication object.
    AuthenticationMismatch,
    /// The later callback came from a different single-use record-state issuer.
    OrderedServerFlightMismatch,
    /// The reviewed authentication no longer retained the exact decoded leaf.
    LeafCertificateMismatch,
    /// The later callback retained a different signature-verdict object.
    CertificateVerifyVerdictMismatch,
}

impl fmt::Display for OrderedRetryReviewedServerCertificateContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AuthenticationMismatch => {
                "ordered retry Certificate and CertificateVerify retain different reviewed authentication"
            }
            Self::OrderedServerFlightMismatch => {
                "ordered retry Certificate and CertificateVerify came from different server record owners"
            }
            Self::LeafCertificateMismatch => {
                "ordered retry Certificate continuity retained a different decoded leaf"
            }
            Self::CertificateVerifyVerdictMismatch => {
                "ordered retry Certificate continuity retained a different CertificateVerify verdict"
            }
        })
    }
}

impl std::error::Error for OrderedRetryReviewedServerCertificateContinuityError {}

/// Sealed receipt for the authenticated ordered retry server Certificate leaf.
///
/// This value intentionally derives neither `Copy` nor `Clone`. Callback-
/// scoped message bytes cannot escape their higher-ranked handler, so this
/// receipt instead retains the complete long-lived reviewed-authentication
/// reference and opaque exact-object identities established by the complete
/// leaf binding.
///
/// The issuer lifetime cannot be detached or widened to `'static`; therefore
/// the original single-use issuer stays borrowed until this receipt is
/// completed or dropped.
///
/// ```compile_fail
/// use tls13_reviewed_server_path_certificate_verify::
///     PendingOrderedRetryReviewedServerCertificateContinuity;
///
/// fn detach_issuer<'authentication, 'issuer, Authentication: ?Sized>(
///     receipt: PendingOrderedRetryReviewedServerCertificateContinuity<
///         'authentication,
///         'issuer,
///         Authentication,
///     >,
/// ) -> PendingOrderedRetryReviewedServerCertificateContinuity<
///     'authentication,
///     'static,
///     Authentication,
/// > {
///     receipt
/// }
/// ```
pub struct PendingOrderedRetryReviewedServerCertificateContinuity<
    'authentication,
    'issuer,
    Authentication: ?Sized,
> {
    authentication: &'authentication Authentication,
    ordered_server_flight_identity: OrderedRetryServerFlightIdentity<'issuer>,
    leaf_certificate_identity: *const (),
    certificate_verify_verdict_identity: *const (),
}

impl<'authentication, 'issuer, Authentication: ?Sized>
    PendingOrderedRetryReviewedServerCertificateContinuity<'authentication, 'issuer, Authentication>
{
    /// Borrow the exact reviewed destination, path, key, and signature graph.
    #[must_use]
    pub const fn authentication(&self) -> &'authentication Authentication {
        self.authentication
    }
}

impl<Authentication: ?Sized> fmt::Debug
    for PendingOrderedRetryReviewedServerCertificateContinuity<'_, '_, Authentication>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingOrderedRetryReviewedServerCertificateContinuity")
            .field("authentication", &"[REDACTED]")
            .field("ordered_server_flight_identity", &"[REDACTED]")
            .field("leaf_certificate_identity", &"[REDACTED]")
            .field("certificate_verify_verdict_identity", &"[REDACTED]")
            .finish()
    }
}

/// Initialize long-lived ordered continuity from the exact authenticated retry
/// Certificate leaf and reviewed server-authentication join.
pub fn begin_ordered_retry_reviewed_server_certificate_continuity<
    'retry_leaf,
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'certificate,
    'encoded,
    'authentication,
    Authentication,
>(
    bound_leaf: &BoundRetryServerCertificateLeafToReviewedServerAuthentication<
        'retry_leaf,
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
        'authentication,
        Authentication,
    >,
) -> PendingOrderedRetryReviewedServerCertificateContinuity<'authentication, 'issuer, Authentication>
where
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
{
    let retry_leaf = bound_leaf.retry_leaf();
    PendingOrderedRetryReviewedServerCertificateContinuity {
        authentication: bound_leaf.authentication(),
        ordered_server_flight_identity: retry_leaf
            .server_certificate()
            .pending()
            .ordered_server_flight_identity(),
        leaf_certificate_identity: core::ptr::from_ref(retry_leaf.leaf_certificate()).cast(),
        certificate_verify_verdict_identity:
            retry_reviewed_server_authentication_evidence_sealed::Sealed::certificate_verify_verdict_identity(
                bound_leaf.authentication(),
            ),
    }
}

/// Completed ordered retry Certificate-to-CertificateVerify continuity.
///
/// The sealed receipt retains the reviewed server-authentication graph and the
/// exact identities established at both authenticated ordered callbacks. It
/// intentionally retains no callback-scoped message bytes.
pub struct CompletedOrderedRetryReviewedServerCertificateContinuity<
    'authentication,
    'issuer,
    Authentication: ?Sized,
> {
    certificate: PendingOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'issuer,
        Authentication,
    >,
}

impl<'authentication, 'issuer, Authentication: ?Sized>
    CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'issuer,
        Authentication,
    >
{
    /// Borrow the exact reviewed destination, path, key, and signature graph.
    #[must_use]
    pub const fn authentication(&self) -> &'authentication Authentication {
        self.certificate.authentication
    }
}

impl<Authentication: ?Sized> fmt::Debug
    for CompletedOrderedRetryReviewedServerCertificateContinuity<'_, '_, Authentication>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompletedOrderedRetryReviewedServerCertificateContinuity")
            .field("certificate", &"[REDACTED]")
            .field("certificate_verify", &"[REDACTED]")
            .finish()
    }
}

struct OrderedRetryReviewedServerCertificateContinuityIdentities<Identity> {
    authentication: *const (),
    ordered_server_flight: Identity,
    leaf_certificate: *const (),
    certificate_verify_verdict: *const (),
}

fn validate_ordered_retry_reviewed_server_certificate_continuity<
    CertificateIdentity,
    CertificateVerifyIdentity,
>(
    certificate: &OrderedRetryReviewedServerCertificateContinuityIdentities<CertificateIdentity>,
    certificate_verify: &OrderedRetryReviewedServerCertificateContinuityIdentities<
        CertificateVerifyIdentity,
    >,
) -> Result<(), OrderedRetryReviewedServerCertificateContinuityError>
where
    CertificateIdentity: PartialEq<CertificateVerifyIdentity>,
{
    if certificate.authentication != certificate_verify.authentication {
        return Err(OrderedRetryReviewedServerCertificateContinuityError::AuthenticationMismatch);
    }
    if certificate.ordered_server_flight != certificate_verify.ordered_server_flight {
        return Err(
            OrderedRetryReviewedServerCertificateContinuityError::OrderedServerFlightMismatch,
        );
    }
    if certificate.leaf_certificate != certificate_verify.leaf_certificate {
        return Err(OrderedRetryReviewedServerCertificateContinuityError::LeafCertificateMismatch);
    }
    if certificate.certificate_verify_verdict != certificate_verify.certificate_verify_verdict {
        return Err(
            OrderedRetryReviewedServerCertificateContinuityError::CertificateVerifyVerdictMismatch,
        );
    }
    Ok(())
}

/// Complete one long-lived authenticated ordered retry Certificate receipt
/// with the exact later ordered `CertificateVerify` callback binding.
pub fn complete_ordered_retry_reviewed_server_certificate_continuity<
    'authentication,
    'certificate_issuer,
    'ordered_authentication,
    'ordered,
    'certificate_verify_issuer,
    'encoded,
    Authentication,
    Ordered,
>(
    certificate: PendingOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    certificate_verify: &BoundReviewedServerAuthenticationToOrderedRetryCertificateVerify<
        'ordered_authentication,
        'ordered,
        Authentication,
        Ordered,
    >,
) -> Result<
    CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    OrderedRetryReviewedServerCertificateContinuityError,
>
where
    Authentication: RetryReviewedServerAuthenticationEvidence<'encoded> + ?Sized,
    Ordered: OrderedRetryServerCertificateVerifyEvidence<'certificate_verify_issuer> + ?Sized,
{
    let ordered_certificate_verify = certificate_verify.ordered_certificate_verify();
    let ordered_server_flight_identity =
        ordered_retry_server_certificate_verify_evidence_sealed::Sealed::ordered_server_flight_identity(
            ordered_certificate_verify,
        );
    let authentication_leaf =
        retry_reviewed_server_authentication_evidence_sealed::Sealed::server_certificate(
            certificate.authentication,
        );
    let ordered_verdict = ordered_retry_server_certificate_verify_evidence_sealed::Sealed::certificate_verify_verdict_identity(ordered_certificate_verify);
    validate_ordered_retry_reviewed_server_certificate_continuity(
        &OrderedRetryReviewedServerCertificateContinuityIdentities {
            authentication: core::ptr::from_ref(certificate.authentication).cast(),
            ordered_server_flight: certificate.ordered_server_flight_identity,
            leaf_certificate: certificate.leaf_certificate_identity,
            certificate_verify_verdict: certificate.certificate_verify_verdict_identity,
        },
        &OrderedRetryReviewedServerCertificateContinuityIdentities {
            authentication: core::ptr::from_ref(certificate_verify.authentication()).cast(),
            ordered_server_flight: ordered_server_flight_identity,
            leaf_certificate: core::ptr::from_ref(authentication_leaf).cast(),
            certificate_verify_verdict: ordered_verdict,
        },
    )?;

    Ok(CompletedOrderedRetryReviewedServerCertificateContinuity { certificate })
}

/// Stable payload-free failure while joining ordered reviewed authentication
/// continuity to the exact ordered server `Finished` proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedRetryReviewedServerFinishedContinuityError {
    /// The Finished callback came from a different single-use record owner.
    OrderedServerFlightMismatch,
}

impl fmt::Display for OrderedRetryReviewedServerFinishedContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ordered retry Certificate continuity and Finished came from different server record owners",
        )
    }
}

impl std::error::Error for OrderedRetryReviewedServerFinishedContinuityError {}

fn validate_ordered_retry_reviewed_server_finished_continuity<
    CertificateVerifyIdentity,
    FinishedIdentity,
>(
    certificate_verify: CertificateVerifyIdentity,
    finished: FinishedIdentity,
) -> Result<(), OrderedRetryReviewedServerFinishedContinuityError>
where
    CertificateVerifyIdentity: PartialEq<FinishedIdentity>,
{
    if certificate_verify != finished {
        return Err(OrderedRetryReviewedServerFinishedContinuityError::OrderedServerFlightMismatch);
    }
    Ok(())
}

/// Complete ordered retry server authentication continuity through `Finished`.
///
/// This value owns both non-copyable inputs after their lifetime-bearing
/// single-use record-owner identities match. It retains the complete reviewed
/// destination, path-derived key, CertificateVerify verdict, retry-derived
/// Finished-key proof, pre-Finished snapshot, and exact peer verify-data
/// evidence without retaining any callback-scoped message borrow.
pub struct CompletedOrderedRetryReviewedServerFinishedContinuity<
    'authentication,
    'certificate_issuer,
    'finished_proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'finished_issuer,
    Authentication: ?Sized,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    certificate_verify: CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    finished: VerifiedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    >,
}

impl<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication: ?Sized,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    CompletedOrderedRetryReviewedServerFinishedContinuity<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication,
        Evidence,
    >
{
    /// Borrow the completed Certificate-to-CertificateVerify continuity proof.
    #[must_use]
    pub const fn certificate_verify(
        &self,
    ) -> &CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    > {
        &self.certificate_verify
    }

    /// Borrow the exact ordered retry server Finished proof.
    #[must_use]
    pub const fn finished(
        &self,
    ) -> &VerifiedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    > {
        &self.finished
    }
}

impl<Authentication: ?Sized, Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for CompletedOrderedRetryReviewedServerFinishedContinuity<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Authentication,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompletedOrderedRetryReviewedServerFinishedContinuity")
            .field("certificate_verify", &"[REDACTED]")
            .field("finished", &"[REDACTED]")
            .finish()
    }
}

/// Join the complete ordered Certificate-to-CertificateVerify receipt to the
/// exact ordered retry server `Finished` proof from the same record owner.
pub fn complete_ordered_retry_reviewed_server_finished_continuity<
    'authentication,
    'certificate_issuer,
    'finished_proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'finished_issuer,
    Authentication: ?Sized,
    Evidence,
>(
    certificate_verify: CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    finished: VerifiedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    >,
) -> Result<
    CompletedOrderedRetryReviewedServerFinishedContinuity<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication,
        Evidence,
    >,
    OrderedRetryReviewedServerFinishedContinuityError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    validate_ordered_retry_reviewed_server_finished_continuity(
        certificate_verify
            .certificate
            .ordered_server_flight_identity,
        finished.ordered_server_flight_identity(),
    )?;

    Ok(CompletedOrderedRetryReviewedServerFinishedContinuity {
        certificate_verify,
        finished,
    })
}

/// Reviewed ordered retry server authentication through the committed Finished.
///
/// The two owned proofs retain the reviewed Certificate-to-CertificateVerify
/// chain and the exact authenticated Finished, its record-boundary commitment,
/// and its post-message transcript snapshot. This is not authorization to
/// install keys, send application data, or use HTTPS.
pub struct CompletedOrderedRetryReviewedCommittedServerFinishedContinuity<
    'authentication,
    'certificate_issuer,
    'finished_proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'finished_issuer,
    Authentication: ?Sized,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    certificate_verify: CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    committed_finished: CommittedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    >,
}

impl<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication: ?Sized,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    CompletedOrderedRetryReviewedCommittedServerFinishedContinuity<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication,
        Evidence,
    >
{
    /// Borrow the reviewed certificate continuity receipt and its exact authentication.
    #[must_use]
    pub const fn certificate_verify(
        &self,
    ) -> &CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    > {
        &self.certificate_verify
    }

    /// Borrow the exact committed Finished proof, including the post-Finished snapshot.
    #[must_use]
    pub const fn committed_finished(
        &self,
    ) -> &CommittedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    > {
        &self.committed_finished
    }

    /// Consume the reviewed Finished proof to derive both initial application secrets.
    ///
    /// The Finished-key proof retains the exact retry base schedule, and the
    /// committed receipt retains the snapshot through the same server Finished.
    /// No caller can replace either input with a value from another session.
    /// Storage policy must match (including the record ceiling) before invoking
    /// the primitive, which stages both directions before publishing either.
    ///
    /// ```compile_fail,E0382
    /// use tls13_application_traffic_secrets::ApplicationTrafficSecretStorage;
    /// use tls13_retry_handshake_traffic_secrets_binding::ExactRetryBaseSecrets;
    /// use tls13_reviewed_server_path_certificate_verify::
    ///     CompletedOrderedRetryReviewedCommittedServerFinishedContinuity;
    ///
    /// fn derive_twice<A: ?Sized + 'static, E: ExactRetryBaseSecrets + ?Sized + 'static>(
    ///     proof: CompletedOrderedRetryReviewedCommittedServerFinishedContinuity<
    ///         'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, A, E,
    ///     >,
    ///     first: &mut ApplicationTrafficSecretStorage,
    ///     second: &mut ApplicationTrafficSecretStorage,
    /// ) {
    ///     let _ = proof.derive_application_traffic_secrets(first);
    ///     let _ = proof.derive_application_traffic_secrets(second);
    /// }
    /// ```
    pub fn derive_application_traffic_secrets<'storage>(
        self,
        storage: &'storage mut ApplicationTrafficSecretStorage,
    ) -> Result<
        BoundReviewedRetryApplicationTrafficSecrets<'storage, Self>,
        ReviewedRetryApplicationTrafficSecretsError,
    > {
        let retry = self
            .committed_finished
            .finished()
            .server_finished_key()
            .handshake_traffic_secrets()
            .evidence();
        if retry.selected_suite() != storage.selected_suite() {
            return Err(ReviewedRetryApplicationTrafficSecretsError::CipherSuiteMismatch);
        }
        let traffic_secrets = derive_application_traffic_secrets(
            retry.base_secrets(),
            &self.committed_finished.transcript_snapshot(),
            storage,
        )
        .map_err(ReviewedRetryApplicationTrafficSecretsError::Derivation)?;

        Ok(BoundReviewedRetryApplicationTrafficSecrets {
            continuity: self,
            traffic_secrets,
        })
    }
}

/// Stable failure categories for reviewed retry application-secret derivation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedRetryApplicationTrafficSecretsError {
    /// Storage uses a different suite or record ceiling than the reviewed retry.
    CipherSuiteMismatch,
    /// The repository-owned derivation failed without publishing either secret.
    Derivation(ApplicationTrafficSecretError),
}

impl fmt::Display for ReviewedRetryApplicationTrafficSecretsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CipherSuiteMismatch => formatter.write_str(
                "reviewed retry and application traffic storage use different suite policies",
            ),
            Self::Derivation(error) => {
                write!(formatter, "reviewed retry application traffic: {error}")
            }
        }
    }
}

impl std::error::Error for ReviewedRetryApplicationTrafficSecretsError {}

/// Initial application traffic secrets plus the complete reviewed handshake proof.
///
/// Only the consuming method on the reviewed committed Finished proof can
/// construct this result. The proof type remains attached to caller-owned,
/// wipe-on-drop secret storage. No record key or epoch is installed here.
pub struct BoundReviewedRetryApplicationTrafficSecrets<'storage, Continuity> {
    continuity: Continuity,
    traffic_secrets: ApplicationTrafficSecrets<'storage>,
}

impl<'storage, Continuity> BoundReviewedRetryApplicationTrafficSecrets<'storage, Continuity> {
    /// Borrow the complete reviewed committed Finished continuity proof.
    #[must_use]
    pub const fn continuity(&self) -> &Continuity {
        &self.continuity
    }

    /// Borrow the initial client and server application traffic secret view.
    #[must_use]
    pub const fn traffic_secrets(&self) -> &ApplicationTrafficSecrets<'storage> {
        &self.traffic_secrets
    }
}

impl<Continuity> fmt::Debug for BoundReviewedRetryApplicationTrafficSecrets<'_, Continuity> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundReviewedRetryApplicationTrafficSecrets([REDACTED])")
    }
}

impl<Authentication: ?Sized, Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for CompletedOrderedRetryReviewedCommittedServerFinishedContinuity<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        Authentication,
        Evidence,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("CompletedOrderedRetryReviewedCommittedServerFinishedContinuity([REDACTED])")
    }
}

/// Consume reviewed certificate continuity and the committed Finished from the same issuer.
///
/// A callback-only verdict cannot supply the committed record boundary. A
/// different record owner fails closed without returning either input, even
/// when its transcript, Finished bytes, and key material are identical.
///
/// ```compile_fail,E0308
/// use tls13_retry_handshake_traffic_key_material_binding::VerifiedOrderedRetryServerFinished;
/// use tls13_retry_handshake_traffic_secrets_binding::ExactRetryBaseSecrets;
/// use tls13_reviewed_server_path_certificate_verify::{
///     complete_ordered_retry_reviewed_committed_server_finished_continuity,
///     CompletedOrderedRetryReviewedServerCertificateContinuity,
/// };
///
/// fn callback_only<A: ?Sized + 'static, E: ExactRetryBaseSecrets + ?Sized + 'static>(
///     certificate: CompletedOrderedRetryReviewedServerCertificateContinuity<'static, 'static, A>,
///     finished: VerifiedOrderedRetryServerFinished<
///         'static, 'static, 'static, 'static, 'static, 'static, E,
///     >,
/// ) {
///     let _ = complete_ordered_retry_reviewed_committed_server_finished_continuity(
///         certificate, finished,
///     );
/// }
/// ```
pub fn complete_ordered_retry_reviewed_committed_server_finished_continuity<
    'authentication,
    'certificate_issuer,
    'finished_proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'finished_issuer,
    Authentication: ?Sized,
    Evidence: ExactRetryBaseSecrets + ?Sized,
>(
    certificate_verify: CompletedOrderedRetryReviewedServerCertificateContinuity<
        'authentication,
        'certificate_issuer,
        Authentication,
    >,
    committed_finished: CommittedOrderedRetryServerFinished<
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Evidence,
    >,
) -> Result<
    CompletedOrderedRetryReviewedCommittedServerFinishedContinuity<
        'authentication,
        'certificate_issuer,
        'finished_proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'finished_issuer,
        Authentication,
        Evidence,
    >,
    OrderedRetryReviewedServerFinishedContinuityError,
> {
    validate_ordered_retry_reviewed_server_finished_continuity(
        certificate_verify
            .certificate
            .ordered_server_flight_identity,
        committed_finished
            .finished()
            .ordered_server_flight_identity(),
    )?;

    Ok(
        CompletedOrderedRetryReviewedCommittedServerFinishedContinuity {
            certificate_verify,
            committed_finished,
        },
    )
}

/// Verify exact server CertificateVerify-to-Finished-key transcript continuity
/// with the target key derived only through the complete reviewed path.
///
/// The selected transcript is consumed. Every failure returns only a stable
/// redacted category and no transcript or partial verification evidence.
/// The caller-supplied Finished key is checked for server role and exact suite,
/// but this boundary does not establish its key-schedule or traffic-secret
/// provenance and does not retain that provenance in its result.
#[allow(clippy::too_many_arguments)]
pub fn verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity<
    'reviewed,
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
    'body,
    'key_policy,
>(
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
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
    transcript: SelectedTls13Transcript,
    authorized_scheme: AuthorizedSignatureScheme,
    certificate_verify_body: &'body [u8],
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    signed_input_output: &mut [u8],
    handshake_output: &mut [u8],
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
    finished_key: &FinishedKey<'_>,
    peer_verify_data: &[u8],
) -> Result<
    VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity<
        'reviewed,
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
        'body,
    >,
    ReviewedServerPathFinishedKeyTranscriptContinuityError,
> {
    let certificate = reviewed_server_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .server_policy()
        .certificate();
    let public_key = validate_rsa_public_key(
        certificate.tbs_certificate().subject_public_key_info(),
        rsa_key_policy,
        rsa_key_limits,
    )
    .map_err(
        |error| ReviewedServerPathFinishedKeyTranscriptContinuityError {
            kind: ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind::PublicKey(
                error.kind(),
            ),
        },
    )?;
    let certificate_verify = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized_scheme,
        certificate_verify_body,
        public_key,
        signed_input_output,
        handshake_output,
        data_block_scratch,
        signature_limits,
    )
    .map_err(
        |error| ReviewedServerPathFinishedKeyTranscriptContinuityError {
            kind: ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind::CertificateVerify(
                error.kind(),
            ),
        },
    )?;
    let continuity = verify_finished_after_owned_certificate_verify(
        certificate_verify,
        finished_key,
        peer_verify_data,
    )
    .map_err(
        |error| ReviewedServerPathFinishedKeyTranscriptContinuityError {
            kind: ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind::Finished(
                error.kind(),
            ),
        },
    )?;

    Ok(
        VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity {
            reviewed_server_path,
            continuity,
        },
    )
}

/// Verify one server `CertificateVerify` with the exact reviewed path target.
#[allow(clippy::too_many_arguments)]
pub fn verify_reviewed_server_path_certificate_verify<
    'reviewed,
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
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
>(
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
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
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedReviewedServerPathCertificateVerify<
        'reviewed,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
    ReviewedServerPathCertificateVerifyError,
> {
    require_server_role(hashed_input.bound_input().input().role())?;

    let certificate = reviewed_server_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .server_policy()
        .certificate();
    let verification = verify_server_certificate_verify_after_role(
        certificate,
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(VerifiedReviewedServerPathCertificateVerify {
        reviewed_server_path,
        verification,
    })
}

/// Verify one server `CertificateVerify` through the exact reviewed path
/// already bound to the HTTPS DNS destination.
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_server_path_certificate_verify<
    'destination,
    'server_name,
    'host,
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
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
>(
    https_destination_reviewed_server_path: &'destination BoundHttpsDnsDestinationToReviewedServerPath<
        'server_name,
        'host,
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
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
    ReviewedServerPathCertificateVerifyError,
> {
    let verification = verify_reviewed_server_path_certificate_verify(
        https_destination_reviewed_server_path.reviewed_server_path(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify {
            https_destination_reviewed_server_path,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination, reviewed path, and retained anchor self-signature proof.
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_server_path_anchor_self_signature_certificate_verify<
    'destination,
    'server_name,
    'host,
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
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
>(
    https_destination_reviewed_server_path_anchor_self_signature: &'destination BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        'server_name,
        'host,
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
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify<
        'destination,
        'server_name,
        'host,
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
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
    ReviewedServerPathCertificateVerifyError,
> {
    let verification = verify_reviewed_server_path_certificate_verify(
        https_destination_reviewed_server_path_anchor_self_signature
            .reviewed_server_path_anchor_self_signature()
            .reviewed_server_path(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify {
            https_destination_reviewed_server_path_anchor_self_signature,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination and complete direct no-check fixed-path revocation proof.
#[allow(private_bounds)]
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_direct_no_check_complete_fixed_path_revocation_certificate_verify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
    Evidence,
>(
    https_destination_complete_revocation: &'destination Evidence,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >,
    ReviewedServerPathCertificateVerifyError,
>
where
    Evidence: ?Sized + direct_revocation_evidence_sealed::Sealed<'target_encoded>,
{
    require_server_role(hashed_input.bound_input().input().role())?;

    let verification = verify_server_certificate_verify_after_role(
        https_destination_complete_revocation.server_certificate(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify {
            https_destination_complete_revocation,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination and complete nested no-check fixed-path revocation proof.
#[allow(private_bounds)]
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_nested_no_check_complete_fixed_path_revocation_certificate_verify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
    Evidence,
>(
    https_destination_complete_revocation: &'destination Evidence,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >,
    ReviewedServerPathCertificateVerifyError,
>
where
    Evidence: ?Sized + nested_revocation_evidence_sealed::Sealed<'target_encoded>,
{
    require_server_role(hashed_input.bound_input().input().role())?;

    let verification = verify_server_certificate_verify_after_role(
        https_destination_complete_revocation.server_certificate(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify {
            https_destination_complete_revocation,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination and complete double-nested no-check fixed-path revocation proof.
#[allow(private_bounds)]
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_double_nested_no_check_complete_fixed_path_revocation_certificate_verify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
    Evidence,
>(
    https_destination_complete_revocation: &'destination Evidence,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >,
    ReviewedServerPathCertificateVerifyError,
>
where
    Evidence: ?Sized + double_nested_revocation_evidence_sealed::Sealed<'target_encoded>,
{
    require_server_role(hashed_input.bound_input().input().role())?;

    let verification = verify_server_certificate_verify_after_role(
        https_destination_complete_revocation.server_certificate(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify {
            https_destination_complete_revocation,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination and complete recursive-responder no-check fixed-path revocation
/// proof.
#[allow(private_bounds)]
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_recursive_responder_no_check_complete_fixed_path_revocation_certificate_verify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
    Evidence,
>(
    https_destination_complete_revocation: &'destination Evidence,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >,
    ReviewedServerPathCertificateVerifyError,
>
where
    Evidence: ?Sized + recursive_revocation_evidence_sealed::Sealed<'target_encoded>,
{
    require_server_role(hashed_input.bound_input().input().role())?;

    let verification = verify_server_certificate_verify_after_role(
        https_destination_complete_revocation.server_certificate(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify {
            https_destination_complete_revocation,
            verification,
        },
    )
}

/// Verify one server `CertificateVerify` through the exact HTTPS DNS
/// destination and complete alternative intermediate-status-responder
/// no-check fixed-path revocation proof.
#[allow(private_bounds)]
#[allow(clippy::too_many_arguments)]
pub fn verify_https_dns_destination_reviewed_intermediate_status_responder_revocation_responder_no_check_complete_fixed_path_revocation_certificate_verify<
    'destination,
    'target_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
    Evidence,
>(
    https_destination_complete_revocation: &'destination Evidence,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
        'destination,
        'target_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        Evidence,
    >,
    ReviewedServerPathCertificateVerifyError,
>
where
    Evidence: ?Sized + alternative_revocation_evidence_sealed::Sealed<'target_encoded>,
{
    require_server_role(hashed_input.bound_input().input().role())?;

    let verification = verify_server_certificate_verify_after_role(
        https_destination_complete_revocation.server_certificate(),
        hashed_input,
        message,
        rsa_key_policy,
        rsa_key_limits,
        data_block_scratch,
        signature_limits,
    )?;

    Ok(
        VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify {
            https_destination_complete_revocation,
            verification,
        },
    )
}

macro_rules! impl_complete_revocation_certificate_verify_sealed {
    ($verification:ident) => {
        impl<
                'destination,
                'target_encoded,
                'hashed,
                'bound,
                'transcript,
                'output,
                'message,
                'body,
                Evidence: ?Sized,
            > complete_revocation_certificate_verify_sealed::Sealed
            for $verification<
                'destination,
                'target_encoded,
                'hashed,
                'bound,
                'transcript,
                'output,
                'message,
                'body,
                Evidence,
            >
        {
            fn selected_suite(&self) -> SelectedCipherSuite {
                self.verification()
                    .hashed_input()
                    .bound_input()
                    .transcript()
                    .selected_suite()
            }
        }
    };
}

impl_complete_revocation_certificate_verify_sealed!(
    VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify
);
impl_complete_revocation_certificate_verify_sealed!(
    VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify
);
impl_complete_revocation_certificate_verify_sealed!(
    VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify
);
impl_complete_revocation_certificate_verify_sealed!(
    VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify
);
impl_complete_revocation_certificate_verify_sealed!(
    VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify
);

/// Non-copyable evidence that an exact complete-revocation server
/// `CertificateVerify` verdict advanced the no-PSK server flight to Finished.
pub struct OrderedNoPskServerFlightReviewedRevocationCertificateVerify<
    'flight,
    'verification,
    Verification: ?Sized,
> {
    server_flight: &'flight mut NoPskServerFlight,
    verification: &'verification Verification,
}

impl<'flight, 'verification, Verification: ?Sized>
    OrderedNoPskServerFlightReviewedRevocationCertificateVerify<
        'flight,
        'verification,
        Verification,
    >
{
    /// Return the server flight now positioned to accept `Finished`.
    #[must_use]
    pub const fn server_flight(&self) -> &NoPskServerFlight {
        self.server_flight
    }

    /// Return the exact complete-revocation `CertificateVerify` verdict.
    #[must_use]
    pub const fn verification(&self) -> &'verification Verification {
        self.verification
    }
}

impl<Verification: ?Sized> fmt::Debug
    for OrderedNoPskServerFlightReviewedRevocationCertificateVerify<'_, '_, Verification>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OrderedNoPskServerFlightReviewedRevocationCertificateVerify")
    }
}

/// Advance an exact no-PSK server flight only with one of the five complete-
/// revocation HTTPS destination-bound server `CertificateVerify` verdicts.
///
/// An unexpected state fails through the ordering policy before mutation. On
/// success the non-copyable result retains both the mutable flight and the
/// exact verification proof so a later Finished composition can consume the
/// transition without detaching the authenticated certificate and revocation
/// graph.
#[allow(private_bounds)]
pub fn advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify<
    'flight,
    'verification,
    Verification: ?Sized + complete_revocation_certificate_verify_sealed::Sealed,
>(
    server_flight: &'flight mut NoPskServerFlight,
    verification: &'verification Verification,
) -> Result<
    OrderedNoPskServerFlightReviewedRevocationCertificateVerify<
        'flight,
        'verification,
        Verification,
    >,
    ServerFlightError,
> {
    server_flight.accept(Tls13HandshakeType::CertificateVerify)?;

    Ok(
        OrderedNoPskServerFlightReviewedRevocationCertificateVerify {
            server_flight,
            verification,
        },
    )
}

/// Stable, payload-free failures while completing the reviewed server flight.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReviewedServerFinishedTransitionErrorKind {
    /// The Finished verdict authenticated the client sender rather than server.
    WrongEndpointRole,
    /// The Finished verdict selected a different suite than CertificateVerify.
    CipherSuiteMismatch,
    /// The retained server flight did not accept Finished at its current state.
    UnexpectedFlightState,
}

/// One reviewed server-Finished transition failure with no retained peer data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReviewedServerFinishedTransitionError {
    kind: ReviewedServerFinishedTransitionErrorKind,
}

impl ReviewedServerFinishedTransitionError {
    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> ReviewedServerFinishedTransitionErrorKind {
        self.kind
    }
}

impl fmt::Display for ReviewedServerFinishedTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reviewed TLS 1.3 server Finished transition error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReviewedServerFinishedTransitionError {}

/// Non-copyable evidence that exact reviewed CertificateVerify and server
/// Finished verdicts completed one no-PSK server flight under the same suite.
pub struct CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished<
    'flight,
    'verification,
    'finished,
    Verification: ?Sized,
> {
    server_flight: &'flight mut NoPskServerFlight,
    verification: &'verification Verification,
    finished: &'finished VerifiedFinished,
}

impl<'verification, 'finished, Verification: ?Sized>
    CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished<
        '_,
        'verification,
        'finished,
        Verification,
    >
{
    /// Return the completed exact server flight.
    #[must_use]
    pub const fn server_flight(&self) -> &NoPskServerFlight {
        self.server_flight
    }

    /// Return the exact complete-revocation CertificateVerify verdict.
    #[must_use]
    pub const fn verification(&self) -> &'verification Verification {
        self.verification
    }

    /// Return the exact server Finished verdict.
    #[must_use]
    pub const fn finished(&self) -> &'finished VerifiedFinished {
        self.finished
    }
}

impl<Verification: ?Sized> fmt::Debug
    for CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished<
        '_,
        '_,
        '_,
        Verification,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished")
    }
}

/// Complete the exact retained no-PSK server flight only with a server-role
/// Finished verdict for the same cipher suite as retained CertificateVerify.
///
/// Role and suite checks happen before the ordering policy is invoked. The
/// result retains both cryptographic verdicts and the completed flight. This
/// bounded transition does not establish that Finished's retained transcript
/// hash follows the exact CertificateVerify message in one encoded stream.
#[allow(private_bounds)]
pub fn complete_no_psk_server_flight_after_reviewed_revocation_finished<
    'flight,
    'verification,
    'finished,
    Verification: ?Sized + complete_revocation_certificate_verify_sealed::Sealed,
>(
    ordered: OrderedNoPskServerFlightReviewedRevocationCertificateVerify<
        'flight,
        'verification,
        Verification,
    >,
    finished: &'finished VerifiedFinished,
) -> Result<
    CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished<
        'flight,
        'verification,
        'finished,
        Verification,
    >,
    ReviewedServerFinishedTransitionError,
> {
    if finished.role() != FinishedRole::Server {
        return Err(ReviewedServerFinishedTransitionError {
            kind: ReviewedServerFinishedTransitionErrorKind::WrongEndpointRole,
        });
    }
    if finished.selected_suite() != ordered.verification.selected_suite() {
        return Err(ReviewedServerFinishedTransitionError {
            kind: ReviewedServerFinishedTransitionErrorKind::CipherSuiteMismatch,
        });
    }
    ordered
        .server_flight
        .accept(Tls13HandshakeType::Finished)
        .map_err(|_| ReviewedServerFinishedTransitionError {
            kind: ReviewedServerFinishedTransitionErrorKind::UnexpectedFlightState,
        })?;

    Ok(
        CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished {
            server_flight: ordered.server_flight,
            verification: ordered.verification,
            finished,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn verify_server_certificate_verify_after_role<
    'certificate,
    'encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
>(
    certificate: &'certificate X509Certificate<'encoded>,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'encoded,
    >,
    ReviewedServerPathCertificateVerifyError,
> {
    let public_key = validate_leaf_public_key(certificate, rsa_key_policy, rsa_key_limits)?;

    verify_transcript_bound_certificate_verify(
        hashed_input,
        message,
        public_key,
        data_block_scratch,
        signature_limits,
    )
    .map_err(|error| ReviewedServerPathCertificateVerifyError {
        kind: ReviewedServerPathCertificateVerifyErrorKind::Verification(error.kind()),
    })
}

fn validate_leaf_public_key<'encoded, 'key_policy>(
    certificate: &X509Certificate<'encoded>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
) -> Result<ValidatedRsaPublicKey<'encoded>, ReviewedServerPathCertificateVerifyError> {
    validate_rsa_public_key(
        certificate.tbs_certificate().subject_public_key_info(),
        rsa_key_policy,
        rsa_key_limits,
    )
    .map_err(|error| ReviewedServerPathCertificateVerifyError {
        kind: ReviewedServerPathCertificateVerifyErrorKind::PublicKey(error.kind()),
    })
}

fn require_server_role(
    role: CertificateVerifyRole,
) -> Result<(), ReviewedServerPathCertificateVerifyError> {
    if role == CertificateVerifyRole::Server {
        Ok(())
    } else {
        Err(ReviewedServerPathCertificateVerifyError {
            kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify,
        alternative_revocation_evidence_sealed,
        begin_ordered_retry_reviewed_server_certificate_continuity,
        bind_retry_server_certificate_leaf_to_reviewed_server_authentication,
        bind_reviewed_server_authentication_to_ordered_retry_certificate_verify,
        complete_no_psk_server_flight_after_reviewed_revocation_finished,
        complete_ordered_retry_reviewed_committed_server_finished_continuity,
        complete_ordered_retry_reviewed_server_certificate_continuity,
        complete_ordered_retry_reviewed_server_finished_continuity,
        complete_revocation_certificate_verify_sealed, direct_revocation_evidence_sealed,
        double_nested_revocation_evidence_sealed, nested_revocation_evidence_sealed,
        ordered_retry_server_certificate_verify_evidence_sealed,
        recursive_revocation_evidence_sealed, require_server_role,
        retry_reviewed_server_authentication_evidence_sealed, validate_leaf_public_key,
        validate_ordered_retry_reviewed_server_certificate_continuity,
        validate_ordered_retry_reviewed_server_finished_continuity,
        verify_https_dns_destination_reviewed_direct_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_double_nested_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_intermediate_status_responder_revocation_responder_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_nested_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_recursive_responder_no_check_complete_fixed_path_revocation_certificate_verify,
        CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished,
        OrderedNoPskServerFlightReviewedRevocationCertificateVerify,
        OrderedRetryReviewedServerAuthenticationBindingError,
        OrderedRetryReviewedServerCertificateContinuityError,
        OrderedRetryReviewedServerCertificateContinuityIdentities,
        OrderedRetryReviewedServerFinishedContinuityError,
        OrderedRetryServerCertificateVerifyEvidence, RetryReviewedServerAuthenticationEvidence,
        ReviewedRetryApplicationTrafficSecretsError, ReviewedServerFinishedTransitionError,
        ReviewedServerFinishedTransitionErrorKind, ReviewedServerPathCertificateVerifyError,
        ReviewedServerPathCertificateVerifyErrorKind,
        VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedReviewedServerPathCertificateVerify,
    };
    use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
    use der_asn1::Asn1Limits;
    use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
    use tls13_application_traffic_secrets::{
        derive_application_traffic_secrets, ApplicationTrafficSecretStorage,
    };
    use tls13_certificate_message::Tls13CertificateMessageLimits;
    use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
    use tls13_certificate_verify_message::CertificateVerifyMessage;
    use tls13_certificate_verify_transcript_binding::{
        bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    };
    use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
    use tls13_cipher_suite_policy::{
        CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
    };
    use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
    use tls13_extension_block::ExtensionBlockLimits;
    use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
    use tls13_finished::{derive_finished_key, FinishedKeyStorage, FinishedRole, VerifiedFinished};
    use tls13_handshake::{Tls13HandshakePolicy, Tls13HandshakeType};
    use tls13_handshake_traffic_secrets::HandshakeTrafficSecretStorage;
    use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
    use tls13_key_schedule::BaseSecretStorage;
    use tls13_key_share::KeyShareLimits;
    use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
    use tls13_no_psk_server_flight::{NoPskServerFlight, ServerFlightErrorKind, ServerFlightState};
    use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
    use tls13_record_protection::{seal_tls13_record, Tls13Plaintext};
    use tls13_record_protection_state::RecordProtectionState;
    use tls13_retry_handshake_traffic_key_material_binding::{
        bind_retry_ordered_server_flight_transcript, bind_retry_server_certificate_leaf,
        verify_ordered_retry_server_finished, HandshakeTrafficSender,
        OrderedRetryServerFlightIdentity, RetryHandshakeRecordProtectionStateIssuer,
        RetryHandshakeRecordReassembler, RetryOrderedServerFlight,
    };
    use tls13_retry_handshake_traffic_secrets_binding::{
        bind_retry_handshake_traffic_secrets, bind_retry_server_finished_key,
        bind_retry_server_transcript_continuation,
    };
    use tls13_retry_key_schedule_binding::bind_retry_key_schedule;
    use tls13_retry_transcript_binding::bind_retry_transcript;
    use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
    use tls13_server_hello_message::{
        decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
    };
    use tls13_server_hello_retry_selection::{
        bind_final_server_key_share_message, bind_retry_key_share_messages,
        bind_server_hello_retry_selection, BoundRetryServerKeyShareMessages,
    };
    use tls13_server_hello_selection::bind_server_hello_selection;
    use tls13_server_hello_version_selection::bind_server_hello_version_selection;
    use tls13_signature_scheme_policy::{SignatureScheme, SignatureSchemePolicy};
    use tls13_supported_groups::NamedGroupListLimits;
    use tls13_supported_versions::ClientVersionOffer;
    use tls13_traffic_key_material::{derive_traffic_key_material, TrafficKeyMaterialStorage};
    use tls13_transcript::{PendingTls13Transcript, TranscriptPolicy};
    use tls13_x25519_key_share_binding::derive_message_bound_retry_x25519_shared_secret;
    use x509_certificate::{decode_x509_certificate, X509Certificate};
    use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy};

    const SHA256_WITH_RSA_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const RSA_ENCRYPTION_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const NULL_DER: &[u8] = &[0x05, 0x00];
    const PUBLIC_LEAF_HEX: &str =
        include_str!("../tests/fixtures/apple_www_2026_11_11_leaf.der.hex");
    const PUBLIC_TRANSCRIPT_HEX: &str =
        include_str!("../tests/fixtures/apple_www_2026_11_18_transcript_through_certificate.hex");
    const PUBLIC_SIGNATURE_HEX: &str =
        include_str!("../tests/fixtures/apple_www_2026_11_18_certificate_verify_signature.hex");
    const ALLOWED_EXPONENTS: &[u64] = &[65_537];
    const CONTINUITY_ALICE_PRIVATE: [u8; 32] = [
        0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66,
        0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9,
        0x2c, 0x2a,
    ];
    const CONTINUITY_ALICE_PUBLIC: [u8; 32] = [
        0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7,
        0x5a, 0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b,
        0x4e, 0x6a,
    ];
    const CONTINUITY_BOB_PUBLIC: [u8; 32] = [
        0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35,
        0x37, 0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88,
        0x2b, 0x4f,
    ];
    const CONTINUITY_SESSION_ID: &[u8] = b"ordered-continuity";
    const CONTINUITY_CLIENT_RANDOM: [u8; 32] = [0x11; 32];
    const CONTINUITY_SERVER_RANDOM: [u8; 32] = [0x22; 32];
    const TLS13_VERSION: u16 = 0x0304;

    fn continuity_u24(value: usize) -> [u8; 3] {
        assert!(value <= 0x00ff_ffff);
        [
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        ]
    }

    fn continuity_fixed_hash(suite: SelectedCipherSuite) -> FixedHkdfHash {
        match suite.parameters().hkdf_hash() {
            HkdfHash::Sha256 => FixedHkdfHash::Sha256,
            HkdfHash::Sha384 => FixedHkdfHash::Sha384,
        }
    }

    fn continuity_group_policy() -> NamedGroupPolicy {
        NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
    }

    fn continuity_client_share(public_key: &[u8; 32]) -> [u8; 38] {
        let mut bytes = [0_u8; 38];
        bytes[..6].copy_from_slice(&[0x00, 0x24, 0x00, 0x1d, 0x00, 0x20]);
        bytes[6..].copy_from_slice(public_key);
        bytes
    }

    fn continuity_server_share(public_key: &[u8; 32]) -> [u8; 36] {
        let mut bytes = [0_u8; 36];
        bytes[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
        bytes[4..].copy_from_slice(public_key);
        bytes
    }

    fn continuity_append_extensions(body: &mut Vec<u8>, extensions: &[(u16, &[u8])]) {
        let payload_length = extensions
            .iter()
            .map(|(_, data)| 4 + data.len())
            .sum::<usize>();
        body.extend_from_slice(&(payload_length as u16).to_be_bytes());
        for (extension_type, data) in extensions {
            body.extend_from_slice(&extension_type.to_be_bytes());
            body.extend_from_slice(&(data.len() as u16).to_be_bytes());
            body.extend_from_slice(data);
        }
    }

    fn continuity_client_body(extensions: &[(u16, &[u8])]) -> Vec<u8> {
        let mut body = vec![0x03, 0x03];
        body.extend_from_slice(&CONTINUITY_CLIENT_RANDOM);
        body.push(CONTINUITY_SESSION_ID.len() as u8);
        body.extend_from_slice(CONTINUITY_SESSION_ID);
        body.extend_from_slice(&[0x00, 0x02]);
        body.extend_from_slice(&CipherSuite::TlsAes128GcmSha256.wire_id().to_be_bytes());
        body.extend_from_slice(&[0x01, 0x00]);
        let version_data = [0x02, 0x03, 0x04];
        let mut all_extensions = vec![(43, version_data.as_slice())];
        all_extensions.extend_from_slice(extensions);
        continuity_append_extensions(&mut body, &all_extensions);
        body
    }

    fn continuity_server_body(random: [u8; 32], extensions: &[(u16, &[u8])]) -> Vec<u8> {
        let mut body = vec![0x03, 0x03];
        body.extend_from_slice(&random);
        body.push(CONTINUITY_SESSION_ID.len() as u8);
        body.extend_from_slice(CONTINUITY_SESSION_ID);
        body.extend_from_slice(&CipherSuite::TlsAes128GcmSha256.wire_id().to_be_bytes());
        body.push(0x00);
        let version_data = TLS13_VERSION.to_be_bytes();
        let mut all_extensions = vec![(43, version_data.as_slice())];
        all_extensions.extend_from_slice(extensions);
        continuity_append_extensions(&mut body, &all_extensions);
        body
    }

    fn continuity_version_selection<'client, 'server>(
        client_bytes: &'client [u8],
        server_bytes: &'server [u8],
    ) -> tls13_server_hello_version_selection::BoundServerHelloVersionSelection<
        'client,
        'static,
        'server,
        'static,
        'static,
    > {
        static AES128_SUITE: [u16; 1] = [0x1301];
        static VERSIONS: [u16; 1] = [TLS13_VERSION];
        let client = decode_client_hello(
            client_bytes,
            ClientHelloLimits::new(4, ExtensionBlockLimits::new(256, 5).unwrap()).unwrap(),
        )
        .unwrap();
        let server = decode_tls13_server_hello(
            server_bytes,
            Tls13ServerHelloLimits::new(256, 256, 4).unwrap(),
        )
        .unwrap();
        let entries =
            [CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1_000).unwrap()];
        let offer =
            ClientCipherSuiteOffer::new(&AES128_SUITE, CipherSuitePolicy::new(&entries).unwrap())
                .unwrap();
        let selection = bind_server_hello_selection(client, offer, server).unwrap();
        bind_server_hello_version_selection(
            selection,
            ClientVersionOffer::new(&VERSIONS).unwrap(),
            ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap(),
        )
        .unwrap()
    }

    fn with_continuity_retry_messages<T>(
        callback: impl FnOnce(
            &BoundRetryServerKeyShareMessages<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        ) -> T,
    ) -> T {
        let p256_key = [0x04; 65];
        let supported_groups = [0x00, 0x04, 0x00, 0x17, 0x00, 0x1d];
        let mut initial_share = [0_u8; 71];
        initial_share[..6].copy_from_slice(&[0x00, 0x45, 0x00, 0x17, 0x00, 0x41]);
        initial_share[6..].copy_from_slice(&p256_key);
        let retry_share = [0x00, 0x1d];
        let retried_share = continuity_client_share(&CONTINUITY_ALICE_PUBLIC);
        let final_share = continuity_server_share(&CONTINUITY_BOB_PUBLIC);
        let signature_algorithms = [0, 2, 8, 4];
        let first_client = continuity_client_body(&[
            (5, &[]),
            (10, &supported_groups),
            (13, &signature_algorithms),
            (51, &initial_share),
        ]);
        let retry_server =
            continuity_server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, &[(51, &retry_share)]);
        let second_client = continuity_client_body(&[
            (5, &[]),
            (10, &supported_groups),
            (13, &signature_algorithms),
            (51, &retried_share),
        ]);
        let final_server = continuity_server_body(CONTINUITY_SERVER_RANDOM, &[(51, &final_share)]);
        let selection = bind_server_hello_retry_selection(
            continuity_version_selection(&first_client, &retry_server),
            continuity_version_selection(&second_client, &final_server),
        )
        .unwrap();
        let messages = bind_retry_key_share_messages(
            selection,
            continuity_group_policy(),
            NamedGroupListLimits::new(2).unwrap(),
            KeyShareLimits::new(2).unwrap(),
        )
        .unwrap();
        let final_messages = bind_final_server_key_share_message(&messages).unwrap();
        callback(&final_messages)
    }

    struct TestDirectRevocationEvidence<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
    }

    impl<'certificate, 'encoded> direct_revocation_evidence_sealed::Sealed<'encoded>
        for TestDirectRevocationEvidence<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }
    }

    struct TestNestedRevocationEvidence<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
    }

    impl<'certificate, 'encoded> nested_revocation_evidence_sealed::Sealed<'encoded>
        for TestNestedRevocationEvidence<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }
    }

    struct TestDoubleNestedRevocationEvidence<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
    }

    impl<'certificate, 'encoded> double_nested_revocation_evidence_sealed::Sealed<'encoded>
        for TestDoubleNestedRevocationEvidence<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }
    }

    struct TestRecursiveRevocationEvidence<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
    }

    impl<'certificate, 'encoded> recursive_revocation_evidence_sealed::Sealed<'encoded>
        for TestRecursiveRevocationEvidence<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }
    }

    struct TestAlternativeRevocationEvidence<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
    }

    struct TestCompleteRevocationCertificateVerify {
        selected_suite: SelectedCipherSuite,
    }

    struct TestRetryReviewedServerAuthentication<'certificate, 'encoded> {
        certificate: &'certificate X509Certificate<'encoded>,
        verdict_identity: *const (),
    }

    impl<'certificate, 'encoded>
        retry_reviewed_server_authentication_evidence_sealed::Sealed<'encoded>
        for TestRetryReviewedServerAuthentication<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }

        fn certificate_verify_verdict_identity(&self) -> *const () {
            self.verdict_identity
        }
    }

    impl<'certificate, 'encoded> RetryReviewedServerAuthenticationEvidence<'encoded>
        for TestRetryReviewedServerAuthentication<'certificate, 'encoded>
    {
    }

    struct TestOrderedRetryServerCertificateVerify {
        verdict_identity: *const (),
    }

    impl<'issuer> ordered_retry_server_certificate_verify_evidence_sealed::Sealed<'issuer>
        for TestOrderedRetryServerCertificateVerify
    {
        fn ordered_server_flight_identity(&self) -> OrderedRetryServerFlightIdentity<'issuer> {
            panic!("ordered flight identity is not used by verdict-only tests")
        }

        fn certificate_verify_verdict_identity(&self) -> *const () {
            self.verdict_identity
        }
    }

    impl<'issuer> OrderedRetryServerCertificateVerifyEvidence<'issuer>
        for TestOrderedRetryServerCertificateVerify
    {
    }

    struct TestContinuityOrderedRetryServerCertificateVerify<'issuer> {
        ordered_server_flight_identity: OrderedRetryServerFlightIdentity<'issuer>,
        verdict_identity: *const (),
    }

    impl<'issuer> ordered_retry_server_certificate_verify_evidence_sealed::Sealed<'issuer>
        for TestContinuityOrderedRetryServerCertificateVerify<'issuer>
    {
        fn ordered_server_flight_identity(&self) -> OrderedRetryServerFlightIdentity<'issuer> {
            self.ordered_server_flight_identity
        }

        fn certificate_verify_verdict_identity(&self) -> *const () {
            self.verdict_identity
        }
    }

    impl<'issuer> OrderedRetryServerCertificateVerifyEvidence<'issuer>
        for TestContinuityOrderedRetryServerCertificateVerify<'issuer>
    {
    }

    impl complete_revocation_certificate_verify_sealed::Sealed
        for TestCompleteRevocationCertificateVerify
    {
        fn selected_suite(&self) -> SelectedCipherSuite {
            self.selected_suite
        }
    }

    impl<'certificate, 'encoded> alternative_revocation_evidence_sealed::Sealed<'encoded>
        for TestAlternativeRevocationEvidence<'certificate, 'encoded>
    {
        fn server_certificate(&self) -> &X509Certificate<'encoded> {
            self.certificate
        }
    }

    fn fixture(hexadecimal: &str) -> Vec<u8> {
        let digits = hexadecimal
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        assert!(digits.len().is_multiple_of(2));
        digits
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
        let entry = CipherSuitePolicyEntry::new(suite, 8_192).unwrap();
        CipherSuitePolicy::new(&[entry])
            .unwrap()
            .select(suite.wire_id())
            .unwrap()
    }

    fn verified_finished(
        selected_suite: SelectedCipherSuite,
        role: FinishedRole,
    ) -> VerifiedFinished {
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x11; 32]).unwrap();
        let mut storage = FinishedKeyStorage::new(selected_suite, role);
        let key = derive_finished_key(&prk, &mut storage).unwrap();
        let transcript_hash = [0x24; 32];
        let mut verify_data = [0; 32];
        key.write_verify_data(&transcript_hash, &mut verify_data)
            .unwrap();
        key.verify_peer(&transcript_hash, &verify_data).unwrap()
    }

    fn length(length: usize) -> Vec<u8> {
        if length < 128 {
            return vec![length as u8];
        }
        let bytes = length.to_be_bytes();
        let first = bytes.iter().position(|byte| *byte != 0).unwrap();
        let significant = &bytes[first..];
        let mut encoded = vec![0x80 | significant.len() as u8];
        encoded.extend_from_slice(significant);
        encoded
    }

    fn der(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut encoded = vec![tag];
        encoded.extend(length(value.len()));
        encoded.extend_from_slice(value);
        encoded
    }

    fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
        der(0x30, &fields.concat())
    }

    fn positive_integer(magnitude: &[u8]) -> Vec<u8> {
        let mut contents = Vec::with_capacity(magnitude.len() + 1);
        if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
            contents.push(0);
        }
        contents.extend_from_slice(magnitude);
        der(0x02, &contents)
    }

    fn algorithm(oid: &[u8]) -> Vec<u8> {
        sequence(&[der(0x06, oid), NULL_DER.to_vec()])
    }

    fn test_certificate() -> Vec<u8> {
        let mut modulus = vec![0_u8; 256];
        modulus[0] = 0x80;
        modulus[255] = 1;
        let rsa_key = sequence(&[
            positive_integer(&modulus),
            positive_integer(&[0x01, 0x00, 0x01]),
        ]);
        let mut key_bits = vec![0];
        key_bits.extend_from_slice(&rsa_key);
        let spki = sequence(&[algorithm(RSA_ENCRYPTION_OID), der(0x03, &key_bits)]);
        let attribute = sequence(&[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, b"example")]);
        let name = sequence(&[der(0x31, &attribute)]);
        let validity = sequence(&[der(0x17, b"260101000000Z"), der(0x17, b"270101000000Z")]);
        let tbs = sequence(&[
            der(0xa0, &der(0x02, &[0x02])),
            der(0x02, &[0x01]),
            algorithm(SHA256_WITH_RSA_OID),
            name.clone(),
            validity,
            name,
            spki,
        ]);
        sequence(&[tbs, algorithm(SHA256_WITH_RSA_OID), der(0x03, &[0, 0xaa])])
    }

    #[test]
    fn ordered_retry_composition_retains_exact_reviewed_authentication_verdict() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let verdict = 0x5a_u8;
        let verdict_identity = core::ptr::from_ref(&verdict).cast();
        let authentication = TestRetryReviewedServerAuthentication {
            certificate: &certificate,
            verdict_identity,
        };
        let ordered = TestOrderedRetryServerCertificateVerify { verdict_identity };

        let bound = bind_reviewed_server_authentication_to_ordered_retry_certificate_verify(
            &authentication,
            &ordered,
        )
        .unwrap();

        assert!(core::ptr::eq(bound.authentication(), &authentication));
        assert!(core::ptr::eq(bound.ordered_certificate_verify(), &ordered,));
        let debug = format!("{bound:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("90"));
    }

    #[test]
    fn ordered_retry_composition_rejects_value_equal_detached_verdict() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let reviewed_verdict = 0x5a_u8;
        let detached_verdict = 0x5a_u8;
        let authentication = TestRetryReviewedServerAuthentication {
            certificate: &certificate,
            verdict_identity: core::ptr::from_ref(&reviewed_verdict).cast(),
        };
        let ordered = TestOrderedRetryServerCertificateVerify {
            verdict_identity: core::ptr::from_ref(&detached_verdict).cast(),
        };

        let error = bind_reviewed_server_authentication_to_ordered_retry_certificate_verify(
            &authentication,
            &ordered,
        )
        .unwrap_err();

        assert_eq!(
            error,
            OrderedRetryReviewedServerAuthenticationBindingError::CertificateVerifyVerdictMismatch,
        );
        assert_eq!(
            error.to_string(),
            "ordered retry CertificateVerify and reviewed server authentication verdicts differ",
        );
        assert_eq!(format!("{error:?}"), "CertificateVerifyVerdictMismatch");
    }

    #[test]
    fn public_ordered_retry_certificate_continuity_spans_real_callbacks() {
        run_ordered_retry_certificate_continuity(ContinuityCase::CallbackOnly);
    }

    #[test]
    fn reviewed_committed_finished_joins_only_the_same_record_owner() {
        run_ordered_retry_certificate_continuity(ContinuityCase::Committed);
        run_ordered_retry_certificate_continuity(ContinuityCase::DifferentOwner);
    }

    #[test]
    fn reviewed_application_traffic_uses_only_the_committed_post_finished_transcript() {
        run_ordered_retry_certificate_continuity(ContinuityCase::DeriveApplication);
    }

    #[test]
    fn reviewed_application_traffic_rejects_different_policy_for_the_same_cipher() {
        run_ordered_retry_certificate_continuity(ContinuityCase::DifferentApplicationPolicy);
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ContinuityCase {
        CallbackOnly,
        Committed,
        DifferentOwner,
        DeriveApplication,
        DifferentApplicationPolicy,
    }

    fn run_ordered_retry_certificate_continuity(case: ContinuityCase) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let verdict = 0x5a_u8;
        let authentication = TestRetryReviewedServerAuthentication {
            certificate: &leaf,
            verdict_identity: core::ptr::from_ref(&verdict).cast(),
        };
        let mut x25519_output = [0xa5; 32];

        with_continuity_retry_messages(|messages| {
            let secret = derive_message_bound_retry_x25519_shared_secret(
                messages,
                &CONTINUITY_ALICE_PRIVATE,
                &mut x25519_output,
            )
            .unwrap();
            let transcript = bind_retry_transcript(
                messages.messages().selection(),
                TranscriptPolicy::new(16_384).unwrap(),
            )
            .unwrap();
            let suite = transcript.transcript().selected_suite();
            let mut base_storage = BaseSecretStorage::new(suite).unwrap();
            let base = bind_retry_key_schedule(&secret, &transcript, &mut base_storage).unwrap();
            let mut traffic_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
            let traffic =
                bind_retry_handshake_traffic_secrets(&base, &mut traffic_storage).unwrap();
            let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
            let mut finished_storage = FinishedKeyStorage::new(suite, FinishedRole::Server);
            let server_finished =
                bind_retry_server_finished_key(issuer.traffic_secrets(), &mut finished_storage)
                    .unwrap();
            let continuation = bind_retry_server_transcript_continuation(&server_finished).unwrap();
            let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
            let server_state = issuer
                .issue(HandshakeTrafficSender::Server, &mut server_storage)
                .unwrap();
            let handshake_policy = Tls13HandshakePolicy::new(
                &[
                    Tls13HandshakeType::EncryptedExtensions,
                    Tls13HandshakeType::Certificate,
                    Tls13HandshakeType::CertificateVerify,
                    Tls13HandshakeType::Finished,
                ],
                16_384,
            )
            .unwrap();
            let mut reassembly_storage = [0xa5; 16_384];
            let reassembler = RetryHandshakeRecordReassembler::new(
                server_state,
                handshake_policy,
                &mut reassembly_storage,
            )
            .unwrap();
            let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
            // Both tested record owners can be selected through a mutable
            // reference below, so their reassembly buffers must outlive both.
            let mut detached_reassembly_storage = [0xa5; 16_384];
            let mut bound =
                bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();

            let server_prk = HkdfPrk::import_exact(
                continuity_fixed_hash(suite),
                issuer.traffic_secrets().traffic_secrets().server_secret(),
            )
            .unwrap();
            let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
            let peer_material =
                derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
            let mut peer_state = RecordProtectionState::new(peer_material).unwrap();

            let encoded_extensions = [0, 0];
            let certificate_list_length = 3 + leaf_encoded.len() + encoded_extensions.len();
            let mut certificate_body = Vec::with_capacity(4 + certificate_list_length);
            certificate_body.push(0);
            certificate_body.extend_from_slice(&continuity_u24(certificate_list_length));
            certificate_body.extend_from_slice(&continuity_u24(leaf_encoded.len()));
            certificate_body.extend_from_slice(&leaf_encoded);
            certificate_body.extend_from_slice(&encoded_extensions);
            let certificate_verify_body = [0x08, 0x04, 0x00, 0x01, 0x00];
            let mut plaintext = Vec::with_capacity(19 + certificate_body.len());
            plaintext.extend_from_slice(&[8, 0, 0, 2, 0, 0]);
            plaintext.push(11);
            plaintext.extend_from_slice(&continuity_u24(certificate_body.len()));
            plaintext.extend_from_slice(&certificate_body);
            plaintext.push(15);
            plaintext.extend_from_slice(&continuity_u24(certificate_verify_body.len()));
            plaintext.extend_from_slice(&certificate_verify_body);

            let ciphertext_policy = Tls13CiphertextPolicy::new(8_192).unwrap();
            let inner_policy =
                Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 8_176).unwrap();
            let mut scratch = [0xa5; 8_192];
            let mut wire = [0xcc; 8_192];
            let used = seal_tls13_record(
                &mut peer_state,
                &ciphertext_policy,
                &inner_policy,
                Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
                &mut scratch,
                &mut wire,
            )
            .unwrap();
            let mut opened = [0x5a; 8_192];
            let mut certificate_receipt = None;
            let mut completed_certificate_verify = None;
            // A separate single-use issuer shares the same retry schedule, so
            // it opens the same fixture bytes and produces the same digests.
            // The only significant difference for the join is its identity.
            let mut detached_traffic_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
            let detached_traffic =
                bind_retry_handshake_traffic_secrets(&base, &mut detached_traffic_storage).unwrap();
            let detached_issuer = RetryHandshakeRecordProtectionStateIssuer::new(detached_traffic);
            let mut detached_finished_storage =
                FinishedKeyStorage::new(suite, FinishedRole::Server);
            let detached_finished_key = bind_retry_server_finished_key(
                detached_issuer.traffic_secrets(),
                &mut detached_finished_storage,
            )
            .unwrap();
            let mut detached_server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
            let detached_state = detached_issuer
                .issue(HandshakeTrafficSender::Server, &mut detached_server_storage)
                .unwrap();
            let detached_reassembler = RetryHandshakeRecordReassembler::new(
                detached_state,
                handshake_policy,
                &mut detached_reassembly_storage,
            )
            .unwrap();
            let mut detached_bound = bind_retry_ordered_server_flight_transcript(
                RetryOrderedServerFlight::new(detached_reassembler).unwrap(),
                bind_retry_server_transcript_continuation(&detached_finished_key).unwrap(),
            )
            .unwrap();

            let pre_finished_snapshot = {
                let result = bound.open_accept_and_advance(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |pending| match pending.message().handshake_type() {
                        Tls13HandshakeType::EncryptedExtensions => {
                            pending
                                .validate_encrypted_extensions(
                                    ExtensionBlockLimits::new(64, 4).unwrap(),
                                )
                                .unwrap();
                            true
                        }
                        Tls13HandshakeType::Certificate => {
                            let proof = pending
                                .validate_server_certificate(
                                    Tls13CertificateMessageLimits::new(
                                        certificate_body.len(),
                                        0,
                                        certificate_list_length,
                                        1,
                                        leaf_encoded.len(),
                                        16,
                                    ),
                                    ExtensionBlockLimits::new(16, 4).unwrap(),
                                )
                                .unwrap();
                            let leaf_proof =
                                bind_retry_server_certificate_leaf(&proof, &leaf).unwrap();
                            let joined =
                                bind_retry_server_certificate_leaf_to_reviewed_server_authentication(
                                    &leaf_proof,
                                    &authentication,
                                )
                                .unwrap();
                            certificate_receipt = Some(
                                begin_ordered_retry_reviewed_server_certificate_continuity(
                                    &joined,
                                ),
                            );
                            true
                        }
                        Tls13HandshakeType::CertificateVerify => {
                            let ordered =
                                TestContinuityOrderedRetryServerCertificateVerify {
                                    ordered_server_flight_identity: pending
                                        .ordered_server_flight_identity(),
                                    verdict_identity: core::ptr::from_ref(&verdict).cast(),
                                };
                            let joined =
                                bind_reviewed_server_authentication_to_ordered_retry_certificate_verify(
                                    &authentication,
                                    &ordered,
                                )
                                .unwrap();
                            let receipt =
                                complete_ordered_retry_reviewed_server_certificate_continuity(
                                    certificate_receipt.take().unwrap(),
                                    &joined,
                                )
                                .unwrap();
                            assert!(core::ptr::eq(receipt.authentication(), &authentication));
                            completed_certificate_verify = Some(receipt);
                            true
                        }
                        other => panic!("unexpected handshake type {other:?}"),
                    },
                )
                .unwrap();

                assert_eq!(result.completed_messages(), 3);
                assert_eq!(result.state(), ServerFlightState::ExpectFinished);
                result.transcript_snapshot()
            };

            let (finished_owner, finished_key) = if case == ContinuityCase::DifferentOwner {
                let detached_prefix = detached_bound
                    .open_accept_and_advance(
                        &ciphertext_policy,
                        &inner_policy,
                        &wire[..used],
                        &mut opened,
                        |_| true,
                    )
                    .unwrap();
                assert_eq!(detached_prefix.transcript_snapshot(), pre_finished_snapshot);
                (&mut detached_bound, &detached_finished_key)
            } else {
                (&mut bound, &server_finished)
            };

            let mut verify_data = [0_u8; 32];
            server_finished
                .finished_key()
                .write_verify_data(pre_finished_snapshot.as_bytes(), &mut verify_data)
                .unwrap();
            let mut finished_plaintext = Vec::with_capacity(4 + verify_data.len());
            finished_plaintext.push(20);
            finished_plaintext.extend_from_slice(&continuity_u24(verify_data.len()));
            finished_plaintext.extend_from_slice(&verify_data);
            let used = seal_tls13_record(
                &mut peer_state,
                &ciphertext_policy,
                &inner_policy,
                Tls13Plaintext::new(TlsContentType::Handshake, &finished_plaintext, 0),
                &mut scratch,
                &mut wire,
            )
            .unwrap();
            let mut verified_finished = None;
            let result = finished_owner
                .open_accept_and_advance(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |pending| {
                        let finished =
                            verify_ordered_retry_server_finished(&pending, finished_key).unwrap();
                        verified_finished = Some(finished);
                        true
                    },
                )
                .unwrap();

            assert_eq!(result.completed_messages(), 1);
            assert_eq!(result.state(), ServerFlightState::Complete);
            let verified = verified_finished.unwrap();
            let certificate_verify = completed_certificate_verify.unwrap();
            if case == ContinuityCase::CallbackOnly {
                let completed = complete_ordered_retry_reviewed_server_finished_continuity(
                    certificate_verify,
                    verified,
                )
                .unwrap();
                assert!(core::ptr::eq(
                    completed.certificate_verify().authentication(),
                    &authentication
                ));
                assert_eq!(
                    completed.finished().transcript_snapshot(),
                    pre_finished_snapshot
                );
                let debug = format!("{completed:?}");
                assert!(debug.contains("[REDACTED]"));
                assert!(!debug.contains(&format!("{:x?}", verify_data)));
                return;
            }
            let mut expected = transcript.reconstruct_transcript().unwrap();
            expected.update(&plaintext).unwrap();
            expected.update(&finished_plaintext).unwrap();
            let committed = result.bind_committed_finished(verified).unwrap();
            assert_eq!(committed.transcript_snapshot(), expected.snapshot());
            assert_eq!(
                committed.finished().verification().peer_verify_data(),
                verify_data
            );

            let completed = complete_ordered_retry_reviewed_committed_server_finished_continuity(
                certificate_verify,
                committed,
            );
            if case == ContinuityCase::DifferentOwner {
                assert_eq!(
                    completed.unwrap_err(),
                    OrderedRetryReviewedServerFinishedContinuityError::OrderedServerFlightMismatch,
                );
                return;
            }
            let completed = completed.unwrap();
            assert!(core::ptr::eq(
                completed.certificate_verify().authentication(),
                &authentication,
            ));
            assert_eq!(
                completed
                    .committed_finished()
                    .finished()
                    .transcript_snapshot(),
                pre_finished_snapshot,
            );
            assert_eq!(
                completed.committed_finished().transcript_snapshot(),
                expected.snapshot()
            );
            let debug = format!("{completed:?}");
            assert!(debug.contains("[REDACTED]"));
            assert!(!debug.contains(&format!("{:x?}", verify_data)));

            if case == ContinuityCase::DifferentApplicationPolicy {
                let cipher = suite.parameters().suite();
                let other_limit = if suite.record_limit() == 1 { 2 } else { 1 };
                let entry = CipherSuitePolicyEntry::new(cipher, other_limit).unwrap();
                let wrong_suite = CipherSuitePolicy::new(&[entry])
                    .unwrap()
                    .select(cipher.wire_id())
                    .unwrap();
                assert_eq!(suite.parameters(), wrong_suite.parameters());
                assert_ne!(suite.record_limit(), wrong_suite.record_limit());
                let mut storage = ApplicationTrafficSecretStorage::new(wrong_suite).unwrap();
                let error = completed
                    .derive_application_traffic_secrets(&mut storage)
                    .unwrap_err();
                assert_eq!(
                    error,
                    ReviewedRetryApplicationTrafficSecretsError::CipherSuiteMismatch
                );
                assert_eq!(
                    error.to_string(),
                    "reviewed retry and application traffic storage use different suite policies"
                );
                return;
            }
            if case != ContinuityCase::DeriveApplication {
                return;
            }

            let mut storage = ApplicationTrafficSecretStorage::new(suite).unwrap();
            let derived = completed
                .derive_application_traffic_secrets(&mut storage)
                .unwrap();
            assert!(core::ptr::eq(
                derived.continuity().certificate_verify().authentication(),
                &authentication
            ));
            assert_eq!(
                derived
                    .continuity()
                    .committed_finished()
                    .transcript_snapshot(),
                expected.snapshot()
            );
            let mut expected_storage = ApplicationTrafficSecretStorage::new(suite).unwrap();
            let expected_secrets = derive_application_traffic_secrets(
                base.base_secrets(),
                &expected.snapshot(),
                &mut expected_storage,
            )
            .unwrap();
            let mut pre_finished_storage = ApplicationTrafficSecretStorage::new(suite).unwrap();
            let wrong_secrets = derive_application_traffic_secrets(
                base.base_secrets(),
                &pre_finished_snapshot,
                &mut pre_finished_storage,
            )
            .unwrap();
            assert_eq!(derived.traffic_secrets().selected_suite(), suite);
            assert_eq!(
                derived.traffic_secrets().client_secret(),
                expected_secrets.client_secret()
            );
            assert_eq!(
                derived.traffic_secrets().server_secret(),
                expected_secrets.server_secret()
            );
            assert_ne!(
                derived.traffic_secrets().client_secret(),
                wrong_secrets.client_secret()
            );
            assert_ne!(
                derived.traffic_secrets().server_secret(),
                wrong_secrets.server_secret()
            );
            assert_ne!(
                derived.traffic_secrets().client_secret(),
                derived.traffic_secrets().server_secret()
            );
            assert_eq!(
                format!("{derived:?}"),
                "BoundReviewedRetryApplicationTrafficSecrets([REDACTED])"
            );
        });
    }

    #[test]
    fn ordered_retry_finished_continuity_rejects_a_different_record_owner() {
        assert_eq!(
            validate_ordered_retry_reviewed_server_finished_continuity(7_u8, 7_u8),
            Ok(()),
        );
        assert_eq!(
            validate_ordered_retry_reviewed_server_finished_continuity(7_u8, 8_u8),
            Err(OrderedRetryReviewedServerFinishedContinuityError::OrderedServerFlightMismatch,),
        );
    }

    #[test]
    fn ordered_retry_certificate_continuity_accepts_only_one_exact_chain() {
        let authentication = 0x11_u8;
        let leaf = 0x22_u8;
        let verdict = 0x33_u8;
        let authentication_identity = core::ptr::from_ref(&authentication).cast();
        let leaf_identity = core::ptr::from_ref(&leaf).cast();
        let verdict_identity = core::ptr::from_ref(&verdict).cast();

        assert_eq!(
            validate_ordered_retry_reviewed_server_certificate_continuity(
                &OrderedRetryReviewedServerCertificateContinuityIdentities {
                    authentication: authentication_identity,
                    ordered_server_flight: 7_u8,
                    leaf_certificate: leaf_identity,
                    certificate_verify_verdict: verdict_identity,
                },
                &OrderedRetryReviewedServerCertificateContinuityIdentities {
                    authentication: authentication_identity,
                    ordered_server_flight: 7_u8,
                    leaf_certificate: leaf_identity,
                    certificate_verify_verdict: verdict_identity,
                },
            ),
            Ok(()),
        );
    }

    #[test]
    fn ordered_retry_certificate_continuity_rejects_each_detached_identity() {
        let authentication = 0x11_u8;
        let detached_authentication = 0x11_u8;
        let leaf = 0x22_u8;
        let detached_leaf = 0x22_u8;
        let verdict = 0x33_u8;
        let detached_verdict = 0x33_u8;
        let authentication_identity = core::ptr::from_ref(&authentication).cast();
        let detached_authentication_identity = core::ptr::from_ref(&detached_authentication).cast();
        let leaf_identity = core::ptr::from_ref(&leaf).cast();
        let detached_leaf_identity = core::ptr::from_ref(&detached_leaf).cast();
        let verdict_identity = core::ptr::from_ref(&verdict).cast();
        let detached_verdict_identity = core::ptr::from_ref(&detached_verdict).cast();

        let mismatch = |later_authentication, later_owner, later_leaf, later_verdict| {
            validate_ordered_retry_reviewed_server_certificate_continuity(
                &OrderedRetryReviewedServerCertificateContinuityIdentities {
                    authentication: authentication_identity,
                    ordered_server_flight: 7_u8,
                    leaf_certificate: leaf_identity,
                    certificate_verify_verdict: verdict_identity,
                },
                &OrderedRetryReviewedServerCertificateContinuityIdentities {
                    authentication: later_authentication,
                    ordered_server_flight: later_owner,
                    leaf_certificate: later_leaf,
                    certificate_verify_verdict: later_verdict,
                },
            )
            .unwrap_err()
        };

        assert_eq!(
            mismatch(
                detached_authentication_identity,
                7,
                leaf_identity,
                verdict_identity,
            ),
            OrderedRetryReviewedServerCertificateContinuityError::AuthenticationMismatch,
        );
        assert_eq!(
            mismatch(authentication_identity, 8, leaf_identity, verdict_identity,),
            OrderedRetryReviewedServerCertificateContinuityError::OrderedServerFlightMismatch,
        );
        assert_eq!(
            mismatch(
                authentication_identity,
                7,
                detached_leaf_identity,
                verdict_identity,
            ),
            OrderedRetryReviewedServerCertificateContinuityError::LeafCertificateMismatch,
        );
        assert_eq!(
            mismatch(
                authentication_identity,
                7,
                leaf_identity,
                detached_verdict_identity,
            ),
            OrderedRetryReviewedServerCertificateContinuityError::CertificateVerifyVerdictMismatch,
        );
    }

    #[test]
    fn only_the_server_role_is_authorized() {
        assert_eq!(require_server_role(CertificateVerifyRole::Server), Ok(()));
        assert_eq!(
            require_server_role(CertificateVerifyRole::Client),
            Err(ReviewedServerPathCertificateVerifyError {
                kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            })
        );
    }

    #[test]
    fn derives_the_exact_reviewed_leaf_rsa_key_under_caller_policy() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap();

        let key = validate_leaf_public_key(&certificate, policy, Asn1Limits::default()).unwrap();

        assert_eq!(key.modulus_bits(), 2_048);
        assert_eq!(key.public_exponent(), 65_537);
        assert_eq!(key.modulus().first(), Some(&0x80));
        assert_eq!(key.modulus().last(), Some(&1));
    }

    #[test]
    fn leaf_key_policy_failure_is_stable_and_redacted() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[3]).unwrap();

        let error = validate_leaf_public_key(&certificate, policy, Asn1Limits::default())
            .expect_err("the reviewed leaf exponent is not allowed");

        assert_eq!(
            error.kind(),
            ReviewedServerPathCertificateVerifyErrorKind::PublicKey(
                RsaPublicKeyErrorKind::PublicExponentNotAllowed
            )
        );
        assert!(!error.to_string().contains("65537"));
    }

    #[test]
    fn result_surface_is_nominal_and_non_copyable() {
        type Proof = VerifiedReviewedServerPathCertificateVerify<
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
        >;
        assert!(
            core::any::type_name::<Proof>().contains("VerifiedReviewedServerPathCertificateVerify")
        );

        type RevocationProof =
            VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                u8,
            >;
        assert!(core::any::type_name::<RevocationProof>().contains(
            "VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify"
        ));

        type NestedRevocationProof =
            VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                u8,
            >;
        assert!(core::any::type_name::<NestedRevocationProof>().contains(
            "VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify"
        ));

        type DoubleNestedRevocationProof =
            VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                u8,
            >;
        assert!(core::any::type_name::<DoubleNestedRevocationProof>().contains(
            "VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify"
        ));

        type RecursiveRevocationProof =
            VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                u8,
            >;
        assert!(core::any::type_name::<RecursiveRevocationProof>().contains(
            "VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify"
        ));

        type AlternativeRevocationProof =
            VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                u8,
            >;
        assert!(core::any::type_name::<AlternativeRevocationProof>().contains(
            "VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify"
        ));

        fn assert_direct_evidence<'encoded, Evidence>()
        where
            Evidence: direct_revocation_evidence_sealed::Sealed<'encoded>,
        {
        }
        type OfficialDirectEvidence =
            super::BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation<
                'static,
                'static,
                'static,
                'static,
                u8,
                u8,
            >;
        assert_direct_evidence::<'static, OfficialDirectEvidence>();

        fn assert_nested_evidence<'encoded, Evidence>()
        where
            Evidence: nested_revocation_evidence_sealed::Sealed<'encoded>,
        {
        }
        type OfficialNestedEvidence =
            super::BoundHttpsDnsDestinationToReviewedNestedNoCheckCompleteFixedPathRevocation<
                'static,
                'static,
                'static,
                'static,
                u8,
                u8,
                u8,
            >;
        assert_nested_evidence::<'static, OfficialNestedEvidence>();

        fn assert_double_nested_evidence<'encoded, Evidence>()
        where
            Evidence: double_nested_revocation_evidence_sealed::Sealed<'encoded>,
        {
        }
        type OfficialDoubleNestedEvidence =
            super::BoundHttpsDnsDestinationToReviewedDoubleNestedNoCheckCompleteFixedPathRevocation<
                'static,
                'static,
                'static,
                'static,
                u8,
                u8,
                u8,
                u8,
            >;
        assert_double_nested_evidence::<'static, OfficialDoubleNestedEvidence>();

        fn assert_recursive_evidence<'encoded, Evidence>()
        where
            Evidence: recursive_revocation_evidence_sealed::Sealed<'encoded>,
        {
        }
        type OfficialRecursiveEvidence =
            super::BoundHttpsDnsDestinationToReviewedRecursiveResponderNoCheckCompleteFixedPathRevocation<
                'static,
                'static,
                'static,
                'static,
                u8,
                u8,
                u8,
                u8,
            >;
        assert_recursive_evidence::<'static, OfficialRecursiveEvidence>();

        fn assert_alternative_evidence<'encoded, Evidence>()
        where
            Evidence: alternative_revocation_evidence_sealed::Sealed<'encoded>,
        {
        }
        type OfficialAlternativeEvidence =
            super::BoundHttpsDnsDestinationToReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocation<
                'static,
                'static,
                'static,
                'static,
                u8,
                u8,
                u8,
            >;
        assert_alternative_evidence::<'static, OfficialAlternativeEvidence>();

        fn assert_ordered_transition_admission<Verification>()
        where
            Verification: complete_revocation_certificate_verify_sealed::Sealed,
        {
        }
        assert_ordered_transition_admission::<RevocationProof>();
        assert_ordered_transition_admission::<NestedRevocationProof>();
        assert_ordered_transition_admission::<DoubleNestedRevocationProof>();
        assert_ordered_transition_admission::<RecursiveRevocationProof>();
        assert_ordered_transition_admission::<AlternativeRevocationProof>();

        type OrderedProof =
            OrderedNoPskServerFlightReviewedRevocationCertificateVerify<'static, 'static, u8>;
        assert!(core::any::type_name::<OrderedProof>()
            .contains("OrderedNoPskServerFlightReviewedRevocationCertificateVerify"));
        type CompletedProof = CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished<
            'static,
            'static,
            'static,
            u8,
        >;
        assert!(core::any::type_name::<CompletedProof>()
            .contains("CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished",));
    }

    fn exercise_direct_revocation_entry_point(role: CertificateVerifyRole, expect_success: bool) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let evidence = TestDirectRevocationEvidence { certificate: &leaf };
        let transcript_encoded = fixture(PUBLIC_TRANSCRIPT_HEX);
        let suite_entry =
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
        let selected_suite = CipherSuitePolicy::new(&[suite_entry])
            .unwrap()
            .select(CipherSuite::TlsAes256GcmSha384.wire_id())
            .unwrap();
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
        pending.update(&transcript_encoded).unwrap();
        let transcript = pending.select(selected_suite);
        let scheme = SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
            .unwrap()
            .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
            .unwrap();
        let signature = fixture(PUBLIC_SIGNATURE_HEX);
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let mut scratch = [0xa5; 223];
        let before_scratch = scratch;
        let expected_input;
        {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
            let hashed = hash_transcript_bound_certificate_verify_input(scheme, &bound);
            let message = CertificateVerifyMessage::new(
                SignatureScheme::RsaPssRsaeSha256.wire_id(),
                &signature,
            )
            .unwrap();
            expected_input = bound.input().as_bytes().to_vec();
            let result = verify_https_dns_destination_reviewed_direct_no_check_complete_fixed_path_revocation_certificate_verify(
                &evidence,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

            if expect_success {
                let verified = result.expect("captured server signature must verify");
                let expected_key = validate_rsa_public_key(
                    leaf.tbs_certificate().subject_public_key_info(),
                    RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                    Asn1Limits::default(),
                )
                .unwrap();
                assert!(core::ptr::eq(
                    verified.https_destination_complete_revocation(),
                    &evidence,
                ));
                assert!(core::ptr::eq(
                    verified.verification().hashed_input(),
                    &hashed
                ));
                assert!(core::ptr::eq(
                    verified.verification().verification().signature().message(),
                    &message,
                ));
                assert_eq!(
                    verified
                        .verification()
                        .verification()
                        .signature()
                        .public_key()
                        .modulus(),
                    expected_key.modulus(),
                );
                assert_eq!(
                    format!("{verified:?}"),
                    "VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify",
                );
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
                );
            }
        }
        assert_eq!(&output[..expected_input.len()], expected_input);
        assert!(output[expected_input.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        if !expect_success {
            assert_eq!(scratch, before_scratch);
        }
    }

    fn exercise_nested_revocation_entry_point(role: CertificateVerifyRole, expect_success: bool) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let evidence = TestNestedRevocationEvidence { certificate: &leaf };
        let transcript_encoded = fixture(PUBLIC_TRANSCRIPT_HEX);
        let suite_entry =
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
        let selected_suite = CipherSuitePolicy::new(&[suite_entry])
            .unwrap()
            .select(CipherSuite::TlsAes256GcmSha384.wire_id())
            .unwrap();
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
        pending.update(&transcript_encoded).unwrap();
        let transcript = pending.select(selected_suite);
        let scheme = SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
            .unwrap()
            .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
            .unwrap();
        let signature = fixture(PUBLIC_SIGNATURE_HEX);
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let mut scratch = [0xa5; 223];
        let before_scratch = scratch;
        let expected_input;
        {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
            let hashed = hash_transcript_bound_certificate_verify_input(scheme, &bound);
            let message = CertificateVerifyMessage::new(
                SignatureScheme::RsaPssRsaeSha256.wire_id(),
                &signature,
            )
            .unwrap();
            expected_input = bound.input().as_bytes().to_vec();
            let result = verify_https_dns_destination_reviewed_nested_no_check_complete_fixed_path_revocation_certificate_verify(
                &evidence,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

            if expect_success {
                let verified = result.expect("captured server signature must verify");
                let expected_key = validate_rsa_public_key(
                    leaf.tbs_certificate().subject_public_key_info(),
                    RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                    Asn1Limits::default(),
                )
                .unwrap();
                assert!(core::ptr::eq(
                    verified.https_destination_complete_revocation(),
                    &evidence,
                ));
                assert!(core::ptr::eq(
                    verified.verification().hashed_input(),
                    &hashed
                ));
                assert!(core::ptr::eq(
                    verified.verification().verification().signature().message(),
                    &message,
                ));
                assert_eq!(
                    verified
                        .verification()
                        .verification()
                        .signature()
                        .public_key()
                        .modulus(),
                    expected_key.modulus(),
                );
                assert_eq!(
                    format!("{verified:?}"),
                    "VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify",
                );
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
                );
            }
        }
        assert_eq!(&output[..expected_input.len()], expected_input);
        assert!(output[expected_input.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        if !expect_success {
            assert_eq!(scratch, before_scratch);
        }
    }

    fn exercise_double_nested_revocation_entry_point(
        role: CertificateVerifyRole,
        expect_success: bool,
    ) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let evidence = TestDoubleNestedRevocationEvidence { certificate: &leaf };
        let transcript_encoded = fixture(PUBLIC_TRANSCRIPT_HEX);
        let suite_entry =
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
        let selected_suite = CipherSuitePolicy::new(&[suite_entry])
            .unwrap()
            .select(CipherSuite::TlsAes256GcmSha384.wire_id())
            .unwrap();
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
        pending.update(&transcript_encoded).unwrap();
        let transcript = pending.select(selected_suite);
        let scheme = SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
            .unwrap()
            .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
            .unwrap();
        let signature = fixture(PUBLIC_SIGNATURE_HEX);
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let mut scratch = [0xa5; 223];
        let before_scratch = scratch;
        let expected_input;
        {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
            let hashed = hash_transcript_bound_certificate_verify_input(scheme, &bound);
            let message = CertificateVerifyMessage::new(
                SignatureScheme::RsaPssRsaeSha256.wire_id(),
                &signature,
            )
            .unwrap();
            expected_input = bound.input().as_bytes().to_vec();
            let result = verify_https_dns_destination_reviewed_double_nested_no_check_complete_fixed_path_revocation_certificate_verify(
                &evidence,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

            if expect_success {
                let verified = result.expect("captured server signature must verify");
                let expected_key = validate_rsa_public_key(
                    leaf.tbs_certificate().subject_public_key_info(),
                    RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                    Asn1Limits::default(),
                )
                .unwrap();
                assert!(core::ptr::eq(
                    verified.https_destination_complete_revocation(),
                    &evidence,
                ));
                assert!(core::ptr::eq(
                    verified.verification().hashed_input(),
                    &hashed,
                ));
                assert!(core::ptr::eq(
                    verified.verification().verification().signature().message(),
                    &message,
                ));
                assert_eq!(
                    verified
                        .verification()
                        .verification()
                        .signature()
                        .public_key()
                        .modulus(),
                    expected_key.modulus(),
                );
                assert_eq!(
                    format!("{verified:?}"),
                    "VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify",
                );
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
                );
            }
        }
        assert_eq!(&output[..expected_input.len()], expected_input);
        assert!(output[expected_input.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        if !expect_success {
            assert_eq!(scratch, before_scratch);
        }
    }

    fn exercise_recursive_revocation_entry_point(
        role: CertificateVerifyRole,
        expect_success: bool,
    ) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let evidence = TestRecursiveRevocationEvidence { certificate: &leaf };
        let transcript_encoded = fixture(PUBLIC_TRANSCRIPT_HEX);
        let suite_entry =
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
        let selected_suite = CipherSuitePolicy::new(&[suite_entry])
            .unwrap()
            .select(CipherSuite::TlsAes256GcmSha384.wire_id())
            .unwrap();
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
        pending.update(&transcript_encoded).unwrap();
        let transcript = pending.select(selected_suite);
        let scheme = SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
            .unwrap()
            .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
            .unwrap();
        let signature = fixture(PUBLIC_SIGNATURE_HEX);
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let mut scratch = [0xa5; 223];
        let before_scratch = scratch;
        let expected_input;
        {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
            let hashed = hash_transcript_bound_certificate_verify_input(scheme, &bound);
            let message = CertificateVerifyMessage::new(
                SignatureScheme::RsaPssRsaeSha256.wire_id(),
                &signature,
            )
            .unwrap();
            expected_input = bound.input().as_bytes().to_vec();
            let result = verify_https_dns_destination_reviewed_recursive_responder_no_check_complete_fixed_path_revocation_certificate_verify(
                &evidence,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

            if expect_success {
                let verified = result.expect("captured server signature must verify");
                let expected_key = validate_rsa_public_key(
                    leaf.tbs_certificate().subject_public_key_info(),
                    RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                    Asn1Limits::default(),
                )
                .unwrap();
                assert!(core::ptr::eq(
                    verified.https_destination_complete_revocation(),
                    &evidence,
                ));
                assert!(core::ptr::eq(
                    verified.verification().hashed_input(),
                    &hashed,
                ));
                assert!(core::ptr::eq(
                    verified.verification().verification().signature().message(),
                    &message,
                ));
                assert_eq!(
                    verified
                        .verification()
                        .verification()
                        .signature()
                        .public_key()
                        .modulus(),
                    expected_key.modulus(),
                );
                assert_eq!(
                    format!("{verified:?}"),
                    "VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify",
                );
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
                );
            }
        }
        assert_eq!(&output[..expected_input.len()], expected_input);
        assert!(output[expected_input.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        if !expect_success {
            assert_eq!(scratch, before_scratch);
        }
    }

    fn exercise_alternative_revocation_entry_point(
        role: CertificateVerifyRole,
        expect_success: bool,
    ) {
        let leaf_encoded = fixture(PUBLIC_LEAF_HEX);
        let leaf = decode_x509_certificate(&leaf_encoded, Asn1Limits::default()).unwrap();
        let evidence = TestAlternativeRevocationEvidence { certificate: &leaf };
        let transcript_encoded = fixture(PUBLIC_TRANSCRIPT_HEX);
        let suite_entry =
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
        let selected_suite = CipherSuitePolicy::new(&[suite_entry])
            .unwrap()
            .select(CipherSuite::TlsAes256GcmSha384.wire_id())
            .unwrap();
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
        pending.update(&transcript_encoded).unwrap();
        let transcript = pending.select(selected_suite);
        let scheme = SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
            .unwrap()
            .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
            .unwrap();
        let signature = fixture(PUBLIC_SIGNATURE_HEX);
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let mut scratch = [0xa5; 223];
        let before_scratch = scratch;
        let expected_input;
        {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
            let hashed = hash_transcript_bound_certificate_verify_input(scheme, &bound);
            let message = CertificateVerifyMessage::new(
                SignatureScheme::RsaPssRsaeSha256.wire_id(),
                &signature,
            )
            .unwrap();
            expected_input = bound.input().as_bytes().to_vec();
            let result = verify_https_dns_destination_reviewed_intermediate_status_responder_revocation_responder_no_check_complete_fixed_path_revocation_certificate_verify(
                &evidence,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

            if expect_success {
                let verified = result.expect("captured server signature must verify");
                let expected_key = validate_rsa_public_key(
                    leaf.tbs_certificate().subject_public_key_info(),
                    RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                    Asn1Limits::default(),
                )
                .unwrap();
                assert!(core::ptr::eq(
                    verified.https_destination_complete_revocation(),
                    &evidence,
                ));
                assert!(core::ptr::eq(
                    verified.verification().hashed_input(),
                    &hashed,
                ));
                assert!(core::ptr::eq(
                    verified.verification().verification().signature().message(),
                    &message,
                ));
                assert_eq!(
                    verified
                        .verification()
                        .verification()
                        .signature()
                        .public_key()
                        .modulus(),
                    expected_key.modulus(),
                );
                assert_eq!(
                    format!("{verified:?}"),
                    "VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify",
                );
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
                );
            }
        }
        assert_eq!(&output[..expected_input.len()], expected_input);
        assert!(output[expected_input.len()..]
            .iter()
            .all(|byte| *byte == 0xa5));
        if !expect_success {
            assert_eq!(scratch, before_scratch);
        }
    }

    #[test]
    fn direct_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_direct_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn direct_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_direct_revocation_entry_point(CertificateVerifyRole::Client, false);
    }

    #[test]
    fn nested_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_nested_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn nested_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_nested_revocation_entry_point(CertificateVerifyRole::Client, false);
    }

    #[test]
    fn double_nested_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_double_nested_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn double_nested_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_double_nested_revocation_entry_point(CertificateVerifyRole::Client, false);
    }

    #[test]
    fn recursive_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_recursive_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn recursive_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_recursive_revocation_entry_point(CertificateVerifyRole::Client, false);
    }

    #[test]
    fn alternative_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_alternative_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn alternative_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_alternative_revocation_entry_point(CertificateVerifyRole::Client, false);
    }

    #[test]
    fn ordered_transition_retains_exact_verification_and_client_auth_requirement() {
        let mut server_flight = NoPskServerFlight::new();
        server_flight
            .accept(Tls13HandshakeType::EncryptedExtensions)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::CertificateRequest)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::Certificate)
            .unwrap();
        let verification = TestCompleteRevocationCertificateVerify {
            selected_suite: selected_suite(CipherSuite::TlsAes128GcmSha256),
        };

        {
            let ordered =
                advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify(
                    &mut server_flight,
                    &verification,
                )
                .unwrap();

            assert_eq!(
                ordered.server_flight().state(),
                ServerFlightState::ExpectFinished
            );
            assert!(ordered.server_flight().client_authentication_requested());
            assert!(core::ptr::eq(ordered.verification(), &verification));
            assert_eq!(
                format!("{ordered:?}"),
                "OrderedNoPskServerFlightReviewedRevocationCertificateVerify",
            );
        }

        assert_eq!(server_flight.state(), ServerFlightState::ExpectFinished);
        assert!(server_flight.client_authentication_requested());
    }

    #[test]
    fn ordered_transition_rejects_wrong_state_without_mutation() {
        let mut server_flight = NoPskServerFlight::new();
        let verification = TestCompleteRevocationCertificateVerify {
            selected_suite: selected_suite(CipherSuite::TlsAes128GcmSha256),
        };

        let error = advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify(
            &mut server_flight,
            &verification,
        )
        .unwrap_err();

        assert_eq!(error.kind(), ServerFlightErrorKind::UnexpectedMessage);
        assert_eq!(
            server_flight.state(),
            ServerFlightState::ExpectEncryptedExtensions,
        );
        assert!(!server_flight.client_authentication_requested());
    }

    #[test]
    fn finished_transition_completes_exact_flight_and_retains_both_verdicts() {
        let suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let verification = TestCompleteRevocationCertificateVerify {
            selected_suite: suite,
        };
        let finished = verified_finished(suite, FinishedRole::Server);
        let mut server_flight = NoPskServerFlight::new();
        server_flight
            .accept(Tls13HandshakeType::EncryptedExtensions)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::CertificateRequest)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::Certificate)
            .unwrap();
        let ordered = advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify(
            &mut server_flight,
            &verification,
        )
        .unwrap();

        {
            let completed = complete_no_psk_server_flight_after_reviewed_revocation_finished(
                ordered, &finished,
            )
            .unwrap();

            assert_eq!(
                completed.server_flight().state(),
                ServerFlightState::Complete
            );
            assert!(completed.server_flight().client_authentication_requested());
            assert!(core::ptr::eq(completed.verification(), &verification));
            assert!(core::ptr::eq(completed.finished(), &finished));
            assert_eq!(
                format!("{completed:?}"),
                "CompletedNoPskServerFlightReviewedRevocationCertificateVerifyFinished",
            );
        }

        assert!(server_flight.is_complete());
        assert!(server_flight.client_authentication_requested());
    }

    #[test]
    fn finished_transition_rejects_client_role_without_flight_mutation() {
        let suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let verification = TestCompleteRevocationCertificateVerify {
            selected_suite: suite,
        };
        let finished = verified_finished(suite, FinishedRole::Client);
        let mut server_flight = NoPskServerFlight::new();
        server_flight
            .accept(Tls13HandshakeType::EncryptedExtensions)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::Certificate)
            .unwrap();
        let ordered = advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify(
            &mut server_flight,
            &verification,
        )
        .unwrap();

        let error =
            complete_no_psk_server_flight_after_reviewed_revocation_finished(ordered, &finished)
                .unwrap_err();

        assert_eq!(
            error.kind(),
            ReviewedServerFinishedTransitionErrorKind::WrongEndpointRole,
        );
        assert_eq!(server_flight.state(), ServerFlightState::ExpectFinished);
        assert!(!server_flight.client_authentication_requested());
    }

    #[test]
    fn finished_transition_rejects_suite_substitution_without_flight_mutation() {
        let certificate_verify_suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let finished_suite = selected_suite(CipherSuite::TlsChacha20Poly1305Sha256);
        let verification = TestCompleteRevocationCertificateVerify {
            selected_suite: certificate_verify_suite,
        };
        let finished = verified_finished(finished_suite, FinishedRole::Server);
        let mut server_flight = NoPskServerFlight::new();
        server_flight
            .accept(Tls13HandshakeType::EncryptedExtensions)
            .unwrap();
        server_flight
            .accept(Tls13HandshakeType::Certificate)
            .unwrap();
        let ordered = advance_no_psk_server_flight_after_reviewed_revocation_certificate_verify(
            &mut server_flight,
            &verification,
        )
        .unwrap();

        let error =
            complete_no_psk_server_flight_after_reviewed_revocation_finished(ordered, &finished)
                .unwrap_err();

        assert_eq!(
            error.kind(),
            ReviewedServerFinishedTransitionErrorKind::CipherSuiteMismatch,
        );
        assert_eq!(server_flight.state(), ServerFlightState::ExpectFinished);
        assert!(!server_flight.client_authentication_requested());
    }

    #[test]
    fn stable_error_discloses_no_payload() {
        let error = ReviewedServerPathCertificateVerifyError {
            kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
        };
        let display = error.to_string();
        let debug = format!("{error:?}");

        assert_eq!(
            display,
            "reviewed server-path CertificateVerify error WrongEndpointRole"
        );
        assert_eq!(
            debug,
            "ReviewedServerPathCertificateVerifyError { kind: WrongEndpointRole }"
        );
        for secret in ["api.example.test", "transcript", "signature", "modulus"] {
            assert!(!display.contains(secret));
            assert!(!debug.contains(secret));
        }

        let finished_error = ReviewedServerFinishedTransitionError {
            kind: ReviewedServerFinishedTransitionErrorKind::CipherSuiteMismatch,
        };
        let finished_display = finished_error.to_string();
        let finished_debug = format!("{finished_error:?}");
        assert_eq!(
            finished_display,
            "reviewed TLS 1.3 server Finished transition error CipherSuiteMismatch",
        );
        assert_eq!(
            finished_debug,
            "ReviewedServerFinishedTransitionError { kind: CipherSuiteMismatch }",
        );
        for secret in ["verify-data", "transcript", "signature", "suite 0x1301"] {
            assert!(!finished_display.contains(secret));
            assert!(!finished_debug.contains(secret));
        }
    }
}
