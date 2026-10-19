//! # Data-driven RFC 5280 certificate policy gates
//!
//! This crate applies certificate-wide policy to already decoded values. It
//! deliberately embeds no general extension or algorithm registry, trust,
//! cryptographic authority, clock, TLS, or network. Its role gate enforces only
//! algorithm-independent relationships among Basic Constraints, Key Usage,
//! and Extended Key Usage. Its algorithm gate selects one exact caller-supplied
//! profile without parsing keys or signatures or choosing a verifier.
//! Its server-identity gate decodes one Subject Alternative Name extension and
//! applies the repository-owned DNS-ID/IP-ID matcher without Common Name
//! fallback. Its validity gate classifies an explicit caller-owned observation
//! against the decoded inclusive interval without reading a clock. Its purpose
//! gate requires one exact caller-supplied Extended Key Usage OID over already
//! validated end-entity role evidence. None of these gates establishes
//! certificate trust.

#![deny(unsafe_code)]

use der_asn1::{Asn1Decoder, Asn1Element, Asn1Limits, DerBitString, ObjectIdentifier};
use std::error::Error;
use std::fmt;
use tls_server_identity::{
    verify_server_identity, MatchKind, PresentedIdentities, ReferenceIdentity,
};
use x509_algorithm_identifier::X509AlgorithmIdentifier;
use x509_basic_constraints::{decode_basic_constraints, BasicConstraints, BASIC_CONSTRAINTS_OID};
use x509_certificate::X509Certificate;
use x509_extended_key_usage::{
    decode_extended_key_usage, ExtendedKeyUsage, EXTENDED_KEY_USAGE_OID,
};
use x509_key_usage::{decode_key_usage, KeyUsage, KeyUsageBit, KEY_USAGE_OID};
use x509_subject_alt_name::{decode_subject_alt_name, SUBJECT_ALT_NAME_OID};
use x509_time::X509Time;
use x509_validity::X509ValidityStatus;

/// Exact parameter-presence policy for one AlgorithmIdentifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509AlgorithmParametersPolicy<'a> {
    /// Parameters must be absent.
    Absent,
    /// Parameters must equal these complete canonical DER element bytes.
    ExactDer(&'a [u8]),
}

impl X509AlgorithmParametersPolicy<'_> {
    fn matches(self, parameters: Option<Asn1Element<'_>>) -> bool {
        match (self, parameters) {
            (Self::Absent, None) => true,
            (Self::ExactDer(expected), Some(actual)) => expected == actual.encoded(),
            (Self::Absent | Self::ExactDer(_), _) => false,
        }
    }
}

/// One inclusive, nonzero bit-length range from a validated profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509BitLengthRange {
    minimum_bits: usize,
    maximum_bits: usize,
}

impl X509BitLengthRange {
    const fn validated(minimum_bits: usize, maximum_bits: usize) -> Option<Self> {
        if minimum_bits == 0 || minimum_bits > maximum_bits {
            None
        } else {
            Some(Self {
                minimum_bits,
                maximum_bits,
            })
        }
    }

    pub const fn minimum_bits(self) -> usize {
        self.minimum_bits
    }

    pub const fn maximum_bits(self) -> usize {
        self.maximum_bits
    }

    const fn contains(self, bit_length: usize) -> bool {
        self.minimum_bits <= bit_length && bit_length <= self.maximum_bits
    }
}

/// Stable caller-profile construction failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateAlgorithmProfileDefinitionErrorKind {
    EmptySignatureAlgorithm,
    EmptySubjectPublicKeyAlgorithm,
    InvalidSubjectPublicKeyBitLengthRange,
    InvalidSignatureBitLengthRange,
}

/// A redacted invalid-profile error that retains no policy data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateAlgorithmProfileDefinitionError {
    kind: X509CertificateAlgorithmProfileDefinitionErrorKind,
}

