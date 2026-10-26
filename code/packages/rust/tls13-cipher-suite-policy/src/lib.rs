//! Data-driven TLS 1.3 cipher-suite and per-key record policy.
//!
//! A TLS 1.3 cipher suite names one AEAD and one hash used by HKDF.  The suite
//! also fixes the AEAD key, IV, and authentication-tag lengths.  Keeping those
//! values in one immutable descriptor prevents later protocol layers from
//! accidentally combining the wire identifier for one suite with parameters
//! from another.
//!
//! This crate deliberately stops at policy.  It does not negotiate a suite,
//! derive traffic secrets, store key material, construct nonce state, or invoke
//! cryptography.
//!
//! ```
//! use tls13_cipher_suite_policy::{
//!     CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
//! };
//!
//! let entries = [CipherSuitePolicyEntry::new(
//!     CipherSuite::TlsAes128GcmSha256,
//!     1 << 20,
//! )?];
//! let policy = CipherSuitePolicy::new(&entries)?;
//! let selected = policy.select(0x1301)?;
//!
//! assert_eq!(selected.parameters().key_length(), 16);
//! assert_eq!(selected.parameters().iv_length(), 12);
//! assert_eq!(selected.record_limit(), 1 << 20);
//! # Ok::<(), tls13_cipher_suite_policy::PolicyError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// A conservative integer ceiling below RFC 8446 section 5.5's `2^24.5`
/// full-size-record AES-GCM limit.
pub const AES_GCM_MAX_RECORDS_PER_KEY: u64 = 1 << 24;

/// The number of cipher suites recognized by this package.
pub const SUPPORTED_CIPHER_SUITE_COUNT: usize = 3;

/// Repository-targeted TLS 1.3 AEAD algorithm identifiers.
///
/// These values are routing metadata.  Possessing one does not prove that an
/// implementation exists or is suitable for production use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AeadAlgorithm {
    /// AES-GCM with a 128-bit key.
    Aes128Gcm,
    /// AES-GCM with a 256-bit key.
    Aes256Gcm,
    /// ChaCha20 with Poly1305 authentication.
    Chacha20Poly1305,
}

/// Hash algorithms used by the TLS 1.3 HKDF key schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HkdfHash {
    /// SHA-256 with a 32-byte output.
    Sha256,
    /// SHA-384 with a 48-byte output.
    Sha384,
}

impl HkdfHash {
    /// Return the digest output length used as TLS 1.3 `Hash.length`.
    #[must_use]
    pub const fn output_length(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
        }
    }
}

/// The supported RFC 8446 TLS 1.3 cipher suites.
///
/// AES-CCM suites remain intentionally absent until separate repository-owned
/// primitives and usage limits are reviewed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CipherSuite {
    /// `TLS_AES_128_GCM_SHA256` (`0x1301`).
    TlsAes128GcmSha256,
    /// `TLS_AES_256_GCM_SHA384` (`0x1302`).
    TlsAes256GcmSha384,
    /// `TLS_CHACHA20_POLY1305_SHA256` (`0x1303`).
    TlsChacha20Poly1305Sha256,
}

impl CipherSuite {
    /// Decode one exact supported TLS 1.3 cipher-suite wire identifier.
    pub const fn from_wire_id(wire_id: u16) -> Result<Self, PolicyError> {
        match wire_id {
            0x1301 => Ok(Self::TlsAes128GcmSha256),
            0x1302 => Ok(Self::TlsAes256GcmSha384),
            0x1303 => Ok(Self::TlsChacha20Poly1305Sha256),
            _ => Err(PolicyError::UnknownCipherSuite),
        }
    }

    /// Return this suite's two-byte TLS wire identifier.
    #[must_use]
    pub const fn wire_id(self) -> u16 {
        match self {
            Self::TlsAes128GcmSha256 => 0x1301,
            Self::TlsAes256GcmSha384 => 0x1302,
            Self::TlsChacha20Poly1305Sha256 => 0x1303,
        }
    }

