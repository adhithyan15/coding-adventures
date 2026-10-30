//! # Exact-certificate X.509 intermediate policy composition
//!
//! Path validation is a chain of evidence, not one boolean. In particular,
//! the role, algorithm, self-issued classification, and validity checks for an
//! intermediate certificate must all describe the same parsed object. A set
//! of detached successes can otherwise be mixed accidentally across
//! byte-equal or entirely unrelated certificates.
//!
//! This crate consumes the certificate-bound evidence produced by
//! `x509-certificate-policy`, checks exact object identity, requires the
//! issuing-CA role, and derives validity from the retained certificate at one
//! explicit caller-owned observation. A follow-up binds optional critical
//! NameConstraints decoding to that exact intermediate evidence. It performs
//! no path traversal, constraint evaluation, link authentication, trust-anchor
//! decision, revocation, TLS, or network work.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate_policy::{
    validate_certificate_validity_at, ClassifiedCertificateSelfIssued,
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateRolePolicy,
    ValidatedCertificateValidity, X509CertificateRole, X509CertificateValidityErrorKind,
};
use x509_name_constraints::{
    decode_name_constraints, NameConstraints, NameConstraintsErrorKind, NAME_CONSTRAINTS_OID,
};
use x509_time::X509Time;

/// Stable, payload-free intermediate-certificate policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509IntermediateCertificatePolicyErrorKind {
    /// At least one input retains a different decoded certificate object.
    EvidenceMismatch,
    /// The retained role policy is not for an issuing certificate authority.
    NotIssuingCertificateAuthority,
    /// The explicit observation was outside the certificate validity interval.
    Validity(X509CertificateValidityErrorKind),
}

/// A redacted intermediate-certificate policy failure.
///
/// The error retains no certificate bytes, Names, algorithm identifiers, or
/// time values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509IntermediateCertificatePolicyError {
    kind: X509IntermediateCertificatePolicyErrorKind,
}

impl X509IntermediateCertificatePolicyError {
    const fn new(kind: X509IntermediateCertificatePolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    pub const fn kind(self) -> X509IntermediateCertificatePolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509IntermediateCertificatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 intermediate certificate policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509IntermediateCertificatePolicyError {}

/// Stable optional NameConstraints policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509IntermediateNameConstraintsPolicyErrorKind {
    /// RFC 5280 requires a present NameConstraints extension to be critical.
    NonCriticalNameConstraints,
    /// The critical extension value did not satisfy the supported structure.
    InvalidNameConstraints(NameConstraintsErrorKind),
}

/// A redacted intermediate NameConstraints policy failure.
///
/// The error retains no certificate, extension, subtree, or offset bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509IntermediateNameConstraintsPolicyError {
    kind: X509IntermediateNameConstraintsPolicyErrorKind,
}

impl X509IntermediateNameConstraintsPolicyError {
    const fn new(kind: X509IntermediateNameConstraintsPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    pub const fn kind(self) -> X509IntermediateNameConstraintsPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509IntermediateNameConstraintsPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 intermediate NameConstraints policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509IntermediateNameConstraintsPolicyError {}

/// Non-copyable policy evidence for one exact decoded intermediate certificate.
///
/// The retained inputs prove caller-owned critical-extension recognition,
/// issuing-CA role semantics, one caller-selected algorithm profile, and one
/// caller-owned self-issued Name classification for this exact object. The
/// derived token proves only that one explicit observation was inside the
/// certificate's inclusive validity interval.
pub struct ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    self_issued_classification: ClassifiedCertificateSelfIssued<'certificate, 'encoded>,
    validity: ValidatedCertificateValidity,
}

impl<'certificate, 'encoded> ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
    /// Return the exact certificate retained by all policy evidence.
    pub const fn certificate(&self) -> &'certificate x509_certificate::X509Certificate<'encoded> {
        self.role_policy.certificate()
    }

    /// Return the consumed certificate-wide issuing-CA role policy.
    pub const fn role_policy(&self) -> &ValidatedCertificateRolePolicy<'certificate, 'encoded> {
        &self.role_policy
    }

    /// Return the consumed certificate-bound algorithm policy.
    pub const fn algorithm_policy(
        &self,
    ) -> &ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
        &self.algorithm_policy
    }

    /// Return the consumed caller-owned self-issued classification.
    pub const fn self_issued_classification(
        &self,
    ) -> &ClassifiedCertificateSelfIssued<'certificate, 'encoded> {
        &self.self_issued_classification
    }

    /// Return the explicit-time validity result.
    pub const fn validity(&self) -> ValidatedCertificateValidity {
        self.validity
    }
}

impl fmt::Debug for ValidatedX509IntermediateCertificatePolicy<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509IntermediateCertificatePolicy")
    }
}

