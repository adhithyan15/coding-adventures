//! # Exact server-policy and reviewed-link binding
//!
//! This crate closes one evidence-mixing gap between target-server policy and
//! one authenticated RSA SHA-256 issuer link terminating at a reviewed root,
//! or to one authenticated target-to-intermediate-to-reviewed-root path. It
//! also applies selected reviewed-anchor DNS constraints to the exact target
//! certificate's Subject Alternative Name values for the direct-link case. It
//! performs no new signature work, root lookup, path search, revocation
//! processing, or network effects.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_name_constraints::{
    decode_name_constraints_contents, evaluate_permitted_dns_names, DnsConstraintErrorKind,
    NameConstraintsErrorKind, PermittedDnsNames,
};
use x509_rsa_sha256_reviewed_anchor_link::{
    BoundX509RsaSha256ReviewedAnchorLink, BoundX509RsaSha256ReviewedIntermediatePath,
};
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;
use x509_subject_alt_name::{
    decode_subject_alt_name, SubjectAltNameErrorKind, SUBJECT_ALT_NAME_OID,
};

/// Stable, payload-free reviewed server-link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ReviewedServerLinkErrorKind {
    /// The inputs retain different decoded child certificate objects.
    EvidenceMismatch,
    /// The selected reviewed anchor's exact constraint bytes were invalid or
    /// outside the supported bounded subset.
    InvalidAnchorNameConstraints(NameConstraintsErrorKind),
    /// The exact target certificate did not contain Subject Alternative Name.
    MissingTargetSubjectAlternativeName,
    /// The exact target certificate's Subject Alternative Name was invalid.
    InvalidTargetSubjectAlternativeName(SubjectAltNameErrorKind),
    /// At least one target dNSName was invalid or outside every permitted
    /// subtree.
    TargetDnsNameConstraints(DnsConstraintErrorKind),
}

/// A redacted reviewed server-link binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256ReviewedServerLinkError {
    kind: X509RsaSha256ReviewedServerLinkErrorKind,
}

impl X509RsaSha256ReviewedServerLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha256ReviewedServerLinkErrorKind::EvidenceMismatch,
        }
    }

    const fn new(kind: X509RsaSha256ReviewedServerLinkErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    pub const fn kind(self) -> X509RsaSha256ReviewedServerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256ReviewedServerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 reviewed server link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256ReviewedServerLinkError {}

/// Non-copyable evidence binding target-server policy to one reviewed link.
///
/// The retained target evidence covers caller-owned critical-extension and
/// algorithm policy, end-entity role, SAN-only identity, explicit-time
/// validity, and one exact Extended Key Usage purpose. The retained link
/// evidence covers one RSA SHA-256 child-to-issuer signature and exact issuer
/// membership in the reviewed Mozilla snapshot. This wrapper does not apply
/// anchor constraints, construct a multi-hop path, check revocation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509RsaSha256ReviewedServerLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha256ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    BoundX509RsaSha256ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed exact-certificate target-server policy.
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded> {
        &self.server_policy
    }

    /// Return the consumed authenticated link and reviewed-root binding.
    pub const fn reviewed_anchor_link(
        &self,
    ) -> &BoundX509RsaSha256ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.reviewed_anchor_link
    }
}

impl fmt::Debug for BoundX509RsaSha256ReviewedServerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha256ReviewedServerLink")
    }
}

/// Non-copyable evidence binding target-server policy to one reviewed
/// target-to-intermediate-to-root path.
///
/// The retained target evidence covers caller-owned critical-extension and
/// algorithm policy, end-entity role, SAN-only identity, explicit-time
/// validity, and one exact Extended Key Usage purpose. The retained path keeps
/// both authenticated RSA SHA-256 links, complete intermediate policy, the
/// advanced path-length state, and exact reviewed-root selection. This wrapper
/// does not apply anchor constraints, search or generalize paths, check
/// revocation, or grant TLS, HTTP, OAuth, or network authority.
pub struct BoundX509RsaSha256ReviewedIntermediateServerPath<
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
    reviewed_intermediate_path: BoundX509RsaSha256ReviewedIntermediatePath<
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
    BoundX509RsaSha256ReviewedIntermediateServerPath<
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
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'server_certificate, 'child_encoded> {
        &self.server_policy
    }

    /// Return the consumed authenticated two-link path and reviewed root.
    pub const fn reviewed_intermediate_path(
        &self,
    ) -> &BoundX509RsaSha256ReviewedIntermediatePath<
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
    for BoundX509RsaSha256ReviewedIntermediateServerPath<'_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha256ReviewedIntermediateServerPath")
    }
}

