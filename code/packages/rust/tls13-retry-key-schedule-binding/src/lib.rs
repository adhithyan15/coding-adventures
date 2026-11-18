//! Exact-message TLS 1.3 retry key-schedule binding.
//!
//! This crate joins a message-bound retry transcript and message-bound X25519
//! secret only when they retain the same exact four-message selection proof.
//! It accepts no detached message, suite, transcript hash, or secret input.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_key_schedule::{
    derive_ephemeral_base_secrets, BaseSecretStorage, BaseSecrets, KeyScheduleError,
};
use tls13_retry_transcript_binding::BoundRetryTranscript;
use tls13_x25519_key_share_binding::MessageBoundRetryX25519SharedSecret;

/// Stable payload-free retry schedule binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryKeyScheduleBindingError {
    /// The transcript and X25519 secret retain different message proofs.
    MessageProofMismatch,
    /// Caller-owned schedule storage retains a different selected suite.
    CipherSuiteMismatch,
    /// Repository-owned base-secret derivation failed.
    KeySchedule(KeyScheduleError),
}

impl fmt::Display for RetryKeyScheduleBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MessageProofMismatch => {
                "TLS 1.3 retry transcript and shared secret use different message proofs"
            }
            Self::CipherSuiteMismatch => {
                "TLS 1.3 retry transcript and base-secret storage use different cipher suites"
            }
            Self::KeySchedule(_) => "TLS 1.3 retry base-secret derivation failed",
        })
    }
}

impl std::error::Error for RetryKeyScheduleBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeySchedule(error) => Some(error),
            Self::MessageProofMismatch | Self::CipherSuiteMismatch => None,
        }
    }
}

/// Base secrets attached to the exact retry transcript and X25519 proof.
///
/// This value intentionally derives neither `Copy` nor `Clone`.
pub struct BoundRetryBaseSecrets<
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
> {
    shared_secret: &'secret_ref MessageBoundRetryX25519SharedSecret<
        'secret_messages,
        'output,
        'server_share_bound,
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
    >,
    transcript: &'transcript_ref BoundRetryTranscript<
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
    >,
    base_secrets: BaseSecrets<'storage>,
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
    >
    BoundRetryBaseSecrets<
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
    /// Borrow the exact message-bound X25519 secret proof.
    #[must_use]
    pub const fn shared_secret(
        &self,
    ) -> &'secret_ref MessageBoundRetryX25519SharedSecret<
        'secret_messages,
        'output,
        'server_share_bound,
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
    > {
        self.shared_secret
    }

    /// Borrow the exact message-bound retry transcript.
    #[must_use]
    pub const fn transcript(
        &self,
    ) -> &'transcript_ref BoundRetryTranscript<
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
    > {
        self.transcript
    }

    /// Borrow the base-secret view into caller-owned wipe-on-drop storage.
    #[must_use]
    pub const fn base_secrets(&self) -> &BaseSecrets<'storage> {
        &self.base_secrets
    }
}

impl fmt::Debug
    for BoundRetryBaseSecrets<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryBaseSecrets")
            .field("shared_secret", &"[SECRET]")
            .field("transcript", &"[RETAINED]")
            .field("base_secrets", &"[SECRET]")
            .finish()
    }
}

/// Bind one exact retry exchange to the repository-owned no-PSK base schedule.
///
/// Proof and suite substitution fail before schedule derivation can clear or
/// write `storage`.
pub fn bind_retry_key_schedule<
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
>(
    shared_secret: &'secret_ref MessageBoundRetryX25519SharedSecret<
        'secret_messages,
        'output,
        'server_share_bound,
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
    >,
    transcript: &'transcript_ref BoundRetryTranscript<
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
    >,
    storage: &'storage mut BaseSecretStorage,
) -> Result<
    BoundRetryBaseSecrets<
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
    >,
    RetryKeyScheduleBindingError,
> {
    let secret_messages = shared_secret.messages().messages().selection();
    if !core::ptr::eq(secret_messages, transcript.messages()) {
        return Err(RetryKeyScheduleBindingError::MessageProofMismatch);
    }

    if storage.selected_suite() != transcript.transcript().selected_suite() {
        return Err(RetryKeyScheduleBindingError::CipherSuiteMismatch);
    }

    let base_secrets =
        derive_ephemeral_base_secrets(shared_secret.shared_secret().as_bytes(), storage)
            .map_err(RetryKeyScheduleBindingError::KeySchedule)?;
    Ok(BoundRetryBaseSecrets {
        shared_secret,
        transcript,
        base_secrets,
    })
}