    /// Return the immutable standard parameters bound to this suite.
    #[must_use]
    pub const fn parameters(self) -> CipherSuiteParameters {
        match self {
            Self::TlsAes128GcmSha256 => CipherSuiteParameters::new(
                self,
                AeadAlgorithm::Aes128Gcm,
                HkdfHash::Sha256,
                16,
                AES_GCM_MAX_RECORDS_PER_KEY,
            ),
            Self::TlsAes256GcmSha384 => CipherSuiteParameters::new(
                self,
                AeadAlgorithm::Aes256Gcm,
                HkdfHash::Sha384,
                32,
                AES_GCM_MAX_RECORDS_PER_KEY,
            ),
            Self::TlsChacha20Poly1305Sha256 => CipherSuiteParameters::new(
                self,
                AeadAlgorithm::Chacha20Poly1305,
                HkdfHash::Sha256,
                32,
                u64::MAX,
            ),
        }
    }

    const fn policy_index(self) -> usize {
        match self {
            Self::TlsAes128GcmSha256 => 0,
            Self::TlsAes256GcmSha384 => 1,
            Self::TlsChacha20Poly1305Sha256 => 2,
        }
    }
}

/// Immutable RFC-bound parameters for one supported suite.
///
/// Fields are private so callers cannot associate a standard wire identifier
/// with different algorithm or length metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CipherSuiteParameters {
    suite: CipherSuite,
    aead: AeadAlgorithm,
    hkdf_hash: HkdfHash,
    key_length: u8,
    iv_length: u8,
    tag_length: u8,
    maximum_records_per_key: u64,
}

impl CipherSuiteParameters {
    const fn new(
        suite: CipherSuite,
        aead: AeadAlgorithm,
        hkdf_hash: HkdfHash,
        key_length: u8,
        maximum_records_per_key: u64,
    ) -> Self {
        Self {
            suite,
            aead,
            hkdf_hash,
            key_length,
            // Every currently supported RFC 8446 AEAD has a 96-bit nonce and
            // a 128-bit authentication tag.
            iv_length: 12,
            tag_length: 16,
            maximum_records_per_key,
        }
    }

    /// Return the suite to which every remaining field is bound.
    #[must_use]
    pub const fn suite(self) -> CipherSuite {
        self.suite
    }

    /// Return the AEAD routing identifier.
    #[must_use]
    pub const fn aead(self) -> AeadAlgorithm {
        self.aead
    }

    /// Return the HKDF hash routing identifier.
    #[must_use]
    pub const fn hkdf_hash(self) -> HkdfHash {
        self.hkdf_hash
    }

    /// Return the exact traffic-key length in bytes.
    #[must_use]
    pub const fn key_length(self) -> usize {
        self.key_length as usize
    }

    /// Return the exact static-IV and per-record nonce length in bytes.
    #[must_use]
    pub const fn iv_length(self) -> usize {
        self.iv_length as usize
    }

    /// Return the exact authentication-tag length in bytes.
    #[must_use]
    pub const fn tag_length(self) -> usize {
        self.tag_length as usize
    }

    /// Return the maximum record count accepted for one traffic key.
    #[must_use]
    pub const fn maximum_records_per_key(self) -> u64 {
        self.maximum_records_per_key
    }
}

/// Stable, redacted cipher-suite policy failures.
///
/// Errors retain neither wire identifiers nor caller-selected record limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PolicyError {
    /// The wire identifier is not one of the three supported suites.
    UnknownCipherSuite,
    /// The caller attempted to construct a policy with no suites.
    EmptyPolicy,
    /// The caller repeated a suite in one allowlist.
    DuplicateCipherSuite,
    /// A zero-record ceiling was requested.
    InvalidRecordLimit,
    /// A record ceiling exceeded the standard suite maximum.
    RecordLimitExceeded,
    /// A known suite was not present in the caller's allowlist.
    DisallowedCipherSuite,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownCipherSuite => "unknown TLS 1.3 cipher suite",
            Self::EmptyPolicy => "empty TLS 1.3 cipher-suite policy",
            Self::DuplicateCipherSuite => "duplicate TLS 1.3 cipher suite",
            Self::InvalidRecordLimit => "invalid TLS 1.3 record limit",
            Self::RecordLimitExceeded => "TLS 1.3 record limit exceeds suite maximum",
            Self::DisallowedCipherSuite => "TLS 1.3 cipher suite is not allowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyError {}