/// Non-copyable direct reviewed-server-link evidence after anchor DNS policy.
///
/// Success proves either that the selected snapshot entry carries no
/// out-of-band name constraints or that its supported permitted DNS subtrees
/// were applied to every dNSName in the exact target certificate. This is not
/// evidence for excluded or non-DNS constraints, a multi-hop path, revocation,
/// TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509RsaSha256ReviewedServerAnchorPolicy<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    reviewed_server_link: BoundX509RsaSha256ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    permitted_dns_names: Option<PermittedDnsNames>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    ValidatedX509RsaSha256ReviewedServerAnchorPolicy<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed direct reviewed-server-link evidence.
    pub const fn reviewed_server_link(
        &self,
    ) -> &BoundX509RsaSha256ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.reviewed_server_link
    }

    /// Return DNS-constraint evidence when the selected anchor carried the
    /// supported out-of-band constraint form.
    pub const fn permitted_dns_names(&self) -> Option<PermittedDnsNames> {
        self.permitted_dns_names
    }
}

impl fmt::Debug for ValidatedX509RsaSha256ReviewedServerAnchorPolicy<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha256ReviewedServerAnchorPolicy")
    }
}

/// Non-copyable reviewed two-link server-path evidence after anchor DNS policy.
///
/// Success proves either that the selected snapshot entry carries no
/// out-of-band name constraints or that its supported permitted DNS subtrees
/// were applied to every dNSName in the exact target certificate retained by
/// the complete target-to-intermediate-to-reviewed-root path. This is not
/// evidence for excluded or non-DNS constraints, generalized path validation,
/// revocation, TLS, HTTP, OAuth, or network authority.
pub struct ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy<
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
    reviewed_intermediate_server_path: BoundX509RsaSha256ReviewedIntermediateServerPath<
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
    permitted_dns_names: Option<PermittedDnsNames>,
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
    ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy<
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
    /// Return the consumed complete reviewed two-link server-path evidence.
    pub const fn reviewed_intermediate_server_path(
        &self,
    ) -> &BoundX509RsaSha256ReviewedIntermediateServerPath<
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
        &self.reviewed_intermediate_server_path
    }

    /// Return DNS-constraint evidence when the selected anchor carried the
    /// supported out-of-band constraint form.
    pub const fn permitted_dns_names(&self) -> Option<PermittedDnsNames> {
        self.permitted_dns_names
    }
}

impl fmt::Debug
    for ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy<
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
        formatter.write_str("ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy")
    }
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    std::ptr::eq(left, right)
}

