//! # Fixed-memory TLS 1.3 transcript hashing
//!
//! TLS hashes the ClientHello before the server-selected cipher suite tells the
//! client whether the key schedule uses SHA-256 or SHA-384.  This crate keeps
//! both repository-owned hash states until selection, then consumes one
//! policy-authorized [`SelectedCipherSuite`] and retains only its bound state.
//!
//! The caller supplies exact bytes from caller-accepted handshake message
//! encodings. Parsing framing and deciding whether a message is valid at the
//! current state transition belong to later layers.
//! A caller-selected cumulative byte ceiling bounds external work before either
//! state changes.

#![forbid(unsafe_code)]

use coding_adventures_sha256::Sha256Hasher;
use coding_adventures_sha384::Sha384;
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};

/// SHA-256's FIPS 180-4 byte limit, the stricter admitted transcript bound.
pub const TLS13_MAX_TRANSCRIPT_BYTES: u64 = u64::MAX / 8;

/// Stable, payload-free transcript policy and state failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TranscriptErrorKind {
    /// A cumulative wire-byte ceiling of zero or above the hash bound was used.
    TranscriptLimitOutOfRange,
    /// New caller bytes would exceed the configured cumulative ceiling.
    TranscriptLimitExceeded,
    /// A repository hash rejected the cumulative FIPS message length.
    HashMessageTooLong,
    /// The synthetic HelloRetryRequest rewrite was requested more than once.
    HelloRetryRequestAlreadyApplied,
}

/// One redacted transcript failure.
///
/// Only the stable category is retained; no handshake bytes, digest bytes,
/// caller length, cipher suite, or hash identifier enters the error value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscriptError {
    kind: TranscriptErrorKind,
}

impl TranscriptError {
    const fn new(kind: TranscriptErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> TranscriptErrorKind {
        self.kind
    }
}

impl fmt::Display for TranscriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 transcript error {:?}", self.kind)
    }
}

impl std::error::Error for TranscriptError {}

/// Caller-owned cumulative transcript work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscriptPolicy {
    maximum_observed_bytes: u64,
}

impl TranscriptPolicy {
    /// Validate a nonzero cumulative byte ceiling.
    pub const fn new(maximum_observed_bytes: u64) -> Result<Self, TranscriptError> {
        if maximum_observed_bytes == 0 || maximum_observed_bytes > TLS13_MAX_TRANSCRIPT_BYTES {
            return Err(TranscriptError::new(
                TranscriptErrorKind::TranscriptLimitOutOfRange,
            ));
        }
        Ok(Self {
            maximum_observed_bytes,
        })
    }

    /// Return the validated cumulative wire-byte ceiling.
    #[must_use]
    pub const fn maximum_observed_bytes(self) -> u64 {
        self.maximum_observed_bytes
    }
}

/// Pre-selection transcript state retaining both supported hash algorithms.
///
/// This state is intentionally not cloneable.  Selection consumes it so one
/// parsed ClientHello cannot accidentally leave multiple independently
/// advancing protocol histories.
pub struct PendingTls13Transcript {
    policy: TranscriptPolicy,
    observed_bytes: u64,
    sha256: Sha256Hasher,
    sha384: Sha384,
}

impl PendingTls13Transcript {
    /// Construct an empty dual-hash transcript under caller policy.
    #[must_use]
    pub const fn new(policy: TranscriptPolicy) -> Self {
        Self {
            policy,
            observed_bytes: 0,
            sha256: Sha256Hasher::new(),
            sha384: Sha384::new(),
        }
    }

    /// Hash one segment of an exact caller-accepted handshake encoding.
    ///
    /// Segment boundaries do not affect the result.  The cumulative work check
    /// happens before either retained hash state changes.
    pub fn update(&mut self, bytes: &[u8]) -> Result<(), TranscriptError> {
        let next_observed = checked_observed_bytes(self.policy, self.observed_bytes, bytes.len())?;

        // The policy is capped at SHA-256's much stricter FIPS length bound, so
        // both checked updates are guaranteed to accept after the common
        // preflight.  Retain explicit error mapping rather than an infallible
        // wrapper so the boundary remains fail-closed if a hash API tightens.
        self.sha256
            .try_update(bytes)
            .map_err(|_| TranscriptError::new(TranscriptErrorKind::HashMessageTooLong))?;
        self.sha384
            .update(bytes)
            .map_err(|_| TranscriptError::new(TranscriptErrorKind::HashMessageTooLong))?;
        self.observed_bytes = next_observed;
        Ok(())
    }

    /// Consume both pending states and retain the selected suite's exact hash.
    #[must_use]
    pub fn select(self, selected_suite: SelectedCipherSuite) -> SelectedTls13Transcript {
        let algorithm = selected_suite.parameters().hkdf_hash();
        let state = match algorithm {
            HkdfHash::Sha256 => SelectedHashState::Sha256(self.sha256),
            HkdfHash::Sha384 => SelectedHashState::Sha384(self.sha384),
        };
        SelectedTls13Transcript {
            policy: self.policy,
            observed_bytes: self.observed_bytes,
            selected_suite,
            state,
            hello_retry_request_applied: false,
        }
    }

