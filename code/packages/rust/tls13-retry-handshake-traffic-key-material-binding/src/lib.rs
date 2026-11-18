//! Exact-message TLS 1.3 retry handshake traffic-key binding.
//!
//! This crate consumes only the existing exact-message retry handshake
//! traffic-secret proof and derives one sender's traffic key and static IV into
//! caller-owned wipe-on-drop storage.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk, HkdfPrkImportError};
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_record_protection_state::{RecordProtectionState, RecordProtectionStateError};
use tls13_retry_handshake_traffic_secrets_binding::{
    BoundRetryHandshakeTrafficSecrets, ExactRetryBaseSecrets,
};
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterial, TrafficKeyMaterialError,
    TrafficKeyMaterialStorage,
};

/// Sender whose retained handshake traffic secret supplies one key and IV.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeTrafficSender {
    /// Use the exact retained client handshake traffic secret.
    Client,
    /// Use the exact retained server handshake traffic secret.
    Server,
}

/// Stable payload-free failures for exact retry traffic-key binding.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryHandshakeTrafficKeyMaterialBindingError {
    /// Caller-owned output storage retains a different selected suite.
    CipherSuiteMismatch,
    /// The retained hash-length traffic secret could not enter fixed PRK storage.
    TrafficSecretImport(HkdfPrkImportError),
    /// Repository-owned traffic-key or static-IV derivation failed.
    KeyMaterial(TrafficKeyMaterialError),
}

impl fmt::Display for RetryHandshakeTrafficKeyMaterialBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CipherSuiteMismatch => {
                "TLS 1.3 retry traffic secret and key-material storage use different cipher suites"
            }
            Self::TrafficSecretImport(_) => "TLS 1.3 retry handshake traffic-secret import failed",
            Self::KeyMaterial(_) => "TLS 1.3 retry handshake traffic-key derivation failed",
        })
    }
}

impl std::error::Error for RetryHandshakeTrafficKeyMaterialBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TrafficSecretImport(error) => Some(error),
            Self::KeyMaterial(error) => Some(error),
            Self::CipherSuiteMismatch => None,
        }
    }
}

/// Stable payload-free failure while creating exact retry record state.
#[derive(Debug, Eq, PartialEq)]
pub struct RetryHandshakeRecordProtectionStateBindingError(RecordProtectionStateError);

impl fmt::Display for RetryHandshakeRecordProtectionStateBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TLS 1.3 retry handshake record-state composition failed")
    }
}

impl std::error::Error for RetryHandshakeRecordProtectionStateBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<RecordProtectionStateError> for RetryHandshakeRecordProtectionStateBindingError {
    fn from(error: RecordProtectionStateError) -> Self {
        Self(error)
    }
}

/// One sender's key material attached to exact retry traffic-secret evidence.
///
/// This value intentionally derives neither `Copy` nor `Clone`.
pub struct BoundRetryHandshakeTrafficKeyMaterial<
    'bound,
    'evidence,
    'secret_storage,
    'material,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    traffic_secrets:
        &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence>,
    sender: HandshakeTrafficSender,
    key_material: TrafficKeyMaterial<'material>,
}

impl<'bound, 'evidence, 'secret_storage, 'material, Evidence>
    BoundRetryHandshakeTrafficKeyMaterial<'bound, 'evidence, 'secret_storage, 'material, Evidence>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.traffic_secrets
    }

    /// Return the sender whose retained secret supplied this key material.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.sender
    }

    /// Return the exact policy-selected suite retained by every component.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.key_material.selected_suite()
    }

    /// Borrow the sender's traffic key and static IV in caller-owned storage.
    #[must_use]
    pub const fn key_material(&self) -> &TrafficKeyMaterial<'material> {
        &self.key_material
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryHandshakeTrafficKeyMaterial<'_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryHandshakeTrafficKeyMaterial")
            .field("traffic_secrets", &"[RETAINED]")
            .field("sender", &self.sender)
            .field("key_material", &"[SECRET]")
            .finish()
    }
}

