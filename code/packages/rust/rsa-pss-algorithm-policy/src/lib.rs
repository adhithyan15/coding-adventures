//! # Data-driven RSA-PSS algorithm policy
//!
//! RFC 8017 RSA-PSS verification is parameterized by a message hash, an MGF1
//! hash, a salt length, and a trailer field. This crate applies a caller-owned
//! allowlist and salt-work ceiling to those already-decoded public parameters.
//!
//! It does not parse an AlgorithmIdentifier, hash, run MGF1, perform RSA
//! arithmetic, or establish a signature verdict.

#![deny(unsafe_code)]

use core::fmt;

/// RFC 8017's supported trailer field identifying the `0xbc` trailer octet.
pub const RSA_PSS_TRAILER_FIELD_BC: u64 = 1;

/// Repository-owned digest implementations available to future PSS routing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssDigestAlgorithm {
    /// SHA-256 with a 32-octet output.
    Sha256,
    /// SHA-384 with a 48-octet output.
    Sha384,
    /// SHA-512 with a 64-octet output.
    Sha512,
}

impl RsaPssDigestAlgorithm {
    /// Return the fixed output length of the selected digest.
    pub const fn output_length(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }
}

/// One caller-allowed message-hash, MGF1-hash, and salt-length combination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssAlgorithmProfile {
    message_hash: RsaPssDigestAlgorithm,
    mgf1_hash: RsaPssDigestAlgorithm,
    salt_length: usize,
}

impl RsaPssAlgorithmProfile {
    /// Construct one explicit public profile value.
    pub const fn new(
        message_hash: RsaPssDigestAlgorithm,
        mgf1_hash: RsaPssDigestAlgorithm,
        salt_length: usize,
    ) -> Self {
        Self {
            message_hash,
            mgf1_hash,
            salt_length,
        }
    }

    /// Return the selected message hash.
    pub const fn message_hash(self) -> RsaPssDigestAlgorithm {
        self.message_hash
    }

    /// Return the selected MGF1 hash.
    pub const fn mgf1_hash(self) -> RsaPssDigestAlgorithm {
        self.mgf1_hash
    }

    /// Return the selected salt length.
    pub const fn salt_length(self) -> usize {
        self.salt_length
    }
}

/// Already-decoded public parameters presented for policy validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssAlgorithmParameters {
    profile: RsaPssAlgorithmProfile,
    trailer_field: u64,
}

impl RsaPssAlgorithmParameters {
    /// Construct one candidate parameter set from decoded public values.
    pub const fn new(
        message_hash: RsaPssDigestAlgorithm,
        mgf1_hash: RsaPssDigestAlgorithm,
        salt_length: usize,
        trailer_field: u64,
    ) -> Self {
        Self {
            profile: RsaPssAlgorithmProfile::new(message_hash, mgf1_hash, salt_length),
            trailer_field,
        }
    }

    /// Return the complete candidate profile.
    pub const fn profile(self) -> RsaPssAlgorithmProfile {
        self.profile
    }

    /// Return the decoded trailer field.
    pub const fn trailer_field(self) -> u64 {
        self.trailer_field
    }
}

/// Stable policy-definition failures that retain no caller policy values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssAlgorithmPolicyErrorKind {
    /// No algorithm profile is allowed.
    EmptyAllowedProfiles,
    /// One allowed profile exceeds the policy salt-work ceiling.
    AllowedProfileSaltLengthExceedsMaximum,
    /// The allowlist repeats an identical profile.
    DuplicateAllowedProfile,
}

/// A redacted RSA-PSS algorithm-policy construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssAlgorithmPolicyError {
    kind: RsaPssAlgorithmPolicyErrorKind,
}

