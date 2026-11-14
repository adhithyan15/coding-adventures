//! # Reviewed-server OCSP revocation binding
//!
//! This crate composes one complete authenticated OCSP target-revocation proof
//! with either the fixed RSA SHA-256 or the data-driven routed constructed
//! reviewed-server path evidence. The routed path can consume the fixed RSA
//! SHA-256 OCSP proof, its routed-response counterpart, or the fully routed
//! result whose response, responder-certificate, and target-certificate
//! signatures each preserve data-driven RSA SHA-256, SHA-384, or SHA-512
//! selection. The target and issuer must be the same exact decoded certificate
//! objects in both inputs.
//! Byte-equal independent decodes are rejected.
//! The fully routed proof can also bind to the strongest supported Name-policy-
//! backed arbitrary-depth path after target, intermediate, subordinate-CA, and
//! reviewed-anchor Name-constraint processing. That bridge retains both
//! complete inputs plus exact-observation target and target-nearest issuer
//! validity without authorizing the responder path or completing revocation.
//!
//! This is an evidence-composition, exact-time validity, bounded responder
//! DNS-policy, and exact responder no-check boundary for the fixed path. A
//! fully routed binding can also be reified as one exact responder path,
//! checked against its selected anchor's and exact intermediate's supported
//! DNS constraints, checked for exact responder no-check policy, or joined to
//! separately authenticated revocation evidence for that exact responder at
//! the original observation time. A separate routed proof can bind exact
//! intermediate status to the selected reviewed root at that same time, and
//! the routed no-check branch can consume that status evidence directly under
//! a distinct retained type. That composition can reify the added status
//! proof's own direct delegated responder-to-reviewed-root path for subsequent
//! policy without accepting substitute evidence, then apply the exact selected
//! root's supported out-of-band DNS constraints and either RFC 6960 no-check
//! policy or separately authenticated exact revocation to that added responder.
//! The no-check, separately authenticated original-responder, and nested
//! responder-of-responder no-check branches can project their added
//! intermediate-status responder through that same policy sequence without
//! accepting substitute evidence. Routed flows still stop before complete
//! revocation composition. The separately authenticated original-responder
//! branch can also reify its added revocation proof's own exact responder-to-
//! reviewed-root path while retaining the complete branch, and apply the
//! selected root's supported out-of-band DNS constraints to that exact
//! recursive responder. It stops before intermediate name constraints and
//! no-check or further authenticated revocation policy for that responder.
//! They perform no new cryptography, path search,
//! revocation lookup, clock, TLS, HTTP, OAuth, network, or external-provider
//! operation. A projection retains the complete proof under a distinct type
//! that names the delegated responder's already-proven exact path through the
//! policy-validated intermediate to the reviewed root; subsequent policies
//! derive the responder SAN plus reviewed-anchor and exact-intermediate
//! constraints internally. The strongest Name-policy-backed path projection
//! can now also apply its exact selected reviewed root's supported out-of-band
//! permitted-DNS constraints to that retained responder without accepting
//! detached evidence. It still stops before intermediate DNS, responder no-
//! check or authenticated revocation, complete Name/path or revocation policy,
//! TLS, HTTP, OAuth, or network readiness.

#![deny(unsafe_code)]

use core::{fmt, marker::PhantomData};
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate_policy::{
    validate_certificate_validity_at, ValidatedCertificateValidity,
    X509CertificateValidityErrorKind,
};
use x509_name_constraints::{
    decode_name_constraints, decode_name_constraints_contents, evaluate_dns_names,
    ConstrainedDnsNames, DnsConstraintErrorKind, NameConstraintsErrorKind, NAME_CONSTRAINTS_OID,
};
use x509_ocsp_authenticated_target_revocation::{
    AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation,
    AuthenticatedX509OcspRsaSha256TargetRevocation, AuthenticatedX509OcspRsaTargetRevocation,
};
use x509_rsa_constructed_path_signature_routing::{
    ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy,
    ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy,
};
use x509_rsa_sha256_reviewed_server_link::ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy;
use x509_subject_alt_name::{
    decode_subject_alt_name, SubjectAltNameErrorKind, SUBJECT_ALT_NAME_OID,
};

/// RFC 6960 `id-pkix-ocsp-nocheck` certificate-extension object identifier.
pub const OCSP_NO_CHECK_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 48, 1, 5];

const DER_NULL: &[u8] = &[0x05, 0x00];

/// Stable payload-free failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedServerRevocationBindingErrorKind {
    TargetMismatch,
    IssuerMismatch,
    TargetCertificateValidity(X509CertificateValidityErrorKind),
    IssuerCertificateValidity(X509CertificateValidityErrorKind),
}

/// A redacted reviewed-server revocation binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedServerRevocationBindingError {
    kind: X509OcspReviewedServerRevocationBindingErrorKind,
}

impl X509OcspReviewedServerRevocationBindingError {
    const fn target_mismatch() -> Self {
        Self {
            kind: X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch,
        }
    }

    const fn issuer_mismatch() -> Self {
        Self {
            kind: X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch,
        }
    }

    const fn target_certificate_validity(kind: X509CertificateValidityErrorKind) -> Self {
        Self {
            kind: X509OcspReviewedServerRevocationBindingErrorKind::TargetCertificateValidity(kind),
        }
    }

    const fn issuer_certificate_validity(kind: X509CertificateValidityErrorKind) -> Self {
        Self {
            kind: X509OcspReviewedServerRevocationBindingErrorKind::IssuerCertificateValidity(kind),
        }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedServerRevocationBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedServerRevocationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-server revocation binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedServerRevocationBindingError {}

fn retain_path_validity_at_observation<IssuerValidity>(
    target_validity: Result<ValidatedCertificateValidity, X509CertificateValidityErrorKind>,
    issuer_validity: IssuerValidity,
) -> Result<
    (ValidatedCertificateValidity, ValidatedCertificateValidity),
    X509OcspReviewedServerRevocationBindingError,
>
where
    IssuerValidity:
        FnOnce() -> Result<ValidatedCertificateValidity, X509CertificateValidityErrorKind>,
{
    let target_validity = target_validity
        .map_err(X509OcspReviewedServerRevocationBindingError::target_certificate_validity)?;
    let issuer_validity = issuer_validity()
        .map_err(X509OcspReviewedServerRevocationBindingError::issuer_certificate_validity)?;
    Ok((target_validity, issuer_validity))
}

/// Non-copyable evidence joining exact revocation to one reviewed server path.
///
/// The retained revocation input proves that one issuer-authorized delegated
/// responder signed an exact, good, fresh, single-status response for the
/// exact target and issuer and that the issuer signed the target. The retained
/// path input proves target-server policy, two RSA SHA-256 links to an exact
/// reviewed Mozilla root, path-length policy, selected-anchor DNS policy, and
/// supported intermediate DNS constraints for those same exact target and
/// issuer objects.
///
/// This wrapper does not establish responder-path trust or revocation, general
/// certificate-path validation, complete extension policy, TLS, HTTP, OAuth
/// transport, or network authority.
pub struct BoundX509OcspReviewedServerRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_revocation: AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    reviewed_server_path:
        ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    target_validity_at_observation: ValidatedCertificateValidity,
    issuer_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
    BoundX509OcspReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed complete authenticated target-revocation proof.
    pub const fn authenticated_revocation(
        &self,
    ) -> &AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.authenticated_revocation
    }

    /// Return the consumed complete constructed reviewed-server path proof.
    pub const fn reviewed_server_path(
        &self,
    ) -> &ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.reviewed_server_path
    }

    /// Return target-certificate validity at the OCSP observation time.
    pub const fn target_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.target_validity_at_observation
    }

    /// Return issuer-certificate validity at the OCSP observation time.
    pub const fn issuer_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.issuer_validity_at_observation
    }
}

impl fmt::Debug
    for BoundX509OcspReviewedServerRevocation<
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
        formatter.write_str("BoundX509OcspReviewedServerRevocation")
    }
}

/// Non-copyable evidence joining exact revocation to one routed server path.
///
/// The retained revocation input proves one exact authenticated RSA SHA-256
/// OCSP status. The retained routed path proves target-server policy, two
/// data-driven RSA SHA-256, SHA-384, or SHA-512 links to an exact reviewed
/// Mozilla root, path-length policy, selected-anchor DNS policy, and supported
/// intermediate DNS/IP constraints for the same exact target and issuer.
///
/// This wrapper does not reify routed responder-path policy, check responder or
/// intermediate revocation, generalize certificate-path validation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspRoutedReviewedServerRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_revocation: AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    target_validity_at_observation: ValidatedCertificateValidity,
    issuer_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
    BoundX509OcspRoutedReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed complete authenticated target-revocation proof.
    #[must_use]
    pub const fn authenticated_revocation(
        &self,
    ) -> &AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.authenticated_revocation
    }

    /// Return the consumed complete routed reviewed-server path proof.
    #[must_use]
    pub const fn routed_reviewed_server_path(
        &self,
    ) -> &ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.routed_reviewed_server_path
    }

    /// Return target-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn target_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.target_validity_at_observation
    }

    /// Return issuer-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn issuer_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.issuer_validity_at_observation
    }
}

impl fmt::Debug
    for BoundX509OcspRoutedReviewedServerRevocation<
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
        formatter.write_str("BoundX509OcspRoutedReviewedServerRevocation")
    }
}

/// Non-copyable evidence joining routed OCSP revocation to one routed path.
///
/// The retained revocation input proves one exact authenticated OCSP status
/// whose response signature was selected by data-driven RSA SHA-256,
/// SHA-384, or SHA-512 policy. Its responder and target certificate issuer
/// links remain fixed RSA SHA-256 proofs. The retained routed path proves
/// target-server policy plus independently routed RSA SHA-256, SHA-384, or
/// SHA-512 verification for both exact certificate adjacencies to one reviewed
/// Mozilla root.
///
/// This wrapper does not route the OCSP certificate links, reify routed
/// responder-path policy, check responder or intermediate revocation,
/// generalize certificate-path validation, or grant TLS, HTTP, OAuth, or
/// network authority.
pub struct BoundX509OcspRoutedResponseReviewedServerRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_revocation: AuthenticatedX509OcspRsaTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    target_validity_at_observation: ValidatedCertificateValidity,
    issuer_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
    BoundX509OcspRoutedResponseReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed routed authenticated target-revocation proof.
    #[must_use]
    pub const fn authenticated_revocation(
        &self,
    ) -> &AuthenticatedX509OcspRsaTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.authenticated_revocation
    }

    /// Return the consumed routed reviewed-server path proof.
    #[must_use]
    pub const fn routed_reviewed_server_path(
        &self,
    ) -> &ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.routed_reviewed_server_path
    }

    /// Return target-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn target_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.target_validity_at_observation
    }

    /// Return issuer-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn issuer_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.issuer_validity_at_observation
    }
}

impl fmt::Debug
    for BoundX509OcspRoutedResponseReviewedServerRevocation<
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
        formatter.write_str("BoundX509OcspRoutedResponseReviewedServerRevocation")
    }
}

/// Non-copyable evidence joining fully routed OCSP revocation to one routed path.
///
/// The retained revocation input proves one exact authenticated OCSP status
/// whose response signature, responder-certificate issuer signature, and
/// target-certificate issuer signature were each selected by data-driven RSA
/// SHA-256, SHA-384, or SHA-512 policy. The retained routed path independently
/// proves both exact certificate adjacencies to one reviewed Mozilla root.
///
/// This wrapper can be nominally reified as one exact routed responder path by
/// a separate type-restricted constructor. It does not apply routed responder
/// DNS, no-check, or revocation policy, generalize certificate-path validation,
/// or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspRoutedIssuerReviewedServerRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
> {
    authenticated_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    target_validity_at_observation: ValidatedCertificateValidity,
    issuer_validity_at_observation: ValidatedCertificateValidity,
}

/// Fully routed OCSP status bound to the strongest supported Name-policy path.
///
/// The concrete producer accepts only a fully routed authenticated OCSP target-
/// revocation proof and the complete Name-policy-backed arbitrary-depth path
/// after supported target, intermediate, subordinate-CA, and reviewed-anchor
/// Name constraints. It requires exact target and target-nearest issuer object
/// identity, then validates both path certificates at the retained OCSP
/// observation time.
///
/// This non-copyable wrapper does not fetch status, authorize the delegated
/// responder path, establish status for every non-root certificate, complete
/// Name or path validation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
    Revocation,
    StrongNamePolicyPath,
> {
    authenticated_revocation: Revocation,
    strong_name_policy_path: StrongNamePolicyPath,
    target_validity_at_observation: ValidatedCertificateValidity,
    issuer_validity_at_observation: ValidatedCertificateValidity,
}

impl<Revocation, StrongNamePolicyPath>
    BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        Revocation,
        StrongNamePolicyPath,
    >
{
    /// Return the consumed fully routed authenticated target-revocation proof.
    #[must_use]
    pub const fn authenticated_revocation(&self) -> &Revocation {
        &self.authenticated_revocation
    }

    /// Return the consumed strongest supported Name-policy path proof.
    #[must_use]
    pub const fn strong_name_policy_path(&self) -> &StrongNamePolicyPath {
        &self.strong_name_policy_path
    }

    /// Return target-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn target_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.target_validity_at_observation
    }

    /// Return issuer-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn issuer_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.issuer_validity_at_observation
    }
}

impl<Revocation, StrongNamePolicyPath> fmt::Debug
    for BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        Revocation,
        StrongNamePolicyPath,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation")
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
    BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the consumed fully routed authenticated target-revocation proof.
    #[must_use]
    pub const fn authenticated_revocation(
        &self,
    ) -> &AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.authenticated_revocation
    }

    /// Return the consumed routed reviewed-server path proof.
    #[must_use]
    pub const fn routed_reviewed_server_path(
        &self,
    ) -> &ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        &self.routed_reviewed_server_path
    }

    /// Return target-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn target_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.target_validity_at_observation
    }

    /// Return issuer-certificate validity at the OCSP observation time.
    #[must_use]
    pub const fn issuer_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.issuer_validity_at_observation
    }
}

impl fmt::Debug
    for BoundX509OcspRoutedIssuerReviewedServerRevocation<
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
        formatter.write_str("BoundX509OcspRoutedIssuerReviewedServerRevocation")
    }
}

/// Bind authenticated revocation to the same exact reviewed server path.
///
/// Failure order is stable: target identity, issuer identity, target validity,
/// then issuer validity. Object identity is required so independently decoded
/// byte-equal certificates cannot cross this boundary. Success retains both
/// complete non-copyable inputs plus validity of those exact path certificates
/// at the OCSP observation time, without external effects.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_revocation_to_reviewed_server_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    authenticated_revocation: AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    reviewed_server_path:
        ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
) -> Result<
    BoundX509OcspReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509OcspReviewedServerRevocationBindingError,
> {
    let revocation_target = authenticated_revocation.target_issuer_link().child();
    let server_path = reviewed_server_path
        .anchor_policy()
        .reviewed_intermediate_server_path();
    let path_target = server_path.server_policy().certificate();
    if !std::ptr::eq(revocation_target, path_target) {
        return Err(X509OcspReviewedServerRevocationBindingError::target_mismatch());
    }

    let revocation_issuer = authenticated_revocation
        .target_issuer_link()
        .candidate_issuer();
    let path_issuer = server_path
        .reviewed_intermediate_path()
        .authenticated_path()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    if !std::ptr::eq(revocation_issuer, path_issuer) {
        return Err(X509OcspReviewedServerRevocationBindingError::issuer_mismatch());
    }

    let observation_time = authenticated_revocation.revocation().observation_time();
    let (target_validity_at_observation, issuer_validity_at_observation) =
        retain_path_validity_at_observation(
            validate_certificate_validity_at(path_target, &observation_time)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(path_issuer, &observation_time)
                    .map_err(|error| error.kind())
            },
        )?;

    Ok(BoundX509OcspReviewedServerRevocation {
        authenticated_revocation,
        reviewed_server_path,
        target_validity_at_observation,
        issuer_validity_at_observation,
    })
}

/// Bind authenticated revocation to the same exact routed reviewed path.
///
/// Failure order is stable: target identity, issuer identity, target validity,
/// then issuer validity. Object identity is required so independently decoded
/// byte-equal certificates cannot cross this boundary. Success retains both
/// complete non-copyable inputs plus validity of those exact path certificates
/// at the OCSP observation time, without external effects.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_revocation_to_routed_reviewed_server_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    authenticated_revocation: AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
) -> Result<
    BoundX509OcspRoutedReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509OcspReviewedServerRevocationBindingError,
> {
    let revocation_target = authenticated_revocation.target_issuer_link().child();
    let server_path = routed_reviewed_server_path
        .anchor_policy()
        .reviewed_server_path();
    let path_target = server_path.server_policy().certificate();
    if !std::ptr::eq(revocation_target, path_target) {
        return Err(X509OcspReviewedServerRevocationBindingError::target_mismatch());
    }

    let revocation_issuer = authenticated_revocation
        .target_issuer_link()
        .candidate_issuer();
    let path_issuer = server_path
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    if !std::ptr::eq(revocation_issuer, path_issuer) {
        return Err(X509OcspReviewedServerRevocationBindingError::issuer_mismatch());
    }

    let observation_time = authenticated_revocation.revocation().observation_time();
    let (target_validity_at_observation, issuer_validity_at_observation) =
        retain_path_validity_at_observation(
            validate_certificate_validity_at(path_target, &observation_time)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(path_issuer, &observation_time)
                    .map_err(|error| error.kind())
            },
        )?;

    Ok(BoundX509OcspRoutedReviewedServerRevocation {
        authenticated_revocation,
        routed_reviewed_server_path,
        target_validity_at_observation,
        issuer_validity_at_observation,
    })
}

/// Bind routed authenticated revocation to the same exact routed path.
///
/// Failure order is stable: target identity, issuer identity, target validity,
/// then issuer validity. Object identity is required so independently decoded
/// byte-equal certificates cannot cross this boundary. Success retains both
/// complete non-copyable routed inputs plus validity of those exact path
/// certificates at the OCSP observation time, without external effects.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_response_to_routed_reviewed_server_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    authenticated_revocation: AuthenticatedX509OcspRsaTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
) -> Result<
    BoundX509OcspRoutedResponseReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509OcspReviewedServerRevocationBindingError,
> {
    let revocation_target = authenticated_revocation.target_issuer_link().child();
    let server_path = routed_reviewed_server_path
        .anchor_policy()
        .reviewed_server_path();
    let path_target = server_path.server_policy().certificate();
    if !std::ptr::eq(revocation_target, path_target) {
        return Err(X509OcspReviewedServerRevocationBindingError::target_mismatch());
    }

    let revocation_issuer = authenticated_revocation
        .target_issuer_link()
        .candidate_issuer();
    let path_issuer = server_path
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    if !std::ptr::eq(revocation_issuer, path_issuer) {
        return Err(X509OcspReviewedServerRevocationBindingError::issuer_mismatch());
    }

    let observation_time = authenticated_revocation.revocation().observation_time();
    let (target_validity_at_observation, issuer_validity_at_observation) =
        retain_path_validity_at_observation(
            validate_certificate_validity_at(path_target, &observation_time)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(path_issuer, &observation_time)
                    .map_err(|error| error.kind())
            },
        )?;

    Ok(BoundX509OcspRoutedResponseReviewedServerRevocation {
        authenticated_revocation,
        routed_reviewed_server_path,
        target_validity_at_observation,
        issuer_validity_at_observation,
    })
}

/// Bind fully routed authenticated revocation to the same exact routed path.
///
/// Failure order is stable: target identity, issuer identity, target validity,
/// then issuer validity. Object identity is required so independently decoded
/// byte-equal certificates cannot cross this boundary. Success retains both
/// complete non-copyable routed inputs plus validity of those exact path
/// certificates at the OCSP observation time.
///
/// This pure public-data composition performs no new cryptography, credential
/// access, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, or external effect. It does not reify responder-path
/// policy, perform TLS or HTTP, authorize OAuth transport, or grant network
/// authority.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    authenticated_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    routed_reviewed_server_path:
        ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy<
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
) -> Result<
    BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    X509OcspReviewedServerRevocationBindingError,
> {
    let target_signature = authenticated_revocation.target_issuer_signature();
    let revocation_target = target_signature.child_algorithm_policy().certificate();
    let server_path = routed_reviewed_server_path
        .anchor_policy()
        .reviewed_server_path();
    let path_target = server_path.server_policy().certificate();
    if !std::ptr::eq(revocation_target, path_target) {
        return Err(X509OcspReviewedServerRevocationBindingError::target_mismatch());
    }

    let revocation_issuer = target_signature.issuer_rsa_algorithm_policy().certificate();
    let path_issuer = server_path
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    if !std::ptr::eq(revocation_issuer, path_issuer) {
        return Err(X509OcspReviewedServerRevocationBindingError::issuer_mismatch());
    }

    let observation_time = authenticated_revocation.revocation().observation_time();
    let (target_validity_at_observation, issuer_validity_at_observation) =
        retain_path_validity_at_observation(
            validate_certificate_validity_at(path_target, &observation_time)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(path_issuer, &observation_time)
                    .map_err(|error| error.kind())
            },
        )?;

    Ok(BoundX509OcspRoutedIssuerReviewedServerRevocation {
        authenticated_revocation,
        routed_reviewed_server_path,
        target_validity_at_observation,
        issuer_validity_at_observation,
    })
}

/// Bind fully routed OCSP status to the strongest supported Name-policy path.
///
/// Failure order is stable: target identity, issuer identity, target validity,
/// then issuer validity. The target comes only from the retained complete
/// server policy, while the target-nearest issuer comes only from the retained
/// depth-zero routed signature. Independently decoded byte-equal certificates
/// cannot cross this boundary. Success retains both complete non-copyable
/// inputs and exact-time validity evidence without external effects.
///
/// This does not fetch status, authorize or reify responder-path policy,
/// establish status for every non-root certificate, complete Name or path
/// validation, perform TLS or HTTP, authorize OAuth transport, or grant network
/// authority.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'revocation_target,
    'revocation_target_encoded,
    'revocation_issuer,
    'revocation_issuer_encoded,
    'evidence,
    'initialized,
    'path,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    authenticated_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'revocation_target,
        'revocation_target_encoded,
        'revocation_issuer,
        'revocation_issuer_encoded,
    >,
    strong_name_policy_path:
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
) -> Result<
    BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'revocation_target,
            'revocation_target_encoded,
            'revocation_issuer,
            'revocation_issuer_encoded,
        >,
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    >,
    X509OcspReviewedServerRevocationBindingError,
