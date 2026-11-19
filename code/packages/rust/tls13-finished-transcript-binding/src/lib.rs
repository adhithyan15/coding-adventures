//! Exact selected-transcript binding for TLS 1.3 Finished verification.
//!
//! This crate snapshots one immutable repository-owned selected transcript and
//! passes only that snapshot to one already-derived Finished key. Exact
//! selected-suite equality is checked before HMAC work, and successful evidence
//! retains the transcript, its snapshot, and the repository-owned Finished
//! verdict together.
//!
//! It does not decide which handshake encodings entered the transcript, choose
//! the correct BaseKey or sender role, sequence a handshake, authenticate a
//! certificate, protect records, perform I/O, or provide TLS, HTTPS, HTTP, or
//! OAuth authority.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_finished::{FinishedError, FinishedKey, VerifiedFinished};
use tls13_transcript::{SelectedTls13Transcript, TranscriptHash};

/// Stable, payload-free selected-transcript Finished failure categories.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TranscriptBoundFinishedErrorKind {
    /// The Finished key and transcript retain different selected suites.
    CipherSuiteMismatch,
    /// The Finished key unexpectedly rejected the transcript-hash algorithm.
    HashMismatch,
    /// The exact peer verify-data length was not Hash.length.
    PeerVerifyDataLengthMismatch,
    /// Repository-owned Finished verify-data computation failed.
    FinishedComputation,
    /// The peer verify-data did not authenticate the exact transcript snapshot.
    VerifyDataMismatch,
}

/// One redacted selected-transcript Finished verification failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscriptBoundFinishedError {
    kind: TranscriptBoundFinishedErrorKind,
}

impl TranscriptBoundFinishedError {
    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> TranscriptBoundFinishedErrorKind {
        self.kind
    }
}

impl fmt::Display for TranscriptBoundFinishedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 transcript-bound Finished error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for TranscriptBoundFinishedError {}

/// Non-copyable evidence retaining one exact transcript and Finished verdict.
pub struct VerifiedTranscriptBoundFinished<'transcript> {
    transcript: &'transcript SelectedTls13Transcript,
    snapshot: TranscriptHash,
    verification: VerifiedFinished,
}

impl<'transcript> VerifiedTranscriptBoundFinished<'transcript> {
    /// Return the exact selected transcript that was snapshot and verified.
    #[must_use]
    pub const fn transcript(&self) -> &'transcript SelectedTls13Transcript {
        self.transcript
    }

    /// Return the exact suite-bound snapshot supplied to Finished verification.
    #[must_use]
    pub const fn snapshot(&self) -> TranscriptHash {
        self.snapshot
    }

    /// Return the repository-owned Finished verification evidence.
    #[must_use]
    pub const fn verification(&self) -> &VerifiedFinished {
        &self.verification
    }
}

impl fmt::Debug for VerifiedTranscriptBoundFinished<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedTranscriptBoundFinished")
    }
}

/// Verify peer Finished data only against an internally captured snapshot of
/// the exact selected transcript.
///
/// A selected-suite mismatch fails before snapshotting or HMAC work. On
/// success the immutable transcript borrow prevents that state from advancing
/// while the retained proof is live.
pub fn verify_transcript_bound_finished<'transcript>(
    transcript: &'transcript SelectedTls13Transcript,
    finished_key: &FinishedKey<'_>,
    peer_verify_data: &[u8],
) -> Result<VerifiedTranscriptBoundFinished<'transcript>, TranscriptBoundFinishedError> {
    if transcript.selected_suite() != finished_key.selected_suite() {
        return Err(TranscriptBoundFinishedError {
            kind: TranscriptBoundFinishedErrorKind::CipherSuiteMismatch,
        });
    }

    let snapshot = transcript.snapshot();
    let verification = finished_key
        .verify_peer(snapshot.as_bytes(), peer_verify_data)
        .map_err(map_finished_error)?;

    Ok(VerifiedTranscriptBoundFinished {
        transcript,
        snapshot,
        verification,
    })
}

fn map_finished_error(error: FinishedError) -> TranscriptBoundFinishedError {
    let kind = match error {
        FinishedError::HashMismatch | FinishedError::TranscriptHashLengthMismatch => {
            TranscriptBoundFinishedErrorKind::HashMismatch
        }
        FinishedError::PeerVerifyDataLengthMismatch => {
            TranscriptBoundFinishedErrorKind::PeerVerifyDataLengthMismatch
        }
        FinishedError::VerifyDataMismatch => TranscriptBoundFinishedErrorKind::VerifyDataMismatch,
        FinishedError::OutputLengthMismatch
        | FinishedError::KeyDerivation(_)
        | FinishedError::Mac(_) => TranscriptBoundFinishedErrorKind::FinishedComputation,
    };
    TranscriptBoundFinishedError { kind }
}

#[cfg(test)]
mod tests {
    use super::{
        verify_transcript_bound_finished, TranscriptBoundFinishedErrorKind,
        VerifiedTranscriptBoundFinished,
    };
    use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
    use tls13_cipher_suite_policy::{
        CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
    };
    use tls13_finished::{derive_finished_key, FinishedKey, FinishedKeyStorage, FinishedRole};
    use tls13_transcript::{PendingTls13Transcript, SelectedTls13Transcript, TranscriptPolicy};

    fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
        let entry = CipherSuitePolicyEntry::new(suite, 1_024).unwrap();
        CipherSuitePolicy::new(&[entry])
            .unwrap()
            .select(suite.wire_id())
            .unwrap()
    }

    fn selected_transcript(suite: SelectedCipherSuite, bytes: &[u8]) -> SelectedTls13Transcript {
        let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(1_024).unwrap());
        pending.update(bytes).unwrap();
        pending.select(suite)
    }

    fn with_finished_key<Result>(
        suite: SelectedCipherSuite,
        role: FinishedRole,
        operation: impl FnOnce(FinishedKey<'_>) -> Result,
    ) -> Result {
        let (algorithm, bytes): (FixedHkdfHash, &[u8]) = match suite.parameters().hkdf_hash() {
            HkdfHash::Sha256 => (FixedHkdfHash::Sha256, &[0x11; 32]),
            HkdfHash::Sha384 => (FixedHkdfHash::Sha384, &[0x22; 48]),
        };
        let prk = HkdfPrk::import_exact(algorithm, bytes).unwrap();
        let mut storage = FinishedKeyStorage::new(suite, role);
        operation(derive_finished_key(&prk, &mut storage).unwrap())
    }

    fn valid_verify_data(key: &FinishedKey<'_>, transcript: &SelectedTls13Transcript) -> Vec<u8> {
        let mut output = vec![0; transcript.snapshot().as_bytes().len()];
        key.write_verify_data(transcript.snapshot().as_bytes(), &mut output)
            .unwrap();
        output
    }

    #[test]
    fn binds_sha256_finished_to_the_exact_transcript_snapshot() {
        let suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let transcript = selected_transcript(suite, b"ordered handshake bytes");

        with_finished_key(suite, FinishedRole::Server, |key| {
            let verify_data = valid_verify_data(&key, &transcript);
            let verified =
                verify_transcript_bound_finished(&transcript, &key, &verify_data).unwrap();

            assert!(core::ptr::eq(verified.transcript(), &transcript));
            assert_eq!(verified.snapshot(), transcript.snapshot());
            assert_eq!(verified.verification().selected_suite(), suite);
            assert_eq!(verified.verification().role(), FinishedRole::Server);
            assert_eq!(
                verified.verification().transcript_hash(),
                transcript.snapshot().as_bytes(),
            );
            assert_eq!(format!("{verified:?}"), "VerifiedTranscriptBoundFinished");
        });
    }

    #[test]
    fn binds_sha384_finished_to_the_exact_transcript_snapshot() {
        let suite = selected_suite(CipherSuite::TlsAes256GcmSha384);
        let transcript = selected_transcript(suite, b"sha384 handshake bytes");

        with_finished_key(suite, FinishedRole::Client, |key| {
            let verify_data = valid_verify_data(&key, &transcript);
            let verified =
                verify_transcript_bound_finished(&transcript, &key, &verify_data).unwrap();

            assert_eq!(verified.snapshot().algorithm(), HkdfHash::Sha384);
            assert_eq!(verified.verification().role(), FinishedRole::Client);
            assert_eq!(verified.verification().transcript_hash().len(), 48);
        });
    }

    #[test]
    fn suite_substitution_fails_before_peer_length_processing() {
        let transcript_suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let key_suite = selected_suite(CipherSuite::TlsChacha20Poly1305Sha256);
        let transcript = selected_transcript(transcript_suite, b"same hash, different suite");

        with_finished_key(key_suite, FinishedRole::Server, |key| {
            let error = verify_transcript_bound_finished(&transcript, &key, &[]).unwrap_err();
            assert_eq!(
                error.kind(),
                TranscriptBoundFinishedErrorKind::CipherSuiteMismatch,
            );
        });
    }

    #[test]
    fn peer_length_and_verify_data_failures_are_stable_and_redacted() {
        let suite = selected_suite(CipherSuite::TlsAes128GcmSha256);
        let transcript = selected_transcript(suite, b"sensitive peer transcript");

        with_finished_key(suite, FinishedRole::Server, |key| {
            let length_error =
                verify_transcript_bound_finished(&transcript, &key, &[0; 31]).unwrap_err();
            assert_eq!(
                length_error.kind(),
                TranscriptBoundFinishedErrorKind::PeerVerifyDataLengthMismatch,
            );

            let mut verify_data = valid_verify_data(&key, &transcript);
            verify_data[0] ^= 1;
            let mismatch =
                verify_transcript_bound_finished(&transcript, &key, &verify_data).unwrap_err();
            assert_eq!(
                mismatch.kind(),
                TranscriptBoundFinishedErrorKind::VerifyDataMismatch,
            );
            assert_eq!(
                mismatch.to_string(),
                "TLS 1.3 transcript-bound Finished error VerifyDataMismatch",
            );
            assert!(!format!("{mismatch:?}").contains("sensitive"));
        });
    }

    #[test]
    fn evidence_surface_is_nominal_and_non_copyable() {
        assert!(
            core::any::type_name::<VerifiedTranscriptBoundFinished<'static>>()
                .contains("VerifiedTranscriptBoundFinished")
        );
    }
}