/// One validated caller authorization entry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CipherSuitePolicyEntry {
    suite: CipherSuite,
    record_limit: u64,
}

impl CipherSuitePolicyEntry {
    /// Bind an allowed suite to an explicit nonzero per-key record ceiling.
    pub fn new(suite: CipherSuite, record_limit: u64) -> Result<Self, PolicyError> {
        if record_limit == 0 {
            return Err(PolicyError::InvalidRecordLimit);
        }
        if record_limit > suite.parameters().maximum_records_per_key() {
            return Err(PolicyError::RecordLimitExceeded);
        }
        Ok(Self {
            suite,
            record_limit,
        })
    }

    /// Return the authorized suite.
    #[must_use]
    pub const fn suite(self) -> CipherSuite {
        self.suite
    }

    /// Return the caller-selected record ceiling.
    #[must_use]
    pub const fn record_limit(self) -> u64 {
        self.record_limit
    }
}

/// Allocation-free allowlist and per-suite record policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CipherSuitePolicy {
    /// Zero means disallowed because validated record limits are always nonzero.
    record_limits: [u64; SUPPORTED_CIPHER_SUITE_COUNT],
}

impl CipherSuitePolicy {
    /// Validate a non-empty, duplicate-free suite allowlist.
    pub fn new(entries: &[CipherSuitePolicyEntry]) -> Result<Self, PolicyError> {
        if entries.is_empty() {
            return Err(PolicyError::EmptyPolicy);
        }

        let mut record_limits = [0; SUPPORTED_CIPHER_SUITE_COUNT];
        for entry in entries {
            let index = entry.suite.policy_index();
            if record_limits[index] != 0 {
                return Err(PolicyError::DuplicateCipherSuite);
            }
            record_limits[index] = entry.record_limit;
        }

        Ok(Self { record_limits })
    }

    /// Select one exact wire identifier already authorized by this policy.
    pub fn select(&self, wire_id: u16) -> Result<SelectedCipherSuite, PolicyError> {
        let suite = CipherSuite::from_wire_id(wire_id)?;
        let record_limit = self.record_limits[suite.policy_index()];
        if record_limit == 0 {
            return Err(PolicyError::DisallowedCipherSuite);
        }

        Ok(SelectedCipherSuite {
            parameters: suite.parameters(),
            record_limit,
        })
    }

    /// Return whether one typed suite is in this policy.
    #[must_use]
    pub const fn allows(&self, suite: CipherSuite) -> bool {
        self.record_limits[suite.policy_index()] != 0
    }
}

/// One policy-authorized suite with all routing and length parameters bound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SelectedCipherSuite {
    parameters: CipherSuiteParameters,
    record_limit: u64,
}

impl SelectedCipherSuite {
    /// Return the immutable standard descriptor.
    #[must_use]
    pub const fn parameters(self) -> CipherSuiteParameters {
        self.parameters
    }