>{
    let target_signature = authenticated_revocation.target_issuer_signature();
    let revocation_target = target_signature.child_algorithm_policy().certificate();
    let target_closure = strong_name_policy_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure();
    let path_target = target_closure.server_policy().certificate();
    if !std::ptr::eq(revocation_target, path_target) {
        return Err(X509OcspReviewedServerRevocationBindingError::target_mismatch());
    }

    let revocation_issuer = target_signature.issuer_rsa_algorithm_policy().certificate();
    let path_issuer = target_closure
        .target_signature()
        .signature()
        .issuer_rsa_algorithm_policy()
        .certificate();
    if !std::ptr::eq(revocation_issuer, path_issuer) {
        return Err(X509OcspReviewedServerRevocationBindingError::issuer_mismatch());
    }

    let observation_time = authenticated_revocation.revocation().observation_time();
    let (target_validity_at_observation, issuer_validity_at_observation) =
        retain_path_validity_at_observation(
            validate_certificate_validity_at(path_target, &observation_time)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(path_issuer, &observation_time)
                    .map_err(|error| error.kind())
            },
        )?;

    Ok(
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation {
            authenticated_revocation,
            strong_name_policy_path,
            target_validity_at_observation,
            issuer_validity_at_observation,
        },
    )
}

/// Non-copyable evidence extending one reviewed revocation proof to its responder.
///
/// The consumed input already retains complete responder policy, its exact
/// authenticated responder-to-issuer link, the same issuer's complete
/// intermediate policy and path-length state, its authenticated link to the
/// exact Mozilla-reviewed root, and responder/issuer validity at the OCSP
/// observation time. Routed inputs additionally retain every selected RSA
/// signature route across the OCSP response, responder and target certificate
/// issuer links, and reviewed server path. The consumed type's safe
/// constructors already prove that the responder-policy object is the
/// responder-link child and that the responder-link issuer is the reviewed-path
/// intermediate. This wrapper reifies that transitive invariant for later
/// path-policy composition; it performs no additional validation.
///
/// The fixed RSA SHA-256 constructor does not apply name constraints to the
/// responder certificate. The routed constructor additionally stops before
/// responder DNS and no-check policy. Neither form checks responder or
/// intermediate revocation, generalizes path construction or validation,
/// performs TLS or HTTP, authorizes OAuth transport, or grants network
/// authority.
pub struct BoundX509OcspReviewedResponderPath<Proof> {
    reviewed_server_revocation: Proof,
}

impl<Proof> BoundX509OcspReviewedResponderPath<Proof> {
    /// Return the complete reviewed-server revocation proof retained intact.
    pub const fn reviewed_server_revocation(&self) -> &Proof {
        &self.reviewed_server_revocation
    }
}

impl<Proof> fmt::Debug for BoundX509OcspReviewedResponderPath<Proof> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedResponderPath")
    }
}

/// Reify the authenticated responder's exact reviewed path suffix.
///
/// Earlier safe constructors already enforce every exact-object identity in
/// this path. This infallible nominal projection retains the complete prior
/// proof without new validation, cryptography, clock reads, network access,
/// credential access, or external effects.
#[allow(clippy::type_complexity)]
pub fn reify_x509_ocsp_reviewed_responder_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    reviewed_server_revocation: BoundX509OcspReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> BoundX509OcspReviewedResponderPath<
    BoundX509OcspReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
> {
    BoundX509OcspReviewedResponderPath {
        reviewed_server_revocation,
    }
}

/// Reify the fully routed authenticated responder's exact reviewed path suffix.
///
/// Earlier safe constructors already enforce every exact-object identity in
/// the responder-to-issuer and issuer-to-reviewed-root path. This infallible
/// nominal projection retains the complete proof, all five selected RSA
/// signature routes, and responder, intermediate, and target validity at the
/// OCSP observation time. It accepts no caller-supplied certificate, path,
/// algorithm, key, signature, time, or policy input.
///
/// This pure public-data composition performs no parsing, cryptography,
/// credential access or editing, signing, storage, clock acquisition, provider
/// action, audit or trace emission, I/O, network access, or other external
/// effect. It stops before routed responder DNS, no-check, or revocation policy,
/// generalized path validation, TLS, HTTP, OAuth readiness, or network
/// authority.
#[allow(clippy::type_complexity)]
pub fn reify_x509_ocsp_routed_issuer_reviewed_responder_path<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    reviewed_server_revocation: BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> BoundX509OcspReviewedResponderPath<
    BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
> {
    BoundX509OcspReviewedResponderPath {
        reviewed_server_revocation,
    }
}

/// Reify the authenticated responder suffix retained by the strong path bridge.
///
/// The consumed binding can only be produced after fully routed OCSP target
/// status is joined by exact target and target-nearest issuer identity to the
/// strongest supported Name-policy reviewed-server path. This infallible
/// nominal projection retains that complete non-copyable binding and accepts
/// no detached evidence.
///
/// This performs no validation, parsing, cryptography, clock acquisition, I/O,
/// or other external effect. It stops before responder DNS, no-check, responder
/// or intermediate revocation, complete Name/path or revocation validation,
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn reify_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_path<
    Revocation,
    StrongNamePolicyPath,
>(
    strong_binding: BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        Revocation,
        StrongNamePolicyPath,
    >,
) -> BoundX509OcspReviewedResponderPath<
    BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        Revocation,
        StrongNamePolicyPath,
    >,
> {
    BoundX509OcspReviewedResponderPath {
        reviewed_server_revocation: strong_binding,
    }
}

/// Stable payload-free selected-anchor DNS-policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedResponderAnchorDnsPolicyErrorKind {
    /// The selected reviewed anchor's exact constraint bytes were invalid or
    /// outside the supported bounded subset.
    InvalidAnchorNameConstraints(NameConstraintsErrorKind),
    /// The exact responder certificate did not contain Subject Alternative Name.
    MissingResponderSubjectAlternativeName,
    /// The exact responder certificate's Subject Alternative Name was invalid.
    InvalidResponderSubjectAlternativeName(SubjectAltNameErrorKind),
    /// At least one responder dNSName was invalid or outside the selected
    /// anchor's supported permitted constraint set.
    ResponderDnsNameConstraints(DnsConstraintErrorKind),
}

/// A redacted selected-anchor DNS-policy failure for one reviewed responder path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedResponderAnchorDnsPolicyError {
    kind: X509OcspReviewedResponderAnchorDnsPolicyErrorKind,
}

impl X509OcspReviewedResponderAnchorDnsPolicyError {
    const fn new(kind: X509OcspReviewedResponderAnchorDnsPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedResponderAnchorDnsPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedResponderAnchorDnsPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-responder anchor DNS policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedResponderAnchorDnsPolicyError {}

/// Stable payload-free intermediate DNS-policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedResponderIntermediateDnsPolicyErrorKind {
    /// The exact intermediate carried NameConstraints without the RFC 5280
    /// critical flag.
    NonCriticalIntermediateNameConstraints,
    /// The exact intermediate's NameConstraints extension was invalid or
    /// outside the supported bounded subset.
    InvalidIntermediateNameConstraints(NameConstraintsErrorKind),
    /// The exact responder certificate did not contain Subject Alternative Name.
    MissingResponderSubjectAlternativeName,
    /// The exact responder certificate's Subject Alternative Name was invalid.
    InvalidResponderSubjectAlternativeName(SubjectAltNameErrorKind),
    /// At least one responder dNSName was invalid, excluded, or outside every
    /// permitted intermediate subtree when a permitted set was present.
    ResponderIntermediateDnsNameConstraints(DnsConstraintErrorKind),
}

/// A redacted intermediate DNS-policy failure for one reviewed responder path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedResponderIntermediateDnsPolicyError {
    kind: X509OcspReviewedResponderIntermediateDnsPolicyErrorKind,
}

impl X509OcspReviewedResponderIntermediateDnsPolicyError {
    const fn new(kind: X509OcspReviewedResponderIntermediateDnsPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedResponderIntermediateDnsPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedResponderIntermediateDnsPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-responder intermediate DNS policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedResponderIntermediateDnsPolicyError {}

/// Stable payload-free responder no-check policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedResponderNoCheckPolicyErrorKind {
    /// The exact authenticated responder did not carry `id-pkix-ocsp-nocheck`.
    MissingNoCheckExtension,
    /// The extension value was not the exact DER `NULL` required by RFC 6960.
    InvalidNoCheckExtensionValue,
}

/// A redacted responder no-check failure for one reviewed responder path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedResponderNoCheckPolicyError {
    kind: X509OcspReviewedResponderNoCheckPolicyErrorKind,
}

impl X509OcspReviewedResponderNoCheckPolicyError {
    const fn new(kind: X509OcspReviewedResponderNoCheckPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedResponderNoCheckPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedResponderNoCheckPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-responder no-check policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedResponderNoCheckPolicyError {}

/// Stable payload-free routed responder-revocation binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedResponderRevocationBindingErrorKind {
    /// The authenticated revocation proof targets another decoded certificate.
    TargetMismatch,
    /// The authenticated revocation proof names another decoded issuer.
    IssuerMismatch,
    /// The proof was evaluated at a different explicit observation time.
    ObservationTimeMismatch,
}

/// A redacted exact routed responder-revocation binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedResponderRevocationBindingError {
    kind: X509OcspReviewedResponderRevocationBindingErrorKind,
}

impl X509OcspReviewedResponderRevocationBindingError {
    const fn new(kind: X509OcspReviewedResponderRevocationBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedResponderRevocationBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedResponderRevocationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-responder revocation binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedResponderRevocationBindingError {}

/// Stable payload-free routed intermediate-revocation binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspReviewedIntermediateRevocationBindingErrorKind {
    /// The authenticated revocation proof targets another decoded certificate.
    TargetMismatch,
    /// The authenticated revocation proof names another decoded issuer.
    IssuerMismatch,
    /// The proof was evaluated at a different explicit observation time.
    ObservationTimeMismatch,
}

/// A redacted exact routed intermediate-revocation binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspReviewedIntermediateRevocationBindingError {
    kind: X509OcspReviewedIntermediateRevocationBindingErrorKind,
}

impl X509OcspReviewedIntermediateRevocationBindingError {
    const fn new(kind: X509OcspReviewedIntermediateRevocationBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspReviewedIntermediateRevocationBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspReviewedIntermediateRevocationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP reviewed-intermediate revocation binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspReviewedIntermediateRevocationBindingError {}

/// Non-copyable reviewed responder-path evidence after selected-anchor DNS policy.
///
/// Success proves either that the exact reviewed terminal selected by the
/// retained construction carries no out-of-band name constraints or that its
/// supported permitted DNS subtrees were applied to every dNSName in the exact
/// authenticated responder certificate. It is not evidence for intermediate,
/// excluded, or non-DNS constraints, generalized path validation, revocation,
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedResponderAnchorDnsPolicy<Proof> {
    responder_path: BoundX509OcspReviewedResponderPath<Proof>,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof> ValidatedX509OcspReviewedResponderAnchorDnsPolicy<Proof> {
    /// Return the consumed complete reviewed responder-path proof.
    pub const fn responder_path(&self) -> &BoundX509OcspReviewedResponderPath<Proof> {
        &self.responder_path
    }

    /// Return count-only DNS evidence when the selected anchor carried constraints.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof> fmt::Debug for ValidatedX509OcspReviewedResponderAnchorDnsPolicy<Proof> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedResponderAnchorDnsPolicy")
    }
}

/// Non-copyable reviewed responder-path evidence after intermediate DNS policy.
///
/// Success retains the complete selected-anchor responder-policy proof and
/// proves either that the exact retained intermediate has no NameConstraints
/// extension or that its supported critical permitted-and-excluded-DNS subset
/// was applied to every dNSName in the exact authenticated responder. It is not
/// generalized path validation, revocation, TLS, HTTP, OAuth, or network
/// authority.
pub struct ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof> {
    anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy<Proof>,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof> ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof> {
    /// Return the consumed complete selected-anchor responder-policy proof.
    pub const fn anchor_policy(&self) -> &ValidatedX509OcspReviewedResponderAnchorDnsPolicy<Proof> {
        &self.anchor_policy
    }

    /// Return count-only DNS evidence when the exact intermediate constrained DNS.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof> fmt::Debug for ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedResponderIntermediateDnsPolicy")
    }
}

/// Non-copyable reviewed responder evidence after RFC 6960 no-check policy.
///
/// Success retains the complete intermediate-DNS responder proof and proves
/// that its exact authenticated responder carries one structurally unique
/// `id-pkix-ocsp-nocheck` extension whose value is exact DER `NULL`. The
/// retained path already proves that the exact intermediate signed and
/// authorized this responder and that the responder was valid at the exact
/// OCSP observation time. This covers only RFC 6960's trust-for-certificate-
/// lifetime responder-revocation branch. It does not check intermediate
/// revocation, generalize path validation, or grant TLS, HTTP, OAuth, or
/// network authority.
pub struct ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof> {
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof>,
}

impl<Proof> ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof> {
    /// Return the consumed complete intermediate-DNS responder proof.
    pub const fn intermediate_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof> {
        &self.intermediate_dns_policy
    }
}

impl<Proof> fmt::Debug for ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedResponderNoCheckPolicy")
    }
}

/// Non-copyable routed responder evidence with authenticated revocation status.
///
/// Success retains the complete intermediate-DNS responder proof and one
/// independently authenticated, fully routed OCSP target-revocation proof.
/// The safe constructor requires that proof to target the same exact responder,
/// name the same exact intermediate issuer, and use the original server-status
/// OCSP observation time. It does not establish revocation policy for the
/// second proof's own responder or for the retained intermediate, generalize
/// path validation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedResponderRevocation<Proof, Revocation> {
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof>,
    responder_revocation: Revocation,
}

impl<Proof, Revocation> BoundX509OcspReviewedResponderRevocation<Proof, Revocation> {
    /// Return the complete routed responder proof retained intact.
    pub const fn intermediate_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<Proof> {
        &self.intermediate_dns_policy
    }

    /// Return the exact authenticated responder-revocation proof.
    pub const fn responder_revocation(&self) -> &Revocation {
        &self.responder_revocation
    }
}

impl<Proof, Revocation> fmt::Debug for BoundX509OcspReviewedResponderRevocation<Proof, Revocation> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedResponderRevocation")
    }
}

/// Non-copyable evidence reifying a responder-revocation proof's own responder path.
///
/// The retained separately authenticated responder-revocation branch already
/// proves that its added proof's exact delegated responder passed certificate-
/// wide role, algorithm, OCSP-signing-purpose, and observation-time validity
/// policy; that repository-owned routed RSA verification authenticated its
/// response and certificate signatures; and that its exact responder-to-
/// intermediate-to-reviewed-root path is the one retained by that proof. This
/// nominal wrapper reifies that path without accepting substitute evidence or
/// performing additional validation.
///
/// It does not apply reviewed-root or intermediate name constraints to the
/// added responder, establish no-check or separately authenticated revocation
/// policy for it, generalize path validation, or grant TLS, HTTP, OAuth, or
/// network authority.
pub struct BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation> {
    responder_revocation: BoundX509OcspReviewedResponderRevocation<Proof, Revocation>,
}

impl<Proof, Revocation> BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation> {
    /// Return the complete separately authenticated responder branch intact.
    pub const fn responder_revocation(
        &self,
    ) -> &BoundX509OcspReviewedResponderRevocation<Proof, Revocation> {
        &self.responder_revocation
    }
}

impl<Proof, Revocation> fmt::Debug
    for BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedResponderRevocationResponderPath")
    }
}

/// Non-copyable responder-revocation path after reviewed-root DNS policy.
///
/// Success proves either that the exact selected reviewed root carries no
/// out-of-band name constraints or that its supported permitted DNS subtrees
/// were applied to every dNSName in the separately authenticated revocation
/// proof's exact responder certificate. It is not evidence for intermediate,
/// excluded, or non-DNS constraints, generalized path validation, revocation,
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation> {
    responder_path: BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation>,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof, Revocation>
    ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation>
{
    /// Return the consumed complete responder-revocation responder path.
    pub const fn responder_path(
        &self,
    ) -> &BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation> {
        &self.responder_path
    }

    /// Return count-only DNS evidence when the selected root carried constraints.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof, Revocation> fmt::Debug
    for ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy")
    }
}

/// Non-copyable responder-revocation path after intermediate DNS policy.
///
/// Success retains the complete reviewed-root DNS proof and proves either
/// that the separately authenticated revocation proof's exact intermediate
/// has no NameConstraints extension or that its supported critical permitted-
/// and-excluded-DNS subset was applied to every dNSName in that proof's exact
/// responder certificate. It is not evidence for no-check, separately
/// authenticated responder revocation, generalized path validation, complete
/// revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
    Proof,
    Revocation,
> {
    anchor_dns_policy:
        ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation>,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof, Revocation>
    ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<Proof, Revocation>
{
    /// Return the consumed complete reviewed-root DNS proof.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation>
    {
        &self.anchor_dns_policy
    }

    /// Return count-only DNS evidence when the exact intermediate constrained DNS.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof, Revocation> fmt::Debug
    for ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
        Proof,
        Revocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy")
    }
}

/// Non-copyable responder-revocation path after RFC 6960 no-check policy.
///
/// Success retains the complete root-and-intermediate DNS proof and proves
/// that the separately authenticated revocation proof's exact responder
/// carries one structurally unique `id-pkix-ocsp-nocheck` extension whose
/// value is exact DER `NULL`. This covers only RFC 6960's trust-for-
/// certificate-lifetime responder-revocation branch. It is not evidence for
/// separately authenticated revocation of this responder, complete
/// revocation, generalized path validation, TLS, HTTP, OAuth, or network
/// authority.
pub struct ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<Proof, Revocation> {
    intermediate_dns_policy:
        ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
            Proof,
            Revocation,
        >,
}

impl<Proof, Revocation>
    ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<Proof, Revocation>
{
    /// Return the consumed complete root-and-intermediate DNS proof.
    pub const fn intermediate_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
        Proof,
        Revocation,
    > {
        &self.intermediate_dns_policy
    }
}

impl<Proof, Revocation> fmt::Debug
    for ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<Proof, Revocation>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy")
    }
}

/// Non-copyable responder no-check evidence with intermediate status.
///
/// Success retains the complete responder-of-responder RFC 6960 no-check
/// proof and one independently authenticated, fully routed OCSP target-
/// revocation proof. The safe constructor requires the second proof to target
/// the same exact retained intermediate, name the same exact selected reviewed
/// root, and use the original server-status observation time. It does not
/// establish revocation policy for the intermediate-status proof's own
/// responder, complete revocation, generalized path validation, TLS, HTTP,
/// OAuth, or network authority.
pub struct BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
> {
    responder_no_check_policy: ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<
        Proof,
        ResponderRevocation,
    >,
    intermediate_revocation: IntermediateRevocation,
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    /// Return the complete responder-of-responder no-check proof intact.
    pub const fn responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<
        Proof,
        ResponderRevocation,
    > {
        &self.responder_no_check_policy
    }

    /// Return the exact authenticated intermediate-revocation proof.
    pub const fn intermediate_revocation(&self) -> &IntermediateRevocation {
        &self.intermediate_revocation
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation> fmt::Debug
    for BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation",
        )
    }
}

/// Non-copyable routed reviewed-path evidence with intermediate revocation status.
///
/// Success retains the complete fully routed reviewed server-revocation proof
/// and one independently authenticated, fully routed OCSP target-revocation
/// proof. The safe constructor requires that proof to target the same exact
/// retained intermediate, name the same exact selected reviewed root, and use
/// the original server-status OCSP observation time. This evidence remains
/// independent of responder policy and does not establish revocation policy
/// for the second proof's own responder, generalize path validation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedIntermediateRevocation<Proof, Revocation> {
    reviewed_server_revocation: Proof,
    intermediate_revocation: Revocation,
}

impl<Proof, Revocation> BoundX509OcspReviewedIntermediateRevocation<Proof, Revocation> {
    /// Return the complete routed reviewed-server proof retained intact.
    pub const fn reviewed_server_revocation(&self) -> &Proof {
        &self.reviewed_server_revocation
    }

    /// Return the exact authenticated intermediate-revocation proof.
    pub const fn intermediate_revocation(&self) -> &Revocation {
        &self.intermediate_revocation
    }
}

impl<Proof, Revocation> fmt::Debug
    for BoundX509OcspReviewedIntermediateRevocation<Proof, Revocation>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedIntermediateRevocation")
    }
}

/// Non-copyable routed no-check evidence with intermediate revocation status.
///
/// Success retains the complete fully routed responder no-check proof and one
/// independently authenticated, fully routed OCSP target-revocation proof.
/// The safe constructor requires that proof to target the same exact retained
/// intermediate, name the same exact selected reviewed root, and use the
/// original server-status OCSP observation time. It does not establish
/// revocation policy for the added proof's own responder, generalize path
/// validation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation> {
    responder_no_check_policy: ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof>,
    intermediate_revocation: Revocation,
}

impl<Proof, Revocation> BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation> {
    /// Return the complete routed responder no-check proof retained intact.
    pub const fn responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof> {
        &self.responder_no_check_policy
    }

    /// Return the exact authenticated intermediate-revocation proof.
    pub const fn intermediate_revocation(&self) -> &Revocation {
        &self.intermediate_revocation
    }
}

impl<Proof, Revocation> fmt::Debug
    for BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedNoCheckIntermediateRevocation")
    }
}

/// Non-copyable routed responder-revocation evidence with intermediate status.
///
/// Success retains both the complete separately authenticated responder-
/// revocation branch and one independently authenticated, fully routed OCSP
/// target-revocation proof for the exact retained intermediate. The safe
/// constructor requires the second proof to name the same selected reviewed
/// root issuer and use the original server-status observation time. It does
/// not establish revocation policy for either added proof's own responder,
/// generalize path validation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
> {
    responder_revocation: BoundX509OcspReviewedResponderRevocation<Proof, ResponderRevocation>,
    intermediate_revocation: IntermediateRevocation,
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    /// Return the complete separately authenticated responder branch intact.
    pub const fn responder_revocation(
        &self,
    ) -> &BoundX509OcspReviewedResponderRevocation<Proof, ResponderRevocation> {
        &self.responder_revocation
    }

    /// Return the exact authenticated intermediate-revocation proof.
    pub const fn intermediate_revocation(&self) -> &IntermediateRevocation {
        &self.intermediate_revocation
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation> fmt::Debug
    for BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedResponderRevocationIntermediateRevocation")
    }
}

/// Non-copyable evidence reifying the added status responder's reviewed path.
///
/// The retained original-responder policy plus intermediate-status composition
/// already proves that the added proof's exact delegated responder passed
/// certificate-wide role, algorithm, OCSP-signing-purpose, and observation-
/// time validity policy; that repository-owned routed RSA verification
/// authenticated its response and responder certificate; and that its exact
/// issuer is the same selected Mozilla-reviewed root retained by the original
/// proof. This nominal wrapper reifies that direct responder-to-reviewed-root
/// path without accepting substitute evidence or performing additional
/// validation. The default source preserves the existing no-check branch;
/// repository-owned constructors can retain another complete source under the
/// third parameter.
///
/// It does not apply the selected root's supported DNS constraints to the
/// responder, establish no-check or separately authenticated revocation policy
/// for it, generalize path validation, or grant TLS, HTTP, OAuth, or network
/// authority.
pub struct BoundX509OcspReviewedIntermediateStatusResponderPath<
    Proof,
    Revocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation>,
