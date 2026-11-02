//! # Exact RSA SHA-512 reviewed server-link binding
//!
//! This crate closes one evidence-mixing gap between target-server policy and
//! one authenticated RSA SHA-512 issuer link terminating at a reviewed root,
//! or to one authenticated target-to-intermediate-to-reviewed-root path. Its
//! follow-up policy decodes only that selected anchor's optional NameConstraints
//! and the exact direct-link target's Subject Alternative Name under separate
//! caller-owned limits. It performs no new signature work, root lookup, path
//! search, revocation processing, or network effect.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_name_constraints::{
    decode_name_constraints_contents, evaluate_dns_names, ConstrainedDnsNames,
    DnsConstraintErrorKind, NameConstraintsErrorKind,
};
use x509_rsa_sha512_reviewed_anchor_link::{
    BoundX509RsaSha512ReviewedAnchorLink, BoundX509RsaSha512ReviewedIntermediatePath,
};
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;
use x509_subject_alt_name::{
    decode_subject_alt_name, SubjectAltNameErrorKind, SUBJECT_ALT_NAME_OID,
};

/// Stable, payload-free reviewed server-link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512ReviewedServerLinkErrorKind {
    /// The inputs retain different decoded target certificate objects.
    EvidenceMismatch,
    /// The selected reviewed anchor's exact constraint bytes were invalid or
    /// outside the supported bounded subset.
    InvalidAnchorNameConstraints(NameConstraintsErrorKind),
    /// The exact target certificate did not contain Subject Alternative Name.
    MissingTargetSubjectAlternativeName,
    /// The exact target certificate's Subject Alternative Name was invalid.
    InvalidTargetSubjectAlternativeName(SubjectAltNameErrorKind),
    /// At least one target dNSName was invalid or violated the selected
    /// anchor's supported permitted constraint set.
    TargetDnsNameConstraints(DnsConstraintErrorKind),
}

/// A redacted reviewed server-link binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512ReviewedServerLinkError {
    kind: X509RsaSha512ReviewedServerLinkErrorKind,
}

impl X509RsaSha512ReviewedServerLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha512ReviewedServerLinkErrorKind::EvidenceMismatch,
        }
    }

    const fn new(kind: X509RsaSha512ReviewedServerLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512ReviewedServerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512ReviewedServerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 reviewed server-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512ReviewedServerLinkError {}

/// Non-copyable target-server policy bound to one reviewed RSA SHA-512 link.
///
/// The retained target evidence covers caller-owned critical-extension and
/// algorithm policy, end-entity role, SAN-only identity, explicit-time
/// validity, and one exact Extended Key Usage purpose. The retained link
/// evidence covers one RSA SHA-512 child-to-issuer signature and exact issuer
/// membership in the reviewed Mozilla snapshot. This wrapper does not apply
/// anchor constraints, construct a multi-hop path, check revocation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509RsaSha512ReviewedServerLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed exact-certificate target-server policy.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded> {
        &self.server_policy
    }

    /// Return the consumed authenticated link and reviewed-root binding.
    #[must_use]
    pub const fn reviewed_anchor_link(
        &self,
    ) -> &BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.reviewed_anchor_link
    }
}

impl fmt::Debug for BoundX509RsaSha512ReviewedServerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha512ReviewedServerLink")
    }
}

/// Non-copyable target-server policy bound to one reviewed two-link SHA-512
/// path.
///
/// The retained target evidence covers caller-owned critical-extension and
/// algorithm policy, end-entity role, SAN-only identity, explicit-time
/// validity, and one exact Extended Key Usage purpose. The retained path keeps
/// complete intermediate policy, both authenticated SHA-512 links, and exact
/// reviewed-root selection. This wrapper does not advance path-length state,
/// apply constraints, construct or search paths, check revocation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509RsaSha512ReviewedIntermediateServerPath<
    'server_certificate,
    'path_child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
> {
    server_policy: ValidatedX509ServerCertificatePolicy<'server_certificate, 'child_encoded>,
    reviewed_intermediate_path: BoundX509RsaSha512ReviewedIntermediatePath<
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
}

impl<
        'server_certificate,
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >
    BoundX509RsaSha512ReviewedIntermediateServerPath<
        'server_certificate,
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >
{
    /// Return the consumed exact-certificate target-server policy.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'server_certificate, 'child_encoded> {
        &self.server_policy
    }

    /// Return the consumed authenticated two-link path and reviewed root.
    #[must_use]
    pub const fn reviewed_intermediate_path(
        &self,
    ) -> &BoundX509RsaSha512ReviewedIntermediatePath<
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    > {
        &self.reviewed_intermediate_path
    }
}

impl fmt::Debug
    for BoundX509RsaSha512ReviewedIntermediateServerPath<'_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha512ReviewedIntermediateServerPath")
    }
}