    /// Return the caller-selected per-key record ceiling.
    #[must_use]
    pub const fn record_limit(self) -> u64 {
        self.record_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_descriptors_bind_exact_wire_algorithms_and_lengths() {
        let cases = [
            (
                CipherSuite::TlsAes128GcmSha256,
                0x1301,
                AeadAlgorithm::Aes128Gcm,
                HkdfHash::Sha256,
                16,
                32,
                AES_GCM_MAX_RECORDS_PER_KEY,
            ),
            (
                CipherSuite::TlsAes256GcmSha384,
                0x1302,
                AeadAlgorithm::Aes256Gcm,
                HkdfHash::Sha384,
                32,
                48,
                AES_GCM_MAX_RECORDS_PER_KEY,
            ),
            (
                CipherSuite::TlsChacha20Poly1305Sha256,
                0x1303,
                AeadAlgorithm::Chacha20Poly1305,
                HkdfHash::Sha256,
                32,
                32,
                u64::MAX,
            ),
        ];

        for (suite, wire_id, aead, hash, key_length, hash_length, maximum) in cases {
            let parameters = suite.parameters();
            assert_eq!(suite.wire_id(), wire_id);
            assert_eq!(CipherSuite::from_wire_id(wire_id), Ok(suite));
            assert_eq!(parameters.suite(), suite);
            assert_eq!(parameters.aead(), aead);
            assert_eq!(parameters.hkdf_hash(), hash);
            assert_eq!(hash.output_length(), hash_length);
            assert_eq!(parameters.key_length(), key_length);
            assert_eq!(parameters.iv_length(), 12);
            assert_eq!(parameters.tag_length(), 16);
            assert_eq!(parameters.maximum_records_per_key(), maximum);
        }
    }

    #[test]
    fn unsupported_and_non_tls13_wire_ids_fail_closed() {
        for wire_id in [0x0000, 0x1300, 0x1304, 0x1305, 0x1306, u16::MAX] {
            assert_eq!(
                CipherSuite::from_wire_id(wire_id),
                Err(PolicyError::UnknownCipherSuite)
            );
        }
    }

    #[test]
    fn entries_require_nonzero_limits_within_suite_maxima() {
        assert_eq!(
            CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 0),
            Err(PolicyError::InvalidRecordLimit)
        );
        assert_eq!(
            CipherSuitePolicyEntry::new(
                CipherSuite::TlsAes256GcmSha384,
                AES_GCM_MAX_RECORDS_PER_KEY + 1,
            ),
            Err(PolicyError::RecordLimitExceeded)
        );

        let aes = CipherSuitePolicyEntry::new(
            CipherSuite::TlsAes128GcmSha256,
            AES_GCM_MAX_RECORDS_PER_KEY,
        )
        .expect("standard AES limit is valid");
        assert_eq!(aes.suite(), CipherSuite::TlsAes128GcmSha256);
        assert_eq!(aes.record_limit(), AES_GCM_MAX_RECORDS_PER_KEY);

        let chacha = CipherSuitePolicyEntry::new(CipherSuite::TlsChacha20Poly1305Sha256, u64::MAX)
            .expect("sequence-bound ChaCha limit is valid");
        assert_eq!(chacha.record_limit(), u64::MAX);
    }

    #[test]
    fn policies_are_nonempty_and_duplicate_free() {
        assert_eq!(CipherSuitePolicy::new(&[]), Err(PolicyError::EmptyPolicy));

        let first = CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 10)
            .expect("entry is valid");
        let duplicate = CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 20)
            .expect("entry is independently valid");
        assert_eq!(
            CipherSuitePolicy::new(&[first, duplicate]),
            Err(PolicyError::DuplicateCipherSuite)
        );
    }

    #[test]
    fn selection_requires_exact_policy_authorization() {
        let aes = CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 99)
            .expect("entry is valid");
        let chacha = CipherSuitePolicyEntry::new(CipherSuite::TlsChacha20Poly1305Sha256, 77)
            .expect("entry is valid");
        let policy = CipherSuitePolicy::new(&[aes, chacha]).expect("policy is valid");

        assert!(policy.allows(CipherSuite::TlsAes128GcmSha256));
        assert!(!policy.allows(CipherSuite::TlsAes256GcmSha384));
        let selected = policy.select(0x1303).expect("suite is allowed");
        assert_eq!(
            selected.parameters().suite(),
            CipherSuite::TlsChacha20Poly1305Sha256
        );
        assert_eq!(selected.record_limit(), 77);
        assert_eq!(
            policy.select(0x1302),
            Err(PolicyError::DisallowedCipherSuite)
        );
        assert_eq!(policy.select(0x1304), Err(PolicyError::UnknownCipherSuite));
    }

    #[test]
    fn errors_are_stable_and_redacted() {
        let cases = [
            (
                PolicyError::UnknownCipherSuite,
                "unknown TLS 1.3 cipher suite",
            ),
            (
                PolicyError::EmptyPolicy,
                "empty TLS 1.3 cipher-suite policy",
            ),
            (
                PolicyError::DuplicateCipherSuite,
                "duplicate TLS 1.3 cipher suite",
            ),
            (
                PolicyError::InvalidRecordLimit,
                "invalid TLS 1.3 record limit",
            ),
            (
                PolicyError::RecordLimitExceeded,
                "TLS 1.3 record limit exceeds suite maximum",
            ),
            (
                PolicyError::DisallowedCipherSuite,
                "TLS 1.3 cipher suite is not allowed",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