> {
    source: Source,
    evidence_types: PhantomData<fn() -> (Proof, Revocation)>,
}

impl<Proof, Revocation> BoundX509OcspReviewedIntermediateStatusResponderPath<Proof, Revocation> {
    /// Return the complete no-check plus intermediate-status proof intact.
    pub const fn no_check_intermediate_revocation(
        &self,
    ) -> &BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation> {
        &self.source
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    BoundX509OcspReviewedIntermediateStatusResponderPath<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >
{
    /// Return the complete authenticated-responder plus intermediate proof.
    pub const fn responder_revocation_intermediate_revocation(
        &self,
    ) -> &BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    > {
        &self.source
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    BoundX509OcspReviewedIntermediateStatusResponderPath<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >
{
    /// Return the complete responder no-check plus intermediate-status proof.
    pub const fn responder_revocation_responder_no_check_intermediate_revocation(
        &self,
    ) -> &BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    > {
        &self.source
    }
}

impl<Proof, Revocation, Source> fmt::Debug
    for BoundX509OcspReviewedIntermediateStatusResponderPath<Proof, Revocation, Source>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedIntermediateStatusResponderPath")
    }
}

/// Non-copyable added-status responder evidence after reviewed-root DNS policy.
///
/// Success proves either that the exact selected Mozilla-reviewed root carries
/// no out-of-band name constraints or that its supported permitted DNS
/// subtrees were applied to every dNSName in the added intermediate-status
/// proof's exact authenticated responder certificate. It is not evidence for
/// excluded or non-DNS root constraints, responder no-check or separately
/// authenticated responder revocation, generalized path validation, TLS,
/// HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
    Proof,
    Revocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation>,
> {
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderPath<Proof, Revocation, Source>,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof, Revocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<Proof, Revocation, Source>
{
    /// Return the consumed complete added-status responder-path proof.
    pub const fn responder_path(
        &self,
    ) -> &BoundX509OcspReviewedIntermediateStatusResponderPath<Proof, Revocation, Source> {
        &self.responder_path
    }

    /// Return count-only DNS evidence when the selected root carried constraints.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof, Revocation, Source> fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        Revocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy")
    }
}

/// Non-copyable added-status responder evidence after RFC 6960 no-check policy.
///
/// Success retains the complete reviewed-root DNS proof and proves that the
/// added intermediate-status proof's exact authenticated responder carries one
/// structurally unique `id-pkix-ocsp-nocheck` extension whose value is exact
/// DER `NULL`. This covers only RFC 6960's trust-for-certificate-lifetime
/// responder-revocation branch. It is not evidence for separately authenticated
/// responder revocation, complete revocation, generalized path validation,
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
    Proof,
    Revocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, Revocation>,
> {
    anchor_dns_policy: ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        Revocation,
        Source,
    >,
}

impl<Proof, Revocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<Proof, Revocation, Source>
{
    /// Return the consumed complete added-status responder DNS proof.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        Revocation,
        Source,
    > {
        &self.anchor_dns_policy
    }
}

impl<Proof, Revocation, Source> fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<Proof, Revocation, Source>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy")
    }
}

/// Non-copyable complete revocation policy for one exact direct no-check path.
///
/// The retained proof covers the server target and intermediate with
/// authenticated good-and-fresh OCSP status, and covers both the original
/// responder and the intermediate-status proof's responder with exact RFC
/// 6960 no-check policy. The selected Mozilla-reviewed root remains the trust
/// anchor, not a revocation target. This nominal completion evidence does not
/// generalize path construction or validation, acquire or refresh revocation
/// data, read a clock, or provide TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
> {
    intermediate_status_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
        >,
}

impl<Proof, IntermediateRevocation>
    ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
    >
{
    /// Return the consumed complete direct no-check proof intact.
    pub const fn intermediate_status_responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
    > {
        &self.intermediate_status_responder_no_check_policy
    }
}

impl<Proof, IntermediateRevocation> fmt::Debug
    for ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy")
    }
}

/// Non-copyable complete revocation policy for one exact nested fixed path.
///
/// The retained proof covers every non-root certificate in this one fixed
/// chain with either authenticated good-and-fresh OCSP status or an exact RFC
/// 6960 no-check policy. The selected Mozilla-reviewed root remains the trust
/// anchor, not a revocation target. This nominal completion evidence does not
/// generalize path construction or validation, acquire or refresh revocation
/// data, read a clock, or provide TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
> {
    intermediate_status_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
                Proof,
                ResponderRevocation,
                IntermediateRevocation,
            >,
        >,
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    /// Return the consumed complete nested no-check proof intact.
    pub const fn intermediate_status_responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    > {
        &self.intermediate_status_responder_no_check_policy
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation> fmt::Debug
    for ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy")
    }
}

/// Non-copyable added-status responder evidence with authenticated revocation.
///
/// Success retains the complete reviewed-root DNS proof plus one independently
/// authenticated, fully routed OCSP target-revocation proof. The safe
/// constructor requires the second proof to target the added intermediate-
/// status proof's exact responder, name that proof's exact selected reviewed
/// root issuer, and use the same explicit observation time. It does not
/// establish revocation policy for the second proof's own responder, complete
/// revocation, generalized path validation, TLS, HTTP, OAuth, or network
/// authority.
pub struct BoundX509OcspReviewedIntermediateStatusResponderRevocation<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    anchor_dns_policy: ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        Source,
    >,
    responder_revocation: ResponderRevocation,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source>
    BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    /// Return the complete added-responder reviewed-root DNS proof intact.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        Source,
    > {
        &self.anchor_dns_policy
    }

    /// Return the exact authenticated added-responder revocation proof.
    pub const fn responder_revocation(&self) -> &ResponderRevocation {
        &self.responder_revocation
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source> fmt::Debug
    for BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspReviewedIntermediateStatusResponderRevocation")
    }
}

/// Non-copyable evidence reifying the added revocation proof's responder path.
///
/// The retained intermediate-status responder-revocation branch already
/// proves that its added proof's exact delegated responder passed certificate-
/// wide role, algorithm, OCSP-signing-purpose, and observation-time validity
/// policy; that repository-owned routed RSA verification authenticated the
/// response and certificate signatures; and that the exact responder path
/// terminates at the selected Mozilla-reviewed root. This nominal wrapper
/// reifies that path without accepting substitute evidence or performing
/// additional validation.
///
/// It does not apply reviewed-root or intermediate name constraints to the
/// delegated responder, establish no-check or separately authenticated
/// revocation policy for it, complete revocation policy, generalize path
/// validation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    responder_revocation: BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source>
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    /// Return the complete added-responder revocation branch intact.
    pub const fn responder_revocation(
        &self,
    ) -> &BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    > {
        &self.responder_revocation
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source> fmt::Debug
    for BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath")
    }
}

/// Non-copyable added-responder revocation path after reviewed-root DNS policy.
///
/// Success proves either that the exact selected reviewed root carries no
/// out-of-band name constraints or that its supported permitted DNS subtrees
/// were applied to every dNSName in the added revocation proof's exact
/// delegated responder certificate. It is not evidence for intermediate,
/// excluded, or non-DNS constraints, responder no-check or separately
/// authenticated revocation, complete revocation, generalized path
/// validation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    /// Return the consumed complete delegated-responder path proof.
    pub const fn responder_path(
        &self,
    ) -> &BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    > {
        &self.responder_path
    }

    /// Return count-only DNS evidence when the selected root carried constraints.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source> fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy",
        )
    }
}

/// Non-copyable recursive revocation evidence for the added proof's responder.
///
/// Success retains the complete added-responder revocation reviewed-root DNS
/// proof plus one independently authenticated, fully routed OCSP target-
/// revocation proof. The safe constructor requires the recursive proof to
/// target the added revocation proof's own exact authenticated responder, name
/// that proof's exact selected reviewed-root issuer, and use the original
/// explicit observation time. It does not establish revocation policy for the
/// recursive proof's own responder, complete revocation, generalized path
/// validation, TLS, HTTP, OAuth, or network authority.
pub struct BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            Source,
        >,
    recursive_responder_revocation: RecursiveResponderRevocation,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    /// Return the complete added-proof responder DNS evidence intact.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    > {
        &self.anchor_dns_policy
    }

    /// Return the exact recursive authenticated responder-revocation proof.
    pub const fn recursive_responder_revocation(&self) -> &RecursiveResponderRevocation {
        &self.recursive_responder_revocation
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    fmt::Debug
    for BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation",
        )
    }
}

/// Non-copyable path for the recursive revocation proof's own responder.
///
/// The retained recursive responder-revocation branch already proves that its
/// recursive proof's exact delegated responder passed certificate-wide role,
/// algorithm, OCSP-signing-purpose, and observation-time validity policy; that
/// repository-owned routed RSA verification authenticated the response and
/// certificate signatures; and that the exact responder path terminates at
/// the selected Mozilla-reviewed root. This nominal wrapper reifies that path
/// without accepting substitute evidence or performing additional validation.
///
/// It does not apply reviewed-root name policy to the delegated responder,
/// establish no-check or separately authenticated revocation policy for it,
/// complete revocation policy, generalize path validation, or grant TLS, HTTP,
/// OAuth, or network authority.
pub struct BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    recursive_responder_revocation:
        BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    /// Return the complete recursive responder-revocation branch intact.
    pub const fn recursive_responder_revocation(
        &self,
    ) -> &BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    > {
        &self.recursive_responder_revocation
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    fmt::Debug
    for BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath",
        )
    }
}

/// Recursive responder path after selected reviewed-root DNS policy.
///
/// Success proves either that the exact selected reviewed root carries no
/// out-of-band name constraints or that its supported permitted DNS subtrees
/// were applied to every dNSName in the recursive revocation proof's exact
/// delegated responder certificate. It is not evidence for intermediate,
/// excluded, or non-DNS constraints, responder no-check or further
/// authenticated revocation, complete revocation, generalized path
/// validation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    responder_path:
        BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    /// Return the consumed complete recursive delegated-responder path proof.
    pub const fn responder_path(
        &self,
    ) -> &BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    > {
        &self.responder_path
    }

    /// Return count-only DNS evidence when the selected root carried constraints.
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy",
        )
    }
}

/// Recursive responder path after RFC 6960 no-check policy.
///
/// Success retains the complete selected-root DNS proof and proves that the
/// recursive revocation proof's exact delegated responder carries one
/// structurally unique `id-pkix-ocsp-nocheck` extension whose value is exact
/// DER `NULL`. This covers only RFC 6960's trust-for-certificate-lifetime
/// responder-revocation branch. It is not evidence for separately
/// authenticated revocation of this responder, complete revocation,
/// generalized path validation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    /// Return the consumed complete selected-root DNS proof.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    > {
        &self.anchor_dns_policy
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation, Source>
    fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy",
        )
    }
}

/// Non-copyable complete revocation policy for one recursive fixed path.
///
/// The retained default-source proof covers the server target, original
/// responder, intermediate, added intermediate-status responder, that
/// responder's revocation proof signer, and the recursive revocation proof's
/// own signer with authenticated good-and-fresh OCSP status or exact RFC 6960
/// no-check policy. The selected Mozilla-reviewed root remains the trust
/// anchor, not a revocation target. This nominal completion evidence does not
/// generalize path construction or validation, acquire or refresh revocation
/// data, read a clock, or provide TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
> {
    recursive_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
        >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation>
    ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
    >
{
    /// Return the consumed complete recursive no-check proof intact.
    pub const fn recursive_responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
    >{
        &self.recursive_responder_no_check_policy
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, RecursiveResponderRevocation> fmt::Debug
    for ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy",
        )
    }
}

/// Non-copyable added-responder revocation path after RFC 6960 no-check policy.
///
/// Success retains the complete reviewed-root DNS proof and proves that the
/// added revocation proof's exact delegated responder carries one structurally
/// unique `id-pkix-ocsp-nocheck` extension whose value is exact DER `NULL`.
/// This covers only RFC 6960's trust-for-certificate-lifetime responder-
/// revocation branch. It is not evidence for separately authenticated
/// revocation of this responder, complete revocation, generalized path
/// validation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source = BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
> {
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            Source,
        >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source>
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    /// Return the consumed complete reviewed-root DNS proof.
    pub const fn anchor_dns_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    > {
        &self.anchor_dns_policy
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation, Source> fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy",
        )
    }
}

/// Non-copyable complete revocation policy for one exact alternative fixed path.
///
/// The retained default-source proof covers the server target, original
/// responder, intermediate, added intermediate-status responder, and the
/// added responder-revocation proof's own responder with authenticated good-
/// and-fresh OCSP status or exact RFC 6960 no-check policy. The selected
/// Mozilla-reviewed root remains the trust anchor, not a revocation target.
/// This nominal completion evidence does not generalize path construction or
/// validation, acquire or refresh revocation data, read a clock, or provide
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
> {
    responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
        >,
}

impl<Proof, IntermediateRevocation, ResponderRevocation>
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
    >
{
    /// Return the consumed complete alternative-branch proof intact.
    pub const fn responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
    > {
        &self.responder_no_check_policy
    }
}

impl<Proof, IntermediateRevocation, ResponderRevocation> fmt::Debug
    for ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy",
        )
    }
}

/// Non-copyable complete revocation policy for one double-nested fixed path.
///
/// The retained source-specific proof covers the server target, original
/// responder, the original responder-revocation proof's own responder,
/// intermediate, added intermediate-status responder, and the added
/// responder-revocation proof's own responder with authenticated good-and-
/// fresh OCSP status or exact RFC 6960 no-check policy. The selected Mozilla-
/// reviewed root remains the trust anchor, not a revocation target. This
/// nominal completion evidence does not generalize path construction or
/// validation, acquire or refresh revocation data, read a clock, or provide
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    OriginalResponderRevocation,
    IntermediateRevocation,
    AddedResponderRevocation,
> {
    responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            AddedResponderRevocation,
            BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
                Proof,
                OriginalResponderRevocation,
                IntermediateRevocation,
            >,
        >,
}

impl<Proof, OriginalResponderRevocation, IntermediateRevocation, AddedResponderRevocation>
    ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        OriginalResponderRevocation,
        IntermediateRevocation,
        AddedResponderRevocation,
    >
{
    /// Return the consumed complete double-nested proof intact.
    pub const fn responder_no_check_policy(
        &self,
    ) -> &ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        AddedResponderRevocation,
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            OriginalResponderRevocation,
            IntermediateRevocation,
        >,
    > {
        &self.responder_no_check_policy
    }
}

impl<Proof, OriginalResponderRevocation, IntermediateRevocation, AddedResponderRevocation>
    fmt::Debug
    for ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy<
        Proof,
        OriginalResponderRevocation,
        IntermediateRevocation,
        AddedResponderRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy",
        )
    }
}

/// Reify the responder-revocation proof's own exact responder path.
///
/// The safe responder-revocation constructor already binds one complete fully
/// routed authenticated proof to the original responder, its exact
/// intermediate issuer, and the original observation time. That retained
/// proof already authenticates its own delegated responder through the same
/// exact intermediate to the selected reviewed root. This infallible nominal
/// projection retains the complete branch and accepts no caller-supplied
/// certificate, path, root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root or intermediate
/// name policy for the added responder, that responder's no-check or
/// authenticated-revocation policy, complete revocation policy, generalized
/// path validation, TLS, HTTP, OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path<
    Proof,
    Revocation,
>(
    responder_revocation: BoundX509OcspReviewedResponderRevocation<Proof, Revocation>,
) -> BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation> {
    BoundX509OcspReviewedResponderRevocationResponderPath {
        responder_revocation,
    }
}

/// Reify the added responder-revocation proof's own exact responder path.
///
/// The safe added-responder revocation constructor already binds one complete
/// fully routed authenticated proof to the intermediate-status responder, its
/// exact reviewed-root issuer, and the original observation time. That
/// retained proof already authenticates its own delegated responder to the
/// selected reviewed root. This infallible nominal projection preserves the
/// complete source branch and accepts no caller-supplied certificate, path,
/// root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root or intermediate
/// name policy for the delegated responder, that responder's no-check or
/// authenticated-revocation policy, complete revocation policy, generalized
/// path validation, TLS, HTTP, OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source,
>(
    responder_revocation: BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
) -> BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source,
> {
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath {
        responder_revocation,
    }
}

