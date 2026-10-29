//! Cipher-suite-bound fixed-memory TLS 1.3 Finished verify-data.
//!
//! RFC 8446 section 4.4.4 derives a `finished_key` from one already-derived
//! BaseKey and authenticates the current handshake transcript hash with HMAC.
//! This crate binds that operation to one policy-authorized cipher suite so a
//! detached hash choice or length cannot reinterpret the selected protocol.
//!
//! It does not own transcript construction or handshake sequencing, derive
//! the BaseKey, authenticate certificates, protect records, perform I/O, or
//! provide TLS, HTTPS, HTTP, or OAuth authority.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use coding_adventures_hmac::{hmac_sha256, hmac_sha384, verify, HmacError};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_hkdf_label::{hkdf_expand_label_fixed, HkdfExpandLabelError};

/// Largest hash output admitted by the current cipher-suite policy.
pub const MAX_FINISHED_LENGTH: usize = 48;

const FINISHED_LABEL: &[u8] = b"finished";
const EMPTY_CONTEXT: &[u8] = b"";

/// Sender role whose BaseKey and Finished transition this operation represents.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FinishedRole {
    /// The client-side Finished operation.
    Client,
    /// The server-side Finished operation.
    Server,
}

/// Stable, payload-free Finished derivation and verification failures.
#[derive(Debug, PartialEq, Eq)]
pub enum FinishedError {
    /// The BaseKey hash did not match the policy-selected cipher suite.
    HashMismatch,
    /// The caller-supplied transcript hash was not exactly Hash.length bytes.
    TranscriptHashLengthMismatch,
    /// Caller-owned generation output was not exactly Hash.length bytes.
    OutputLengthMismatch,
    /// Peer verify-data was not exactly Hash.length bytes.
    PeerVerifyDataLengthMismatch,
    /// Finished-key derivation through repository-owned HKDF failed.
    KeyDerivation(HkdfExpandLabelError),
    /// Verify-data computation through repository-owned HMAC failed.
    Mac(HmacError),
    /// Peer verify-data did not authenticate the supplied transcript hash.
    VerifyDataMismatch,
}

impl fmt::Display for FinishedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HashMismatch => formatter
                .write_str("TLS 1.3 Finished BaseKey hash does not match the selected suite"),
            Self::TranscriptHashLengthMismatch => {
                formatter.write_str("TLS 1.3 Finished transcript-hash length mismatch")
            }
            Self::OutputLengthMismatch => {
                formatter.write_str("TLS 1.3 Finished output length mismatch")
            }
            Self::PeerVerifyDataLengthMismatch => {
                formatter.write_str("TLS 1.3 peer Finished length mismatch")
            }
            Self::KeyDerivation(error) => {
                write!(formatter, "TLS 1.3 Finished key derivation failed: {error}")
            }
            Self::Mac(error) => write!(formatter, "TLS 1.3 Finished HMAC failed: {error}"),
            Self::VerifyDataMismatch => {
                formatter.write_str("TLS 1.3 peer Finished verification failed")
            }
        }
    }
}

impl std::error::Error for FinishedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyDerivation(error) => Some(error),
            Self::Mac(error) => Some(error),
            Self::HashMismatch
            | Self::TranscriptHashLengthMismatch
            | Self::OutputLengthMismatch
            | Self::PeerVerifyDataLengthMismatch
            | Self::VerifyDataMismatch => None,
        }
    }
}

/// Caller-owned storage for one selected suite's derived Finished key.
///
/// Construction stores only public suite policy and zero bytes. A successful
/// [`derive_finished_key`] call returns a borrowed [`FinishedKey`] view. The
/// storage has no key accessor, so zero or stale bytes cannot be used without
/// fresh success evidence.
pub struct FinishedKeyStorage {
    selected_suite: SelectedCipherSuite,
    role: FinishedRole,
    key: Zeroizing<[u8; MAX_FINISHED_LENGTH]>,
}

impl FinishedKeyStorage {
    /// Allocate zeroed wipe-on-drop storage for one selected suite.
    #[must_use]
    pub fn new(selected_suite: SelectedCipherSuite, role: FinishedRole) -> Self {
        Self {
            selected_suite,
            role,
            key: Zeroizing::new([0; MAX_FINISHED_LENGTH]),
        }
    }

    /// Return the public suite policy retained by this storage.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    /// Return the client/server role bound to this storage.
    #[must_use]
    pub const fn role(&self) -> FinishedRole {
        self.role
    }

    fn clear(&mut self) {
        self.key[..].zeroize();
    }
}

impl fmt::Debug for FinishedKeyStorage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FinishedKeyStorage")
            .field("suite", &self.selected_suite.parameters().suite())
            .field("role", &self.role)
            .field("key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Success-only borrowed access to one derived Finished key.
///
/// This view is non-cloneable and exposes no key bytes. The mutable borrow
/// required for another derivation cannot coexist with a live view.
pub struct FinishedKey<'storage> {
    storage: &'storage FinishedKeyStorage,
}