impl X509CertificateAlgorithmProfileDefinitionError {
    const fn new(kind: X509CertificateAlgorithmProfileDefinitionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificateAlgorithmProfileDefinitionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificateAlgorithmProfileDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate algorithm profile definition error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificateAlgorithmProfileDefinitionError {}

/// One caller-supplied exact certificate algorithm and encoded-size profile.
///
/// This value is policy data only. It does not enable an algorithm or select a
/// cryptographic implementation. The subject-public-key range measures the
/// encoded BIT STRING, not algorithm-specific strength such as an RSA modulus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateAlgorithmProfile<'a> {
    signature_algorithm: &'a [u64],
    signature_parameters: X509AlgorithmParametersPolicy<'a>,
    subject_public_key_algorithm: &'a [u64],
    subject_public_key_parameters: X509AlgorithmParametersPolicy<'a>,
    subject_public_key_bit_length: X509BitLengthRange,
    signature_bit_length: X509BitLengthRange,
}

impl<'a> X509CertificateAlgorithmProfile<'a> {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        signature_algorithm: &'a [u64],
        signature_parameters: X509AlgorithmParametersPolicy<'a>,
        subject_public_key_algorithm: &'a [u64],
        subject_public_key_parameters: X509AlgorithmParametersPolicy<'a>,
        subject_public_key_minimum_bits: usize,
        subject_public_key_maximum_bits: usize,
        signature_minimum_bits: usize,
        signature_maximum_bits: usize,
    ) -> Result<Self, X509CertificateAlgorithmProfileDefinitionError> {
        if signature_algorithm.is_empty() {
            return Err(X509CertificateAlgorithmProfileDefinitionError::new(
                X509CertificateAlgorithmProfileDefinitionErrorKind::EmptySignatureAlgorithm,
            ));
        }
        if subject_public_key_algorithm.is_empty() {
            return Err(X509CertificateAlgorithmProfileDefinitionError::new(
                X509CertificateAlgorithmProfileDefinitionErrorKind::EmptySubjectPublicKeyAlgorithm,
            ));
        }
        let Some(subject_public_key_bit_length) = X509BitLengthRange::validated(
            subject_public_key_minimum_bits,
            subject_public_key_maximum_bits,
        ) else {
            return Err(X509CertificateAlgorithmProfileDefinitionError::new(
                X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSubjectPublicKeyBitLengthRange,
            ));
        };
        let Some(signature_bit_length) =
            X509BitLengthRange::validated(signature_minimum_bits, signature_maximum_bits)
        else {
            return Err(X509CertificateAlgorithmProfileDefinitionError::new(
                X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSignatureBitLengthRange,
            ));
        };

        Ok(Self {
            signature_algorithm,
            signature_parameters,
            subject_public_key_algorithm,
            subject_public_key_parameters,
            subject_public_key_bit_length,
            signature_bit_length,
        })
    }

    pub const fn signature_algorithm(&self) -> &'a [u64] {
        self.signature_algorithm
    }

    pub const fn signature_parameters(&self) -> X509AlgorithmParametersPolicy<'a> {
        self.signature_parameters
    }

    pub const fn subject_public_key_algorithm(&self) -> &'a [u64] {
        self.subject_public_key_algorithm
    }

    pub const fn subject_public_key_parameters(&self) -> X509AlgorithmParametersPolicy<'a> {
        self.subject_public_key_parameters
    }

    pub const fn subject_public_key_bit_length(&self) -> X509BitLengthRange {
        self.subject_public_key_bit_length
    }

    pub const fn signature_bit_length(&self) -> X509BitLengthRange {
        self.signature_bit_length
    }

    fn matches(
        &self,
        signature_algorithm: X509AlgorithmIdentifier<'_>,
        subject_public_key_algorithm: X509AlgorithmIdentifier<'_>,
    ) -> bool {
        signature_algorithm
            .algorithm()
            .equals(self.signature_algorithm)
            && self
                .signature_parameters
                .matches(signature_algorithm.parameters())
            && subject_public_key_algorithm
                .algorithm()
                .equals(self.subject_public_key_algorithm)
            && self
                .subject_public_key_parameters
                .matches(subject_public_key_algorithm.parameters())
    }
}

/// Stable certificate algorithm-policy failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateAlgorithmErrorKind {
    UnsupportedAlgorithmProfile,
    AmbiguousAlgorithmProfile,
    SubjectPublicKeyLengthOutOfPolicy,
    SignatureLengthOutOfPolicy,
}