/// Apply selected reviewed-root DNS policy to the added proof's responder.
///
/// The exact delegated responder certificate and selected Mozilla-reviewed
/// root constraints are derived only through the consumed repository-owned
/// path. Entries without constraints succeed without reading responder
/// Subject Alternative Name. Separate caller-owned limits bound both ASN.1
/// parses.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, intermediate name constraints,
/// no-check or separately authenticated revocation policy for the delegated
/// responder, complete revocation policy, generalized path validation, TLS,
/// HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source,
>(
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>
where
    ResponderRevocation: IntermediateStatusResponderEvidence,
    Source: IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>,
{
    let responder_revocation = responder_path.responder_revocation();
    let responder_certificate = responder_revocation
        .responder_revocation()
        .authenticated_responder_certificate();
    let source = &responder_revocation
        .anchor_dns_policy()
        .responder_path()
        .source;
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        source.selected_reviewed_root_name_constraints(),
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

/// Bind authenticated revocation to the added proof's own exact responder.
///
/// This recursive alternative consumes the complete reviewed-root DNS proof
/// for the added revocation proof's delegated responder plus a separate fully
/// routed authenticated OCSP target-revocation proof. The recursive proof must
/// target that same exact responder certificate, name the same exact selected
/// reviewed root, and use the original explicit observation time. Failure
/// order is stable: target identity, issuer identity, then observation time.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before revocation policy for the recursive proof's own responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source,
>(
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            Source,
        >,
    recursive_responder_revocation: RecursiveResponderRevocation,
) -> Result<
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderRevocationBindingError,
>
where
    ResponderRevocation: IntermediateStatusResponderRevocationBindingEvidence,
    RecursiveResponderRevocation: RoutedTargetRevocationBindingEvidence,
{
    let responder_revocation = anchor_dns_policy
        .responder_path()
        .responder_revocation()
        .responder_revocation();

    validate_responder_revocation_binding_inputs(
        responder_revocation.authenticated_responder_certificate(),
        responder_revocation.intermediate_status_issuer_certificate(),
        responder_revocation.intermediate_status_observation_time(),
        recursive_responder_revocation.revocation_target_certificate(),
        recursive_responder_revocation.revocation_issuer_certificate(),
        recursive_responder_revocation.revocation_observation_time(),
    )?;

    Ok(
        BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation {
            anchor_dns_policy,
            recursive_responder_revocation,
        },
    )
}

/// Reify the recursive responder-revocation proof's own exact responder path.
///
/// The safe recursive responder-revocation constructor already binds one
/// complete fully routed authenticated proof to the added proof's responder,
/// its exact reviewed-root issuer, and the original observation time. That
/// retained recursive proof already authenticates its own delegated responder
/// to the selected reviewed root. This infallible nominal projection preserves
/// the complete generic source and accepts no caller-supplied certificate,
/// path, root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root name policy for the
/// delegated responder, that responder's no-check or authenticated-revocation
/// policy, complete revocation policy, generalized path validation, TLS, HTTP,
/// OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_path<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source,
>(
    recursive_responder_revocation:
        BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
) -> BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source,
> {
    BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath {
        recursive_responder_revocation,
    }
}

/// Apply selected reviewed-root DNS policy to the recursive proof's responder.
///
/// The exact delegated responder certificate and selected Mozilla-reviewed
/// root constraints are derived only through the consumed repository-owned
/// path. Entries without constraints succeed without reading responder
/// Subject Alternative Name. Separate caller-owned limits bound both ASN.1
/// parses.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, intermediate name constraints,
/// no-check or further separately authenticated revocation policy for the
/// delegated responder, complete revocation policy, generalized path
/// validation, TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source,
>(
    responder_path:
        BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>
where
    RecursiveResponderRevocation: IntermediateStatusResponderEvidence,
    Source: IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>,
{
    let recursive_responder_revocation = responder_path.recursive_responder_revocation();
    let responder_certificate = recursive_responder_revocation
        .recursive_responder_revocation()
        .authenticated_responder_certificate();
    let source = &recursive_responder_revocation
        .anchor_dns_policy()
        .responder_path()
        .responder_revocation()
        .anchor_dns_policy()
        .responder_path()
        .source;
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        source.selected_reviewed_root_name_constraints(),
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

/// Require RFC 6960 no-check policy for the recursive delegated responder.
///
/// This consumes the complete selected-root DNS proof and derives the
/// recursive revocation proof's exact delegated responder certificate only
/// through the retained repository-owned evidence. A structurally unique
/// `id-pkix-ocsp-nocheck` extension must be present with exact DER `NULL`
/// contents. RFC 6960 recommends a non-critical extension but does not require
/// it; a critical instance can reach this semantic gate only when the earlier
/// certificate-wide role policy explicitly recognized its OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before separately authenticated revocation for this responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
    Source,
>(
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
            Source,
        >,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        RecursiveResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderNoCheckPolicyError,
>
where
    RecursiveResponderRevocation: IntermediateStatusResponderEvidence,
{
    let responder_certificate = anchor_dns_policy
        .responder_path()
        .recursive_responder_revocation()
        .recursive_responder_revocation()
        .authenticated_responder_certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy {
            anchor_dns_policy,
        },
    )
}

/// Complete revocation policy for the exact recursive no-check fixed path.
///
/// The input is deliberately restricted to the default source branch. Its
/// type can be constructed only after the retained server target, original
/// responder, intermediate, added intermediate-status responder, added
/// responder-revocation proof signer, and recursive revocation proof signer
/// have each passed their branch-specific authenticated-status or RFC 6960
/// no-check policy. This infallible nominal composition accepts no substitute
/// certificate, path, root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It does not generalize path construction or
/// validation, fetch, cache, acquire, or refresh revocation data, or provide
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn complete_x509_ocsp_routed_issuer_reviewed_recursive_responder_no_check_fixed_path_revocation_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
>(
    recursive_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            RecursiveResponderRevocation,
        >,
) -> ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    RecursiveResponderRevocation,
> {
    ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy {
        recursive_responder_no_check_policy,
    }
}

/// Require RFC 6960 no-check policy for the delegated responder.
///
/// This consumes the complete reviewed-root DNS proof and derives the exact
/// delegated responder certificate only through that retained proof. A
/// structurally unique `id-pkix-ocsp-nocheck` extension must be present with
/// exact DER `NULL` contents. RFC 6960 recommends a non-critical extension but
/// does not require it; a critical instance can reach this semantic gate only
/// when the earlier certificate-wide role policy explicitly recognized its
/// OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before separately authenticated revocation for this responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source,
>(
    anchor_dns_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
            Source,
        >,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderNoCheckPolicyError,
>
where
    ResponderRevocation: IntermediateStatusResponderEvidence,
{
    let responder_certificate = anchor_dns_policy
        .responder_path()
        .responder_revocation()
        .responder_revocation()
        .authenticated_responder_certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy {
            anchor_dns_policy,
        },
    )
}

/// Complete revocation policy for the exact alternative fixed path.
///
/// The input is deliberately restricted to the default source branch. Its
/// type can be constructed only after the retained server target, original
/// responder, intermediate, added intermediate-status responder, and the
/// added responder-revocation proof's own responder have each passed their
/// branch-specific authenticated-status or RFC 6960 no-check policy. This
/// infallible nominal composition accepts no substitute certificate, path,
/// root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It does not generalize path construction or
/// validation, fetch, cache, acquire, or refresh revocation data, or provide
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn complete_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_fixed_path_revocation_policy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
>(
    responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            ResponderRevocation,
        >,
) -> ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
>{
    ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy {
        responder_no_check_policy,
    }
}

/// Complete revocation policy for the exact double-nested fixed path.
///
/// The input is deliberately restricted to the source branch that already
/// retains separately authenticated original-responder revocation plus exact
/// RFC 6960 no-check policy for that proof's own responder. The remaining
/// retained intermediate and added responder branches likewise carry their
/// exact authenticated-status or no-check evidence. This infallible nominal
/// composition accepts no substitute certificate, path, root, algorithm, key,
/// signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It does not generalize path construction or
/// validation, fetch, cache, acquire, or refresh revocation data, or provide
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn complete_x509_ocsp_routed_issuer_reviewed_double_nested_no_check_fixed_path_revocation_policy<
    Proof,
    OriginalResponderRevocation,
    IntermediateRevocation,
    AddedResponderRevocation,
>(
    responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            AddedResponderRevocation,
            BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
                Proof,
                OriginalResponderRevocation,
                IntermediateRevocation,
            >,
        >,
) -> ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    OriginalResponderRevocation,
    IntermediateRevocation,
    AddedResponderRevocation,
> {
    ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy {
        responder_no_check_policy,
    }
}

/// Apply selected reviewed-root DNS policy to the added revocation responder.
///
/// The exact responder certificate and selected Mozilla-reviewed root
/// constraints are derived only through the consumed repository-owned path.
/// Entries without constraints succeed without reading responder Subject
/// Alternative Name. Separate caller-owned limits bound both ASN.1 parses.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, intermediate name constraints,
/// no-check or separately authenticated revocation policy for the added
/// responder, complete revocation policy, generalized path validation, TLS,
/// HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_anchor_dns_policy<
    Proof,
    Revocation,
>(
    responder_path: BoundX509OcspReviewedResponderRevocationResponderPath<Proof, Revocation>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<Proof, Revocation>,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
    Revocation: IntermediateStatusResponderEvidence,
{
    let responder_revocation = responder_path.responder_revocation();
    let responder_certificate = responder_revocation
        .responder_revocation()
        .authenticated_responder_certificate();
    let name_constraints = responder_revocation
        .intermediate_dns_policy()
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation()
        .selected_reviewed_root_name_constraints();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

/// Apply exact retained-intermediate DNS policy to the added revocation responder.
///
/// Both certificates are derived only through the consumed repository-owned
/// proof. An intermediate without NameConstraints succeeds without reading
/// responder Subject Alternative Name. A present NameConstraints extension
/// must be critical, and its supported permitted and excluded DNS subtrees are
/// applied under separate caller-owned ASN.1 limits.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before no-check or separately authenticated revocation policy for the added
/// responder, complete revocation policy, generalized path validation, TLS,
/// HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_intermediate_dns_policy<
    Proof,
    Revocation,
>(
    anchor_dns_policy: ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy<
        Proof,
        Revocation,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<Proof, Revocation>,
    X509OcspReviewedResponderIntermediateDnsPolicyError,
>
where
    Revocation: IntermediateStatusResponderRevocationBindingEvidence,
{
    let responder_revocation = anchor_dns_policy
        .responder_path()
        .responder_revocation()
        .responder_revocation();
    let responder_certificate = responder_revocation.authenticated_responder_certificate();
    let intermediate_certificate = responder_revocation.intermediate_status_issuer_certificate();
    let constrained_dns_names = evaluate_responder_intermediate_dns_policy(
        responder_certificate,
        intermediate_certificate,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy {
            anchor_dns_policy,
            constrained_dns_names,
        },
    )
}

/// Require RFC 6960 no-check policy for the added revocation responder.
///
/// This consumes the complete root-and-intermediate DNS proof and derives the
/// responder certificate only through that retained proof. A structurally
/// unique `id-pkix-ocsp-nocheck` extension must be present with exact DER
/// `NULL` contents. RFC 6960 recommends a non-critical extension but does not
/// require it; a critical instance can reach this semantic gate only when the
/// earlier certificate-wide role policy explicitly recognized its OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before separately authenticated revocation for this responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_policy<
    Proof,
    Revocation,
>(
    intermediate_dns_policy:
        ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
            Proof,
            Revocation,
        >,
) -> Result<
    ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<Proof, Revocation>,
    X509OcspReviewedResponderNoCheckPolicyError,
>
where
    Revocation: IntermediateStatusResponderEvidence,
{
    let responder_certificate = intermediate_dns_policy
        .anchor_dns_policy()
        .responder_path()
        .responder_revocation()
        .responder_revocation()
        .authenticated_responder_certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(
        ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy {
            intermediate_dns_policy,
        },
    )
}

/// Reify the added intermediate-status proof's exact responder path.
///
/// Earlier safe constructors already enforce every exact-object identity and
/// policy invariant in this direct responder-to-reviewed-root path. This
/// infallible nominal projection retains the complete prior composition and
/// accepts no caller-supplied certificate, path, root, algorithm, key,
/// signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root DNS policy for the
/// added responder, that responder's no-check or authenticated-revocation
/// policy, complete revocation policy, generalized path validation, TLS, HTTP,
/// OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path<
    Proof,
    Revocation,
>(
    no_check_intermediate_revocation: BoundX509OcspReviewedNoCheckIntermediateRevocation<
        Proof,
        Revocation,
    >,
) -> BoundX509OcspReviewedIntermediateStatusResponderPath<Proof, Revocation> {
    BoundX509OcspReviewedIntermediateStatusResponderPath {
        source: no_check_intermediate_revocation,
        evidence_types: PhantomData,
    }
}

/// Reify the added intermediate-status responder path from authenticated revocation.
///
/// The consumed composition already binds separately authenticated revocation
/// to the original routed responder and to the exact retained intermediate at
/// the selected reviewed root and original observation time. This infallible
/// nominal projection retains that complete composition and accepts no caller-
/// supplied certificate, path, root, algorithm, key, signature, status, or
/// time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root DNS policy for the
/// added responder, that responder's no-check or authenticated-revocation
/// policy, complete revocation policy, generalized path validation, TLS, HTTP,
/// OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_path<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_revocation_intermediate_revocation:
        BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
) -> BoundX509OcspReviewedIntermediateStatusResponderPath<
    Proof,
    IntermediateRevocation,
    BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >,
> {
    BoundX509OcspReviewedIntermediateStatusResponderPath {
        source: responder_revocation_intermediate_revocation,
        evidence_types: PhantomData,
    }
}

/// Reify the intermediate-status responder path after nested no-check policy.
///
/// The consumed composition already binds separately authenticated revocation
/// to the original routed responder, RFC 6960 no-check policy to that proof's
/// own responder, and separately authenticated status to the exact retained
/// intermediate at the selected reviewed root and original observation time.
/// This infallible nominal projection retains that complete composition and
/// accepts no caller-supplied certificate, path, root, algorithm, key,
/// signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It stops before reviewed-root DNS policy for the
/// intermediate-status proof's responder, that responder's no-check or
/// authenticated-revocation policy, complete revocation policy, generalized
/// path validation, TLS, HTTP, OAuth readiness, or network authority.
pub fn reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_no_check_intermediate_revocation:
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
) -> BoundX509OcspReviewedIntermediateStatusResponderPath<
    Proof,
    IntermediateRevocation,
    BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >,
> {
    BoundX509OcspReviewedIntermediateStatusResponderPath {
        source: responder_no_check_intermediate_revocation,
        evidence_types: PhantomData,
    }
}

/// Apply the selected reviewed root's supported DNS policy to the added responder.
///
/// The exact responder certificate and selected Mozilla-reviewed root are
/// derived only through the consumed intermediate-status responder-path proof.
/// Separate caller-owned limits bound both ASN.1 parses. Entries without
/// constraints succeed without reading the responder SAN or manufacturing DNS
/// evidence.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, no-check or separately
/// authenticated revocation policy for the added responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_anchor_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'intermediate_revocation_binding,
    'intermediate_revocation_basic_response,
    'intermediate_revocation_basic_encoded,
    'intermediate_revocation_single_response,
    'intermediate_revocation_single_encoded,
    'intermediate_revocation_responder,
    'intermediate_revocation_responder_encoded,
>(
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderPath<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'intermediate_revocation_binding,
            'intermediate_revocation_basic_response,
            'intermediate_revocation_basic_encoded,
            'intermediate_revocation_single_response,
            'intermediate_revocation_single_encoded,
            'intermediate_revocation_responder,
            'intermediate_revocation_responder_encoded,
            'candidates,
            'candidate_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'intermediate_revocation_binding,
            'intermediate_revocation_basic_response,
            'intermediate_revocation_basic_encoded,
            'intermediate_revocation_single_response,
            'intermediate_revocation_single_encoded,
            'intermediate_revocation_responder,
            'intermediate_revocation_responder_encoded,
            'candidates,
            'candidate_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
> {
    let no_check_intermediate_revocation = responder_path.no_check_intermediate_revocation();
    let responder_certificate = no_check_intermediate_revocation
        .intermediate_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let name_constraints = no_check_intermediate_revocation
        .responder_no_check_policy()
        .intermediate_dns_policy()
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation()
        .routed_reviewed_server_path()
        .anchor_policy()
        .reviewed_server_path()
        .reviewed_path()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

trait IntermediateStatusResponderEvidence {
    fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
}

trait IntermediateStatusResponderRevocationBindingEvidence:
    IntermediateStatusResponderEvidence
{
    fn intermediate_status_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn intermediate_status_observation_time(&self) -> x509_time::X509Time;
}

trait RoutedTargetRevocationBindingEvidence {
    fn revocation_target_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn revocation_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn revocation_observation_time(&self) -> x509_time::X509Time;
}

trait ReviewedResponderRevocationIntermediateBindingEvidence {
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn server_status_observation_time(&self) -> x509_time::X509Time;
}

trait ReviewedServerIntermediateBindingEvidence {
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_>;
    fn server_status_observation_time(&self) -> x509_time::X509Time;
}

trait ReviewedIntermediateStatusOriginalProofEvidence {
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]>;
}

trait IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation> {
    fn intermediate_revocation(&self) -> &IntermediateRevocation;
}

trait IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>:
    IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]>;
}

impl<Proof, IntermediateRevocation>
    IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>
{
    fn intermediate_revocation(&self) -> &IntermediateRevocation {
        self.intermediate_revocation()
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn intermediate_revocation(&self) -> &IntermediateRevocation {
        self.intermediate_revocation()
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
{
    fn intermediate_revocation(&self) -> &IntermediateRevocation {
        self.intermediate_revocation()
    }
}

impl<Proof, IntermediateRevocation>
    IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
        self.responder_no_check_policy()
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .selected_reviewed_root_name_constraints()
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
        self.responder_revocation()
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .selected_reviewed_root_name_constraints()
    }
}

impl<Proof, ResponderRevocation, IntermediateRevocation>
    IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>
    for BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
        self.responder_no_check_policy()
            .intermediate_dns_policy()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .selected_reviewed_root_name_constraints()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > ReviewedIntermediateStatusOriginalProofEvidence
    for BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
        self.routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .name_constraints()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    > ReviewedServerIntermediateBindingEvidence
    for BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .authenticated_intermediate()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate()
    }

    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate()
    }

    fn server_status_observation_time(&self) -> x509_time::X509Time {
        self.authenticated_revocation()
            .revocation()
            .observation_time()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'evidence,
        'initialized,
        'path,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > ReviewedServerIntermediateBindingEvidence
    for BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
        >,
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    >
{
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.authenticated_revocation()
            .target_issuer_signature()
            .issuer_rsa_algorithm_policy()
            .certificate()
    }

    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.strong_name_policy_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate()
    }

    fn server_status_observation_time(&self) -> x509_time::X509Time {
        self.authenticated_revocation()
            .revocation()
            .observation_time()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'evidence,
        'initialized,
        'path,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > ReviewedIntermediateStatusOriginalProofEvidence
    for BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
        >,
        ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
            'evidence,
            'initialized,
            'path,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
        >,
    >
{
    fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
        self.strong_name_policy_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .reviewed_terminal()
            .reviewed_anchor()
            .name_constraints()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > IntermediateStatusResponderEvidence
    for AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > IntermediateStatusResponderRevocationBindingEvidence
    for AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    fn intermediate_status_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.revocation().cert_id_binding().issuer()
    }

    fn intermediate_status_observation_time(&self) -> x509_time::X509Time {
        self.revocation().observation_time()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > RoutedTargetRevocationBindingEvidence
    for AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    fn revocation_target_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.revocation().cert_id_binding().target()
    }

    fn revocation_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.revocation().cert_id_binding().issuer()
    }

    fn revocation_observation_time(&self) -> x509_time::X509Time {
        self.revocation().observation_time()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
        ResponderRevocation,
    > ReviewedResponderRevocationIntermediateBindingEvidence
    for BoundX509OcspReviewedResponderRevocation<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
        ResponderRevocation,
    >
{
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .authenticated_intermediate()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate()
    }

    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate()
    }

    fn server_status_observation_time(&self) -> x509_time::X509Time {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .revocation()
            .observation_time()
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'evidence,
        'initialized,
        'path,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        ResponderRevocation,
    > ReviewedResponderRevocationIntermediateBindingEvidence
    for BoundX509OcspReviewedResponderRevocation<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'target,
                'target_encoded,
                'issuer,
                'issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
        ResponderRevocation,
    >
{
    fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .target_issuer_signature()
            .issuer_rsa_algorithm_policy()
            .certificate()
    }

    fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .strong_name_policy_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate()
    }

    fn server_status_observation_time(&self) -> x509_time::X509Time {
        self.intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .revocation()
            .observation_time()
    }
}

/// Apply reviewed-root DNS policy to an authenticated-branch status responder.
///
/// This is the authenticated-original-responder counterpart to
/// [`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_anchor_dns_policy`].
/// The added responder certificate and selected reviewed root constraints are
/// derived only through the consumed repository-owned path source. Entries
/// without constraints succeed without reading responder Subject Alternative
/// Name. Separate caller-owned limits bound both ASN.1 parses.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, no-check or separately
/// authenticated revocation policy for the added responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_anchor_dns_policy<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderPath<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
    IntermediateRevocation: IntermediateStatusResponderEvidence,
{
    let source = &responder_path.source;
    let responder_certificate = source
        .intermediate_revocation()
        .authenticated_responder_certificate();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        source.selected_reviewed_root_name_constraints(),
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

/// Apply reviewed-root DNS policy after nested responder no-check policy.
///
/// The added intermediate-status responder certificate and selected reviewed
/// root constraints are derived only through the consumed responder-of-
/// responder no-check composition. Entries without constraints succeed
/// without reading responder Subject Alternative Name. Separate caller-owned
/// limits bound both ASN.1 parses.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before excluded or non-DNS root constraints, no-check or separately
/// authenticated revocation policy for the added responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(clippy::type_complexity, private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_anchor_dns_policy<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_path: BoundX509OcspReviewedIntermediateStatusResponderPath<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
            Proof,
            ResponderRevocation,
            IntermediateRevocation,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>
where
    Proof: ReviewedIntermediateStatusOriginalProofEvidence,
    IntermediateRevocation: IntermediateStatusResponderEvidence,
{
    let source = &responder_path.source;
    let responder_certificate = source
        .intermediate_revocation()
        .authenticated_responder_certificate();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        source.selected_reviewed_root_name_constraints(),
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names,
        },
    )
}

/// Require RFC 6960 no-check policy for the added intermediate-status responder.
///
/// This consumes the complete reviewed-root DNS proof and derives the responder
/// certificate only through that retained proof. A structurally unique
/// `id-pkix-ocsp-nocheck` extension must be present with exact DER `NULL`
/// contents. RFC 6960 recommends a non-critical extension but does not require
/// it; a critical instance can reach this semantic gate only when the earlier
/// certificate-wide role policy explicitly recognized its OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before separately authenticated revocation for this responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy<
    Proof,
    Revocation,
    Source,
>(
    anchor_dns_policy: ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        Revocation,
        Source,
    >,
) -> Result<
    ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<Proof, Revocation, Source>,
    X509OcspReviewedResponderNoCheckPolicyError,
>
where
    Source: IntermediateStatusResponderPathSourceEvidence<Revocation>,
    Revocation: IntermediateStatusResponderEvidence,
{
    let responder_certificate = anchor_dns_policy
        .responder_path()
        .source
        .intermediate_revocation()
        .authenticated_responder_certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy { anchor_dns_policy })
}

/// Complete revocation policy for the exact direct no-check fixed path.
///
/// The input type can be constructed only after authenticated good-and-fresh
/// OCSP status covers the server target and retained intermediate, while exact
/// RFC 6960 no-check policy covers both the original responder and the
/// intermediate-status proof's responder. This infallible nominal composition
/// accepts no substitute certificate, path, root, algorithm, key, signature,
/// status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It does not generalize path construction or
/// validation, fetch, cache, acquire, or refresh revocation data, or provide
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn complete_x509_ocsp_routed_issuer_reviewed_direct_no_check_fixed_path_revocation_policy<
    Proof,
    IntermediateRevocation,
>(
    intermediate_status_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
        >,
) -> ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    IntermediateRevocation,
> {
    ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy {
        intermediate_status_responder_no_check_policy,
    }
}

/// Complete revocation policy for the exact nested no-check fixed path.
///
/// The input type can be constructed only after the retained server target,
/// original responder, responder-revocation proof's own responder,
/// intermediate, and intermediate-status proof's responder have each passed
/// the branch-specific authenticated-status or RFC 6960 no-check policy. This
/// infallible nominal composition accepts no substitute certificate, path,
/// root, algorithm, key, signature, status, or time.
///
/// This pure public-certificate-data composition performs no parsing,
/// cryptography, credential access or editing, signing, storage, clock
/// acquisition, provider action, audit or trace emission, I/O, network access,
/// or other external effect. It does not generalize path construction or
/// validation, fetch, cache, acquire, or refresh revocation data, or provide
/// TLS, HTTP, OAuth readiness, or network authority.
pub const fn complete_x509_ocsp_routed_issuer_reviewed_nested_no_check_fixed_path_revocation_policy<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    intermediate_status_responder_no_check_policy:
        ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<
            Proof,
            IntermediateRevocation,
            BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
                Proof,
                ResponderRevocation,
                IntermediateRevocation,
            >,
        >,
) -> ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
> {
    ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy {
        intermediate_status_responder_no_check_policy,
    }
}

/// Bind authenticated revocation to the added intermediate-status responder.
///
/// This alternative branch consumes the complete added-responder reviewed-root
/// DNS proof plus a separate fully routed authenticated OCSP target-revocation
/// proof. The second proof must target the same exact responder certificate,
/// name the same exact selected reviewed root, and use the original explicit
/// observation time. Failure order is stable: target identity, issuer identity,
/// then observation time.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before revocation policy for the second proof's own responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation<
    Proof,
    IntermediateRevocation,
    ResponderRevocation,
    Source,
>(
    anchor_dns_policy: ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        Proof,
        IntermediateRevocation,
        Source,
    >,
    responder_revocation: ResponderRevocation,
) -> Result<
    BoundX509OcspReviewedIntermediateStatusResponderRevocation<
        Proof,
        IntermediateRevocation,
        ResponderRevocation,
        Source,
    >,
    X509OcspReviewedResponderRevocationBindingError,
>
where
    Source: IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>,
    IntermediateRevocation: IntermediateStatusResponderRevocationBindingEvidence,
    ResponderRevocation: RoutedTargetRevocationBindingEvidence,
{
    let intermediate_status = anchor_dns_policy
        .responder_path()
        .source
        .intermediate_revocation();

    validate_responder_revocation_binding_inputs(
        intermediate_status.authenticated_responder_certificate(),
        intermediate_status.intermediate_status_issuer_certificate(),
        intermediate_status.intermediate_status_observation_time(),
        responder_revocation.revocation_target_certificate(),
        responder_revocation.revocation_issuer_certificate(),
        responder_revocation.revocation_observation_time(),
    )?;

    Ok(BoundX509OcspReviewedIntermediateStatusResponderRevocation {
        anchor_dns_policy,
        responder_revocation,
    })
}

fn validate_responder_revocation_binding_inputs(
    expected_responder: &x509_certificate::X509Certificate<'_>,
    expected_issuer: &x509_certificate::X509Certificate<'_>,
    expected_observation_time: x509_time::X509Time,
    revocation_target: &x509_certificate::X509Certificate<'_>,
    revocation_issuer: &x509_certificate::X509Certificate<'_>,
    revocation_observation_time: x509_time::X509Time,
) -> Result<(), X509OcspReviewedResponderRevocationBindingError> {
    if !std::ptr::eq(expected_responder, revocation_target) {
        return Err(X509OcspReviewedResponderRevocationBindingError::new(
            X509OcspReviewedResponderRevocationBindingErrorKind::TargetMismatch,
        ));
    }
    if !std::ptr::eq(expected_issuer, revocation_issuer) {
        return Err(X509OcspReviewedResponderRevocationBindingError::new(
            X509OcspReviewedResponderRevocationBindingErrorKind::IssuerMismatch,
        ));
    }
    if expected_observation_time.cmp_fields(&revocation_observation_time)
        != core::cmp::Ordering::Equal
    {
        return Err(X509OcspReviewedResponderRevocationBindingError::new(
            X509OcspReviewedResponderRevocationBindingErrorKind::ObservationTimeMismatch,
        ));
    }
    Ok(())
}

fn validate_intermediate_revocation_binding_inputs(
    expected_intermediate: &x509_certificate::X509Certificate<'_>,
    expected_root: &x509_certificate::X509Certificate<'_>,
    expected_observation_time: x509_time::X509Time,
    revocation_target: &x509_certificate::X509Certificate<'_>,
    revocation_issuer: &x509_certificate::X509Certificate<'_>,
    revocation_observation_time: x509_time::X509Time,
) -> Result<(), X509OcspReviewedIntermediateRevocationBindingError> {
    if !std::ptr::eq(expected_intermediate, revocation_target) {
        return Err(X509OcspReviewedIntermediateRevocationBindingError::new(
            X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch,
        ));
    }
    if !std::ptr::eq(expected_root, revocation_issuer) {
        return Err(X509OcspReviewedIntermediateRevocationBindingError::new(
            X509OcspReviewedIntermediateRevocationBindingErrorKind::IssuerMismatch,
        ));
    }
    if expected_observation_time.cmp_fields(&revocation_observation_time)
        != core::cmp::Ordering::Equal
    {
        return Err(X509OcspReviewedIntermediateRevocationBindingError::new(
            X509OcspReviewedIntermediateRevocationBindingErrorKind::ObservationTimeMismatch,
        ));
    }
    Ok(())
}

