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
//! `CertificateVerify`, then advances that state to expect `Finished`. It does
//! not authenticate Finished, records, TLS, HTTP, OAuth, or network readiness.

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
use tls13_certificate_verify_input::CertificateVerifyRole;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    verify_transcript_bound_certificate_verify, HashedTranscriptBoundCertificateVerifyInput,
    VerifiedTranscriptBoundCertificateVerify,
};
use tls13_handshake::Tls13HandshakeType;
use tls13_no_psk_server_flight::{NoPskServerFlight, ServerFlightError};
use tls13_rsa_pss_certificate_verify::Tls13RsaPssCertificateVerifyErrorKind;
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
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

mod complete_revocation_certificate_verify_sealed {
    pub trait Sealed {}
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
        alternative_revocation_evidence_sealed, complete_revocation_certificate_verify_sealed,
        direct_revocation_evidence_sealed, double_nested_revocation_evidence_sealed,
        nested_revocation_evidence_sealed, recursive_revocation_evidence_sealed,
        require_server_role, validate_leaf_public_key,
        verify_https_dns_destination_reviewed_direct_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_double_nested_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_intermediate_status_responder_revocation_responder_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_nested_no_check_complete_fixed_path_revocation_certificate_verify,
        verify_https_dns_destination_reviewed_recursive_responder_no_check_complete_fixed_path_revocation_certificate_verify,
        OrderedNoPskServerFlightReviewedRevocationCertificateVerify,
        ReviewedServerPathCertificateVerifyError, ReviewedServerPathCertificateVerifyErrorKind,
        VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedDoubleNestedNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedNestedNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedHttpsDnsDestinationReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationCertificateVerify,
        VerifiedReviewedServerPathCertificateVerify,
    };
    use der_asn1::Asn1Limits;
    use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
    use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
    use tls13_certificate_verify_message::CertificateVerifyMessage;
    use tls13_certificate_verify_transcript_binding::{
        bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    };
    use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
    use tls13_handshake::Tls13HandshakeType;
    use tls13_no_psk_server_flight::{NoPskServerFlight, ServerFlightErrorKind, ServerFlightState};
    use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
    use tls13_signature_scheme_policy::{SignatureScheme, SignatureSchemePolicy};
    use tls13_transcript::{PendingTls13Transcript, TranscriptPolicy};
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

    struct TestCompleteRevocationCertificateVerify;

    impl complete_revocation_certificate_verify_sealed::Sealed
        for TestCompleteRevocationCertificateVerify
    {
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
        let verification = TestCompleteRevocationCertificateVerify;

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
        let verification = TestCompleteRevocationCertificateVerify;

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
    }
}
