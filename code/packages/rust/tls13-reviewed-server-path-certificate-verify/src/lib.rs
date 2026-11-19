//! TLS 1.3 server `CertificateVerify` bound to a reviewed RSA path leaf.
//!
//! The path-only composition accepts selected-transcript hash evidence and the
//! strongest accumulated reviewed RSA server-path proof. Stronger HTTPS entry
//! points accept that path only through the existing exact DNS destination
//! binding, with or without retained anchor self-signature evidence. A direct
//! revocation entry point accepts the leaf only through the destination-bound
//! complete fixed-path proof. All entry points authorize the server role,
//! derive and validate the exact retained target certificate's RSA key, and
//! delegate signature verification to repository-owned code. They do not
//! establish ordered handshake state, Finished, records, TLS, HTTP, OAuth, or
//! network readiness.

#![forbid(unsafe_code)]

use core::fmt;

use der_asn1::Asn1Limits;
use https_reviewed_server_path_binding::{
    BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedServerPath,
    BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature, ReviewedHttpsServerPath,
};
use tls13_certificate_verify_input::CertificateVerifyRole;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    verify_transcript_bound_certificate_verify, HashedTranscriptBoundCertificateVerifyInput,
    VerifiedTranscriptBoundCertificateVerify,
};
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
        direct_revocation_evidence_sealed, require_server_role, validate_leaf_public_key,
        verify_https_dns_destination_reviewed_direct_no_check_complete_fixed_path_revocation_certificate_verify,
        ReviewedServerPathCertificateVerifyError, ReviewedServerPathCertificateVerifyErrorKind,
        VerifiedHttpsDnsDestinationReviewedDirectNoCheckCompleteFixedPathRevocationCertificateVerify,
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

    #[test]
    fn direct_revocation_entry_point_retains_exact_evidence_and_signature() {
        exercise_direct_revocation_entry_point(CertificateVerifyRole::Server, true);
    }

    #[test]
    fn direct_revocation_entry_point_rejects_client_before_scratch_mutation() {
        exercise_direct_revocation_entry_point(CertificateVerifyRole::Client, false);
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
