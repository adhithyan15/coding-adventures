//! Suite-bound fixed-memory TLS 1.3 handshake traffic-secret derivation.
//!
//! This crate derives only the client and server handshake traffic secrets
//! from an existing base-secret schedule and transcript-hash snapshot. It does
//! not decide which messages produced that snapshot, sequence a handshake,
//! derive traffic keys, authenticate Finished, protect records, or provide TLS
//! or HTTPS.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_hkdf_label::derive_secret_from_transcript_hash_fixed;
use tls13_key_schedule::BaseSecrets;
use tls13_transcript::TranscriptHash;

/// Largest hash output admitted by the selected TLS 1.3 cipher suites.
pub const MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH: usize = 48;

const CLIENT_HANDSHAKE_TRAFFIC_LABEL: &[u8] = b"c hs traffic";
const SERVER_HANDSHAKE_TRAFFIC_LABEL: &[u8] = b"s hs traffic";

/// Stable payload-free failures for handshake traffic-secret derivation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeTrafficSecretError {
    /// Caller storage and base secrets retain different selected suites.
    CipherSuiteMismatch,
    /// The transcript snapshot uses a hash other than the selected suite's.
    TranscriptHashMismatch,
    /// A future suite descriptor exceeded this package's fixed storage.
    UnsupportedParameters,
    /// A repository-owned HKDF or Derive-Secret invariant failed.
    DerivationFailed,
}

impl fmt::Display for HandshakeTrafficSecretError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CipherSuiteMismatch => "TLS 1.3 cipher suite does not match caller storage",
            Self::TranscriptHashMismatch => {
                "TLS 1.3 transcript hash does not match the selected cipher suite"
            }
            Self::UnsupportedParameters => "TLS 1.3 cipher-suite parameters exceed reviewed bounds",
            Self::DerivationFailed => "TLS 1.3 handshake traffic-secret derivation failed",
        })
    }
}

impl std::error::Error for HandshakeTrafficSecretError {}

/// Non-cloneable caller-owned storage for one suite's handshake traffic secrets.
///
/// Construction stores only public suite policy and zero bytes. Successful
/// derivation returns a borrowed [`HandshakeTrafficSecrets`] view; without
/// that success evidence this owner exposes no secret accessors. All backing
/// storage is wiped on drop.
pub struct HandshakeTrafficSecretStorage {
    selected_suite: SelectedCipherSuite,
    client_secret: Zeroizing<[u8; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]>,
    server_secret: Zeroizing<[u8; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]>,
}

impl HandshakeTrafficSecretStorage {
    /// Allocate zeroed storage bound to one policy-selected suite.
    pub fn new(selected_suite: SelectedCipherSuite) -> Result<Self, HandshakeTrafficSecretError> {
        if selected_suite.parameters().hkdf_hash().output_length()
            > MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH
        {
            return Err(HandshakeTrafficSecretError::UnsupportedParameters);
        }
        Ok(Self {
            selected_suite,
            client_secret: Zeroizing::new([0; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]),
            server_secret: Zeroizing::new([0; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]),
        })
    }

    /// Return the public selected suite retained by this storage.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    fn clear(&mut self) {
        self.client_secret[..].zeroize();
        self.server_secret[..].zeroize();
    }
}

impl fmt::Debug for HandshakeTrafficSecretStorage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandshakeTrafficSecretStorage")
            .field("suite", &self.selected_suite.parameters().suite())
            .field("handshake_traffic_secrets", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Success-only borrowed access to one suite-bound pair of traffic secrets.
pub struct HandshakeTrafficSecrets<'storage> {
    storage: &'storage HandshakeTrafficSecretStorage,
}

impl HandshakeTrafficSecrets<'_> {
    /// Return the policy-selected suite retained by this result.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.storage.selected_suite
    }

    /// Borrow the exact Hash-length client handshake traffic secret.
    #[must_use]
    pub fn client_secret(&self) -> &[u8] {
        &self.storage.client_secret[..self.hash_len()]
    }

    /// Borrow the exact Hash-length server handshake traffic secret.
    #[must_use]
    pub fn server_secret(&self) -> &[u8] {
        &self.storage.server_secret[..self.hash_len()]
    }

    fn hash_len(&self) -> usize {
        self.selected_suite()
            .parameters()
            .hkdf_hash()
            .output_length()
    }
}

impl fmt::Debug for HandshakeTrafficSecrets<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandshakeTrafficSecrets")
            .field("suite", &self.selected_suite().parameters().suite())
            .field("hash", &self.selected_suite().parameters().hkdf_hash())
            .field("secret_length", &self.hash_len())
            .finish_non_exhaustive()
    }
}