/// A redacted certificate algorithm-policy failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateAlgorithmError {
    kind: X509CertificateAlgorithmErrorKind,
}

impl X509CertificateAlgorithmError {
    const fn new(kind: X509CertificateAlgorithmErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificateAlgorithmErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificateAlgorithmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate algorithm policy error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificateAlgorithmError {}

/// Stable complete-certificate server-identity failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateServerIdentityErrorKind {
    MissingSubjectAlternativeName,
    InvalidSubjectAlternativeName,
    IdentityMismatch,
}

/// A redacted complete-certificate server-identity failure.
///
/// `extension_index` is present only when the certificate contained a Subject
/// Alternative Name extension. No presented name or certificate byte is
/// retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateServerIdentityError {
    kind: X509CertificateServerIdentityErrorKind,
    extension_index: Option<usize>,
}

impl X509CertificateServerIdentityError {
    const fn new(
        kind: X509CertificateServerIdentityErrorKind,
        extension_index: Option<usize>,
    ) -> Self {
        Self {
            kind,
            extension_index,
        }
    }

    pub const fn kind(self) -> X509CertificateServerIdentityErrorKind {
        self.kind
    }

    pub const fn extension_index(self) -> Option<usize> {
        self.extension_index
    }
}

impl fmt::Display for X509CertificateServerIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension_index {
            Some(index) => write!(
                formatter,
                "X.509 certificate server identity error {:?} at extension index {}",
                self.kind, index
            ),
            None => write!(
                formatter,
                "X.509 certificate server identity error {:?}",
                self.kind
            ),
        }
    }
}

impl Error for X509CertificateServerIdentityError {}

/// Stable complete-certificate validity-at-time failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateValidityErrorKind {
    NotYetValid,
    Expired,
}

/// A redacted validity-at-time failure that retains no calendar fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateValidityError {
    kind: X509CertificateValidityErrorKind,
}

impl X509CertificateValidityError {
    const fn new(kind: X509CertificateValidityErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificateValidityErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificateValidityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate validity policy error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificateValidityError {}

/// One exact caller-supplied Extended Key Usage purpose.
///
/// Purpose selection is application policy data. This type deliberately
/// embeds no purpose registry and gives no OID special treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePurposePolicy<'a> {
    purpose: &'a [u64],
}

impl<'a> X509CertificatePurposePolicy<'a> {
    pub const fn new(
        purpose: &'a [u64],
    ) -> Result<Self, X509CertificatePurposePolicyDefinitionError> {
        if purpose.is_empty() {
            return Err(X509CertificatePurposePolicyDefinitionError::new(
                X509CertificatePurposePolicyDefinitionErrorKind::EmptyPurpose,
            ));
        }
        Ok(Self { purpose })
    }

    pub const fn purpose(self) -> &'a [u64] {
        self.purpose
    }
}

/// Stable caller-purpose-policy construction failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificatePurposePolicyDefinitionErrorKind {
    EmptyPurpose,
}

/// A redacted invalid-purpose-policy error that retains no OID arcs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePurposePolicyDefinitionError {
    kind: X509CertificatePurposePolicyDefinitionErrorKind,
}

impl X509CertificatePurposePolicyDefinitionError {
    const fn new(kind: X509CertificatePurposePolicyDefinitionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificatePurposePolicyDefinitionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePurposePolicyDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate purpose policy definition error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificatePurposePolicyDefinitionError {}

/// Stable certificate-purpose failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificatePurposeErrorKind {
    NotEndEntity,
    MissingExtendedKeyUsage,
    PurposeNotAsserted,
}

/// A redacted purpose-authorization failure that retains no OID arcs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePurposeError {
    kind: X509CertificatePurposeErrorKind,
}

impl X509CertificatePurposeError {
    const fn new(kind: X509CertificatePurposeErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificatePurposeErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePurposeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate purpose policy error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificatePurposeError {}

/// Opaque evidence that one exact purpose was asserted for an end entity.
///
/// This value grants no key operation, signature, path, identity, validity,
/// trust, revocation, TLS, or network authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedCertificatePurpose {
    private: (),
}

/// Opaque evidence that one explicit observation was inside the interval.
///
/// This value proves nothing about the observation's source or freshness and
/// grants no certificate-path, signature, identity, trust, TLS, or network
/// authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedCertificateValidity {
    private: (),
}