fn evaluate_responder_anchor_dns_policy(
    responder_certificate: &x509_certificate::X509Certificate<'_>,
    name_constraints: Option<&[u8]>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<Option<ConstrainedDnsNames>, X509OcspReviewedResponderAnchorDnsPolicyError> {
    let Some(encoded_constraints) = name_constraints else {
        return Ok(None);
    };

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints =
        decode_name_constraints_contents(&mut constraint_decoder, encoded_constraints).map_err(
            |error| {
                X509OcspReviewedResponderAnchorDnsPolicyError::new(
                    X509OcspReviewedResponderAnchorDnsPolicyErrorKind::InvalidAnchorNameConstraints(
                        error.kind(),
                    ),
                )
            },
        )?;
    let extension = responder_certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
        })
        .ok_or_else(|| {
            X509OcspReviewedResponderAnchorDnsPolicyError::new(
                X509OcspReviewedResponderAnchorDnsPolicyErrorKind::MissingResponderSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name = decode_subject_alt_name(&mut subject_alt_name_decoder, extension)
        .map_err(|error| {
            X509OcspReviewedResponderAnchorDnsPolicyError::new(
                X509OcspReviewedResponderAnchorDnsPolicyErrorKind::InvalidResponderSubjectAlternativeName(
                    error.kind(),
                ),
            )
        })?;
    let constrained_dns_names = evaluate_dns_names(&constraints, subject_alt_name.dns_names())
        .map_err(|error| {
            X509OcspReviewedResponderAnchorDnsPolicyError::new(
                X509OcspReviewedResponderAnchorDnsPolicyErrorKind::ResponderDnsNameConstraints(
                    error.kind(),
                ),
            )
        })?;

    Ok(Some(constrained_dns_names))
}

fn evaluate_responder_intermediate_dns_policy(
    responder_certificate: &x509_certificate::X509Certificate<'_>,
    intermediate_certificate: &x509_certificate::X509Certificate<'_>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<Option<ConstrainedDnsNames>, X509OcspReviewedResponderIntermediateDnsPolicyError> {
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
        return Err(X509OcspReviewedResponderIntermediateDnsPolicyError::new(
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::NonCriticalIntermediateNameConstraints,
        ));
    }

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints = decode_name_constraints(&mut constraint_decoder, extension.extension_value())
        .map_err(|error| {
            X509OcspReviewedResponderIntermediateDnsPolicyError::new(
                X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::InvalidIntermediateNameConstraints(
                    error.kind(),
                ),
            )
        })?;
    let subject_alt_name_extension = responder_certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
        })
        .ok_or_else(|| {
            X509OcspReviewedResponderIntermediateDnsPolicyError::new(
                X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::MissingResponderSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name =
        decode_subject_alt_name(&mut subject_alt_name_decoder, subject_alt_name_extension)
            .map_err(|error| {
                X509OcspReviewedResponderIntermediateDnsPolicyError::new(
                    X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::InvalidResponderSubjectAlternativeName(
                        error.kind(),
                    ),
                )
            })?;
    let constrained_dns_names = evaluate_dns_names(&constraints, subject_alt_name.dns_names())
        .map_err(|error| {
            X509OcspReviewedResponderIntermediateDnsPolicyError::new(
                X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::ResponderIntermediateDnsNameConstraints(
                    error.kind(),
                ),
            )
        })?;

    Ok(Some(constrained_dns_names))
}

fn evaluate_responder_no_check_policy(
    responder_certificate: &x509_certificate::X509Certificate<'_>,
) -> Result<(), X509OcspReviewedResponderNoCheckPolicyError> {
    let extension = responder_certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(OCSP_NO_CHECK_OID))
        })
        .ok_or_else(|| {
            X509OcspReviewedResponderNoCheckPolicyError::new(
                X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension,
            )
        })?;

    if extension.extension_value() != DER_NULL {
        return Err(X509OcspReviewedResponderNoCheckPolicyError::new(
            X509OcspReviewedResponderNoCheckPolicyErrorKind::InvalidNoCheckExtensionValue,
        ));
    }

    Ok(())
}

/// Apply the selected reviewed anchor's supported DNS policy to the responder.
///
/// The responder certificate and reviewed terminal are derived only through
/// the consumed exact responder-path proof. Separate caller-owned limits bound
/// both ASN.1 parses. Entries without constraints succeed without reading the
/// responder SAN and without manufacturing DNS evidence.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_reviewed_responder_anchor_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    responder_path: BoundX509OcspReviewedResponderPath<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
> {
    let reviewed_server_revocation = responder_path.reviewed_server_revocation();
    let responder_certificate = reviewed_server_revocation
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let name_constraints = reviewed_server_revocation
        .reviewed_server_path()
        .anchor_policy()
        .reviewed_intermediate_server_path()
        .reviewed_intermediate_path()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
        responder_path,
        constrained_dns_names,
    })
}

/// Apply the selected reviewed anchor's supported DNS policy to a routed responder.
///
/// The exact responder certificate and selected reviewed terminal are derived
/// only through the consumed fully routed responder-path proof. Separate
/// caller-owned limits bound both ASN.1 parses. Entries without constraints
/// succeed without reading the responder SAN or manufacturing DNS evidence.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before intermediate DNS, no-check, or revocation policy, generalized path
/// validation, TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    responder_path: BoundX509OcspReviewedResponderPath<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
> {
    let reviewed_server_revocation = responder_path.reviewed_server_revocation();
    let responder_certificate = reviewed_server_revocation
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let name_constraints = reviewed_server_revocation
        .routed_reviewed_server_path()
        .anchor_policy()
        .reviewed_server_path()
        .reviewed_path()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
        responder_path,
        constrained_dns_names,
    })
}

/// Apply the selected reviewed anchor's supported DNS policy to the responder
/// retained by the strongest Name-policy path bridge.
///
/// The exact responder certificate is derived only through the consumed fully
/// routed OCSP proof. The selected reviewed terminal and its optional out-of-
/// band constraint bytes are derived only through the retained strongest Name-
/// policy path. Separate caller-owned limits bound both ASN.1 parses. Entries
/// without constraints succeed without reading the responder SAN or
/// manufacturing DNS evidence.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before intermediate DNS, no-check, responder or intermediate revocation,
/// complete Name/path or revocation validation, TLS, HTTP, OAuth readiness, or
/// network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_anchor_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'revocation_target,
    'revocation_target_encoded,
    'revocation_issuer,
    'revocation_issuer_encoded,
    'evidence,
    'initialized,
    'path,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    responder_path: BoundX509OcspReviewedResponderPath<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    X509OcspReviewedResponderAnchorDnsPolicyError,
>{
    let strong_binding = responder_path.reviewed_server_revocation();
    let responder_certificate = strong_binding
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let name_constraints = strong_binding
        .strong_name_policy_path()
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .accumulated_intermediates()
        .reviewed_terminal()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_responder_anchor_dns_policy(
        responder_certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
        responder_path,
        constrained_dns_names,
    })
}

/// Apply the exact retained intermediate's supported critical DNS policy to
/// the responder retained by the strongest Name-policy path bridge.
///
/// This consumes the complete strong selected-anchor responder-policy proof.
/// The fully routed authenticated responder and target-nearest intermediate
/// are derived only through that retained non-copyable proof. An absent
/// intermediate NameConstraints extension succeeds without reading responder
/// SAN. A present extension must be critical and decode as the supported
/// permitted-and-excluded-DNS subset under separate caller-owned limits.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before no-check, responder or intermediate revocation, complete Name/path
/// or revocation validation, TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_intermediate_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'revocation_target,
    'revocation_target_encoded,
    'revocation_issuer,
    'revocation_issuer_encoded,
    'evidence,
    'initialized,
    'path,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    X509OcspReviewedResponderIntermediateDnsPolicyError,
>{
    let strong_binding = anchor_policy.responder_path().reviewed_server_revocation();
    let authenticated_revocation = strong_binding.authenticated_revocation();
    let responder_certificate = authenticated_revocation
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let intermediate_certificate = authenticated_revocation
        .target_issuer_signature()
        .issuer_rsa_algorithm_policy()
        .certificate();
    let constrained_dns_names = evaluate_responder_intermediate_dns_policy(
        responder_certificate,
        intermediate_certificate,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
        anchor_policy,
        constrained_dns_names,
    })
}

/// Require RFC 6960 no-check policy for the strongest-path responder.
///
/// This consumes the complete strongest-path responder intermediate-DNS proof
/// and derives the responder certificate only through that retained proof. A
/// structurally unique `id-pkix-ocsp-nocheck` extension must be present with
/// exact DER `NULL` contents. RFC 6960 recommends a non-critical extension but
/// does not require it; a critical instance can reach this semantic gate only
/// when the prerequisite certificate-wide role policy explicitly recognized
/// its OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before responder or intermediate revocation policy, generalized path
/// validation, TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_no_check_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'revocation_target,
    'revocation_target_encoded,
    'revocation_issuer,
    'revocation_issuer_encoded,
    'evidence,
    'initialized,
    'path,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
) -> Result<
    ValidatedX509OcspReviewedResponderNoCheckPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    X509OcspReviewedResponderNoCheckPolicyError,
>{
    let responder_certificate = intermediate_dns_policy
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation()
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(ValidatedX509OcspReviewedResponderNoCheckPolicy {
        intermediate_dns_policy,
    })
}

/// Bind authenticated revocation to the strongest-path responder.
///
/// This alternative to the no-check branch consumes the complete strongest-
/// path responder intermediate-DNS proof and a separate fully routed
/// authenticated OCSP target-revocation proof. The second proof must target
/// the same decoded responder object, name the same decoded target-nearest
/// intermediate issuer, and use the original server-status OCSP observation
/// time. Failure order is stable: target identity, issuer identity, then
/// observation time.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before revocation policy for the second proof's own responder or the
/// retained intermediate, generalized path validation, TLS, HTTP, OAuth
/// readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'revocation_target,
    'revocation_target_encoded,
    'revocation_issuer,
    'revocation_issuer_encoded,
    'evidence,
    'initialized,
    'path,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'responder_revocation_binding,
    'responder_revocation_basic_response,
    'responder_revocation_basic_encoded,
    'responder_revocation_single_response,
    'responder_revocation_single_encoded,
    'responder_revocation_responder,
    'responder_revocation_responder_encoded,
>(
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
    >,
    responder_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'responder_revocation_binding,
        'responder_revocation_basic_response,
        'responder_revocation_basic_encoded,
        'responder_revocation_single_response,
        'responder_revocation_single_encoded,
        'responder_revocation_responder,
        'responder_revocation_responder_encoded,
        'responder,
        'responder_encoded,
        'revocation_issuer,
        'revocation_issuer_encoded,
    >,
) -> Result<
    BoundX509OcspReviewedResponderRevocation<
        BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
            AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
                'binding,
                'basic_response,
                'basic_encoded,
                'single_response,
                'single_encoded,
                'responder,
                'responder_encoded,
                'revocation_target,
                'revocation_target_encoded,
                'revocation_issuer,
                'revocation_issuer_encoded,
            >,
            ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
                'evidence,
                'initialized,
                'path,
                'path_target,
                'path_target_encoded,
                'candidates,
                'candidate_encoded,
                'policy,
                'anchor_certificate,
                'anchor_encoded,
                'role_certificate,
                'role_encoded,
            >,
        >,
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'responder_revocation_binding,
            'responder_revocation_basic_response,
            'responder_revocation_basic_encoded,
            'responder_revocation_single_response,
            'responder_revocation_single_encoded,
            'responder_revocation_responder,
            'responder_revocation_responder_encoded,
            'responder,
            'responder_encoded,
            'revocation_issuer,
            'revocation_issuer_encoded,
        >,
    >,
    X509OcspReviewedResponderRevocationBindingError,
>{
    let strong_binding = intermediate_dns_policy
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation();
    let authenticated_revocation = strong_binding.authenticated_revocation();
    let expected_responder = authenticated_revocation
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let expected_issuer = authenticated_revocation
        .target_issuer_signature()
        .issuer_rsa_algorithm_policy()
        .certificate();
    let expected_observation_time = authenticated_revocation.revocation().observation_time();
    let revocation_binding = responder_revocation.revocation().cert_id_binding();

    validate_responder_revocation_binding_inputs(
        expected_responder,
        expected_issuer,
        expected_observation_time,
        revocation_binding.target(),
        revocation_binding.issuer(),
        responder_revocation.revocation().observation_time(),
    )?;

    Ok(BoundX509OcspReviewedResponderRevocation {
        intermediate_dns_policy,
        responder_revocation,
    })
}

/// Apply the exact routed intermediate's supported critical DNS policy.
///
/// This consumes the complete routed selected-anchor responder-policy proof.
/// The responder and intermediate are derived only through that retained
/// proof, preventing certificate, SAN, intermediate, or path substitution.
/// An absent extension succeeds without new evidence; a present extension
/// must be critical and decode as the supported permitted-and-excluded-DNS
/// subset under separate caller-owned limits.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before routed no-check or revocation policy, generalized path validation,
/// TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderIntermediateDnsPolicyError,
> {
    let reviewed_server_revocation = anchor_policy.responder_path().reviewed_server_revocation();
    let responder_certificate = reviewed_server_revocation
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let intermediate_certificate = reviewed_server_revocation
        .routed_reviewed_server_path()
        .anchor_policy()
        .reviewed_server_path()
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let constrained_dns_names = evaluate_responder_intermediate_dns_policy(
        responder_certificate,
        intermediate_certificate,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
        anchor_policy,
        constrained_dns_names,
    })
}

/// Apply the exact retained intermediate's supported critical DNS policy.
///
/// This consumes the complete selected-anchor responder-policy proof. The
/// responder and intermediate are derived only through that retained path,
/// preventing certificate, SAN, or path substitution. An absent extension
/// succeeds without new evidence; a present extension must be critical and
/// must decode as the supported permitted-and-excluded-DNS subset under
/// caller-owned limits.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_reviewed_responder_intermediate_dns_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderIntermediateDnsPolicyError,
> {
    let reviewed_server_revocation = anchor_policy.responder_path().reviewed_server_revocation();
    let responder_certificate = reviewed_server_revocation
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let intermediate_certificate = reviewed_server_revocation
        .reviewed_server_path()
        .anchor_policy()
        .reviewed_intermediate_server_path()
        .reviewed_intermediate_path()
        .authenticated_path()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let constrained_dns_names = evaluate_responder_intermediate_dns_policy(
        responder_certificate,
        intermediate_certificate,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
        anchor_policy,
        constrained_dns_names,
    })
}

/// Require RFC 6960 no-check policy for the exact routed-path responder.
///
/// This consumes the complete routed responder intermediate-DNS proof and
/// derives the responder certificate only through that retained proof. A
/// structurally unique `id-pkix-ocsp-nocheck` extension must be present with
/// exact DER `NULL` contents. RFC 6960 recommends a non-critical extension but
/// does not require it; a critical instance can reach this semantic gate only
/// when the prerequisite certificate-wide role policy explicitly recognized
/// its OID.
///
/// This pure public-certificate-data policy performs no credential access or
/// editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before responder or intermediate revocation policy, generalized path
/// validation, TLS, HTTP, OAuth readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_routed_issuer_reviewed_responder_no_check_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
) -> Result<
    ValidatedX509OcspReviewedResponderNoCheckPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderNoCheckPolicyError,
> {
    let responder_certificate = intermediate_dns_policy
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation()
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(ValidatedX509OcspReviewedResponderNoCheckPolicy {
        intermediate_dns_policy,
    })
}

/// Bind authenticated revocation to the exact routed-path responder.
///
/// This alternative to the no-check branch consumes the complete routed
/// responder intermediate-DNS proof and a separate fully routed authenticated
/// OCSP target-revocation proof. The second proof must target the same decoded
/// responder object, name the same decoded intermediate issuer, and use the
/// original server-status OCSP observation time. Failure order is stable:
/// target identity, issuer identity, then observation time.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before revocation policy for the second proof's own responder or the
/// retained intermediate, generalized path validation, TLS, HTTP, OAuth
/// readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_responder_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'responder_revocation_binding,
    'responder_revocation_basic_response,
    'responder_revocation_basic_encoded,
    'responder_revocation_single_response,
    'responder_revocation_single_encoded,
    'responder_revocation_responder,
    'responder_revocation_responder_encoded,
>(
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    responder_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'responder_revocation_binding,
        'responder_revocation_basic_response,
        'responder_revocation_basic_encoded,
        'responder_revocation_single_response,
        'responder_revocation_single_encoded,
        'responder_revocation_responder,
        'responder_revocation_responder_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    BoundX509OcspReviewedResponderRevocation<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'responder_revocation_binding,
            'responder_revocation_basic_response,
            'responder_revocation_basic_encoded,
            'responder_revocation_single_response,
            'responder_revocation_single_encoded,
            'responder_revocation_responder,
            'responder_revocation_responder_encoded,
            'responder,
            'responder_encoded,
            'issuer,
            'issuer_encoded,
        >,
    >,
    X509OcspReviewedResponderRevocationBindingError,
> {
    let reviewed_server_revocation = intermediate_dns_policy
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation();
    let expected_responder = reviewed_server_revocation
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let expected_issuer = reviewed_server_revocation
        .routed_reviewed_server_path()
        .anchor_policy()
        .reviewed_server_path()
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let expected_observation_time = reviewed_server_revocation
        .authenticated_revocation()
        .revocation()
        .observation_time();
    let revocation_binding = responder_revocation.revocation().cert_id_binding();

    validate_responder_revocation_binding_inputs(
        expected_responder,
        expected_issuer,
        expected_observation_time,
        revocation_binding.target(),
        revocation_binding.issuer(),
        responder_revocation.revocation().observation_time(),
    )?;

    Ok(BoundX509OcspReviewedResponderRevocation {
        intermediate_dns_policy,
        responder_revocation,
    })
}

/// Bind authenticated revocation to the exact routed-path intermediate.
///
/// This independent branch consumes the complete fully routed reviewed
/// server-revocation proof and a separate fully routed authenticated OCSP
/// target-revocation proof. The second proof must target the same decoded
/// intermediate object, name the same decoded selected reviewed root, and use
/// the original server-status OCSP observation time. Failure order is stable:
/// target identity, issuer identity, then observation time.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It remains
/// independent of responder policy and stops before revocation policy for the
/// second proof's own responder, generalized path validation, TLS, HTTP, OAuth
/// readiness, or network authority.
#[allow(clippy::type_complexity)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_intermediate_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
    'intermediate_revocation_binding,
    'intermediate_revocation_basic_response,
    'intermediate_revocation_basic_encoded,
    'intermediate_revocation_single_response,
    'intermediate_revocation_single_encoded,
    'intermediate_revocation_responder,
    'intermediate_revocation_responder_encoded,
>(
    reviewed_server_revocation: BoundX509OcspRoutedIssuerReviewedServerRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
        'path,
        'server_certificate,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    intermediate_revocation: AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'intermediate_revocation_binding,
        'intermediate_revocation_basic_response,
        'intermediate_revocation_basic_encoded,
        'intermediate_revocation_single_response,
        'intermediate_revocation_single_encoded,
        'intermediate_revocation_responder,
        'intermediate_revocation_responder_encoded,
        'candidates,
        'candidate_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> Result<
    BoundX509OcspReviewedIntermediateRevocation<
        BoundX509OcspRoutedIssuerReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'intermediate_revocation_binding,
            'intermediate_revocation_basic_response,
            'intermediate_revocation_basic_encoded,
            'intermediate_revocation_single_response,
            'intermediate_revocation_single_encoded,
            'intermediate_revocation_responder,
            'intermediate_revocation_responder_encoded,
            'candidates,
            'candidate_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedIntermediateRevocationBindingError,
> {
    let routed_path = reviewed_server_revocation.routed_reviewed_server_path();
    let reviewed_path = routed_path
        .anchor_policy()
        .reviewed_server_path()
        .reviewed_path();
    let expected_intermediate = reviewed_path
        .authenticated_intermediate()
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let expected_root = reviewed_path
        .reviewed_terminal()
        .reviewed_anchor()
        .candidate_certificate();
    let expected_observation_time = reviewed_server_revocation
        .authenticated_revocation()
        .revocation()
        .observation_time();
    let revocation_binding = intermediate_revocation.revocation().cert_id_binding();

    validate_intermediate_revocation_binding_inputs(
        expected_intermediate,
        expected_root,
        expected_observation_time,
        revocation_binding.target(),
        revocation_binding.issuer(),
        intermediate_revocation.revocation().observation_time(),
    )?;

    Ok(BoundX509OcspReviewedIntermediateRevocation {
        reviewed_server_revocation,
        intermediate_revocation,
    })
}

/// Join routed responder no-check policy to exact intermediate status.
///
/// This consumes the complete fully routed responder no-check proof and a
/// separate fully routed authenticated OCSP target-revocation proof. The
/// retained intermediate, selected reviewed root, and original server-status
/// observation time are derived only through the consumed no-check proof.
/// The second proof must target and name those exact certificate objects at
/// that exact time. Failure order is stable: target identity, issuer identity,
/// then observation time.
///
/// Private repository-owned evidence interfaces admit both the fixed-depth
/// and strongest Name-policy-backed proof types while preventing external
/// substitute implementations.
///
/// This pure public-certificate-data composition performs no credential access
/// or editing, signing, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, network access, or other external effect. It stops
/// before revocation policy for the added proof's own responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_no_check_intermediate_revocation<
    Proof,
    IntermediateRevocation,
>(
    responder_no_check_policy: ValidatedX509OcspReviewedResponderNoCheckPolicy<Proof>,
    intermediate_revocation: IntermediateRevocation,
) -> Result<
    BoundX509OcspReviewedNoCheckIntermediateRevocation<Proof, IntermediateRevocation>,
    X509OcspReviewedIntermediateRevocationBindingError,
>
where
    Proof: ReviewedServerIntermediateBindingEvidence,
    IntermediateRevocation: RoutedTargetRevocationBindingEvidence,
{
    let reviewed_server_revocation = responder_no_check_policy
        .intermediate_dns_policy()
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation();

    validate_intermediate_revocation_binding_inputs(
        reviewed_server_revocation.retained_intermediate_certificate(),
        reviewed_server_revocation.selected_reviewed_root_certificate(),
        reviewed_server_revocation.server_status_observation_time(),
        intermediate_revocation.revocation_target_certificate(),
        intermediate_revocation.revocation_issuer_certificate(),
        intermediate_revocation.revocation_observation_time(),
    )?;

    Ok(BoundX509OcspReviewedNoCheckIntermediateRevocation {
        responder_no_check_policy,
        intermediate_revocation,
    })
}

/// Join separately authenticated responder and intermediate status branches.
///
/// This consumes the complete fully routed responder-revocation proof and a
/// separate fully routed authenticated OCSP target-revocation proof. The
/// retained intermediate, selected reviewed root, and original server-status
/// observation time are derived only through the consumed responder branch.
/// The second proof must target and name those exact certificate objects at
/// that exact time. Failure order is stable: target identity, issuer identity,
/// then observation time.
///
/// Private repository-owned evidence interfaces admit the concrete proof types
/// but prevent external substitute implementations. This pure public-
/// certificate-data composition performs no credential access or editing,
/// signing, storage, clock acquisition, provider action, audit or trace
/// emission, I/O, network access, or other external effect. It stops before
/// revocation policy for either added proof's own responder, complete
/// revocation policy, generalized path validation, TLS, HTTP, OAuth readiness,
/// or network authority.
#[allow(private_bounds)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_revocation: BoundX509OcspReviewedResponderRevocation<Proof, ResponderRevocation>,
    intermediate_revocation: IntermediateRevocation,
) -> Result<
    BoundX509OcspReviewedResponderRevocationIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >,
    X509OcspReviewedIntermediateRevocationBindingError,
>
where
    BoundX509OcspReviewedResponderRevocation<Proof, ResponderRevocation>:
        ReviewedResponderRevocationIntermediateBindingEvidence,
    IntermediateRevocation: RoutedTargetRevocationBindingEvidence,
{
    validate_intermediate_revocation_binding_inputs(
        responder_revocation.retained_intermediate_certificate(),
        responder_revocation.selected_reviewed_root_certificate(),
        responder_revocation.server_status_observation_time(),
        intermediate_revocation.revocation_target_certificate(),
        intermediate_revocation.revocation_issuer_certificate(),
        intermediate_revocation.revocation_observation_time(),
    )?;

    Ok(
        BoundX509OcspReviewedResponderRevocationIntermediateRevocation {
            responder_revocation,
            intermediate_revocation,
        },
    )
}

/// Join responder-of-responder no-check policy to exact intermediate status.
///
/// This consumes the complete responder-of-responder RFC 6960 no-check proof
/// and a separate fully routed authenticated OCSP target-revocation proof. The
/// retained intermediate, selected reviewed root, and original server-status
/// observation time are derived only through the consumed no-check proof. The
/// second proof must target and name those exact certificate objects at that
/// exact time. Failure order is stable: target identity, issuer identity, then
/// observation time.
///
/// Private repository-owned evidence interfaces admit the concrete proof types
/// but prevent external substitute implementations. This pure public-
/// certificate-data composition performs no credential access or editing,
/// signing, storage, clock acquisition, provider action, audit or trace
/// emission, I/O, network access, or other external effect. It stops before
/// revocation policy for the intermediate-status proof's own responder,
/// complete revocation policy, generalized path validation, TLS, HTTP, OAuth
/// readiness, or network authority.
#[allow(private_bounds)]
pub fn bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation<
    Proof,
    ResponderRevocation,
    IntermediateRevocation,
>(
    responder_no_check_policy: ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<
        Proof,
        ResponderRevocation,
    >,
    intermediate_revocation: IntermediateRevocation,
) -> Result<
    BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation<
        Proof,
        ResponderRevocation,
        IntermediateRevocation,
    >,
    X509OcspReviewedIntermediateRevocationBindingError,
>
where
    BoundX509OcspReviewedResponderRevocation<Proof, ResponderRevocation>:
        ReviewedResponderRevocationIntermediateBindingEvidence,
    IntermediateRevocation: RoutedTargetRevocationBindingEvidence,
{
    let responder_revocation = responder_no_check_policy
        .intermediate_dns_policy()
        .anchor_dns_policy()
        .responder_path()
        .responder_revocation();
    validate_intermediate_revocation_binding_inputs(
        responder_revocation.retained_intermediate_certificate(),
        responder_revocation.selected_reviewed_root_certificate(),
        responder_revocation.server_status_observation_time(),
        intermediate_revocation.revocation_target_certificate(),
        intermediate_revocation.revocation_issuer_certificate(),
        intermediate_revocation.revocation_observation_time(),
    )?;

    Ok(
        BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation,
        },
    )
}