fn evaluate_selected_anchor_dns_policy(
    certificate: &x509_certificate::X509Certificate<'_>,
    name_constraints: Option<&[u8]>,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<Option<PermittedDnsNames>, X509RsaSha256ReviewedServerLinkError> {
    let Some(encoded_constraints) = name_constraints else {
        return Ok(None);
    };

    let mut constraint_decoder = Asn1Decoder::new(constraint_limits);
    let constraints = decode_name_constraints_contents(
        &mut constraint_decoder,
        encoded_constraints,
    )
    .map_err(|error| {
        X509RsaSha256ReviewedServerLinkError::new(
            X509RsaSha256ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(error.kind()),
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
            X509RsaSha256ReviewedServerLinkError::new(
                X509RsaSha256ReviewedServerLinkErrorKind::MissingTargetSubjectAlternativeName,
            )
        })?;
    let mut subject_alt_name_decoder = Asn1Decoder::new(subject_alt_name_limits);
    let subject_alt_name = decode_subject_alt_name(&mut subject_alt_name_decoder, extension)
        .map_err(|error| {
            X509RsaSha256ReviewedServerLinkError::new(
                X509RsaSha256ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(
                    error.kind(),
                ),
            )
        })?;
    let permitted_dns_names = evaluate_permitted_dns_names(
        &constraints,
        subject_alt_name.dns_names(),
    )
    .map_err(|error| {
        X509RsaSha256ReviewedServerLinkError::new(
            X509RsaSha256ReviewedServerLinkErrorKind::TargetDnsNameConstraints(error.kind()),
        )
    })?;

    Ok(Some(permitted_dns_names))
}

/// Bind target-server policy to one reviewed-root issuer link for the same
/// exact decoded child certificate object.
///
/// Byte equality is deliberately insufficient. Both non-copyable inputs must
/// originate from the same decoded child object. Success retains both complete
/// inputs but does not perform any additional certificate validation.
pub fn bind_x509_rsa_sha256_reviewed_server_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha256ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    BoundX509RsaSha256ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256ReviewedServerLinkError,
> {
    let linked_child = reviewed_anchor_link
        .verified_issuer_link()
        .role_policy_link()
        .verified_link()
        .child();
    if !same_exact_object(server_policy.certificate(), linked_child) {
        return Err(X509RsaSha256ReviewedServerLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha256ReviewedServerLink {
        server_policy,
        reviewed_anchor_link,
    })
}

/// Bind target-server policy to a reviewed two-link path for the same exact
/// decoded target certificate object.
///
/// Byte equality is deliberately insufficient. The server-policy certificate
/// and the child authenticated by the target-to-intermediate step must be the
/// same object. Success retains both complete non-copyable inputs but performs
/// no additional certificate, anchor, path, or transport validation.
pub fn bind_x509_rsa_sha256_reviewed_intermediate_server_path<
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
    reviewed_intermediate_path: BoundX509RsaSha256ReviewedIntermediatePath<
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
    BoundX509RsaSha256ReviewedIntermediateServerPath<
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
    X509RsaSha256ReviewedServerLinkError,
> {
    let authenticated_child = reviewed_intermediate_path
        .authenticated_path()
        .authenticated_intermediate()
        .issuer_policy_link()
        .role_policy_link()
        .verified_link()
        .child();
    if !same_exact_object(server_policy.certificate(), authenticated_child) {
        return Err(X509RsaSha256ReviewedServerLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha256ReviewedIntermediateServerPath {
        server_policy,
        reviewed_intermediate_path,
    })
}

/// Apply supported selected-anchor DNS policy to one complete direct link.
///
/// The reviewed snapshot is data-driven. Entries without constraints succeed
/// without DNS evidence. For entries carrying constraint bytes, this function
/// decodes those exact selected-entry bytes and the exact target certificate's
/// Subject Alternative Name under separate caller-owned limits, then requires
/// every target dNSName to satisfy the permitted subtree set. Callers cannot
/// substitute another snapshot entry, target certificate, or SAN collection.
pub fn validate_x509_rsa_sha256_reviewed_server_anchor_policy<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    reviewed_server_link: BoundX509RsaSha256ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509RsaSha256ReviewedServerAnchorPolicy<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256ReviewedServerLinkError,
> {
    let certificate = reviewed_server_link.server_policy().certificate();
    let name_constraints = reviewed_server_link
        .reviewed_anchor_link()
        .reviewed_anchor()
        .name_constraints();
    let permitted_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(ValidatedX509RsaSha256ReviewedServerAnchorPolicy {
        reviewed_server_link,
        permitted_dns_names,
    })
}

/// Apply supported selected-anchor DNS policy to one complete two-link path.
///
/// The reviewed snapshot is data-driven. Entries without constraints succeed
/// without DNS evidence. For entries carrying constraint bytes, this function
/// decodes those exact selected-entry bytes and the exact target certificate's
/// Subject Alternative Name under separate caller-owned limits, then requires
/// every target dNSName to satisfy the permitted subtree set. The complete
/// target-to-intermediate-to-reviewed-root proof remains retained intact.
pub fn validate_x509_rsa_sha256_reviewed_intermediate_server_anchor_policy<
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
    reviewed_intermediate_server_path: BoundX509RsaSha256ReviewedIntermediateServerPath<
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
    constraint_limits: Asn1Limits,
    subject_alt_name_limits: Asn1Limits,
) -> Result<
    ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy<
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
    X509RsaSha256ReviewedServerLinkError,
> {
    let certificate = reviewed_intermediate_server_path
        .server_policy()
        .certificate();
    let name_constraints = reviewed_intermediate_server_path
        .reviewed_intermediate_path()
        .reviewed_parent_link()
        .reviewed_anchor()
        .name_constraints();
    let permitted_dns_names = evaluate_selected_anchor_dns_policy(
        certificate,
        name_constraints,
        constraint_limits,
        subject_alt_name_limits,
    )?;

    Ok(
        ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy {
            reviewed_intermediate_server_path,
            permitted_dns_names,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use der_asn1::Asn1Limits;
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

    fn ip_address(value: &[u8]) -> Vec<u8> {
        der(0x87, value)
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
    ) -> Result<Option<PermittedDnsNames>, X509RsaSha256ReviewedServerLinkError> {
        let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
        evaluate_selected_anchor_dns_policy(
            &certificate,
            constraints,
            constraint_limits,
            subject_alt_name_limits,
        )
    }

    #[test]
    fn exact_child_identity_accepts_only_the_same_decoded_object() {
        let encoded = certificate(&[]);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let other_certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

        assert!(same_exact_object(&certificate, &certificate));
        assert!(!same_exact_object(&certificate, &other_certificate));
    }

    #[test]
    fn mismatch_error_is_payload_free() {
        let error = X509RsaSha256ReviewedServerLinkError::evidence_mismatch();

        assert_eq!(
            error.kind(),
            X509RsaSha256ReviewedServerLinkErrorKind::EvidenceMismatch
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA SHA-256 reviewed server link error EvidenceMismatch"
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
            X509RsaSha256ReviewedServerLinkErrorKind::TargetDnsNameConstraints(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
            )
        );
        let diagnostic = format!("{error:?} {error}");
        assert!(!diagnostic.contains("api.tr"));
        assert!(!diagnostic.contains("secret.internal"));
    }

    #[test]
    fn dns_constraints_ignore_an_absent_dns_name_form() {
        let encoded = certificate(&[subject_alt_name(&[ip_address(&[192, 0, 2, 1])])]);
        let constraints = permitted_dns_subtrees(&[b".tr"]);

        let result = evaluate(
            &encoded,
            Some(&constraints),
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap()
        .unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn malformed_constraint_and_target_san_fail_with_stable_categories() {
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
            X509RsaSha256ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(_)
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
            X509RsaSha256ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(_)
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
            X509RsaSha256ReviewedServerLinkErrorKind::MissingTargetSubjectAlternativeName
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
            X509RsaSha256ReviewedServerLinkErrorKind::InvalidAnchorNameConstraints(_)
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
            X509RsaSha256ReviewedServerLinkErrorKind::InvalidTargetSubjectAlternativeName(_)
        ));
    }

    #[test]
    fn reviewed_intermediate_server_path_api_retains_both_complete_inputs() {
        type ServerPolicy = ValidatedX509ServerCertificatePolicy<'static, 'static>;
        type ReviewedIntermediatePath =
            x509_rsa_sha256_reviewed_anchor_link::BoundX509RsaSha256ReviewedIntermediatePath<
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
                'static,
            >;
        type ReviewedIntermediateServerPath = BoundX509RsaSha256ReviewedIntermediateServerPath<
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
            X509RsaSha256ReviewedServerLinkError,
        > = bind_x509_rsa_sha256_reviewed_intermediate_server_path;

        fn inspect(evidence: &ReviewedIntermediateServerPath) {
            let _: &ServerPolicy = evidence.server_policy();
            let _: &ReviewedIntermediatePath = evidence.reviewed_intermediate_path();
        }
        let _inspector: fn(&ReviewedIntermediateServerPath) = inspect;
    }
}