/// Opaque evidence that one certificate Subject Alternative Name matched.
///
/// This value proves neither certificate trust nor path, signature, validity,
/// revocation, key-usage, or TLS correctness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedCertificateServerIdentity {
    extension_index: usize,
    match_kind: MatchKind,
}

impl ValidatedCertificateServerIdentity {
    pub const fn extension_index(self) -> usize {
        self.extension_index
    }

    pub const fn match_kind(self) -> MatchKind {
        self.match_kind
    }
}

/// Borrowed algorithm and encoded bit-string evidence after one profile matches.
///
/// This value proves only the exact identifier and length checks. It does not
/// parse keys or signatures, verify a signature, or establish certificate
/// trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedCertificateAlgorithmProfile<'a> {
    profile_index: usize,
    signature_algorithm: X509AlgorithmIdentifier<'a>,
    subject_public_key_algorithm: X509AlgorithmIdentifier<'a>,
    subject_public_key: DerBitString<'a>,
    signature_value: DerBitString<'a>,
}

impl<'a> ValidatedCertificateAlgorithmProfile<'a> {
    pub const fn profile_index(self) -> usize {
        self.profile_index
    }

    pub const fn signature_algorithm(self) -> X509AlgorithmIdentifier<'a> {
        self.signature_algorithm
    }

    pub const fn subject_public_key_algorithm(self) -> X509AlgorithmIdentifier<'a> {
        self.subject_public_key_algorithm
    }

    pub const fn subject_public_key(self) -> DerBitString<'a> {
        self.subject_public_key
    }

    pub const fn signature_value(self) -> DerBitString<'a> {
        self.signature_value
    }
}

/// Stable certificate-policy failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificatePolicyErrorKind {
    /// A critical extension was not recognized by the caller's policy.
    UnsupportedCriticalExtension,
}

/// A redacted certificate-policy failure.
///
/// `extension_index` is the zero-based index in certificate wire order. The
/// error intentionally retains neither the extension OID nor certificate bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePolicyError {
    kind: X509CertificatePolicyErrorKind,
    extension_index: usize,
}

impl X509CertificatePolicyError {
    const fn unsupported_critical_extension(extension_index: usize) -> Self {
        Self {
            kind: X509CertificatePolicyErrorKind::UnsupportedCriticalExtension,
            extension_index,
        }
    }

    pub const fn kind(self) -> X509CertificatePolicyErrorKind {
        self.kind
    }

    pub const fn extension_index(self) -> usize {
        self.extension_index
    }
}

impl fmt::Display for X509CertificatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate policy error {:?} at extension index {}",
            self.kind, self.extension_index
        )
    }
}

impl Error for X509CertificatePolicyError {}

/// One caller-selected, algorithm-independent certificate role.
///
/// Passing a role is a request to check the extension relationships for that
/// role. It does not grant the role or establish certificate trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateRole {
    EndEntity,
    IssuingCertificateAuthority,
}

/// Stable certificate-role failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateRoleErrorKind {
    InvalidBasicConstraints,
    InvalidKeyUsage,
    InvalidExtendedKeyUsage,
    MissingBasicConstraints,
    BasicConstraintsNotCritical,
    CertificateAuthorityNotAsserted,
    MissingKeyUsage,
    CertificateSigningNotAsserted,
    EndEntityAssertsCertificateAuthority,
    EndEntityAssertsCertificateSigning,
}

/// A redacted certificate-role failure.
///
/// `extension_index` is present only when the failure belongs to one extension
/// in certificate wire order. The error retains no OID or input bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateRoleError {
    kind: X509CertificateRoleErrorKind,
    extension_index: Option<usize>,
}

impl X509CertificateRoleError {
    const fn new(kind: X509CertificateRoleErrorKind, extension_index: Option<usize>) -> Self {
        Self {
            kind,
            extension_index,
        }
    }

