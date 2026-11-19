//! Exact-message TLS 1.3 retry handshake traffic-secret binding.
//!
//! This crate consumes only the existing exact-message retry base schedule,
//! snapshots its retained transcript, and derives the client and server
//! handshake traffic secrets into caller-owned wipe-on-drop storage. A second
//! composition derives the server Finished key only from that exact retained
//! server secret. A third reconstructs a fresh selected transcript only from
//! the same exact retry messages and original retained policy, then keeps that
//! state attached to the complete server Finished-key proof.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_finished::{
    derive_finished_key, FinishedError, FinishedKey, FinishedKeyStorage, FinishedRole,
};
use tls13_handshake_traffic_secrets::{
    derive_handshake_traffic_secrets, HandshakeTrafficSecretError, HandshakeTrafficSecretStorage,
    HandshakeTrafficSecrets,
};
use tls13_key_schedule::BaseSecrets;
use tls13_retry_key_schedule_binding::BoundRetryBaseSecrets;
use tls13_transcript::{
    SelectedTls13Transcript, TranscriptError, TranscriptErrorKind, TranscriptHash,
};

mod private {
    /// Prevent external implementations of the exact retry evidence trait.
    pub trait Sealed {}
}

/// Sealed access to the repository-owned exact-message retry base schedule.
///
/// This trait is implemented only for [`BoundRetryBaseSecrets`]. The private
/// supertrait prevents callers from substituting detached base secrets or a
/// detached transcript behind another implementation.
pub trait ExactRetryBaseSecrets: private::Sealed {
    /// Return the suite retained by the exact retry transcript.
    fn selected_suite(&self) -> SelectedCipherSuite;

    /// Snapshot the exact retained retry transcript through ServerHello.
    fn transcript_hash(&self) -> TranscriptHash;

    /// Borrow the base schedule already attached to that transcript.
    fn base_secrets(&self) -> &BaseSecrets<'_>;

    /// Reconstruct a fresh state only from the retained exact retry messages.
    fn reconstruct_transcript(&self) -> Result<SelectedTls13Transcript, TranscriptError>;
}

impl<
        'secret_ref,
        'transcript_ref,
        'storage,
        'secret_messages,
        'output,
        'server_share_bound,
        'transcript_bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    > private::Sealed
    for BoundRetryBaseSecrets<
        'secret_ref,
        'transcript_ref,
        'storage,
        'secret_messages,
        'output,
        'server_share_bound,
        'transcript_bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
{
}

impl<
        'secret_ref,
        'transcript_ref,
        'storage,
        'secret_messages,
        'output,
        'server_share_bound,
        'transcript_bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    > ExactRetryBaseSecrets
    for BoundRetryBaseSecrets<
        'secret_ref,
        'transcript_ref,
        'storage,
        'secret_messages,
        'output,
        'server_share_bound,
        'transcript_bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
{
    fn selected_suite(&self) -> SelectedCipherSuite {
        self.transcript().transcript().selected_suite()
    }

    fn transcript_hash(&self) -> TranscriptHash {
        self.transcript().transcript().snapshot()
    }

    fn base_secrets(&self) -> &BaseSecrets<'_> {
        BoundRetryBaseSecrets::base_secrets(self)
    }

    fn reconstruct_transcript(&self) -> Result<SelectedTls13Transcript, TranscriptError> {
        self.transcript().reconstruct_transcript()
    }
}

/// Stable payload-free failures for exact retry traffic-secret binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryHandshakeTrafficSecretBindingError {
    /// Caller-owned output storage retains a different selected suite.
    CipherSuiteMismatch,
    /// Repository-owned traffic-secret derivation failed.
    TrafficSecret(HandshakeTrafficSecretError),
}

impl fmt::Display for RetryHandshakeTrafficSecretBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CipherSuiteMismatch => {
                "TLS 1.3 retry transcript and traffic-secret storage use different cipher suites"
            }
            Self::TrafficSecret(_) => "TLS 1.3 retry handshake traffic-secret derivation failed",
        })
    }
}

impl std::error::Error for RetryHandshakeTrafficSecretBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TrafficSecret(error) => Some(error),
            Self::CipherSuiteMismatch => None,
        }
    }
}

/// Handshake traffic secrets attached to one exact-message retry base schedule.
///
/// This value intentionally derives neither `Copy` nor `Clone`.
pub struct BoundRetryHandshakeTrafficSecrets<
    'bound,
    'storage,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    evidence: &'bound Evidence,
    traffic_secrets: HandshakeTrafficSecrets<'storage>,
}

impl<'bound, 'storage, Evidence: ExactRetryBaseSecrets + ?Sized>
    BoundRetryHandshakeTrafficSecrets<'bound, 'storage, Evidence>
{
    /// Borrow the complete exact-message retry base schedule.
    #[must_use]
    pub const fn evidence(&self) -> &'bound Evidence {
        self.evidence
    }

    /// Borrow the client and server secrets in caller-owned wipe-on-drop storage.
    #[must_use]
    pub const fn traffic_secrets(&self) -> &HandshakeTrafficSecrets<'storage> {
        &self.traffic_secrets
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryHandshakeTrafficSecrets<'_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryHandshakeTrafficSecrets")
            .field("evidence", &"[RETAINED]")
            .field("traffic_secrets", &"[SECRET]")
            .finish()
    }
}

