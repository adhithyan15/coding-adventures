//! # Reviewed-server OCSP revocation binding
//!
//! This crate composes one complete authenticated OCSP target-revocation proof
//! with the strongest existing constructed reviewed-server path evidence. The
//! target and issuer must be the same exact decoded certificate objects in
//! both inputs. Byte-equal independent decodes are rejected.
//!
//! This is an evidence-composition, exact-time validity, bounded responder
//! DNS-policy, and exact responder no-check boundary. It performs no new
//! cryptography, path search,
//! revocation lookup, clock, TLS, HTTP, OAuth, network, or external-provider
//! operation. A projection retains the complete proof under a distinct type
//! that names the delegated responder's already-proven exact path through the
//! policy-validated intermediate to the reviewed root; subsequent policies
//! derive the responder SAN plus reviewed-anchor and exact-intermediate
//! constraints internally.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate_policy::{
    validate_certificate_validity_at, ValidatedCertificateValidity,
    X509CertificateValidityErrorKind,
};
use x509_name_constraints::{
    decode_name_constraints, decode_name_constraints_contents, evaluate_dns_names,
    ConstrainedDnsNames, DnsConstraintErrorKind, NameConstraintsErrorKind, NAME_CONSTRAINTS_OID,
};
use x509_ocsp_authenticated_target_revocation::AuthenticatedX509OcspRsaSha256TargetRevocation;
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

/// Non-copyable evidence extending one reviewed revocation proof to its responder.
///
/// The consumed input already retains complete responder policy, its exact
/// authenticated responder-to-issuer link, the same issuer's complete
/// intermediate policy and path-length state, its authenticated link to the
/// exact Mozilla-reviewed root, and responder/issuer validity at the OCSP
/// observation time. The consumed type's safe constructors already prove that
/// the responder-policy object is the responder-link child and that the
/// responder-link issuer is the reviewed-path intermediate. This wrapper
/// reifies that transitive invariant for later path-policy composition; it
/// performs no additional validation.
///
/// This fixed RSA SHA-256 responder path does not apply name constraints to
/// the responder certificate, check responder or intermediate revocation,
/// generalize path construction or validation, perform TLS or HTTP, authorize
/// OAuth transport, or grant network authority.
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
}