    pub const fn kind(self) -> X509CertificateRoleErrorKind {
        self.kind
    }

    pub const fn extension_index(self) -> Option<usize> {
        self.extension_index
    }
}

impl fmt::Display for X509CertificateRoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension_index {
            Some(index) => write!(
                formatter,
                "X.509 certificate role error {:?} at extension index {}",
                self.kind, index
            ),
            None => write!(formatter, "X.509 certificate role error {:?}", self.kind),
        }
    }
}

impl Error for X509CertificateRoleError {}

/// Borrowed role-extension evidence after algorithm-independent checks.
///
/// This value is not evidence of a valid path, trusted CA, authenticated
/// server, permitted purpose, or usable public key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedCertificateRoleExtensions<'a> {
    role: X509CertificateRole,
    basic_constraints: Option<BasicConstraints<'a>>,
    key_usage: Option<KeyUsage>,
    extended_key_usage: Option<ExtendedKeyUsage<'a>>,
}

impl<'a> ValidatedCertificateRoleExtensions<'a> {
    pub const fn role(&self) -> X509CertificateRole {
        self.role
    }

    pub const fn basic_constraints(&self) -> Option<BasicConstraints<'a>> {
        self.basic_constraints
    }

    pub const fn key_usage(&self) -> Option<KeyUsage> {
        self.key_usage
    }

    pub fn extended_key_usage(&self) -> Option<&ExtendedKeyUsage<'a>> {
        self.extended_key_usage.as_ref()
    }
}

/// Decode and match one complete certificate's Subject Alternative Name.
///
/// The caller-selected reference is already validated and unrestricted. DNS
/// references are matched only against dNSName values and IP references only
/// against iPAddress values. There is no subject Common Name input or fallback.
/// This gate does not replace [`validate_critical_extensions`] and grants no
/// certificate-path, signature, trust, clock, revocation, TLS, or network
/// authority.
pub fn validate_certificate_server_identity(
    certificate: &X509Certificate<'_>,
    reference: &ReferenceIdentity,
    limits: Asn1Limits,
) -> Result<ValidatedCertificateServerIdentity, X509CertificateServerIdentityError> {
    let extension = certificate
        .tbs_certificate()
        .extensions()
        .and_then(|extensions| {
            extensions
                .iter()
                .enumerate()
                .find(|(_, extension)| extension.extension_id().equals(SUBJECT_ALT_NAME_OID))
        })
        .ok_or_else(|| {
            X509CertificateServerIdentityError::new(
                X509CertificateServerIdentityErrorKind::MissingSubjectAlternativeName,
                None,
            )
        })?;
    let (extension_index, extension) = extension;
    let mut decoder = Asn1Decoder::new(limits);
    let names = decode_subject_alt_name(&mut decoder, extension).map_err(|_| {
        X509CertificateServerIdentityError::new(
            X509CertificateServerIdentityErrorKind::InvalidSubjectAlternativeName,
            Some(extension_index),
        )
    })?;
    let verified = verify_server_identity(
        reference,
        PresentedIdentities::new(names.dns_names(), names.ip_addresses()),
    )
    .map_err(|_| {
        X509CertificateServerIdentityError::new(
            X509CertificateServerIdentityErrorKind::IdentityMismatch,
            Some(extension_index),
        )
    })?;

    Ok(ValidatedCertificateServerIdentity {
        extension_index,
        match_kind: verified.kind(),
    })
}

/// Check the decoded inclusive validity interval at one explicit observation.
///
/// The caller owns time acquisition, validation, UTC conversion, freshness,
/// and audit. This function reads no ambient clock and applies no tolerance.
/// It grants no certificate-path, signature, identity, purpose, revocation,
/// trust-root, TLS, or network authority.
pub fn validate_certificate_validity_at(
    certificate: &X509Certificate<'_>,
    observed: &X509Time,
) -> Result<ValidatedCertificateValidity, X509CertificateValidityError> {
    match certificate.tbs_certificate().validity().classify(observed) {
        X509ValidityStatus::NotYetValid => Err(X509CertificateValidityError::new(
            X509CertificateValidityErrorKind::NotYetValid,
        )),
        X509ValidityStatus::Valid => Ok(ValidatedCertificateValidity { private: () }),
        X509ValidityStatus::Expired => Err(X509CertificateValidityError::new(
            X509CertificateValidityErrorKind::Expired,
        )),
    }
}