/// Non-copyable optional NameConstraints evidence for one exact intermediate.
///
/// The retained base evidence already binds role, algorithm, self-issued, and
/// validity policy to this certificate. A present decoded value proves only
/// structural validity and criticality; it has not been applied to a path.
pub struct ValidatedX509IntermediateNameConstraintsPolicy<'certificate, 'encoded> {
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded>,
    name_constraints: Option<NameConstraints<'encoded>>,
}

impl<'certificate, 'encoded>
    ValidatedX509IntermediateNameConstraintsPolicy<'certificate, 'encoded>
{
    /// Return the complete consumed intermediate-certificate policy.
    pub const fn intermediate_policy(
        &self,
    ) -> &ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
        &self.intermediate_policy
    }

    /// Return the exact certificate retained by the complete evidence.
    pub const fn certificate(&self) -> &'certificate x509_certificate::X509Certificate<'encoded> {
        self.intermediate_policy.certificate()
    }

    /// Return decoded constraints when the exact certificate carried them.
    pub fn name_constraints(&self) -> Option<&NameConstraints<'encoded>> {
        self.name_constraints.as_ref()
    }
}

impl fmt::Debug for ValidatedX509IntermediateNameConstraintsPolicy<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedX509IntermediateNameConstraintsPolicy")
            .field("has_name_constraints", &self.name_constraints.is_some())
            .finish_non_exhaustive()
    }
}

/// Compose intermediate-certificate policy for one exact decoded object.
///
/// Object identity and the issuing-CA role are checked before validity. The
/// time comparison then uses only the certificate retained by `role_policy`;
/// callers cannot supply another certificate to that step. The observation is
/// explicit because clock acquisition and freshness remain caller authority.
pub fn validate_x509_intermediate_certificate_policy<'certificate, 'encoded>(
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    self_issued_classification: ClassifiedCertificateSelfIssued<'certificate, 'encoded>,
    observed: &X509Time,
) -> Result<
    ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded>,
    X509IntermediateCertificatePolicyError,
> {
    let certificate = role_policy.certificate();
    if !std::ptr::eq(certificate, algorithm_policy.certificate())
        || !std::ptr::eq(certificate, self_issued_classification.certificate())
    {
        return Err(X509IntermediateCertificatePolicyError::new(
            X509IntermediateCertificatePolicyErrorKind::EvidenceMismatch,
        ));
    }
    if role_policy.role_extensions().role() != X509CertificateRole::IssuingCertificateAuthority {
        return Err(X509IntermediateCertificatePolicyError::new(
            X509IntermediateCertificatePolicyErrorKind::NotIssuingCertificateAuthority,
        ));
    }

    let validity = validate_certificate_validity_at(certificate, observed).map_err(|error| {
        X509IntermediateCertificatePolicyError::new(
            X509IntermediateCertificatePolicyErrorKind::Validity(error.kind()),
        )
    })?;

    Ok(ValidatedX509IntermediateCertificatePolicy {
        role_policy,
        algorithm_policy,
        self_issued_classification,
        validity,
    })
}

/// Decode optional critical NameConstraints from one validated intermediate.
///
/// Absence succeeds with explicit `None`. Criticality is checked before the
/// caller-owned structural decoder budget is spent. Success retains the
/// complete input evidence, so callers cannot attach the decoded constraints
/// to another certificate. This function does not evaluate the constraints.
pub fn validate_x509_intermediate_name_constraints_policy<'certificate, 'encoded>(
    intermediate_policy: ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded>,
    limits: Asn1Limits,
) -> Result<
    ValidatedX509IntermediateNameConstraintsPolicy<'certificate, 'encoded>,
    X509IntermediateNameConstraintsPolicyError,
> {
    let extension = intermediate_policy
        .certificate()
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .find(|extension| extension.extension_id().equals(NAME_CONSTRAINTS_OID))
        });

    let name_constraints = match extension {
        None => None,
        Some(extension) => {
            if !extension.critical() {
                return Err(X509IntermediateNameConstraintsPolicyError::new(
                    X509IntermediateNameConstraintsPolicyErrorKind::NonCriticalNameConstraints,
                ));
            }
            let mut decoder = Asn1Decoder::new(limits);
            Some(
                decode_name_constraints(&mut decoder, extension.extension_value()).map_err(
                    |error| {
                        X509IntermediateNameConstraintsPolicyError::new(
                            X509IntermediateNameConstraintsPolicyErrorKind::InvalidNameConstraints(
                                error.kind(),
                            ),
                        )
                    },
                )?,
            )
        }
    };

    Ok(ValidatedX509IntermediateNameConstraintsPolicy {
        intermediate_policy,
        name_constraints,
    })
}