/// Bind one exact-message retry base schedule to its handshake traffic secrets.
///
/// Suite substitution fails before `storage` can be cleared or written. The
/// transcript snapshot is obtained only from the retained retry evidence; no
/// detached message, suite, hash, or base-secret input is accepted.
pub fn bind_retry_handshake_traffic_secrets<'bound, 'storage, Evidence>(
    evidence: &'bound Evidence,
    storage: &'storage mut HandshakeTrafficSecretStorage,
) -> Result<
    BoundRetryHandshakeTrafficSecrets<'bound, 'storage, Evidence>,
    RetryHandshakeTrafficSecretBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    if storage.selected_suite() != evidence.selected_suite() {
        return Err(RetryHandshakeTrafficSecretBindingError::CipherSuiteMismatch);
    }

    let transcript_hash = evidence.transcript_hash();
    let traffic_secrets =
        derive_handshake_traffic_secrets(evidence.base_secrets(), &transcript_hash, storage)
            .map_err(RetryHandshakeTrafficSecretBindingError::TrafficSecret)?;

    Ok(BoundRetryHandshakeTrafficSecrets {
        evidence,
        traffic_secrets,
    })
}

/// Stable payload-free failures from exact retry server Finished-key binding.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryServerFinishedKeyBindingError {
    /// Caller-owned Finished-key storage selected the client sender role.
    WrongEndpointRole,
    /// Finished-key storage selected a suite different from the retained proof.
    CipherSuiteMismatch,
    /// The retained exact server secret could not be imported as one PRK.
    SecretImport,
    /// Repository-owned Finished-key derivation failed.
    Finished(FinishedError),
}

impl fmt::Display for RetryServerFinishedKeyBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongEndpointRole => {
                "TLS 1.3 retry Finished-key storage selected the wrong endpoint role"
            }
            Self::CipherSuiteMismatch => {
                "TLS 1.3 retry traffic secrets and Finished-key storage use different cipher suites"
            }
            Self::SecretImport => "TLS 1.3 retry server handshake traffic secret import failed",
            Self::Finished(_) => "TLS 1.3 retry server Finished-key derivation failed",
        })
    }
}

impl std::error::Error for RetryServerFinishedKeyBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Finished(error) => Some(error),
            Self::WrongEndpointRole | Self::CipherSuiteMismatch | Self::SecretImport => None,
        }
    }
}

/// Server Finished key attached to one exact-message retry traffic-secret proof.
///
/// This value intentionally derives neither `Copy` nor `Clone` and retains the
/// complete proof that supplied the exact server handshake traffic secret.
pub struct BoundRetryServerFinishedKey<
    'bound,
    'key_storage,
    'traffic_bound,
    'traffic_storage,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    handshake_traffic_secrets:
        &'bound BoundRetryHandshakeTrafficSecrets<'traffic_bound, 'traffic_storage, Evidence>,
    finished_key: FinishedKey<'key_storage>,
}

/// Stable, payload-free failures while reconstructing exact retry transcript state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryServerTranscriptContinuationError {
    /// Repository-owned exact-message transcript reconstruction failed.
    Transcript(TranscriptErrorKind),
    /// The reconstructed transcript retained a different selected suite.
    CipherSuiteMismatch,
    /// Reconstruction did not reproduce the retained ServerHello snapshot.
    SnapshotMismatch,
}

impl fmt::Display for RetryServerTranscriptContinuationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Transcript(_) => "TLS 1.3 retry transcript reconstruction failed",
            Self::CipherSuiteMismatch => {
                "TLS 1.3 reconstructed retry transcript selected a different cipher suite"
            }
            Self::SnapshotMismatch => {
                "TLS 1.3 reconstructed retry transcript did not reproduce the retained snapshot"
            }
        })
    }
}

impl std::error::Error for RetryServerTranscriptContinuationError {}

/// Fresh retry transcript state retained beside its exact server Finished-key proof.
///
/// This value intentionally derives neither `Copy` nor `Clone`. It exposes the
/// reconstructed state only through an immutable borrow so provenance remains
/// attached until a later consuming composition owns this complete wrapper.
pub struct BoundRetryServerTranscriptContinuation<
    'continuation,
    'finished_bound,
    'key_storage,
    'traffic_bound,
    'traffic_storage,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    server_finished_key: &'continuation BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence,
    >,
    transcript: SelectedTls13Transcript,
}