/// Authorize one exact Extended Key Usage purpose for end-entity evidence.
///
/// The role evidence must come from [`validate_certificate_role_extensions`].
/// Extended Key Usage must be present and contain the exact caller-supplied
/// OID. No OID, including `anyExtendedKeyUsage`, receives wildcard semantics.
/// This gate performs no key operation and establishes no certificate trust.
pub fn validate_certificate_extended_key_purpose(
    role_evidence: &ValidatedCertificateRoleExtensions<'_>,
    policy: X509CertificatePurposePolicy<'_>,
) -> Result<ValidatedCertificatePurpose, X509CertificatePurposeError> {
    if role_evidence.role() != X509CertificateRole::EndEntity {
        return Err(X509CertificatePurposeError::new(
            X509CertificatePurposeErrorKind::NotEndEntity,
        ));
    }
    let Some(extended_key_usage) = role_evidence.extended_key_usage() else {
        return Err(X509CertificatePurposeError::new(
            X509CertificatePurposeErrorKind::MissingExtendedKeyUsage,
        ));
    };
    if !extended_key_usage.contains(policy.purpose()) {
        return Err(X509CertificatePurposeError::new(
            X509CertificatePurposeErrorKind::PurposeNotAsserted,
        ));
    }

    Ok(ValidatedCertificatePurpose { private: () })
}

/// Select exactly one caller-supplied certificate algorithm profile.
///
/// Selection compares both AlgorithmIdentifier OIDs and their exact parameter
/// encodings. Ambiguity is rejected before the selected profile's inclusive
/// public-key and signature bit-length bounds are applied. This function does
/// not parse either bit string or select or invoke a cryptographic verifier.
pub fn validate_certificate_algorithm_profile<'certificate, 'policy>(
    certificate: &X509Certificate<'certificate>,
    profiles: &[X509CertificateAlgorithmProfile<'policy>],
) -> Result<ValidatedCertificateAlgorithmProfile<'certificate>, X509CertificateAlgorithmError> {
    let signature_algorithm = certificate.signature_algorithm();
    let subject_public_key_info = certificate.tbs_certificate().subject_public_key_info();
    let subject_public_key_algorithm = subject_public_key_info.algorithm();
    let mut selected = None;

    for (profile_index, profile) in profiles.iter().enumerate() {
        if profile.matches(signature_algorithm, subject_public_key_algorithm) {
            if selected.is_some() {
                return Err(X509CertificateAlgorithmError::new(
                    X509CertificateAlgorithmErrorKind::AmbiguousAlgorithmProfile,
                ));
            }
            selected = Some((profile_index, profile));
        }
    }

    let Some((profile_index, profile)) = selected else {
        return Err(X509CertificateAlgorithmError::new(
            X509CertificateAlgorithmErrorKind::UnsupportedAlgorithmProfile,
        ));
    };
    let subject_public_key = subject_public_key_info.subject_public_key();
    if !profile
        .subject_public_key_bit_length
        .contains(subject_public_key.bit_len())
    {
        return Err(X509CertificateAlgorithmError::new(
            X509CertificateAlgorithmErrorKind::SubjectPublicKeyLengthOutOfPolicy,
        ));
    }
    let signature_value = certificate.signature_value();
    if !profile
        .signature_bit_length
        .contains(signature_value.bit_len())
    {
        return Err(X509CertificateAlgorithmError::new(
            X509CertificateAlgorithmErrorKind::SignatureLengthOutOfPolicy,
        ));
    }

    Ok(ValidatedCertificateAlgorithmProfile {
        profile_index,
        signature_algorithm,
        subject_public_key_algorithm,
        subject_public_key,
        signature_value,
    })
}