/// Non-copyable reviewed SHA-512 server link after selected-anchor DNS policy.
///
/// Success proves either that the exact reviewed anchor retained by the link
/// carries no out-of-band name constraints or that its supported permitted DNS
/// subtrees were applied to every dNSName in the exact target retained by the
/// server policy. This is not generalized path validation, hostname
/// verification, revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509RsaSha512ReviewedServerAnchorPolicy<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    reviewed_server_link: BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    constrained_dns_names: Option<ConstrainedDnsNames>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    ValidatedX509RsaSha512ReviewedServerAnchorPolicy<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed complete reviewed server-link evidence.
    #[must_use]
    pub const fn reviewed_server_link(
        &self,
    ) -> &BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.reviewed_server_link
    }

    /// Return DNS evidence when the selected reviewed anchor carried
    /// constraints.
    #[must_use]
    pub const fn constrained_dns_names(&self) -> Option<ConstrainedDnsNames> {
        self.constrained_dns_names
    }
}

impl fmt::Debug for ValidatedX509RsaSha512ReviewedServerAnchorPolicy<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha512ReviewedServerAnchorPolicy")
    }
}

/// Bind target-server policy to one reviewed-root issuer link for the same
/// exact decoded target certificate object.
///
/// Byte equality is deliberately insufficient. Both non-copyable inputs must
/// originate from the same decoded target object. Success retains both
/// complete inputs but performs no additional certificate validation.
pub fn bind_x509_rsa_sha512_reviewed_server_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512ReviewedServerLinkError,
> {
    if !same_exact_object(
        server_policy.certificate(),
        reviewed_anchor_link.verified_issuer_link().child(),
    ) {
        return Err(X509RsaSha512ReviewedServerLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha512ReviewedServerLink {
        server_policy,
        reviewed_anchor_link,
    })
}

/// Bind target-server policy to a reviewed two-link SHA-512 path for the same
/// exact decoded target certificate object.
///
/// Byte equality is deliberately insufficient. The server-policy certificate
/// and the child authenticated by the target-to-intermediate link must be the
/// same object. Success retains both complete non-copyable inputs but performs
/// no additional certificate, anchor, path, or transport validation.
pub fn bind_x509_rsa_sha512_reviewed_intermediate_server_path<
    'server_certificate,
    'path_child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
>(
    server_policy: ValidatedX509ServerCertificatePolicy<'server_certificate, 'child_encoded>,
    reviewed_intermediate_path: BoundX509RsaSha512ReviewedIntermediatePath<
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
) -> Result<
    BoundX509RsaSha512ReviewedIntermediateServerPath<
        'server_certificate,
        'path_child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
    X509RsaSha512ReviewedServerLinkError,
> {
    let authenticated_child = reviewed_intermediate_path
        .authenticated_child_link()
        .issuer_policy_link()
        .child();
    if !same_exact_object(server_policy.certificate(), authenticated_child) {
        return Err(X509RsaSha512ReviewedServerLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha512ReviewedIntermediateServerPath {
        server_policy,
        reviewed_intermediate_path,
    })
}

/// Apply the selected reviewed anchor's supported DNS policy to the exact
/// target retained by one complete RSA SHA-512 reviewed server link.
///
/// The target certificate and optional constraint bytes are derived only from
/// the consumed proof. Separate caller-owned limits bound the reviewed-root
/// NameConstraints decoder and the target Subject Alternative Name decoder.
/// Success retains the complete input and does not grant transport authority.
pub fn validate_x509_rsa_sha512_reviewed_server_anchor_policy<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    reviewed_server_link: BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509RsaSha512ReviewedServerAnchorPolicy<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512ReviewedServerLinkError,
> {
    let certificate = reviewed_server_link.server_policy().certificate();
    let name_constraints = reviewed_server_link
        .reviewed_anchor_link()
        .reviewed_anchor()
        .name_constraints();
    let constrained_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509RsaSha512ReviewedServerAnchorPolicy {
        reviewed_server_link,
        constrained_dns_names,
    })
}

fn evaluate_selected_anchor_dns_policy(
    certificate: &x509_certificate::X509Certificate<'_>,
    name_constraints: Option<&[u8]>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<Option<ConstrainedDnsNames>, X509RsaSha512ReviewedServerLinkError> {
    let Some(encoded_constraints) = name_constraints else {
        return Ok(None);
    };

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints = decode_name_constraints_contents(
        &mut constraint_decoder,
        encoded_constraints,
    )
    .map_err(|error| {
        X509RsaSha512ReviewedServerLinkError::new(
            X509RsaSha512ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(error.kind()),
        )
    })?;

    let extension = certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
        })
        .ok_or_else(|| {
            X509RsaSha512ReviewedServerLinkError::new(
                X509RsaSha512ReviewedServerLinkErrorKind::MissingTargetSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name = decode_subject_alt_name(&mut subject_alt_name_decoder, extension)
        .map_err(|error| {
            X509RsaSha512ReviewedServerLinkError::new(
                X509RsaSha512ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(
                    error.kind(),
                ),
            )
        })?;
    let constrained_dns_names = evaluate_dns_names(&constraints, subject_alt_name.dns_names())
        .map_err(|error| {
            X509RsaSha512ReviewedServerLinkError::new(
                X509RsaSha512ReviewedServerLinkErrorKind::TargetDnsNameConstraints(error.kind()),
            )
        })?;

    Ok(Some(constrained_dns_names))
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    std::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use x509_certificate::decode_x509_certificate;

    const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];
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
        constraint_limits: Asn1Limits,
        subject_alt_name_limits: Asn1Limits,
    ) -> Result<Option<ConstrainedDnsNames>, X509RsaSha512ReviewedServerLinkError> {
        let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
        evaluate_selected_anchor_dns_policy(
            &certificate,
            constraints,
            constraint_limits,
            subject_alt_name_limits,
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
        let error = X509RsaSha512ReviewedServerLinkError::evidence_mismatch();

        assert_eq!(
            error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::EvidenceMismatch
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA SHA-512 reviewed server-link error EvidenceMismatch"
        );
    }

    #[test]
    fn absent_anchor_constraints_succeed_without_dns_evidence() {
        let encoded = certificate(&[]);

        let result =
            evaluate(&encoded, None, Asn1Limits::default(), Asn1Limits::default()).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn exact_reviewed_constraint_applies_to_every_target_dns_name() {
        let encoded = certificate(&[subject_alt_name(&[
            dns_name("api.tr"),
            dns_name("*.service.tr"),
        ])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let result = evaluate(
            &encoded,
            Some(&constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.count(), 2);
    }

    #[test]
    fn one_target_dns_name_outside_the_subtrees_fails_closed() {
        let encoded = certificate(&[subject_alt_name(&[
            dns_name("api.tr"),
            dns_name("secret.internal"),
        ])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let error = evaluate(
            &encoded,
            Some(&constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::TargetDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("api.tr"));
        assert!(!diagnostic.contains("secret.internal"));
    }

    #[test]
    fn malformed_constraint_and_target_san_are_redacted() {
        let valid_san = certificate(&[subject_alt_name(&[dns_name("api.tr")])]);
        let invalid_constraints = [0xa0, 0x00];
        let constraint_error = evaluate(
            &valid_san,
            Some(&invalid_constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            constraint_error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(_)
        ));

        let malformed_san =
            certificate(&[extension(SUBJECT_ALT_NAME_OID_DER, b"secret malformed SAN")]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);
        let san_error = evaluate(
            &malformed_san,
            Some(&constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            san_error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(_)
        ));
        assert!(!format!("{san_error:?} {san_error}").contains("secret"));
    }

    #[test]
    fn constrained_target_requires_subject_alt_name() {
        let encoded = certificate(&[]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let error = evaluate(
            &encoded,
            Some(&constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::MissingTargetSubjectAlternativeName
        );
    }

    #[test]
    fn caller_limits_independently_bound_both_decoders() {
        let encoded = certificate(&[subject_alt_name(&[dns_name("api.tr")])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);
        let exhausted = Asn1Limits {
            max_total_elements: 0,
            ..Default::default()
        };

        let constraint_error = evaluate(
            &encoded,
            Some(&constraints),
            exhausted,
            Asn1Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            constraint_error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(_)
        ));

        let san_error = evaluate(
            &encoded,
            Some(&constraints),
            Asn1Limits::default(),
            exhausted,
        )
        .unwrap_err();
        assert!(matches!(
            san_error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(_)
        ));
    }

    #[test]
    fn reviewed_intermediate_server_path_api_retains_both_complete_inputs() {
        type ServerPolicy = ValidatedX509ServerCertificatePolicy<'static, 'static>;
        type ReviewedIntermediatePath =
            x509_rsa_sha512_reviewed_anchor_link::BoundX509RsaSha512ReviewedIntermediatePath<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
            >;
        type ReviewedIntermediateServerPath = BoundX509RsaSha512ReviewedIntermediateServerPath<
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

        let _binder: fn(
            ServerPolicy,
            ReviewedIntermediatePath,
        ) -> Result<
            ReviewedIntermediateServerPath,
            X509RsaSha512ReviewedServerLinkError,
        > = bind_x509_rsa_sha512_reviewed_intermediate_server_path;

        fn inspect(evidence: &ReviewedIntermediateServerPath) {
            let _: &ServerPolicy = evidence.server_policy();
            let _: &ReviewedIntermediatePath = evidence.reviewed_intermediate_path();
        }
        let _inspector: fn(&ReviewedIntermediateServerPath) = inspect;
    }
}
