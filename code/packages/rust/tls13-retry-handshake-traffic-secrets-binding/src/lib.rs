//! Exact-message TLS 1.3 retry handshake traffic-secret binding.
//!
//! This crate consumes only the existing exact-message retry base schedule,
//! snapshots its retained transcript, and derives the client and server
//! handshake traffic secrets into caller-owned wipe-on-drop storage.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_cipher_suite_policy::SelectedCipherSuite;
use tls13_handshake_traffic_secrets::{
    derive_handshake_traffic_secrets, HandshakeTrafficSecretError, HandshakeTrafficSecretStorage,
    HandshakeTrafficSecrets,
};
use tls13_key_schedule::BaseSecrets;
use tls13_retry_key_schedule_binding::BoundRetryBaseSecrets;
use tls13_transcript::TranscriptHash;

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