impl<
        'continuation,
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    BoundRetryServerTranscriptContinuation<
        'continuation,
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence,
    >
{
    /// Borrow the complete exact retry-derived server Finished-key proof.
    #[must_use]
    pub const fn server_finished_key(
        &self,
    ) -> &BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence,
    > {
        self.server_finished_key
    }

    /// Borrow the freshly reconstructed selected transcript through ServerHello.
    #[must_use]
    pub const fn transcript(&self) -> &SelectedTls13Transcript {
        &self.transcript
    }

    /// Consume this proof into its exact retained components.
    ///
    /// The returned transcript carries no stronger authority by itself. A
    /// downstream composition must retain the returned server Finished-key
    /// proof whenever it claims this exact provenance.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        &'continuation BoundRetryServerFinishedKey<
            'finished_bound,
            'key_storage,
            'traffic_bound,
            'traffic_storage,
            Evidence,
        >,
        SelectedTls13Transcript,
    ) {
        (self.server_finished_key, self.transcript)
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryServerTranscriptContinuation<'_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerTranscriptContinuation")
            .field("server_finished_key", &"[RETAINED]")
            .field("transcript", &"[RETAINED]")
            .finish()
    }
}

/// Reconstruct the exact retry transcript beside its server Finished-key proof.
///
/// The sealed exact-retry evidence rebuilds the state from its retained
/// messages and original policy. Suite and snapshot equality are then checked
/// against the same retained proof before the continuation is exposed.
pub fn bind_retry_server_transcript_continuation<
    'continuation,
    'finished_bound,
    'key_storage,
    'traffic_bound,
    'traffic_storage,
    Evidence,
>(
    server_finished_key: &'continuation BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence,
    >,
) -> Result<
    BoundRetryServerTranscriptContinuation<
        'continuation,
        'finished_bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence,
    >,
    RetryServerTranscriptContinuationError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    let evidence = server_finished_key.handshake_traffic_secrets().evidence();
    let transcript = evidence
        .reconstruct_transcript()
        .map_err(|error| RetryServerTranscriptContinuationError::Transcript(error.kind()))?;
    if transcript.selected_suite() != evidence.selected_suite() {
        return Err(RetryServerTranscriptContinuationError::CipherSuiteMismatch);
    }
    if transcript.snapshot() != evidence.transcript_hash() {
        return Err(RetryServerTranscriptContinuationError::SnapshotMismatch);
    }

    Ok(BoundRetryServerTranscriptContinuation {
        server_finished_key,
        transcript,
    })
}

impl<
        'bound,
        'key_storage,
        'traffic_bound,
        'traffic_storage,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    BoundRetryServerFinishedKey<'bound, 'key_storage, 'traffic_bound, 'traffic_storage, Evidence>
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn handshake_traffic_secrets(
        &self,
    ) -> &'bound BoundRetryHandshakeTrafficSecrets<'traffic_bound, 'traffic_storage, Evidence> {
        self.handshake_traffic_secrets
    }

    /// Borrow the server Finished key in caller-owned wipe-on-drop storage.
    #[must_use]
    pub const fn finished_key(&self) -> &FinishedKey<'key_storage> {
        &self.finished_key
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryServerFinishedKey<'_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerFinishedKey")
            .field("handshake_traffic_secrets", &"[RETAINED]")
            .field("finished_key", &"[SECRET]")
            .finish()
    }
}

/// Derive the server Finished key only from exact retry traffic-secret proof.
///
/// Role and suite substitution fail before caller-owned Finished-key storage
/// can be cleared or written. The server handshake traffic secret is selected
/// only through the retained exact-message proof, imported under its selected
/// suite, and delegated to the repository-owned fixed-label Finished boundary.
pub fn bind_retry_server_finished_key<
    'bound,
    'key_storage,
    'traffic_bound,
    'traffic_storage,
    Evidence,
>(
    handshake_traffic_secrets: &'bound BoundRetryHandshakeTrafficSecrets<
        'traffic_bound,
        'traffic_storage,
        Evidence,
    >,
    storage: &'key_storage mut FinishedKeyStorage,
) -> Result<
    BoundRetryServerFinishedKey<'bound, 'key_storage, 'traffic_bound, 'traffic_storage, Evidence>,
    RetryServerFinishedKeyBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    if storage.role() != FinishedRole::Server {
        return Err(RetryServerFinishedKeyBindingError::WrongEndpointRole);
    }
    if storage.selected_suite() != handshake_traffic_secrets.traffic_secrets().selected_suite() {
        return Err(RetryServerFinishedKeyBindingError::CipherSuiteMismatch);
    }

    let hash = fixed_hash(storage.selected_suite().parameters().hkdf_hash());
    let server_secret = handshake_traffic_secrets.traffic_secrets().server_secret();
    let server_prk = HkdfPrk::import_exact(hash, server_secret)
        .map_err(|_| RetryServerFinishedKeyBindingError::SecretImport)?;
    let finished_key = derive_finished_key(&server_prk, storage)
        .map_err(RetryServerFinishedKeyBindingError::Finished)?;

    Ok(BoundRetryServerFinishedKey {
        handshake_traffic_secrets,
        finished_key,
    })
}

const fn fixed_hash(hash: HkdfHash) -> FixedHkdfHash {
    match hash {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}