/// Require RFC 6960 no-check policy for the exact reviewed-path responder.
///
/// This consumes the complete responder intermediate-DNS proof and derives the
/// responder certificate only through that retained path. A structurally
/// unique `id-pkix-ocsp-nocheck` extension must be present with exact DER
/// `NULL` contents. RFC 6960 recommends a non-critical extension but does not
/// require it; a critical instance can reach this semantic gate only when the
/// prerequisite certificate-wide role policy explicitly recognized its OID.
#[allow(clippy::type_complexity)]
pub fn validate_x509_ocsp_reviewed_responder_no_check_policy<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    'path,
    'server_certificate,
    'path_target,
    'path_target_encoded,
    'candidates,
    'candidate_encoded,
>(
    intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
) -> Result<
    ValidatedX509OcspReviewedResponderNoCheckPolicy<
        BoundX509OcspReviewedServerRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'target,
            'target_encoded,
            'issuer,
            'issuer_encoded,
            'path,
            'server_certificate,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    >,
    X509OcspReviewedResponderNoCheckPolicyError,
> {
    let responder_certificate = intermediate_dns_policy
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation()
        .authenticated_revocation()
        .issuer_bound_responder()
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    evaluate_responder_no_check_policy(responder_certificate)?;

    Ok(ValidatedX509OcspReviewedResponderNoCheckPolicy {
        intermediate_dns_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use x509_certificate::decode_x509_certificate;
    use x509_time::decode_x509_time;

    const LEAF_HEX: &str = include_str!("../tests/fixtures/apple-www-leaf.der.hex");
    const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const NAME_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x1e];
    const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];
    const OCSP_NO_CHECK_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x05];
    const NULL_DER: &[u8] = &[0x05, 0x00];

    #[test]
    fn strongest_name_policy_proof_is_admitted_to_generic_private_evidence() {
        fn assert_original_proof_evidence<T: ReviewedIntermediateStatusOriginalProofEvidence>() {}
        fn assert_no_check_intermediate_binding_evidence<
            T: ReviewedServerIntermediateBindingEvidence,
        >() {
        }
        fn assert_intermediate_binding_evidence<
            T: ReviewedResponderRevocationIntermediateBindingEvidence,
        >() {
        }
        fn assert_direct_no_check_completion_accepts<T>() {
            let _: fn(
                ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy<T, u8>,
            ) -> ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy<T, u8> =
                complete_x509_ocsp_routed_issuer_reviewed_direct_no_check_fixed_path_revocation_policy::<
                    T,
                    u8,
                >;
        }

        type StrongestOriginalProof =
            BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
                AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
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
                >,
                ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
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
                >,
            >;

        assert_original_proof_evidence::<StrongestOriginalProof>();
        assert_no_check_intermediate_binding_evidence::<StrongestOriginalProof>();
        assert_direct_no_check_completion_accepts::<StrongestOriginalProof>();
        assert_intermediate_binding_evidence::<
            BoundX509OcspReviewedResponderRevocation<StrongestOriginalProof, u8>,
        >();
    }

    fn fixture(hexadecimal: &str) -> Vec<u8> {
        let digits = hexadecimal
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        digits
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    fn time(encoded: &'static [u8]) -> x509_time::X509Time {
        let mut decoder = Asn1Decoder::new(Asn1Limits::default());
        decode_x509_time(decoder.decode_exact(encoded).unwrap()).unwrap()
    }

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

    fn permitted_dns_subtrees(subtrees: &[&[u8]]) -> Vec<u8> {
        let mut contents = Vec::new();
        for subtree in subtrees {
            contents.extend(der(0x30, &der(0x82, subtree)));
        }
        der(0xa0, &contents)
    }

    fn excluded_dns_subtrees(subtrees: &[&[u8]]) -> Vec<u8> {
        let mut contents = Vec::new();
        for subtree in subtrees {
            contents.extend(der(0x30, &der(0x82, subtree)));
        }
        der(0xa1, &contents)
    }

    fn name_constraints(permitted: &[&[u8]], excluded: &[&[u8]], critical: bool) -> Vec<u8> {
        let mut fields = Vec::new();
        if !permitted.is_empty() {
            fields.extend(permitted_dns_subtrees(permitted));
        }
        if !excluded.is_empty() {
            fields.extend(excluded_dns_subtrees(excluded));
        }
        let value = der(0x30, &fields);
        if critical {
            critical_extension(NAME_CONSTRAINTS_OID_DER, &value)
        } else {
            extension(NAME_CONSTRAINTS_OID_DER, &value)
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
        tbs.extend(name(b"responder.example.test"));
        tbs.extend(der(0x30, &subject_public_key_info));
        if !extensions.is_empty() {
            tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
        }

        let mut fields = der(0x30, &tbs);
        fields.extend(algorithm(SIGNATURE_OID_DER));
        fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
        der(0x30, &fields)
    }

    fn evaluate_responder(
        encoded: &[u8],
        constraints: Option<&[u8]>,
    ) -> Result<Option<ConstrainedDnsNames>, X509OcspReviewedResponderAnchorDnsPolicyError> {
        let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
        evaluate_responder_anchor_dns_policy(
            &certificate,
            constraints,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_responder_intermediate(
        responder_encoded: &[u8],
        intermediate_encoded: &[u8],
    ) -> Result<Option<ConstrainedDnsNames>, X509OcspReviewedResponderIntermediateDnsPolicyError>
    {
        let responder = decode_x509_certificate(responder_encoded, Asn1Limits::default()).unwrap();
        let intermediate =
            decode_x509_certificate(intermediate_encoded, Asn1Limits::default()).unwrap();
        evaluate_responder_intermediate_dns_policy(
            &responder,
            &intermediate,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
    }

    fn evaluate_responder_no_check(
        responder_encoded: &[u8],
    ) -> Result<(), X509OcspReviewedResponderNoCheckPolicyError> {
        let responder = decode_x509_certificate(responder_encoded, Asn1Limits::default()).unwrap();
        evaluate_responder_no_check_policy(&responder)
    }

    struct TestIntermediateStatusResponderEvidence<'certificate, 'encoded> {
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
    }

    impl IntermediateStatusResponderEvidence for TestIntermediateStatusResponderEvidence<'_, '_> {
        fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.added_responder
        }
    }

    fn intermediate_status_anchor_dns_policy<'certificate, 'encoded>(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
    ) -> ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        &'certificate x509_certificate::X509Certificate<'encoded>,
        TestIntermediateStatusResponderEvidence<'certificate, 'encoded>,
    > {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: original_responder,
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let no_check_intermediate_revocation = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: TestIntermediateStatusResponderEvidence {
                original_responder,
                added_responder,
            },
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(
                no_check_intermediate_revocation,
            );

        ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names: None,
        }
    }

    struct TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded> {
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    }

    impl IntermediateStatusResponderEvidence
        for TestIntermediateStatusResponderRevocationEvidence<'_, '_>
    {
        fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.added_responder
        }
    }

    impl IntermediateStatusResponderRevocationBindingEvidence
        for TestIntermediateStatusResponderRevocationEvidence<'_, '_>
    {
        fn intermediate_status_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn intermediate_status_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    impl RoutedTargetRevocationBindingEvidence
        for TestIntermediateStatusResponderRevocationEvidence<'_, '_>
    {
        fn revocation_target_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.original_responder
        }

        fn revocation_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn revocation_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    struct TestTargetRevocationBindingEvidence<'certificate, 'encoded> {
        target: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    }

    impl RoutedTargetRevocationBindingEvidence for TestTargetRevocationBindingEvidence<'_, '_> {
        fn revocation_target_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.target
        }

        fn revocation_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn revocation_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    struct TestGenericIntermediateStatusSource<IntermediateRevocation> {
        intermediate_revocation: IntermediateRevocation,
        name_constraints: Option<&'static [u8]>,
    }

    impl<IntermediateRevocation>
        IntermediateStatusResponderPathSourceEvidence<IntermediateRevocation>
        for TestGenericIntermediateStatusSource<IntermediateRevocation>
    {
        fn intermediate_revocation(&self) -> &IntermediateRevocation {
            &self.intermediate_revocation
        }
    }

    impl<IntermediateRevocation>
        IntermediateStatusResponderAnchorSourceEvidence<IntermediateRevocation>
        for TestGenericIntermediateStatusSource<IntermediateRevocation>
    {
        fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
            self.name_constraints
        }
    }

    struct TestResponderTargetRevocationBindingEvidence<'certificate, 'encoded> {
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        target: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    }

    impl IntermediateStatusResponderEvidence for TestResponderTargetRevocationBindingEvidence<'_, '_> {
        fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.responder
        }
    }

    impl IntermediateStatusResponderRevocationBindingEvidence
        for TestResponderTargetRevocationBindingEvidence<'_, '_>
    {
        fn intermediate_status_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn intermediate_status_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    impl RoutedTargetRevocationBindingEvidence
        for TestResponderTargetRevocationBindingEvidence<'_, '_>
    {
        fn revocation_target_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.target
        }

        fn revocation_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn revocation_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    struct TestReviewedResponderRevocationIntermediateEvidence<'certificate, 'encoded> {
        intermediate: &'certificate x509_certificate::X509Certificate<'encoded>,
        root: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    }

    struct TestResponderRevocationResponderAnchorOriginalEvidence {
        name_constraints: Option<&'static [u8]>,
    }

    impl ReviewedIntermediateStatusOriginalProofEvidence
        for TestResponderRevocationResponderAnchorOriginalEvidence
    {
        fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
            self.name_constraints
        }
    }

    struct TestResponderRevocationResponderAnchorEvidence<'certificate, 'encoded> {
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
    }

    impl IntermediateStatusResponderEvidence
        for TestResponderRevocationResponderAnchorEvidence<'_, '_>
    {
        fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.responder
        }
    }

    struct TestResponderRevocationResponderIntermediateEvidence<'certificate, 'encoded> {
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    }

    impl IntermediateStatusResponderEvidence
        for TestResponderRevocationResponderIntermediateEvidence<'_, '_>
    {
        fn authenticated_responder_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.responder
        }
    }

    impl IntermediateStatusResponderRevocationBindingEvidence
        for TestResponderRevocationResponderIntermediateEvidence<'_, '_>
    {
        fn intermediate_status_issuer_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.issuer
        }

        fn intermediate_status_observation_time(&self) -> x509_time::X509Time {
            self.observation_time
        }
    }

    fn responder_revocation_responder_intermediate_dns_policy<'certificate, 'encoded>(
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
    ) -> ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestResponderRevocationResponderIntermediateEvidence<'certificate, 'encoded>,
    > {
        let responder_revocation = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation:
                            TestResponderRevocationResponderAnchorOriginalEvidence {
                                name_constraints: None,
                            },
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
            responder_revocation: TestResponderRevocationResponderIntermediateEvidence {
                responder,
                issuer,
                observation_time: time(b"\x17\x0d261025120000Z"),
            },
        };

        ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy {
            anchor_dns_policy:
                ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy {
                    responder_path:
                        reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(
                            responder_revocation,
                        ),
                    constrained_dns_names: None,
                },
            constrained_dns_names: None,
        }
    }

    fn responder_revocation_responder_no_check_intermediate_binding_evidence<
        'certificate,
        'encoded,
    >(
        intermediate: &'certificate x509_certificate::X509Certificate<'encoded>,
        root: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    ) -> ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<
        TestReviewedResponderRevocationIntermediateEvidence<'certificate, 'encoded>,
        u8,
    > {
        responder_revocation_responder_no_check_intermediate_binding_evidence_with_constraints(
            intermediate,
            root,
            observation_time,
            None,
        )
    }

    fn responder_revocation_responder_no_check_intermediate_binding_evidence_with_constraints<
        'certificate,
        'encoded,
    >(
        intermediate: &'certificate x509_certificate::X509Certificate<'encoded>,
        root: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    ) -> ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy<
        TestReviewedResponderRevocationIntermediateEvidence<'certificate, 'encoded>,
        u8,
    > {
        let responder_revocation = responder_revocation_intermediate_evidence_with_constraints(
            intermediate,
            root,
            observation_time,
            name_constraints,
        );

        ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy {
            intermediate_dns_policy:
                ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy {
                    anchor_dns_policy:
                        ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy {
                            responder_path: BoundX509OcspReviewedResponderRevocationResponderPath {
                                responder_revocation,
                            },
                            constrained_dns_names: None,
                        },
                    constrained_dns_names: None,
                },
        }
    }

    impl ReviewedIntermediateStatusOriginalProofEvidence
        for TestReviewedResponderRevocationIntermediateEvidence<'_, '_>
    {
        fn selected_reviewed_root_name_constraints(&self) -> Option<&'static [u8]> {
            self.name_constraints
        }
    }

    impl ReviewedResponderRevocationIntermediateBindingEvidence
        for BoundX509OcspReviewedResponderRevocation<
            TestReviewedResponderRevocationIntermediateEvidence<'_, '_>,
            u8,
        >
    {
        fn retained_intermediate_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation()
                .intermediate
        }

        fn selected_reviewed_root_certificate(&self) -> &x509_certificate::X509Certificate<'_> {
            self.intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation()
                .root
        }

        fn server_status_observation_time(&self) -> x509_time::X509Time {
            self.intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation()
                .observation_time
        }
    }

    fn responder_revocation_intermediate_evidence<'certificate, 'encoded>(
        intermediate: &'certificate x509_certificate::X509Certificate<'encoded>,
        root: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    ) -> BoundX509OcspReviewedResponderRevocation<
        TestReviewedResponderRevocationIntermediateEvidence<'certificate, 'encoded>,
        u8,
    > {
        responder_revocation_intermediate_evidence_with_constraints(
            intermediate,
            root,
            observation_time,
            None,
        )
    }

    fn responder_revocation_intermediate_evidence_with_constraints<'certificate, 'encoded>(
        intermediate: &'certificate x509_certificate::X509Certificate<'encoded>,
        root: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    ) -> BoundX509OcspReviewedResponderRevocation<
        TestReviewedResponderRevocationIntermediateEvidence<'certificate, 'encoded>,
        u8,
    > {
        BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation:
                            TestReviewedResponderRevocationIntermediateEvidence {
                                intermediate,
                                root,
                                observation_time,
                                name_constraints,
                            },
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
            responder_revocation: 7,
        }
    }

    fn intermediate_status_revocation_anchor_dns_policy<'certificate, 'encoded>(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    ) -> ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded>,
    > {
        intermediate_status_revocation_anchor_dns_policy_with_constraints(
            original_responder,
            added_responder,
            issuer,
            observation_time,
            None,
        )
    }

    fn intermediate_status_revocation_anchor_dns_policy_with_constraints<'certificate, 'encoded>(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    ) -> ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded>,
    > {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation:
                            TestResponderRevocationResponderAnchorOriginalEvidence {
                                name_constraints,
                            },
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let no_check_intermediate_revocation = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: TestIntermediateStatusResponderRevocationEvidence {
                original_responder,
                added_responder,
                issuer,
                observation_time,
            },
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(
                no_check_intermediate_revocation,
            );

        ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names: None,
        }
    }

    fn intermediate_status_responder_revocation_responder_path<'certificate, 'encoded>(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    ) -> BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded>,
        TestResponderTargetRevocationBindingEvidence<'certificate, 'encoded>,
    > {
        let anchor_dns_policy = intermediate_status_revocation_anchor_dns_policy_with_constraints(
            original_responder,
            added_responder,
            issuer,
            observation_time,
            name_constraints,
        );
        let bound = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            TestResponderTargetRevocationBindingEvidence {
                responder,
                target: added_responder,
                issuer,
                observation_time,
            },
        )
        .unwrap();

        reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path(
            bound,
        )
    }

    fn intermediate_status_responder_revocation_responder_anchor_dns_policy<
        'certificate,
        'encoded,
    >(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
    ) -> ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded>,
        TestResponderTargetRevocationBindingEvidence<'certificate, 'encoded>,
    > {
        let responder_path = intermediate_status_responder_revocation_responder_path(
            original_responder,
            added_responder,
            issuer,
            responder,
            observation_time,
            None,
        );

        validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap()
    }

    fn recursive_intermediate_status_responder_revocation_responder_path<
        'certificate,
        'encoded,
    >(
        original_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        added_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        issuer: &'certificate x509_certificate::X509Certificate<'encoded>,
        responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        recursive_responder: &'certificate x509_certificate::X509Certificate<'encoded>,
        observation_time: x509_time::X509Time,
        name_constraints: Option<&'static [u8]>,
    ) -> BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath<
        TestResponderRevocationResponderAnchorOriginalEvidence,
        TestIntermediateStatusResponderRevocationEvidence<'certificate, 'encoded>,
        TestResponderTargetRevocationBindingEvidence<'certificate, 'encoded>,
        TestResponderTargetRevocationBindingEvidence<'certificate, 'encoded>,
    >{
        let responder_path = intermediate_status_responder_revocation_responder_path(
            original_responder,
            added_responder,
            issuer,
            responder,
            observation_time,
            name_constraints,
        );
        let responder_dns = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let recursive = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation(
            responder_dns,
            TestResponderTargetRevocationBindingEvidence {
                responder: recursive_responder,
                target: responder,
                issuer,
                observation_time,
            },
        )
        .unwrap();

        reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_path(
            recursive,
        )
    }

    #[test]
    fn target_expiry_fails_before_issuer_validity_is_evaluated() {
        let encoded = fixture(LEAF_HEX);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let after_leaf_expiry = time(b"\x17\x0d261217000000Z");

        let error = retain_path_validity_at_observation(
            validate_certificate_validity_at(&certificate, &after_leaf_expiry)
                .map_err(|error| error.kind()),
            || panic!("issuer validity must follow successful target validity"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedServerRevocationBindingErrorKind::TargetCertificateValidity(
                X509CertificateValidityErrorKind::Expired
            )
        );
    }

    #[test]
    fn issuer_validity_failure_is_redacted_after_valid_target() {
        let encoded = fixture(LEAF_HEX);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let within_leaf_validity = time(b"\x17\x0d261025120000Z");
        let after_leaf_expiry = time(b"\x17\x0d261217000000Z");

        let error = retain_path_validity_at_observation(
            validate_certificate_validity_at(&certificate, &within_leaf_validity)
                .map_err(|error| error.kind()),
            || {
                validate_certificate_validity_at(&certificate, &after_leaf_expiry)
                    .map_err(|error| error.kind())
            },
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedServerRevocationBindingErrorKind::IssuerCertificateValidity(
                X509CertificateValidityErrorKind::Expired
            )
        );
        assert_eq!(
            error.to_string(),
            "X.509 OCSP reviewed-server revocation binding error IssuerCertificateValidity(Expired)"
        );
    }

    #[test]
    fn absent_anchor_constraints_do_not_require_a_responder_san() {
        let encoded = certificate(&[]);

        assert_eq!(evaluate_responder(&encoded, None).unwrap(), None);
    }

    #[test]
    fn selected_anchor_constraints_apply_to_every_responder_dns_name() {
        let encoded = certificate(&[subject_alt_name(&[
            dns_name("ocsp.example.test"),
            dns_name("backup.example.test"),
        ])]);
        let constraints = permitted_dns_subtrees(&[b".example.test"]);

        let constrained = evaluate_responder(&encoded, Some(&constraints))
            .unwrap()
            .unwrap();

        assert_eq!(constrained.count(), 2);
    }

    #[test]
    fn constrained_anchor_requires_responder_subject_alt_name() {
        let encoded = certificate(&[]);
        let constraints = permitted_dns_subtrees(&[b".example.test"]);

        let error = evaluate_responder(&encoded, Some(&constraints)).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::MissingResponderSubjectAlternativeName
        );
        assert_eq!(
            error.to_string(),
            "X.509 OCSP reviewed-responder anchor DNS policy error MissingResponderSubjectAlternativeName"
        );
    }

    #[test]
    fn malformed_anchor_constraints_are_redacted() {
        let encoded = certificate(&[]);

        let error = evaluate_responder(&encoded, Some(&[0x05, 0x00])).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::InvalidAnchorNameConstraints(_)
        ));
    }

    #[test]
    fn malformed_responder_subject_alt_name_is_redacted() {
        let encoded = certificate(&[extension(SUBJECT_ALT_NAME_OID_DER, &[0x05, 0x00])]);
        let constraints = permitted_dns_subtrees(&[b".example.test"]);

        let error = evaluate_responder(&encoded, Some(&constraints)).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::InvalidResponderSubjectAlternativeName(_)
        ));
    }

    #[test]
    fn responder_dns_constraint_failure_is_redacted() {
        let encoded = certificate(&[subject_alt_name(&[dns_name("ocsp.other.test")])]);
        let constraints = permitted_dns_subtrees(&[b".example.test"]);

        let error = evaluate_responder(&encoded, Some(&constraints)).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::ResponderDnsNameConstraints(_)
        ));
    }

    #[test]
    fn absent_intermediate_constraints_do_not_require_a_responder_san() {
        let responder = certificate(&[]);
        let intermediate = certificate(&[]);

        assert_eq!(
            evaluate_responder_intermediate(&responder, &intermediate).unwrap(),
            None
        );
    }

    #[test]
    fn critical_intermediate_constraints_apply_to_every_responder_dns_name() {
        let responder = certificate(&[subject_alt_name(&[
            dns_name("ocsp.example.test"),
            dns_name("backup.example.test"),
        ])]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], true)]);

        let constrained = evaluate_responder_intermediate(&responder, &intermediate)
            .unwrap()
            .unwrap();

        assert_eq!(constrained.count(), 2);
    }

    #[test]
    fn critical_intermediate_excluded_dns_constraints_are_enforced() {
        let responder = certificate(&[subject_alt_name(&[dns_name("ocsp.private.example.test")])]);
        let intermediate = certificate(&[name_constraints(
            &[b"example.test"],
            &[b"private.example.test"],
            true,
        )]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::ResponderIntermediateDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameExcluded,
            )
        );
    }

    #[test]
    fn noncritical_intermediate_constraints_fail_closed() {
        let responder = certificate(&[]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], false)]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::NonCriticalIntermediateNameConstraints
        );
        assert_eq!(
            error.to_string(),
            "X.509 OCSP reviewed-responder intermediate DNS policy error NonCriticalIntermediateNameConstraints"
        );
    }

    #[test]
    fn malformed_intermediate_constraints_are_redacted() {
        let responder = certificate(&[]);
        let intermediate =
            certificate(&[critical_extension(NAME_CONSTRAINTS_OID_DER, &[0x05, 0x00])]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::InvalidIntermediateNameConstraints(_)
        ));
    }

    #[test]
    fn constrained_intermediate_requires_responder_subject_alt_name() {
        let responder = certificate(&[]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], true)]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::MissingResponderSubjectAlternativeName
        );
    }

    #[test]
    fn malformed_responder_san_under_intermediate_policy_is_redacted() {
        let responder = certificate(&[extension(SUBJECT_ALT_NAME_OID_DER, &[0x05, 0x00])]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], true)]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::InvalidResponderSubjectAlternativeName(_)
        ));
    }

    #[test]
    fn responder_intermediate_dns_constraint_failure_is_redacted() {
        let responder = certificate(&[subject_alt_name(&[dns_name("ocsp.other.test")])]);
        let intermediate = certificate(&[name_constraints(&[b".example.test"], &[], true)]);

        let error = evaluate_responder_intermediate(&responder, &intermediate).unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderIntermediateDnsPolicyErrorKind::ResponderIntermediateDnsNameConstraints(_)
        ));
        assert!(!format!("{error:?} {error}").contains("other.test"));
    }

    #[test]
    fn exact_noncritical_no_check_null_succeeds() {
        let responder = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);

        evaluate_responder_no_check(&responder).unwrap();
    }

    #[test]
    fn exact_critical_no_check_null_is_semantically_valid() {
        let responder = certificate(&[critical_extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);

        evaluate_responder_no_check(&responder).unwrap();
    }

    #[test]
    fn missing_no_check_extension_fails_closed() {
        let responder = certificate(&[]);

        let error = evaluate_responder_no_check(&responder).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension
        );
        assert_eq!(
            error.to_string(),
            "X.509 OCSP reviewed-responder no-check policy error MissingNoCheckExtension"
        );
    }

    #[test]
    fn malformed_no_check_extension_is_rejected_without_payload() {
        let responder = certificate(&[extension(OCSP_NO_CHECK_OID_DER, &[0x05, 0x01, 0x00])]);

        let error = evaluate_responder_no_check(&responder).unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::InvalidNoCheckExtensionValue
        );
        assert_eq!(
            format!("{error:?} {error}"),
            "X509OcspReviewedResponderNoCheckPolicyError { kind: InvalidNoCheckExtensionValue } X.509 OCSP reviewed-responder no-check policy error InvalidNoCheckExtensionValue"
        );
    }

    #[test]
    fn intermediate_status_no_check_constructor_selects_the_added_responder() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let anchor_dns_policy = intermediate_status_anchor_dns_policy(&original, &added);

        let no_check_policy =
            validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy(
                anchor_dns_policy,
            )
            .unwrap();

        let retained = no_check_policy
            .anchor_dns_policy()
            .responder_path()
            .no_check_intermediate_revocation()
            .intermediate_revocation();
        assert!(std::ptr::eq(retained.original_responder, &original));
        assert!(std::ptr::eq(retained.added_responder, &added));
        assert_eq!(
            format!("{no_check_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy"
        );
    }

    #[test]
    fn intermediate_status_no_check_constructor_rejects_only_original_no_check() {
        let original_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let added_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let anchor_dns_policy = intermediate_status_anchor_dns_policy(&original, &added);

        let error =
            validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy(
                anchor_dns_policy,
            )
            .expect_err("the original responder's no-check must not satisfy the added responder");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension
        );
        assert!(!format!("{error:?} {error}").contains("responder.example.test"));
    }

    #[test]
    fn intermediate_status_no_check_constructor_redacts_malformed_added_extension() {
        let original_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let added_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, &[0x05, 0x01, 0x00])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let anchor_dns_policy = intermediate_status_anchor_dns_policy(&original, &added);

        let error =
            validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy(
                anchor_dns_policy,
            )
            .expect_err("malformed no-check on the added responder must fail closed");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::InvalidNoCheckExtensionValue
        );
        assert!(!format!("{error:?} {error}").contains("responder.example.test"));
    }

    #[test]
    fn intermediate_status_responder_revocation_path_retains_added_evidence() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let anchor_dns_policy = intermediate_status_revocation_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            observation_time,
        );

        let bound = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            TestTargetRevocationBindingEvidence {
                target: &added,
                issuer: &issuer,
                observation_time,
            },
        )
        .unwrap();

        let responder_path = reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path(bound);

        let retained = responder_path.responder_revocation();
        let intermediate_status = retained
            .anchor_dns_policy()
            .responder_path()
            .no_check_intermediate_revocation()
            .intermediate_revocation();
        assert!(std::ptr::eq(
            intermediate_status.original_responder,
            &original
        ));
        assert!(std::ptr::eq(intermediate_status.added_responder, &added));
        assert!(std::ptr::eq(intermediate_status.issuer, &issuer));
        assert!(std::ptr::eq(retained.responder_revocation().target, &added));
        assert_eq!(
            format!("{responder_path:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderPath"
        );
    }

    #[test]
    fn intermediate_status_responder_revocation_responder_anchor_dns_policy_applies_constraints() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[subject_alt_name(&[
            dns_name("ocsp.example.test"),
            dns_name("backup.example.test"),
        ])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_path = intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
            Some(name_constraints),
        );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names().unwrap().count(), 2);
        let retained = anchor_policy.responder_path().responder_revocation();
        assert!(std::ptr::eq(
            retained.responder_revocation().responder,
            &responder,
        ));
        assert!(std::ptr::eq(retained.responder_revocation().target, &added,));
        assert_eq!(
            format!("{anchor_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderAnchorDnsPolicy"
        );
    }

    #[test]
    fn unconstrained_intermediate_status_responder_revocation_root_skips_responder_san() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
            None,
        );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names(), None);
        assert!(std::ptr::eq(
            anchor_policy
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &responder,
        ));
    }

    #[test]
    fn intermediate_status_responder_revocation_responder_anchor_failure_is_redacted() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[subject_alt_name(&[dns_name("ocsp.other.test")])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_path = intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
            Some(name_constraints),
        );

        let error = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::ResponderDnsNameConstraints(_)
        ));
        assert!(!format!("{error:?} {error}").contains("other.test"));
    }

    #[test]
    fn recursive_added_responder_revocation_path_retains_generic_source_and_exact_proof() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let source = TestGenericIntermediateStatusSource {
            intermediate_revocation: TestIntermediateStatusResponderRevocationEvidence {
                original_responder: &original,
                added_responder: &added,
                issuer: &issuer,
                observation_time,
            },
            name_constraints: None,
        };
        let added_responder_dns: ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy<
            TestResponderRevocationResponderAnchorOriginalEvidence,
            _,
            _,
        > = ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedIntermediateStatusResponderPath {
                    source,
                    evidence_types: PhantomData,
                },
                constrained_dns_names: None,
            };
        let added_responder_revocation =
            bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
                added_responder_dns,
                TestResponderTargetRevocationBindingEvidence {
                    responder: &responder,
                    target: &added,
                    issuer: &issuer,
                    observation_time,
                },
            )
            .unwrap();
        let responder_path = reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path(
            added_responder_revocation,
        );
        let responder_dns = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let recursive = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation(
            responder_dns,
            TestTargetRevocationBindingEvidence {
                target: &responder,
                issuer: &issuer,
                observation_time,
            },
        )
        .unwrap();

        assert!(std::ptr::eq(
            recursive.recursive_responder_revocation().target,
            &responder,
        ));
        let retained_source = &recursive
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .anchor_dns_policy()
            .responder_path()
            .source;
        assert!(std::ptr::eq(
            retained_source.intermediate_revocation().original_responder,
            &original,
        ));
        assert_eq!(
            format!("{recursive:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocation"
        );

        let recursive_responder_path = reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_path(
            recursive,
        );
        assert!(std::ptr::eq(
            recursive_responder_path
                .recursive_responder_revocation()
                .recursive_responder_revocation()
                .target,
            &responder,
        ));
        let retained_source = &recursive_responder_path
            .recursive_responder_revocation()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .anchor_dns_policy()
            .responder_path()
            .source;
        assert!(std::ptr::eq(
            retained_source.intermediate_revocation().original_responder,
            &original,
        ));
        assert_eq!(
            format!("{recursive_responder_path:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderPath"
        );
    }

    #[test]
    fn recursive_responder_anchor_dns_policy_applies_selected_root_constraints() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded =
            certificate(&[subject_alt_name(&[dns_name("added-ocsp.example.test")])]);
        let recursive_responder_encoded = certificate(&[subject_alt_name(&[
            dns_name("recursive-ocsp.example.test"),
            dns_name("recursive-backup.example.test"),
        ])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            Some(name_constraints),
        );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names().unwrap().count(), 2);
        assert!(std::ptr::eq(
            anchor_policy
                .responder_path()
                .recursive_responder_revocation()
                .recursive_responder_revocation()
                .responder,
            &recursive_responder,
        ));
        assert_eq!(
            format!("{anchor_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderAnchorDnsPolicy"
        );
    }

    #[test]
    fn unconstrained_recursive_responder_root_skips_responder_san() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let recursive_responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            None,
        );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names(), None);
        assert!(std::ptr::eq(
            anchor_policy
                .responder_path()
                .recursive_responder_revocation()
                .recursive_responder_revocation()
                .responder,
            &recursive_responder,
        ));
    }

    #[test]
    fn recursive_responder_anchor_dns_failure_is_redacted() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded =
            certificate(&[subject_alt_name(&[dns_name("added-ocsp.example.test")])]);
        let recursive_responder_encoded =
            certificate(&[subject_alt_name(&[dns_name("recursive-ocsp.other.test")])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            Some(name_constraints),
        );

        let error = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::ResponderDnsNameConstraints(_)
        ));
        assert!(!format!("{error:?} {error}").contains("other.test"));
    }

    #[test]
    fn recursive_responder_no_check_policy_selects_exact_responder() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let recursive_responder_encoded =
            certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let no_check_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .unwrap();

        assert!(std::ptr::eq(
            no_check_policy
                .anchor_dns_policy()
                .responder_path()
                .recursive_responder_revocation()
                .recursive_responder_revocation()
                .responder,
            &recursive_responder,
        ));
        assert_eq!(
            format!("{no_check_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderRevocationResponderNoCheckPolicy"
        );
    }

    #[test]
    fn recursive_responder_no_check_completes_exact_fixed_path() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let recursive_responder_encoded =
            certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let no_check = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .unwrap();

        let complete = complete_x509_ocsp_routed_issuer_reviewed_recursive_responder_no_check_fixed_path_revocation_policy(
            no_check,
        );

        assert!(std::ptr::eq(
            complete
                .recursive_responder_no_check_policy()
                .anchor_dns_policy()
                .responder_path()
                .recursive_responder_revocation()
                .recursive_responder_revocation()
                .responder,
            &recursive_responder,
        ));
        assert_eq!(
            format!("{complete:?}"),
            "ValidatedX509OcspReviewedRecursiveResponderNoCheckCompleteFixedPathRevocationPolicy"
        );
    }

    #[test]
    fn recursive_responder_no_check_policy_rejects_missing_exact_extension() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let recursive_responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let error = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .expect_err("the earlier responder's no-check must not cover the recursive responder");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension
        );
    }

    #[test]
    fn recursive_responder_no_check_policy_redacts_malformed_value() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let recursive_responder_encoded =
            certificate(&[extension(OCSP_NO_CHECK_OID_DER, &[0x05, 0x01, 0x00])]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let recursive_responder =
            decode_x509_certificate(&recursive_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = recursive_intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            &recursive_responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let error = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .expect_err("a malformed no-check value must fail closed");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::InvalidNoCheckExtensionValue
        );
        assert!(!format!("{error:?} {error}").contains("05 01 00"));
    }

    #[test]
    fn recursive_added_responder_revocation_rejects_wrong_target() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_dns = intermediate_status_responder_revocation_responder_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation(
            responder_dns,
            TestTargetRevocationBindingEvidence {
                target: &added,
                issuer: &issuer,
                observation_time,
            },
        )
        .expect_err("the added responder target must not satisfy its proof responder");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::TargetMismatch
        );
        assert!(!format!("{error:?} {error}").contains("responder.example.test"));
    }

    #[test]
    fn recursive_added_responder_revocation_rejects_wrong_issuer_before_time() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let other_issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let other_issuer =
            decode_x509_certificate(&other_issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_dns = intermediate_status_responder_revocation_responder_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation(
            responder_dns,
            TestTargetRevocationBindingEvidence {
                target: &responder,
                issuer: &other_issuer,
                observation_time: time(b"\x17\x0d271025120000Z"),
            },
        )
        .expect_err("issuer identity must fail before observation time");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::IssuerMismatch
        );
    }

    #[test]
    fn recursive_added_responder_revocation_rejects_wrong_observation_time() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_dns = intermediate_status_responder_revocation_responder_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation(
            responder_dns,
            TestTargetRevocationBindingEvidence {
                target: &responder,
                issuer: &issuer,
                observation_time: time(b"\x17\x0d271025120000Z"),
            },
        )
        .expect_err("cross-time recursive responder status must fail closed");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::ObservationTimeMismatch
        );
    }

    #[test]
    fn intermediate_status_responder_revocation_no_check_completes_exact_fixed_path() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let no_check = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .unwrap();

        assert!(std::ptr::eq(
            no_check
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &responder,
        ));
        assert_eq!(
            format!("{no_check:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckPolicy"
        );

        let complete = complete_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_fixed_path_revocation_policy(no_check);

        assert!(std::ptr::eq(
            complete
                .responder_no_check_policy()
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &responder,
        ));
        assert_eq!(
            format!("{complete:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderRevocationResponderNoCheckCompleteFixedPathRevocationPolicy"
        );
    }

    #[test]
    fn double_nested_no_check_completes_exact_alternative_fixed_path() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let added_responder_encoded = certificate(&[]);
        let final_responder_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let added_responder =
            decode_x509_certificate(&added_responder_encoded, Asn1Limits::default()).unwrap();
        let final_responder =
            decode_x509_certificate(&final_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");

        let original_responder_no_check =
            responder_revocation_responder_no_check_intermediate_binding_evidence(
                &intermediate,
                &root,
                observation_time,
            );
        let intermediate_status = TestIntermediateStatusResponderRevocationEvidence {
            original_responder: &intermediate,
            added_responder: &added_responder,
            issuer: &root,
            observation_time,
        };
        let source = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            original_responder_no_check,
            intermediate_status,
        )
        .unwrap();
        let added_responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path(
            source,
        );
        let added_responder_dns = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_anchor_dns_policy(
            added_responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let added_responder_revocation =
            bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
                added_responder_dns,
                TestResponderTargetRevocationBindingEvidence {
                    responder: &final_responder,
                    target: &added_responder,
                    issuer: &root,
                    observation_time,
                },
            )
            .unwrap();
        let final_responder_path = reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path(
            added_responder_revocation,
        );
        let final_responder_dns = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            final_responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let final_responder_no_check = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_policy(
            final_responder_dns,
        )
        .unwrap();

        let complete = complete_x509_ocsp_routed_issuer_reviewed_double_nested_no_check_fixed_path_revocation_policy(
            final_responder_no_check,
        );

        assert!(std::ptr::eq(
            complete
                .responder_no_check_policy()
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &final_responder,
        ));
        let retained_source = complete
            .responder_no_check_policy()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation_responder_no_check_intermediate_revocation();
        assert!(std::ptr::eq(
            retained_source.intermediate_revocation().added_responder,
            &added_responder,
        ));
        assert_eq!(
            *retained_source
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation(),
            7,
        );
        assert_eq!(
            format!("{complete:?}"),
            "ValidatedX509OcspReviewedDoubleNestedNoCheckCompleteFixedPathRevocationPolicy"
        );
    }

    #[test]
    fn intermediate_status_responder_revocation_no_check_fails_closed_when_missing() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_path = intermediate_status_responder_revocation_responder_path(
            &original,
            &added,
            &issuer,
            &responder,
            observation_time,
            None,
        );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let error = validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_policy(
            anchor_dns_policy,
        )
        .expect_err("a responder without no-check must fail closed");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension
        );
        assert!(!format!("{error:?} {error}").contains("responder.example.test"));
    }

    #[test]
    fn intermediate_status_responder_revocation_rejects_original_responder_target() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let anchor_dns_policy = intermediate_status_revocation_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            TestTargetRevocationBindingEvidence {
                target: &original,
                issuer: &issuer,
                observation_time,
            },
        )
        .expect_err("the original responder must not satisfy the added responder branch");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::TargetMismatch
        );
        assert!(!format!("{error:?} {error}").contains("responder.example.test"));
    }

    #[test]
    fn intermediate_status_responder_revocation_rejects_wrong_issuer_before_time() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let other_issuer_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let other_issuer =
            decode_x509_certificate(&other_issuer_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let anchor_dns_policy = intermediate_status_revocation_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            TestTargetRevocationBindingEvidence {
                target: &added,
                issuer: &other_issuer,
                observation_time: time(b"\x17\x0d271025120000Z"),
            },
        )
        .expect_err("issuer identity must fail before observation time");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::IssuerMismatch
        );
    }

    #[test]
    fn intermediate_status_responder_revocation_rejects_wrong_observation_time() {
        let original_encoded = certificate(&[]);
        let added_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let original = decode_x509_certificate(&original_encoded, Asn1Limits::default()).unwrap();
        let added = decode_x509_certificate(&added_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let anchor_dns_policy = intermediate_status_revocation_anchor_dns_policy(
            &original,
            &added,
            &issuer,
            observation_time,
        );

        let error = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            TestTargetRevocationBindingEvidence {
                target: &added,
                issuer: &issuer,
                observation_time: time(b"\x17\x0d271025120000Z"),
            },
        )
        .expect_err("cross-time responder status must fail closed");

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::ObservationTimeMismatch
        );
    }

    #[test]
    fn exact_responder_revocation_binding_inputs_succeed() {
        let responder_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");

        validate_responder_revocation_binding_inputs(
            &responder,
            &issuer,
            observation_time,
            &responder,
            &issuer,
            observation_time,
        )
        .unwrap();
    }

    #[test]
    fn responder_revocation_binding_rejects_target_before_issuer_and_time() {
        let responder_encoded = certificate(&[]);
        let other_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let other = decode_x509_certificate(&other_encoded, Asn1Limits::default()).unwrap();

        let error = validate_responder_revocation_binding_inputs(
            &responder,
            &responder,
            time(b"\x17\x0d261025120000Z"),
            &other,
            &other,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::TargetMismatch
        );
    }

    #[test]
    fn responder_revocation_binding_rejects_issuer_before_time() {
        let responder_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let other_issuer_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let other_issuer =
            decode_x509_certificate(&other_issuer_encoded, Asn1Limits::default()).unwrap();

        let error = validate_responder_revocation_binding_inputs(
            &responder,
            &issuer,
            time(b"\x17\x0d261025120000Z"),
            &responder,
            &other_issuer,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::IssuerMismatch
        );
    }

    #[test]
    fn responder_revocation_binding_rejects_a_different_observation_time() {
        let responder_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();

        let error = validate_responder_revocation_binding_inputs(
            &responder,
            &issuer,
            time(b"\x17\x0d261025120000Z"),
            &responder,
            &issuer,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderRevocationBindingErrorKind::ObservationTimeMismatch
        );
        assert_eq!(
            format!("{error:?} {error}"),
            "X509OcspReviewedResponderRevocationBindingError { kind: ObservationTimeMismatch } X.509 OCSP reviewed-responder revocation binding error ObservationTimeMismatch"
        );
    }

    #[test]
    fn responder_revocation_wrapper_retains_both_noncopyable_inputs() {
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation: (),
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let bound = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: 7_u8,
        };

        assert_eq!(*bound.responder_revocation(), 7);
        assert_eq!(
            bound
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509OcspReviewedResponderRevocation"
        );
    }

    #[test]
    fn responder_revocation_responder_path_retains_the_complete_branch() {
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation: (),
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let bound = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: 7_u8,
        };

        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(bound);

        assert_eq!(
            *responder_path.responder_revocation().responder_revocation(),
            7
        );
        assert_eq!(
            responder_path
                .responder_revocation()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{responder_path:?}"),
            "BoundX509OcspReviewedResponderRevocationResponderPath"
        );
    }

    #[test]
    fn responder_revocation_responder_anchor_dns_policy_applies_selected_root_constraints() {
        let responder_encoded = certificate(&[subject_alt_name(&[
            dns_name("ocsp.example.test"),
            dns_name("backup.example.test"),
        ])]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation:
                        TestResponderRevocationResponderAnchorOriginalEvidence {
                            name_constraints: Some(name_constraints),
                        },
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let responder_revocation = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: TestResponderRevocationResponderAnchorEvidence {
                responder: &responder,
            },
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(
                responder_revocation,
            );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names().unwrap().count(), 2);
        assert!(std::ptr::eq(
            anchor_policy
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &responder,
        ));
        assert_eq!(
            format!("{anchor_policy:?}"),
            "ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy"
        );
    }

    #[test]
    fn unconstrained_responder_revocation_root_does_not_require_responder_san() {
        let responder_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation:
                        TestResponderRevocationResponderAnchorOriginalEvidence {
                            name_constraints: None,
                        },
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let responder_revocation = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: TestResponderRevocationResponderAnchorEvidence {
                responder: &responder,
            },
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(
                responder_revocation,
            );

        let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(anchor_policy.constrained_dns_names(), None);
        assert!(std::ptr::eq(
            anchor_policy
                .responder_path()
                .responder_revocation()
                .responder_revocation()
                .responder,
            &responder,
        ));
    }

    #[test]
    fn responder_revocation_responder_intermediate_dns_policy_applies_exact_constraints() {
        let responder_encoded = certificate(&[subject_alt_name(&[
            dns_name("ocsp.example.test"),
            dns_name("backup.example.test"),
        ])]);
        let intermediate_encoded = certificate(&[name_constraints(&[b".example.test"], &[], true)]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation:
                        TestResponderRevocationResponderAnchorOriginalEvidence {
                            name_constraints: None,
                        },
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let responder_revocation = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: TestResponderRevocationResponderIntermediateEvidence {
                responder: &responder,
                issuer: &intermediate,
                observation_time,
            },
        };
        let anchor_dns_policy =
            ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy {
                responder_path:
                    reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(
                        responder_revocation,
                    ),
                constrained_dns_names: None,
            };

        let intermediate_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_intermediate_dns_policy(
            anchor_dns_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(
            intermediate_policy.constrained_dns_names().unwrap().count(),
            2
        );
        let retained = intermediate_policy
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .responder_revocation();
        assert!(std::ptr::eq(retained.responder, &responder));
        assert!(std::ptr::eq(retained.issuer, &intermediate));
        assert_eq!(retained.observation_time, observation_time);
        assert_eq!(
            format!("{intermediate_policy:?}"),
            "ValidatedX509OcspReviewedResponderRevocationResponderIntermediateDnsPolicy"
        );
    }

    #[test]
    fn unconstrained_responder_revocation_intermediate_does_not_require_responder_san() {
        let responder_encoded = certificate(&[]);
        let intermediate_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let intermediate_dns_policy = ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
            anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                responder_path: BoundX509OcspReviewedResponderPath {
                    reviewed_server_revocation:
                        TestResponderRevocationResponderAnchorOriginalEvidence {
                            name_constraints: None,
                        },
                },
                constrained_dns_names: None,
            },
            constrained_dns_names: None,
        };
        let responder_revocation = BoundX509OcspReviewedResponderRevocation {
            intermediate_dns_policy,
            responder_revocation: TestResponderRevocationResponderIntermediateEvidence {
                responder: &responder,
                issuer: &intermediate,
                observation_time: time(b"\x17\x0d261025120000Z"),
            },
        };
        let anchor_dns_policy =
            ValidatedX509OcspReviewedResponderRevocationResponderAnchorDnsPolicy {
                responder_path:
                    reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path(
                        responder_revocation,
                    ),
                constrained_dns_names: None,
            };

        let intermediate_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_intermediate_dns_policy(
            anchor_dns_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        assert_eq!(intermediate_policy.constrained_dns_names(), None);
        let retained = intermediate_policy
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .responder_revocation();
        assert!(std::ptr::eq(retained.responder, &responder));
        assert!(std::ptr::eq(retained.issuer, &intermediate));
    }

    #[test]
    fn responder_revocation_responder_no_check_policy_selects_exact_responder() {
        let responder_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let issuer_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let intermediate_dns_policy =
            responder_revocation_responder_intermediate_dns_policy(&responder, &issuer);

        let no_check_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_policy(
            intermediate_dns_policy,
        )
        .unwrap();

        let retained = no_check_policy
            .intermediate_dns_policy()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation()
            .responder_revocation();
        assert!(std::ptr::eq(retained.responder, &responder));
        assert!(std::ptr::eq(retained.issuer, &issuer));
        assert_eq!(
            format!("{no_check_policy:?}"),
            "ValidatedX509OcspReviewedResponderRevocationResponderNoCheckPolicy"
        );
    }

    #[test]
    fn responder_revocation_responder_no_check_policy_rejects_missing_extension() {
        let responder_encoded = certificate(&[]);
        let issuer_encoded = certificate(&[]);
        let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
        let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
        let intermediate_dns_policy =
            responder_revocation_responder_intermediate_dns_policy(&responder, &issuer);

        let error = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_policy(
            intermediate_dns_policy,
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedResponderNoCheckPolicyErrorKind::MissingNoCheckExtension
        );
    }

    #[test]
    fn exact_intermediate_revocation_binding_inputs_succeed() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");

        validate_intermediate_revocation_binding_inputs(
            &intermediate,
            &root,
            observation_time,
            &intermediate,
            &root,
            observation_time,
        )
        .unwrap();
    }

    #[test]
    fn intermediate_revocation_binding_rejects_target_before_issuer_and_time() {
        let intermediate_encoded = certificate(&[]);
        let other_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let other = decode_x509_certificate(&other_encoded, Asn1Limits::default()).unwrap();

        let error = validate_intermediate_revocation_binding_inputs(
            &intermediate,
            &intermediate,
            time(b"\x17\x0d261025120000Z"),
            &other,
            &other,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch
        );
    }

    #[test]
    fn intermediate_revocation_binding_rejects_issuer_before_time() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let other_root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let other_root =
            decode_x509_certificate(&other_root_encoded, Asn1Limits::default()).unwrap();

        let error = validate_intermediate_revocation_binding_inputs(
            &intermediate,
            &root,
            time(b"\x17\x0d261025120000Z"),
            &intermediate,
            &other_root,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::IssuerMismatch
        );
    }

    #[test]
    fn intermediate_revocation_binding_rejects_a_different_observation_time() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();

        let error = validate_intermediate_revocation_binding_inputs(
            &intermediate,
            &root,
            time(b"\x17\x0d261025120000Z"),
            &intermediate,
            &root,
            time(b"\x17\x0d271025120000Z"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::ObservationTimeMismatch
        );
        assert_eq!(
            format!("{error:?} {error}"),
            "X509OcspReviewedIntermediateRevocationBindingError { kind: ObservationTimeMismatch } X.509 OCSP reviewed-intermediate revocation binding error ObservationTimeMismatch"
        );
    }

    #[test]
    fn intermediate_revocation_wrapper_retains_both_noncopyable_inputs() {
        let bound = BoundX509OcspReviewedIntermediateRevocation {
            reviewed_server_revocation: (),
            intermediate_revocation: 7_u8,
        };

        assert_eq!(bound.reviewed_server_revocation(), &());
        assert_eq!(*bound.intermediate_revocation(), 7);
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509OcspReviewedIntermediateRevocation"
        );
    }

    #[test]
    fn authenticated_responder_and_intermediate_revocation_composition_succeeds() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_revocation =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &intermediate,
            issuer: &root,
            observation_time,
        };

        let bound =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                responder_revocation,
                intermediate_revocation,
            )
            .unwrap();

        assert_eq!(*bound.responder_revocation().responder_revocation(), 7);
        assert!(std::ptr::eq(
            bound.intermediate_revocation().target,
            &intermediate
        ));
        assert!(std::ptr::eq(bound.intermediate_revocation().issuer, &root));
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509OcspReviewedResponderRevocationIntermediateRevocation"
        );
    }

    #[test]
    fn authenticated_responder_intermediate_composition_rejects_target_first() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let other_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let other = decode_x509_certificate(&other_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_revocation =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &other,
            issuer: &other,
            observation_time: time(b"\x17\x0d271025120000Z"),
        };

        let error =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                responder_revocation,
                intermediate_revocation,
            )
            .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch
        );
    }

    #[test]
    fn authenticated_responder_intermediate_composition_rejects_issuer_before_time() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let other_root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let other_root =
            decode_x509_certificate(&other_root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_revocation =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &intermediate,
            issuer: &other_root,
            observation_time: time(b"\x17\x0d271025120000Z"),
        };

        let error =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                responder_revocation,
                intermediate_revocation,
            )
            .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::IssuerMismatch
        );
    }

    #[test]
    fn authenticated_responder_intermediate_composition_rejects_time_last() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_revocation =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &intermediate,
            issuer: &root,
            observation_time: time(b"\x17\x0d271025120000Z"),
        };

        let error =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                responder_revocation,
                intermediate_revocation,
            )
            .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::ObservationTimeMismatch
        );
        assert_eq!(
            format!("{error:?} {error}"),
            "X509OcspReviewedIntermediateRevocationBindingError { kind: ObservationTimeMismatch } X.509 OCSP reviewed-intermediate revocation binding error ObservationTimeMismatch"
        );
    }

    #[test]
    fn responder_of_responder_no_check_intermediate_composition_succeeds() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_no_check_policy =
            responder_revocation_responder_no_check_intermediate_binding_evidence(
                &intermediate,
                &root,
                observation_time,
            );
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &intermediate,
            issuer: &root,
            observation_time,
        };

        let bound = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            responder_no_check_policy,
            intermediate_revocation,
        )
        .unwrap();

        let retained_responder_revocation = bound
            .responder_no_check_policy()
            .intermediate_dns_policy()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation();
        assert_eq!(*retained_responder_revocation.responder_revocation(), 7);
        assert!(std::ptr::eq(
            bound.intermediate_revocation().target,
            &intermediate
        ));
        assert!(std::ptr::eq(bound.intermediate_revocation().issuer, &root));
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509OcspReviewedResponderRevocationResponderNoCheckIntermediateRevocation"
        );
    }

    #[test]
    fn responder_of_responder_no_check_intermediate_status_path_retains_complete_composition() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_no_check_policy =
            responder_revocation_responder_no_check_intermediate_binding_evidence(
                &intermediate,
                &root,
                observation_time,
            );
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &intermediate,
            issuer: &root,
            observation_time,
        };
        let composition = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            responder_no_check_policy,
            intermediate_revocation,
        )
        .unwrap();

        let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path(
            composition,
        );

        let retained =
            responder_path.responder_revocation_responder_no_check_intermediate_revocation();
        assert_eq!(
            *retained
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation(),
            7
        );
        assert!(std::ptr::eq(
            retained.intermediate_revocation().target,
            &intermediate
        ));
        assert_eq!(
            format!("{responder_path:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderPath"
        );
    }

    #[test]
    fn nested_no_check_complete_fixed_path_revocation_policy_retains_complete_composition() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let added_responder_encoded = certificate(&[
            subject_alt_name(&[
                dns_name("ocsp.example.test"),
                dns_name("backup.example.test"),
            ]),
            extension(OCSP_NO_CHECK_OID_DER, NULL_DER),
        ]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let added_responder =
            decode_x509_certificate(&added_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_no_check_policy =
            responder_revocation_responder_no_check_intermediate_binding_evidence_with_constraints(
                &intermediate,
                &root,
                observation_time,
                Some(name_constraints),
            );
        let intermediate_status = TestIntermediateStatusResponderRevocationEvidence {
            original_responder: &intermediate,
            added_responder: &added_responder,
            issuer: &root,
            observation_time,
        };
        let composition = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            responder_no_check_policy,
            intermediate_status,
        )
        .unwrap();
        let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path(
            composition,
        );

        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let no_check =
            validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy(
                anchor_dns_policy,
            )
            .unwrap();

        let complete =
            complete_x509_ocsp_routed_issuer_reviewed_nested_no_check_fixed_path_revocation_policy(
                no_check,
            );

        assert_eq!(
            complete
                .intermediate_status_responder_no_check_policy()
                .anchor_dns_policy()
                .constrained_dns_names()
                .unwrap()
                .count(),
            2
        );
        let retained = complete
            .intermediate_status_responder_no_check_policy()
            .anchor_dns_policy()
            .responder_path()
            .responder_revocation_responder_no_check_intermediate_revocation();
        assert!(std::ptr::eq(
            retained.intermediate_revocation().added_responder,
            &added_responder,
        ));
        assert_eq!(
            *retained
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation()
                .responder_revocation(),
            7
        );
        assert_eq!(
            format!("{complete:?}"),
            "ValidatedX509OcspReviewedNestedNoCheckCompleteFixedPathRevocationPolicy"
        );
    }

    #[test]
    fn nested_no_check_intermediate_status_anchor_dns_policy_redacts_name_failure() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let added_responder_encoded =
            certificate(&[subject_alt_name(&[dns_name("ocsp.other.test")])]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let added_responder =
            decode_x509_certificate(&added_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let name_constraints: &'static [u8] =
            Box::leak(permitted_dns_subtrees(&[b".example.test"]).into_boxed_slice());
        let responder_no_check_policy =
            responder_revocation_responder_no_check_intermediate_binding_evidence_with_constraints(
                &intermediate,
                &root,
                observation_time,
                Some(name_constraints),
            );
        let intermediate_status = TestIntermediateStatusResponderRevocationEvidence {
            original_responder: &intermediate,
            added_responder: &added_responder,
            issuer: &root,
            observation_time,
        };
        let composition = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            responder_no_check_policy,
            intermediate_status,
        )
        .unwrap();
        let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path(
            composition,
        );

        let error = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error.kind(),
            X509OcspReviewedResponderAnchorDnsPolicyErrorKind::ResponderDnsNameConstraints(_)
        ));
        assert!(!format!("{error:?} {error}").contains("ocsp.other.test"));
    }

    #[test]
    fn responder_of_responder_no_check_intermediate_composition_rejects_target_first() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let other_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let other = decode_x509_certificate(&other_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_no_check_policy =
            responder_revocation_responder_no_check_intermediate_binding_evidence(
                &intermediate,
                &root,
                observation_time,
            );
        let intermediate_revocation = TestTargetRevocationBindingEvidence {
            target: &other,
            issuer: &other,
            observation_time: time(b"\x17\x0d271025120000Z"),
        };

        let error = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation(
            responder_no_check_policy,
            intermediate_revocation,
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch
        );
    }

    #[test]
    fn no_check_intermediate_revocation_wrapper_retains_both_noncopyable_inputs() {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: (),
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let bound = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: 7_u8,
        };

        assert_eq!(*bound.intermediate_revocation(), 7);
        assert_eq!(
            bound
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509OcspReviewedNoCheckIntermediateRevocation"
        );
    }

    #[test]
    fn intermediate_status_responder_path_retains_the_complete_composition() {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: (),
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let bound = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: 7_u8,
        };

        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(bound);

        assert_eq!(
            *responder_path
                .no_check_intermediate_revocation()
                .intermediate_revocation(),
            7
        );
        assert_eq!(
            responder_path
                .no_check_intermediate_revocation()
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{responder_path:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderPath"
        );
    }

    #[test]
    fn authenticated_responder_intermediate_status_path_retains_the_complete_composition() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let responder_revocation =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let composition = BoundX509OcspReviewedResponderRevocationIntermediateRevocation {
            responder_revocation,
            intermediate_revocation: 9_u8,
        };

        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_path(
                composition,
            );

        assert_eq!(
            *responder_path
                .responder_revocation_intermediate_revocation()
                .intermediate_revocation(),
            9
        );
        assert_eq!(
            *responder_path
                .responder_revocation_intermediate_revocation()
                .responder_revocation()
                .responder_revocation(),
            7
        );
        assert_eq!(
            format!("{responder_path:?}"),
            "BoundX509OcspReviewedIntermediateStatusResponderPath"
        );
    }

    #[test]
    fn authenticated_responder_branch_reaches_added_responder_no_check_policy() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let added_responder_encoded = certificate(&[extension(OCSP_NO_CHECK_OID_DER, NULL_DER)]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let added_responder =
            decode_x509_certificate(&added_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let original_branch =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_status = TestIntermediateStatusResponderRevocationEvidence {
            original_responder: &intermediate,
            added_responder: &added_responder,
            issuer: &root,
            observation_time,
        };
        let composition =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                original_branch,
                intermediate_status,
            )
            .unwrap();
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_path(
                composition,
            );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

        let no_check =
            validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy(
                anchor_dns_policy,
            )
            .unwrap();

        assert_eq!(no_check.anchor_dns_policy().constrained_dns_names(), None);
        assert!(std::ptr::eq(
            no_check
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation_intermediate_revocation()
                .intermediate_revocation()
                .added_responder,
            &added_responder,
        ));
    }

    #[test]
    fn authenticated_responder_branch_reaches_added_responder_revocation_policy() {
        let intermediate_encoded = certificate(&[]);
        let root_encoded = certificate(&[]);
        let added_responder_encoded = certificate(&[]);
        let intermediate =
            decode_x509_certificate(&intermediate_encoded, Asn1Limits::default()).unwrap();
        let root = decode_x509_certificate(&root_encoded, Asn1Limits::default()).unwrap();
        let added_responder =
            decode_x509_certificate(&added_responder_encoded, Asn1Limits::default()).unwrap();
        let observation_time = time(b"\x17\x0d261025120000Z");
        let original_branch =
            responder_revocation_intermediate_evidence(&intermediate, &root, observation_time);
        let intermediate_status = TestIntermediateStatusResponderRevocationEvidence {
            original_responder: &intermediate,
            added_responder: &added_responder,
            issuer: &root,
            observation_time,
        };
        let composition =
            bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation(
                original_branch,
                intermediate_status,
            )
            .unwrap();
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_path(
                composition,
            );
        let anchor_dns_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let responder_revocation = TestTargetRevocationBindingEvidence {
            target: &added_responder,
            issuer: &root,
            observation_time,
        };

        let bound = bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation(
            anchor_dns_policy,
            responder_revocation,
        )
        .unwrap();

        assert!(std::ptr::eq(
            bound.responder_revocation().target,
            &added_responder,
        ));
        assert!(std::ptr::eq(
            bound
                .anchor_dns_policy()
                .responder_path()
                .responder_revocation_intermediate_revocation()
                .intermediate_revocation()
                .added_responder,
            &added_responder,
        ));
    }

    #[test]
    fn intermediate_status_responder_anchor_dns_policy_retains_the_complete_path() {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: (),
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let no_check_intermediate_revocation = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: 7_u8,
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(
                no_check_intermediate_revocation,
            );
        let anchor_policy = ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
            responder_path,
            constrained_dns_names: None,
        };

        assert_eq!(anchor_policy.constrained_dns_names(), None);
        assert_eq!(
            *anchor_policy
                .responder_path()
                .no_check_intermediate_revocation()
                .intermediate_revocation(),
            7
        );
        assert_eq!(
            anchor_policy
                .responder_path()
                .no_check_intermediate_revocation()
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{anchor_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy"
        );
    }

    #[test]
    fn intermediate_status_responder_no_check_policy_retains_the_complete_dns_proof() {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: (),
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let no_check_intermediate_revocation = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: 7_u8,
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(
                no_check_intermediate_revocation,
            );
        let anchor_dns_policy =
            ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
                responder_path,
                constrained_dns_names: None,
            };
        let no_check_policy =
            ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy { anchor_dns_policy };

        assert_eq!(
            no_check_policy.anchor_dns_policy().constrained_dns_names(),
            None
        );
        assert_eq!(
            *no_check_policy
                .anchor_dns_policy()
                .responder_path()
                .no_check_intermediate_revocation()
                .intermediate_revocation(),
            7
        );
        assert_eq!(
            no_check_policy
                .anchor_dns_policy()
                .responder_path()
                .no_check_intermediate_revocation()
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{no_check_policy:?}"),
            "ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy"
        );
    }

    #[test]
    fn direct_no_check_complete_fixed_path_revocation_policy_retains_complete_composition() {
        let responder_no_check_policy = ValidatedX509OcspReviewedResponderNoCheckPolicy {
            intermediate_dns_policy: ValidatedX509OcspReviewedResponderIntermediateDnsPolicy {
                anchor_policy: ValidatedX509OcspReviewedResponderAnchorDnsPolicy {
                    responder_path: BoundX509OcspReviewedResponderPath {
                        reviewed_server_revocation: (),
                    },
                    constrained_dns_names: None,
                },
                constrained_dns_names: None,
            },
        };
        let no_check_intermediate_revocation = BoundX509OcspReviewedNoCheckIntermediateRevocation {
            responder_no_check_policy,
            intermediate_revocation: 7_u8,
        };
        let responder_path =
            reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path(
                no_check_intermediate_revocation,
            );
        let anchor_dns_policy =
            ValidatedX509OcspReviewedIntermediateStatusResponderAnchorDnsPolicy {
                responder_path,
                constrained_dns_names: None,
            };
        let no_check_policy =
            ValidatedX509OcspReviewedIntermediateStatusResponderNoCheckPolicy { anchor_dns_policy };

        let complete =
            complete_x509_ocsp_routed_issuer_reviewed_direct_no_check_fixed_path_revocation_policy(
                no_check_policy,
            );

        assert_eq!(
            *complete
                .intermediate_status_responder_no_check_policy()
                .anchor_dns_policy()
                .responder_path()
                .no_check_intermediate_revocation()
                .intermediate_revocation(),
            7
        );
        assert_eq!(
            complete
                .intermediate_status_responder_no_check_policy()
                .anchor_dns_policy()
                .responder_path()
                .no_check_intermediate_revocation()
                .responder_no_check_policy()
                .intermediate_dns_policy()
                .anchor_policy()
                .responder_path()
                .reviewed_server_revocation(),
            &()
        );
        assert_eq!(
            format!("{complete:?}"),
            "ValidatedX509OcspReviewedDirectNoCheckCompleteFixedPathRevocationPolicy"
        );
    }
}
