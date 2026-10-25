//! # Reviewed-server OCSP revocation binding
//!
//! This crate composes one complete authenticated OCSP target-revocation proof
//! with the strongest existing constructed reviewed-server path evidence. The
//! target and issuer must be the same exact decoded certificate objects in
//! both inputs. Byte-equal independent decodes are rejected.
//!
//! This is an evidence-composition and exact-time validity boundary. It
//! performs no new cryptography, path search, extension processing, revocation
//! lookup, clock, TLS, HTTP, OAuth, network, or external-provider operation.
//! A follow-up projection can retain the resulting proof under a distinct type
//! that names the delegated responder's already-proven exact path through the
//! policy-validated intermediate to the reviewed root.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_policy::{
    validate_certificate_validity_at, ValidatedCertificateValidity,
    X509CertificateValidityErrorKind,
};
use x509_ocsp_authenticated_target_revocation::AuthenticatedX509OcspRsaSha256TargetRevocation;
use x509_rsa_sha256_reviewed_server_link::ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy;

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

#[cfg(test)]
mod tests {
    use super::*;
    use der_asn1::{Asn1Decoder, Asn1Limits};
    use x509_certificate::decode_x509_certificate;
    use x509_time::decode_x509_time;

    const LEAF_HEX: &str = include_str!("../tests/fixtures/apple-www-leaf.der.hex");

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
}