/// Derive the client and server TLS 1.3 handshake traffic secrets.
///
/// The caller must supply a transcript snapshot over exactly the accepted
/// ClientHello through ServerHello handshake encodings for the current path.
/// This primitive checks only the snapshot algorithm; exact-message ownership
/// and state sequencing remain caller responsibilities.
///
/// Existing storage is cleared before validation. Both outputs are staged in
/// wipe-on-drop owners and copied into caller storage only after the complete
/// pair succeeds, then a borrowed success view is returned.
pub fn derive_handshake_traffic_secrets<'storage>(
    base_secrets: &BaseSecrets<'_>,
    transcript_hash: &TranscriptHash,
    storage: &'storage mut HandshakeTrafficSecretStorage,
) -> Result<HandshakeTrafficSecrets<'storage>, HandshakeTrafficSecretError> {
    storage.clear();

    if storage.selected_suite != base_secrets.selected_suite() {
        return Err(HandshakeTrafficSecretError::CipherSuiteMismatch);
    }

    let hash = storage.selected_suite.parameters().hkdf_hash();
    if transcript_hash.algorithm() != hash {
        return Err(HandshakeTrafficSecretError::TranscriptHashMismatch);
    }

    let fixed_hash = fixed_hash(hash);
    let hash_len = fixed_hash.hash_len();
    if hash_len > MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH {
        return Err(HandshakeTrafficSecretError::UnsupportedParameters);
    }

    let handshake_prk = HkdfPrk::import_exact(fixed_hash, base_secrets.handshake_secret())
        .map_err(|_| HandshakeTrafficSecretError::DerivationFailed)?;
    let mut client_secret = Zeroizing::new([0u8; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]);
    let mut server_secret = Zeroizing::new([0u8; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]);

    derive_secret_from_transcript_hash_fixed(
        &handshake_prk,
        CLIENT_HANDSHAKE_TRAFFIC_LABEL,
        transcript_hash.as_bytes(),
        &mut client_secret[..hash_len],
    )
    .map_err(|_| HandshakeTrafficSecretError::DerivationFailed)?;
    derive_secret_from_transcript_hash_fixed(
        &handshake_prk,
        SERVER_HANDSHAKE_TRAFFIC_LABEL,
        transcript_hash.as_bytes(),
        &mut server_secret[..hash_len],
    )
    .map_err(|_| HandshakeTrafficSecretError::DerivationFailed)?;

    storage.client_secret[..hash_len].copy_from_slice(&client_secret[..hash_len]);
    storage.server_secret[..hash_len].copy_from_slice(&server_secret[..hash_len]);

    Ok(HandshakeTrafficSecrets { storage })
}

const fn fixed_hash(hash: HkdfHash) -> FixedHkdfHash {
    match hash {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
    use tls13_key_schedule::{derive_ephemeral_base_secrets, BaseSecretStorage};
    use tls13_transcript::{PendingTls13Transcript, TranscriptPolicy};

    fn selected(suite: CipherSuite) -> SelectedCipherSuite {
        let entry = CipherSuitePolicyEntry::new(suite, 4096).unwrap();
        CipherSuitePolicy::new(&[entry])
            .unwrap()
            .select(suite.wire_id())
            .unwrap()
    }

    #[test]
    fn validation_failure_clears_prior_storage() {
        let base_suite = selected(CipherSuite::TlsAes128GcmSha256);
        let output_suite = selected(CipherSuite::TlsChacha20Poly1305Sha256);
        let mut base_storage = BaseSecretStorage::new(base_suite).unwrap();
        let base_secrets = derive_ephemeral_base_secrets(&[0x44; 32], &mut base_storage).unwrap();
        let pending = PendingTls13Transcript::new(TranscriptPolicy::new(16).unwrap());
        let snapshot = pending.select(base_suite).snapshot();
        let mut output = HandshakeTrafficSecretStorage::new(output_suite).unwrap();
        output.client_secret[..].fill(0x55);
        output.server_secret[..].fill(0xaa);

        assert_eq!(
            derive_handshake_traffic_secrets(&base_secrets, &snapshot, &mut output).unwrap_err(),
            HandshakeTrafficSecretError::CipherSuiteMismatch
        );
        assert_eq!(
            &output.client_secret[..],
            &[0; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]
        );
        assert_eq!(
            &output.server_secret[..],
            &[0; MAX_HANDSHAKE_TRAFFIC_SECRET_LENGTH]
        );
    }
}
