//! # Exact server-policy and reviewed-link binding
//!
//! This crate closes one evidence-mixing gap between target-server policy and
//! one authenticated RSA SHA-256 issuer link terminating at a reviewed root.
//! It also applies a narrow fail-closed policy that accepts only selected
//! snapshot entries without out-of-band name constraints. It performs no new
//! parsing, signature work, root lookup, path construction, revocation
//! processing, or network effects.

#![deny(unsafe_code)]

use core::fmt;
use x509_rsa_sha256_reviewed_anchor_link::BoundX509RsaSha256ReviewedAnchorLink;
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;

/// Stable, payload-free reviewed server-link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ReviewedServerLinkErrorKind {
    /// The inputs retain different decoded child certificate objects.
    EvidenceMismatch,
    /// The selected reviewed anchor carries name constraints this crate cannot
    /// yet parse and apply.
    UnsupportedAnchorNameConstraints,
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

    const fn unsupported_anchor_name_constraints() -> Self {
        Self {
            kind: X509RsaSha256ReviewedServerLinkErrorKind::UnsupportedAnchorNameConstraints,
        }
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

/// Non-copyable direct reviewed-server-link evidence after fail-closed anchor
/// name-constraint policy.
///
/// Success proves that the selected reviewed snapshot entry carries no
/// out-of-band name constraints, so there are no such bytes for a direct path
/// consumer to ignore. This is deliberately not evidence that arbitrary name
/// constraints were parsed or applied, and it grants no multi-hop path,
/// revocation, TLS, HTTP, OAuth, or network authority.
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
}

impl fmt::Debug for ValidatedX509RsaSha256ReviewedServerAnchorPolicy<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaSha256ReviewedServerAnchorPolicy")
    }
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    std::ptr::eq(left, right)
}

fn selected_anchor_has_no_name_constraints(name_constraints: Option<&[u8]>) -> bool {
    name_constraints.is_none()
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

/// Apply the currently supported fail-closed policy to the selected reviewed
/// trust anchor for one complete direct server link.
///
/// The reviewed snapshot is data-driven: entries without name constraints are
/// accepted, while every entry carrying constraint bytes is rejected until a
/// repository-owned parser and subtree evaluator can apply them. Constraint
/// presence is checked through the anchor evidence retained inside the
/// consumed link, so callers cannot substitute a different snapshot entry.
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
) -> Result<
    ValidatedX509RsaSha256ReviewedServerAnchorPolicy<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256ReviewedServerLinkError,
> {
    let name_constraints = reviewed_server_link
        .reviewed_anchor_link()
        .reviewed_anchor()
        .name_constraints();
    if !selected_anchor_has_no_name_constraints(name_constraints) {
        return Err(X509RsaSha256ReviewedServerLinkError::unsupported_anchor_name_constraints());
    }

    Ok(ValidatedX509RsaSha256ReviewedServerAnchorPolicy {
        reviewed_server_link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use der_asn1::Asn1Limits;
    use x509_certificate::decode_x509_certificate;

    const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
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

    fn certificate() -> Vec<u8> {
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

        let mut fields = der(0x30, &tbs);
        fields.extend(algorithm(SIGNATURE_OID_DER));
        fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
        der(0x30, &fields)
    }

    #[test]
    fn exact_child_identity_accepts_only_the_same_decoded_object() {
        let encoded = certificate();
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
    fn anchor_policy_accepts_only_absent_name_constraints() {
        assert!(selected_anchor_has_no_name_constraints(None));
        assert!(!selected_anchor_has_no_name_constraints(Some(&[
            0x30, 0x00,
        ])));
    }

    #[test]
    fn unsupported_anchor_name_constraints_error_is_payload_free() {
        let error = X509RsaSha256ReviewedServerLinkError::unsupported_anchor_name_constraints();

        assert_eq!(
            error.kind(),
            X509RsaSha256ReviewedServerLinkErrorKind::UnsupportedAnchorNameConstraints
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA SHA-256 reviewed server link error UnsupportedAnchorNameConstraints"
        );
    }
}
