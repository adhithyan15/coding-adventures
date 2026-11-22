//! # Exact TLS 1.3 X25519 key-share binding
//!
//! This crate composes one decoded ServerHello key share, its exact retained
//! ClientHello offer, and one caller-owned X25519 private scalar. The scalar's
//! derived public key must match the exact offered X25519 public share before
//! the decoded server public share can reach repository-owned X25519 execution.
//!
//! This boundary does not generate, own, or wipe the caller's private scalar.
//! Its message-bound retry entry point proves that the retained offer and
//! selected share came from one exact four-message retry exchange, but it does
//! not bind that exchange to a transcript or establish TLS, HTTPS, or live-
//! provider readiness.

#![forbid(unsafe_code)]

use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_x25519::x25519_base;
use core::fmt;
use tls13_key_share::DecodedServerKeyShare;
use tls13_named_group_policy::KeyAgreementAlgorithm;
use tls13_server_hello_retry_selection::BoundRetryServerKeyShareMessages;
use tls13_x25519_key_agreement::{
    derive_x25519_shared_secret, KeyAgreementError, KeyAgreementErrorKind, X25519SharedSecret,
};

/// Stable key-share binding failure categories retaining no caller payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyShareBindingErrorKind {
    /// The decoded server share does not route to X25519.
    UnsupportedSelectedGroup,
    /// The retained offer does not contain the selected client share.
    SelectedClientShareMissing,
    /// The repository X25519 primitive rejected public-key derivation.
    ClientPublicKeyDerivationRejected,
    /// The supplied scalar does not produce the retained client public share.
    PrivateScalarDoesNotMatchOffer,
    /// Selected-group-bound shared-secret derivation failed.
    KeyAgreement(KeyAgreementErrorKind),
}

/// A redacted X25519 key-share binding failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KeyShareBindingError {
    kind: KeyShareBindingErrorKind,
}

impl KeyShareBindingError {
    const fn new(kind: KeyShareBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> KeyShareBindingErrorKind {
        self.kind
    }
}

impl fmt::Debug for KeyShareBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyShareBindingError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for KeyShareBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 X25519 key-share binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for KeyShareBindingError {}

impl From<KeyAgreementError> for KeyShareBindingError {
    fn from(error: KeyAgreementError) -> Self {
        Self::new(KeyShareBindingErrorKind::KeyAgreement(error.kind()))
    }
}

/// Bind a caller's X25519 scalar to the exact offered and selected key shares.
///
/// The decoded server share must route to X25519 before `private_scalar`, the
/// retained client public share, the server public share, or `output` are read.
/// A scalar-to-offer mismatch and every downstream failure leave `output`
/// unchanged. On success, dropping the returned view wipes the complete output
/// array through the selected-group-bound key-agreement package.
pub fn derive_bound_x25519_shared_secret<'output>(
    server_share: &DecodedServerKeyShare<'_, '_, '_>,
    private_scalar: &[u8; 32],
    output: &'output mut [u8; 32],
) -> Result<X25519SharedSecret<'output>, KeyShareBindingError> {
    let selected_group = server_share.selected_group();
    if selected_group.parameters().key_agreement() != KeyAgreementAlgorithm::X25519 {
        return Err(KeyShareBindingError::new(
            KeyShareBindingErrorKind::UnsupportedSelectedGroup,
        ));
    }

    let offered_public_key = server_share
        .offered_client_shares()
        .entries()
        .find(|entry| entry.selected_group() == selected_group)
        .ok_or_else(|| {
            KeyShareBindingError::new(KeyShareBindingErrorKind::SelectedClientShareMissing)
        })?;
    let offered_public_key: &[u8; 32] = offered_public_key
        .key_exchange()
        .try_into()
        .expect("selected X25519 client key-exchange length is exactly 32 bytes");

    let derived_public_key = x25519_base(private_scalar).map_err(|_| {
        KeyShareBindingError::new(KeyShareBindingErrorKind::ClientPublicKeyDerivationRejected)
    })?;
    if !ct_eq_fixed(&derived_public_key, offered_public_key) {
        return Err(KeyShareBindingError::new(
            KeyShareBindingErrorKind::PrivateScalarDoesNotMatchOffer,
        ));
    }

    derive_x25519_shared_secret(
        selected_group,
        private_scalar,
        server_share.key_exchange(),
        output,
    )
    .map_err(Into::into)
}

/// One X25519 secret that remains attached to exact retry-message evidence.
///
/// This value intentionally derives neither [`Copy`] nor [`Clone`]. Its
/// shared-secret field retains the existing wipe-on-drop behavior, while the
/// borrowed message proof prevents later composition from substituting a
/// detached ClientHello offer or ServerHello share.
pub struct MessageBoundRetryX25519SharedSecret<
    'messages,
    'output,
    'bound,
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
    messages: &'messages BoundRetryServerKeyShareMessages<
        'bound,
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
    shared_secret: X25519SharedSecret<'output>,
}

impl<
        'messages,
        'output,
        'bound,
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
    MessageBoundRetryX25519SharedSecret<
        'messages,
        'output,
        'bound,
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
    /// Borrow the exact CH1, HRR, CH2, and final-ServerHello evidence.
    #[must_use]
    pub const fn messages(
        &self,
    ) -> &'messages BoundRetryServerKeyShareMessages<
        'bound,
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
        self.messages
    }

    /// Borrow the selected-group-bound shared secret.
    #[must_use]
    pub const fn shared_secret(&self) -> &X25519SharedSecret<'output> {
        &self.shared_secret
    }
}

impl fmt::Debug
    for MessageBoundRetryX25519SharedSecret<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageBoundRetryX25519SharedSecret")
            .field("messages", &"[RETAINED]")
            .field("shared_secret", &"[SECRET]")
            .finish()
    }
}

/// Derive X25519 only from one exact retained retry-message proof.
///
/// The decoded ServerHello share is obtained internally from `messages`; this
/// API accepts no detached offer, share, or named-group policy. All scalar
/// matching, selected-group routing, output invariants, and secret wiping are
/// delegated to [`derive_bound_x25519_shared_secret`].
pub fn derive_message_bound_retry_x25519_shared_secret<
    'messages,
    'output,
    'bound,
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
    messages: &'messages BoundRetryServerKeyShareMessages<
        'bound,
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
    private_scalar: &[u8; 32],
    output: &'output mut [u8; 32],
) -> Result<
    MessageBoundRetryX25519SharedSecret<
        'messages,
        'output,
        'bound,
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
    KeyShareBindingError,
> {
    let shared_secret = derive_bound_x25519_shared_secret(
        messages.server_share().server_share(),
        private_scalar,
        output,
    )?;
    Ok(MessageBoundRetryX25519SharedSecret {
        messages,
        shared_secret,
    })
}