impl RsaPssAlgorithmPolicyError {
    const fn new(kind: RsaPssAlgorithmPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssAlgorithmPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssAlgorithmPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssAlgorithmPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssAlgorithmPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS algorithm policy error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssAlgorithmPolicyError {}

/// Validated caller-owned allowlist for RSA-PSS verification parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssAlgorithmPolicy<'policy> {
    allowed_profiles: &'policy [RsaPssAlgorithmProfile],
    maximum_salt_bytes: usize,
}

impl<'policy> RsaPssAlgorithmPolicy<'policy> {
    /// Validate an explicit profile allowlist and public salt-work ceiling.
    ///
    /// A zero ceiling is valid when every allowed profile uses an empty salt.
    pub fn new(
        allowed_profiles: &'policy [RsaPssAlgorithmProfile],
        maximum_salt_bytes: usize,
    ) -> Result<Self, RsaPssAlgorithmPolicyError> {
        if allowed_profiles.is_empty() {
            return Err(RsaPssAlgorithmPolicyError::new(
                RsaPssAlgorithmPolicyErrorKind::EmptyAllowedProfiles,
            ));
        }
        for (index, profile) in allowed_profiles.iter().copied().enumerate() {
            if profile.salt_length > maximum_salt_bytes {
                return Err(RsaPssAlgorithmPolicyError::new(
                    RsaPssAlgorithmPolicyErrorKind::AllowedProfileSaltLengthExceedsMaximum,
                ));
            }
            if allowed_profiles[..index].contains(&profile) {
                return Err(RsaPssAlgorithmPolicyError::new(
                    RsaPssAlgorithmPolicyErrorKind::DuplicateAllowedProfile,
                ));
            }
        }
        Ok(Self {
            allowed_profiles,
            maximum_salt_bytes,
        })
    }

    /// Return the complete caller-owned profile allowlist.
    pub const fn allowed_profiles(self) -> &'policy [RsaPssAlgorithmProfile] {
        self.allowed_profiles
    }

    /// Return the caller-owned salt-work ceiling.
    pub const fn maximum_salt_bytes(self) -> usize {
        self.maximum_salt_bytes
    }
}

/// Stable validation failures that retain no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssAlgorithmValidationErrorKind {
    /// Only RFC 8017's trailer field value 1 is supported.
    UnsupportedTrailerField,
    /// The candidate salt length exceeds the caller-owned work ceiling.
    SaltLengthExceedsMaximum,
    /// The exact hash, MGF1-hash, and salt combination is not allowlisted.
    ProfileNotAllowed,
}

/// A redacted RSA-PSS algorithm-parameter validation failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssAlgorithmValidationError {
    kind: RsaPssAlgorithmValidationErrorKind,
}

impl RsaPssAlgorithmValidationError {
    const fn new(kind: RsaPssAlgorithmValidationErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssAlgorithmValidationErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssAlgorithmValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssAlgorithmValidationError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssAlgorithmValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS algorithm validation error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssAlgorithmValidationError {}

/// Non-copyable evidence that one exact decoded parameter set passed policy.
pub struct ValidatedRsaPssAlgorithmProfile {
    profile: RsaPssAlgorithmProfile,
}

impl ValidatedRsaPssAlgorithmProfile {
    /// Return the exact allowlisted profile.
    pub const fn profile(&self) -> RsaPssAlgorithmProfile {
        self.profile
    }

    /// Return the selected message hash.
    pub const fn message_hash(&self) -> RsaPssDigestAlgorithm {
        self.profile.message_hash
    }

    /// Return the selected MGF1 hash.
    pub const fn mgf1_hash(&self) -> RsaPssDigestAlgorithm {
        self.profile.mgf1_hash
    }

    /// Return the selected salt length.
    pub const fn salt_length(&self) -> usize {
        self.profile.salt_length
    }
}

impl fmt::Debug for ValidatedRsaPssAlgorithmProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssAlgorithmProfile")
    }
}

/// Validate one already-decoded RSA-PSS parameter set against caller policy.
///
/// Validation order is trailer field, salt-work ceiling, then exact profile
/// membership. The result selects no implementation and proves no signature.
pub fn validate_rsa_pss_algorithm_parameters(
    parameters: RsaPssAlgorithmParameters,
    policy: RsaPssAlgorithmPolicy<'_>,
) -> Result<ValidatedRsaPssAlgorithmProfile, RsaPssAlgorithmValidationError> {
    if parameters.trailer_field != RSA_PSS_TRAILER_FIELD_BC {
        return Err(RsaPssAlgorithmValidationError::new(
            RsaPssAlgorithmValidationErrorKind::UnsupportedTrailerField,
        ));
    }
    if parameters.profile.salt_length > policy.maximum_salt_bytes {
        return Err(RsaPssAlgorithmValidationError::new(
            RsaPssAlgorithmValidationErrorKind::SaltLengthExceedsMaximum,
        ));
    }
    if !policy.allowed_profiles.contains(&parameters.profile) {
        return Err(RsaPssAlgorithmValidationError::new(
            RsaPssAlgorithmValidationErrorKind::ProfileNotAllowed,
        ));
    }
    Ok(ValidatedRsaPssAlgorithmProfile {
        profile: parameters.profile,
    })
}