/// Require caller recognition of every critical certificate extension.
///
/// The recognizer is called exactly once for each critical extension, in wire
/// order, and is not called for non-critical extensions. Evaluation stops at
/// the first rejected critical extension. Returning `true` means only that the
/// caller recognizes the OID; extension-specific decoding and semantic policy
/// remain separate mandatory steps.
pub fn validate_critical_extensions<'a>(
    certificate: &X509Certificate<'a>,
    mut recognizes: impl FnMut(ObjectIdentifier<'a>) -> bool,
) -> Result<(), X509CertificatePolicyError> {
    let Some(extensions) = certificate.tbs_certificate().extensions() else {
        return Ok(());
    };

    for (extension_index, extension) in extensions.iter().enumerate() {
        if extension.critical() && !recognizes(extension.extension_id()) {
            return Err(X509CertificatePolicyError::unsupported_critical_extension(
                extension_index,
            ));
        }
    }

    Ok(())
}

/// Decode and validate algorithm-independent certificate-role extensions.
///
/// One decoder shares `limits` across every present Basic Constraints, Key
/// Usage, and Extended Key Usage value in certificate wire order. Issuing CAs
/// require critical `cA = TRUE` and `keyCertSign`; end entities must assert
/// neither. Extended Key Usage is decoded but deliberately uninterpreted.
///
/// This gate does not replace [`validate_critical_extensions`]. Callers must
/// separately reject unsupported critical extensions and semantically validate
/// every other recognized critical extension before assigning trust.
pub fn validate_certificate_role_extensions<'a>(
    certificate: &X509Certificate<'a>,
    role: X509CertificateRole,
    limits: Asn1Limits,
) -> Result<ValidatedCertificateRoleExtensions<'a>, X509CertificateRoleError> {
    let mut decoder = Asn1Decoder::new(limits);
    let mut basic_constraints = None;
    let mut key_usage = None;
    let mut extended_key_usage = None;

    if let Some(extensions) = certificate.tbs_certificate().extensions() {
        for (extension_index, extension) in extensions.iter().enumerate() {
            let extension_id = extension.extension_id();
            if extension_id.equals(BASIC_CONSTRAINTS_OID) {
                let value = decode_basic_constraints(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidBasicConstraints,
                        Some(extension_index),
                    )
                })?;
                basic_constraints = Some((extension_index, extension.critical(), value));
            } else if extension_id.equals(KEY_USAGE_OID) {
                let value = decode_key_usage(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidKeyUsage,
                        Some(extension_index),
                    )
                })?;
                key_usage = Some((extension_index, value));
            } else if extension_id.equals(EXTENDED_KEY_USAGE_OID) {
                let value = decode_extended_key_usage(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidExtendedKeyUsage,
                        Some(extension_index),
                    )
                })?;
                extended_key_usage = Some((extension_index, value));
            }
        }
    }

    match role {
        X509CertificateRole::IssuingCertificateAuthority => {
            let Some((extension_index, critical, constraints)) = basic_constraints.as_ref() else {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::MissingBasicConstraints,
                    None,
                ));
            };
            if !critical {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::BasicConstraintsNotCritical,
                    Some(*extension_index),
                ));
            }
            if !constraints.is_ca() {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::CertificateAuthorityNotAsserted,
                    Some(*extension_index),
                ));
            }

            let Some((extension_index, usage)) = key_usage.as_ref() else {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::MissingKeyUsage,
                    None,
                ));
            };
            if !usage.contains(KeyUsageBit::KeyCertSign) {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::CertificateSigningNotAsserted,
                    Some(*extension_index),
                ));
            }
        }
        X509CertificateRole::EndEntity => {
            if let Some((extension_index, _, constraints)) = basic_constraints.as_ref() {
                if constraints.is_ca() {
                    return Err(X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::EndEntityAssertsCertificateAuthority,
                        Some(*extension_index),
                    ));
                }
            }
            if let Some((extension_index, usage)) = key_usage.as_ref() {
                if usage.contains(KeyUsageBit::KeyCertSign) {
                    return Err(X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::EndEntityAssertsCertificateSigning,
                        Some(*extension_index),
                    ));
                }
            }
        }
    }

    Ok(ValidatedCertificateRoleExtensions {
        role,
        basic_constraints: basic_constraints.map(|(_, _, value)| value),
        key_usage: key_usage.map(|(_, value)| value),
        extended_key_usage: extended_key_usage.map(|(_, value)| value),
    })
}
