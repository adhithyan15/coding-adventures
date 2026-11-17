//! # Selected-group-bound TLS 1.3 X25519 execution
//!
//! TLS named-group negotiation is useful only if the selected group remains
//! attached to the key-agreement implementation that consumes it. This crate
//! supplies that narrow bridge for X25519. It accepts the unforgeable
//! `SelectedNamedGroup` produced by the repository policy, refuses every
//! other route before reading secret or peer bytes, and delegates scalar
//! multiplication to the repository-owned X25519 primitive.
//!
//! The shared secret is written into caller-owned fixed storage only after all
//! validation and primitive execution succeed. The returned view borrows that
//! storage and wipes it on drop. The caller's private scalar remains borrowed
//! and caller-owned; this package neither generates nor wipes it.

#![forbid(unsafe_code)]

use coding_adventures_x25519::x25519;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt;
use tls13_named_group_policy::{KeyAgreementAlgorithm, PolicyError, SelectedNamedGroup};

/// Stable key-agreement failure categories retaining no caller payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAgreementErrorKind {
    /// The selected named group does not route to X25519.
    UnsupportedSelectedGroup,
    /// Selected-group length policy rejected peer or derived material.
    NamedGroupPolicy(PolicyError),
    /// The repository X25519 primitive rejected the peer public key.
    PeerPublicKeyRejected,
}

/// A redacted selected-group-bound key-agreement failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KeyAgreementError {
    kind: KeyAgreementErrorKind,
}

impl KeyAgreementError {
    const fn new(kind: KeyAgreementErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> KeyAgreementErrorKind {
        self.kind
    }
}

impl fmt::Debug for KeyAgreementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyAgreementError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for KeyAgreementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 key-agreement error {:?}", self.kind)
    }
}

impl std::error::Error for KeyAgreementError {}

/// One selected-group-bound shared secret in caller-owned storage.
///
/// The value is deliberately neither `Copy` nor `Clone`. Dropping it wipes the
/// exact caller-owned output array before releasing the mutable borrow.
pub struct X25519SharedSecret<'output> {
    selected_group: SelectedNamedGroup,
    output: &'output mut [u8; 32],
}

impl X25519SharedSecret<'_> {
    /// Return the exact policy-selected group used for key agreement.
    #[must_use]
    pub const fn selected_group(&self) -> SelectedNamedGroup {
        self.selected_group
    }

    /// Borrow the derived shared-secret bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.output
    }
}

impl fmt::Debug for X25519SharedSecret<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("X25519SharedSecret")
    }
}

impl Drop for X25519SharedSecret<'_> {
    fn drop(&mut self) {
        self.output.zeroize();
    }
}

/// Derive one X25519 shared secret through the exact selected named group.
///
/// Route and length checks complete before `private_scalar`,
/// `peer_key_exchange`, or `output` are read or changed. Primitive rejection
/// also leaves `output` unchanged. On success, dropping the returned view
/// wipes the complete output array.
pub fn derive_x25519_shared_secret<'output>(
    selected_group: SelectedNamedGroup,
    private_scalar: &[u8; 32],
    peer_key_exchange: &[u8],
    output: &'output mut [u8; 32],
) -> Result<X25519SharedSecret<'output>, KeyAgreementError> {
    let parameters = selected_group.parameters();
    if parameters.key_agreement() != KeyAgreementAlgorithm::X25519 {
        return Err(KeyAgreementError::new(
            KeyAgreementErrorKind::UnsupportedSelectedGroup,
        ));
    }

    selected_group
        .validate_key_exchange_length(peer_key_exchange.len())
        .map_err(|error| KeyAgreementError::new(KeyAgreementErrorKind::NamedGroupPolicy(error)))?;
    let peer_public_key: &[u8; 32] = peer_key_exchange
        .try_into()
        .expect("selected X25519 key-exchange length is exactly 32 bytes");

    let derived = Zeroizing::new(
        x25519(private_scalar, peer_public_key)
            .map_err(|_| KeyAgreementError::new(KeyAgreementErrorKind::PeerPublicKeyRejected))?,
    );
    selected_group
        .validate_shared_secret_length(derived.len())
        .map_err(|error| KeyAgreementError::new(KeyAgreementErrorKind::NamedGroupPolicy(error)))?;

    output.copy_from_slice(derived.as_ref());
    Ok(X25519SharedSecret {
        selected_group,
        output,
    })
}