/// One sender's record state attached to exact retry handshake evidence.
///
/// This value intentionally derives neither `Copy` nor `Clone`. The retained
/// record state owns the traffic-key view, while the secret bytes remain in
/// the caller-owned wipe-on-drop storage borrowed by that view.
pub struct BoundRetryHandshakeRecordProtectionState<
    'bound,
    'evidence,
    'secret_storage,
    'material,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    traffic_secrets:
        &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence>,
    sender: HandshakeTrafficSender,
    state: RecordProtectionState<'material>,
}

impl<'bound, 'evidence, 'secret_storage, 'material, Evidence>
    BoundRetryHandshakeRecordProtectionState<
        'bound,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.traffic_secrets
    }

    /// Return the sender whose retained secret supplied this record state.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.sender
    }

    /// Return the exact policy-selected suite retained by every component.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.selected_suite()
    }

    /// Borrow the exact retry-bound record-protection state.
    #[must_use]
    pub const fn record_state(&self) -> &RecordProtectionState<'material> {
        &self.state
    }

    /// Mutably borrow the exact retry-bound record-protection state.
    pub const fn record_state_mut(&mut self) -> &mut RecordProtectionState<'material> {
        &mut self.state
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryHandshakeRecordProtectionState<'_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryHandshakeRecordProtectionState")
            .field("traffic_secrets", &"[RETAINED]")
            .field("sender", &self.sender)
            .field("record_state", &"[STATE]")
            .finish()
    }
}

impl<'bound, 'evidence, 'secret_storage, 'material, Evidence>
    BoundRetryHandshakeTrafficKeyMaterial<'bound, 'evidence, 'secret_storage, 'material, Evidence>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Consume this exact retry traffic-key proof into fresh record state.
    ///
    /// The state starts at sequence zero and accepts no detached nonce policy,
    /// record ceiling, key, IV, AEAD identifier, or selected suite.
    pub fn into_record_protection_state(
        self,
    ) -> Result<
        BoundRetryHandshakeRecordProtectionState<
            'bound,
            'evidence,
            'secret_storage,
            'material,
            Evidence,
        >,
        RetryHandshakeRecordProtectionStateBindingError,
    > {
        let Self {
            traffic_secrets,
            sender,
            key_material,
        } = self;
        let state = RecordProtectionState::new(key_material)
            .map_err(RetryHandshakeRecordProtectionStateBindingError)?;

        Ok(BoundRetryHandshakeRecordProtectionState {
            traffic_secrets,
            sender,
            state,
        })
    }
}

/// Bind one exact retry handshake traffic secret to its traffic key and IV.
///
/// Suite substitution fails before `storage` can be cleared or written. The
/// traffic secret is selected only from the retained exact-message evidence;
/// no detached secret, suite, hash, output length, label, or context is
/// accepted.
pub fn bind_retry_handshake_traffic_key_material<
    'bound,
    'evidence,
    'secret_storage,
    'material,
    Evidence,
>(
    traffic_secrets: &'bound BoundRetryHandshakeTrafficSecrets<
        'evidence,
        'secret_storage,
        Evidence,
    >,
    sender: HandshakeTrafficSender,
    storage: &'material mut TrafficKeyMaterialStorage,
) -> Result<
    BoundRetryHandshakeTrafficKeyMaterial<'bound, 'evidence, 'secret_storage, 'material, Evidence>,
    RetryHandshakeTrafficKeyMaterialBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    let retained = traffic_secrets.traffic_secrets();
    if storage.selected_suite() != retained.selected_suite() {
        return Err(RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch);
    }

    let secret = match sender {
        HandshakeTrafficSender::Client => retained.client_secret(),
        HandshakeTrafficSender::Server => retained.server_secret(),
    };
    let prk = HkdfPrk::import_exact(fixed_hash(retained.selected_suite()), secret)
        .map_err(RetryHandshakeTrafficKeyMaterialBindingError::TrafficSecretImport)?;
    let key_material = derive_traffic_key_material(&prk, storage)
        .map_err(RetryHandshakeTrafficKeyMaterialBindingError::KeyMaterial)?;

    Ok(BoundRetryHandshakeTrafficKeyMaterial {
        traffic_secrets,
        sender,
        key_material,
    })
}

const fn fixed_hash(selected_suite: SelectedCipherSuite) -> FixedHkdfHash {
    match selected_suite.parameters().hkdf_hash() {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}
