//! # Exact TLS 1.3 X25519 key-share binding
//!
//! This crate composes one decoded ServerHello key share, its exact retained
//! ClientHello offer, and one caller-owned X25519 private scalar. The scalar's
//! derived public key must match the exact offered X25519 public share before
//! the decoded server public share can reach repository-owned X25519 execution.
//!
//! This boundary does not generate, own, or wipe the caller's private scalar.
//! It does not prove that the retained offer was transmitted or transcript-
//! bound, and it does not establish TLS, HTTPS, or live-provider readiness.

#![forbid(unsafe_code)]

use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_x25519::x25519_base;
use core::fmt;
use tls13_key_share::DecodedServerKeyShare;
use tls13_named_group_policy::KeyAgreementAlgorithm;
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