    /// Return how many caller-supplied bytes have been accepted.
    #[must_use]
    pub const fn observed_bytes(&self) -> u64 {
        self.observed_bytes
    }
}

/// One selected transcript snapshot with algorithm and length kept together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscriptHash {
    algorithm: HkdfHash,
    bytes: [u8; 48],
    length: u8,
}

impl TranscriptHash {
    fn sha256(digest: [u8; 32]) -> Self {
        let mut bytes = [0_u8; 48];
        bytes[..32].copy_from_slice(&digest);
        Self {
            algorithm: HkdfHash::Sha256,
            bytes,
            length: 32,
        }
    }

    const fn sha384(digest: [u8; 48]) -> Self {
        Self {
            algorithm: HkdfHash::Sha384,
            bytes: digest,
            length: 48,
        }
    }

    /// Return the suite-bound transcript algorithm.
    #[must_use]
    pub const fn algorithm(self) -> HkdfHash {
        self.algorithm
    }

    /// Borrow the exact 32-byte or 48-byte public transcript digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.length)]
    }
}

/// Transcript state after one cipher suite has been policy-authorized.
pub struct SelectedTls13Transcript {
    policy: TranscriptPolicy,
    observed_bytes: u64,
    selected_suite: SelectedCipherSuite,
    state: SelectedHashState,
    hello_retry_request_applied: bool,
}

impl SelectedTls13Transcript {
    /// Hash another segment of an exact caller-accepted handshake encoding.
    pub fn update(&mut self, bytes: &[u8]) -> Result<(), TranscriptError> {
        let next_observed = checked_observed_bytes(self.policy, self.observed_bytes, bytes.len())?;
        self.state.update(bytes)?;
        self.observed_bytes = next_observed;
        Ok(())
    }

    /// Return a non-destructive snapshot of the selected transcript hash.
    #[must_use]
    pub fn snapshot(&self) -> TranscriptHash {
        self.state.snapshot()
    }

    /// Replace ClientHello1 with RFC 8446's synthetic `message_hash` message.
    ///
    /// The caller must invoke this after selecting the suite carried by the
    /// HelloRetryRequest and before hashing that ServerHello representation.
    /// Replacement is assembled in fresh fixed-memory state, so a failure
    /// leaves the original transcript unchanged.
    pub fn apply_hello_retry_request(&mut self) -> Result<(), TranscriptError> {
        if self.hello_retry_request_applied {
            return Err(TranscriptError::new(
                TranscriptErrorKind::HelloRetryRequestAlreadyApplied,
            ));
        }

        let snapshot = self.snapshot();
        let mut replacement = SelectedHashState::new(snapshot.algorithm());
        let digest_length = snapshot.as_bytes().len();
        let header = [254, 0, 0, digest_length as u8];
        replacement.update(&header)?;
        replacement.update(snapshot.as_bytes())?;

        self.state = replacement;
        self.hello_retry_request_applied = true;
        Ok(())
    }

    /// Return the policy-authorized suite that selected this transcript.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    /// Return how many caller-supplied bytes have been accepted.
    ///
    /// The internally generated HelloRetryRequest rewrite is not wire input and
    /// does not change this cumulative external-work counter.
    #[must_use]
    pub const fn observed_bytes(&self) -> u64 {
        self.observed_bytes
    }

    /// Return whether the one-shot HelloRetryRequest rewrite was applied.
    #[must_use]
    pub const fn hello_retry_request_applied(&self) -> bool {
        self.hello_retry_request_applied
    }
}

enum SelectedHashState {
    Sha256(Sha256Hasher),
    Sha384(Sha384),
}

impl SelectedHashState {
    const fn new(algorithm: HkdfHash) -> Self {
        match algorithm {
            HkdfHash::Sha256 => Self::Sha256(Sha256Hasher::new()),
            HkdfHash::Sha384 => Self::Sha384(Sha384::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) -> Result<(), TranscriptError> {
        match self {
            Self::Sha256(hasher) => hasher
                .try_update(bytes)
                .map_err(|_| TranscriptError::new(TranscriptErrorKind::HashMessageTooLong)),
            Self::Sha384(hasher) => hasher
                .update(bytes)
                .map_err(|_| TranscriptError::new(TranscriptErrorKind::HashMessageTooLong)),
        }
    }

    fn snapshot(&self) -> TranscriptHash {
        match self {
            Self::Sha256(hasher) => TranscriptHash::sha256(hasher.digest()),
            Self::Sha384(hasher) => TranscriptHash::sha384(hasher.digest()),
        }
    }
}

fn checked_observed_bytes(
    policy: TranscriptPolicy,
    observed_bytes: u64,
    additional_bytes: usize,
) -> Result<u64, TranscriptError> {
    let additional_bytes = u64::try_from(additional_bytes)
        .map_err(|_| TranscriptError::new(TranscriptErrorKind::TranscriptLimitExceeded))?;
    observed_bytes
        .checked_add(additional_bytes)
        .filter(|next| *next <= policy.maximum_observed_bytes())
        .ok_or_else(|| TranscriptError::new(TranscriptErrorKind::TranscriptLimitExceeded))
}