impl FinishedKey<'_> {
    /// Return the policy-selected suite bound to this key.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.storage.selected_suite
    }

    /// Return the client/server sender role bound to this key.
    #[must_use]
    pub const fn role(&self) -> FinishedRole {
        self.storage.role
    }

    /// Generate verify-data into exact caller-owned storage.
    ///
    /// Public lengths are checked before HMAC begins. On any error, `output`
    /// remains unchanged. The temporary HMAC result is wiped after its exact
    /// prefix is copied into caller storage.
    pub fn write_verify_data(
        &self,
        transcript_hash: &[u8],
        output: &mut [u8],
    ) -> Result<(), FinishedError> {
        self.check_transcript_hash(transcript_hash)?;
        if output.len() != self.hash_length() {
            return Err(FinishedError::OutputLengthMismatch);
        }

        let mut expected = Zeroizing::new([0; MAX_FINISHED_LENGTH]);
        self.compute_verify_data(transcript_hash, &mut expected)?;
        output.copy_from_slice(&expected[..self.hash_length()]);
        Ok(())
    }

    /// Verify peer Finished data with a compiler-preserved full scan.
    ///
    /// Both public inputs must be exactly Hash.length bytes. A success returns
    /// non-cloneable evidence retaining the selected suite and exact public
    /// transcript hash against which the peer value was verified.
    pub fn verify_peer(
        &self,
        transcript_hash: &[u8],
        peer_verify_data: &[u8],
    ) -> Result<VerifiedFinished, FinishedError> {
        self.check_transcript_hash(transcript_hash)?;
        if peer_verify_data.len() != self.hash_length() {
            return Err(FinishedError::PeerVerifyDataLengthMismatch);
        }

        let mut expected = Zeroizing::new([0; MAX_FINISHED_LENGTH]);
        self.compute_verify_data(transcript_hash, &mut expected)?;
        if !verify(&expected[..self.hash_length()], peer_verify_data) {
            return Err(FinishedError::VerifyDataMismatch);
        }

        let mut retained_hash = [0; MAX_FINISHED_LENGTH];
        retained_hash[..transcript_hash.len()].copy_from_slice(transcript_hash);
        Ok(VerifiedFinished {
            selected_suite: self.storage.selected_suite,
            role: self.storage.role,
            transcript_hash: retained_hash,
        })
    }

    const fn hash_length(&self) -> usize {
        self.storage
            .selected_suite
            .parameters()
            .hkdf_hash()
            .output_length()
    }

    fn check_transcript_hash(&self, transcript_hash: &[u8]) -> Result<(), FinishedError> {
        if transcript_hash.len() != self.hash_length() {
            return Err(FinishedError::TranscriptHashLengthMismatch);
        }
        Ok(())
    }

    fn compute_verify_data(
        &self,
        transcript_hash: &[u8],
        output: &mut [u8; MAX_FINISHED_LENGTH],
    ) -> Result<(), FinishedError> {
        match self.storage.selected_suite.parameters().hkdf_hash() {
            HkdfHash::Sha256 => {
                let result = Zeroizing::new(
                    hmac_sha256(&self.storage.key[..32], transcript_hash)
                        .map_err(FinishedError::Mac)?,
                );
                output[..32].copy_from_slice(&result[..]);
            }
            HkdfHash::Sha384 => {
                let result = Zeroizing::new(
                    hmac_sha384(&self.storage.key[..48], transcript_hash)
                        .map_err(FinishedError::Mac)?,
                );
                output[..48].copy_from_slice(&result[..]);
            }
        }
        Ok(())
    }
}

impl fmt::Debug for FinishedKey<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FinishedKey")
            .field("suite", &self.storage.selected_suite.parameters().suite())
            .field("role", &self.storage.role)
            .field(
                "hash",
                &self.storage.selected_suite.parameters().hkdf_hash(),
            )
            .field("key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Derive `HKDF-Expand-Label(BaseKey, "finished", "", Hash.length)`.
///
/// Existing storage is wiped before validation. The BaseKey algorithm must
/// match the selected suite retained by `storage`. Success returns only a
/// borrowed view, so secret key bytes never cross the ABI return slot by value.
pub fn derive_finished_key<'storage>(
    base_key: &HkdfPrk,
    storage: &'storage mut FinishedKeyStorage,
) -> Result<FinishedKey<'storage>, FinishedError> {
    storage.clear();
    let expected_hash = fixed_hash(storage.selected_suite.parameters().hkdf_hash());
    if base_key.algorithm() != expected_hash {
        return Err(FinishedError::HashMismatch);
    }

    let hash_length = expected_hash.hash_len();
    hkdf_expand_label_fixed(
        base_key,
        FINISHED_LABEL,
        EMPTY_CONTEXT,
        &mut storage.key[..hash_length],
    )
    .map_err(FinishedError::KeyDerivation)?;

    Ok(FinishedKey { storage })
}

/// Evidence that peer verify-data authenticated one exact transcript hash.
///
/// This value is intentionally non-cloneable so later handshake-state code can
/// consume one verification result for one state transition.
pub struct VerifiedFinished {
    selected_suite: SelectedCipherSuite,
    role: FinishedRole,
    transcript_hash: [u8; MAX_FINISHED_LENGTH],
}

impl VerifiedFinished {
    /// Return the policy-selected suite used by the verified operation.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    /// Return the client/server sender role authenticated by this evidence.
    #[must_use]
    pub const fn role(&self) -> FinishedRole {
        self.role
    }

    /// Borrow the exact public transcript hash that was verified.
    #[must_use]
    pub fn transcript_hash(&self) -> &[u8] {
        let length = self.selected_suite.parameters().hkdf_hash().output_length();
        &self.transcript_hash[..length]
    }
}

impl fmt::Debug for VerifiedFinished {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedFinished")
            .field("suite", &self.selected_suite.parameters().suite())
            .field("role", &self.role)
            .field(
                "transcript_hash_length",
                &self.selected_suite.parameters().hkdf_hash().output_length(),
            )
            .finish_non_exhaustive()
    }
}

const fn fixed_hash(hash: HkdfHash) -> FixedHkdfHash {
    match hash {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}
