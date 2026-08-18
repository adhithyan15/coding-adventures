//! Host-side broker for opaque Chief of Staff secret leases and direct delivery.
//!
//! Model-facing tools receive only a [`VaultLeaseReceipt`] containing a
//! bearer-capability [`VaultRef`] and its authoritative expiry. They never
//! receive plaintext, ciphertext, or an unwrap key. Raw payload bytes remain
//! in zeroizing storage and can only be atomically consumed by a trusted host
//! handler or moved into a replaceable [`VaultDirectDelivery`] boundary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use coding_adventures_vault_leases::{
    InMemoryLeaseManager, LeaseError, LeaseId, LeaseManager, LeasePayload,
};
use smart_home_core::VaultRef;

const VAULT_REF_PREFIX: &str = "vault-lease:";
const MAX_CONSUMER_AGENT_ID_BYTES: usize = 4 * 1024;

/// Failure reported by a trusted direct-delivery adapter.
///
/// Variants intentionally carry no free-form diagnostic text so adapters
/// cannot accidentally turn an error or log path into a secret channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultDirectDeliveryError {
    /// No trusted consumer is registered for the requested identifier.
    ConsumerNotFound,
    /// The trusted consumer refused the delivery.
    Rejected,
    /// The trusted transport is temporarily unavailable.
    Unavailable,
}

impl fmt::Display for VaultDirectDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConsumerNotFound => f.write_str("direct-delivery consumer not found"),
            Self::Rejected => f.write_str("direct delivery rejected"),
            Self::Unavailable => f.write_str("direct-delivery transport unavailable"),
        }
    }
}

impl std::error::Error for VaultDirectDeliveryError {}

/// Who asked for a direct delivery, what they asked for, and where it goes.
///
/// This descriptor exists because a delivery adapter that is told only the
/// destination cannot actually authorize anything. Given `(consumer, payload)`
/// alone, the strongest rule an adapter can express is a global destination
/// allowlist — and under that rule a caller permitted to send *one* secret to a
/// consumer is equally permitted to send *every* secret to it, because no
/// component in the chain can tell the two requests apart. That is a confused
/// deputy: the adapter holds the authority but not the facts.
///
/// Naming the requester and the secret does not by itself authorize anything.
/// It is the precondition for an adapter that wants to.
///
/// `requesting_agent_id` is only as trustworthy as whatever populated it. If a
/// host lets a caller assert its own identity, an adapter must not treat this
/// field as evidence — see the note in D18D section 7.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultDirectRequest<'a> {
    /// Identity of the agent that invoked the tool, if the host attested one.
    pub requesting_agent_id: Option<&'a str>,
    /// User on whose behalf the request was made, if any.
    ///
    /// Carried separately from the agent because one vault instance may serve
    /// several users; without it an adapter cannot tell *whose* session asked
    /// for a delivery, only which agent process did.
    pub requesting_user_id: Option<&'a str>,
    /// Session the request arrived on, if any.
    pub session_id: Option<&'a str>,
    /// Name of the secret being moved.
    pub secret_name: &'a str,
    /// Already-authorized consumer the payload is destined for.
    pub consumer_agent_id: &'a str,
}

/// Replaceable trusted boundary that accepts direct secret deliveries.
///
/// Implementations may route the owned, zeroizing payload over an authenticated
/// browser, agent, or host channel. They must not return the bytes to the
/// requesting agent or include them in diagnostics.
pub trait VaultDirectDelivery: Send + Sync {
    /// Deliver one owned payload, given the full request context.
    ///
    /// The adapter is the component entitled to accept or refuse. Refusing is a
    /// first-class outcome, not an error condition — see
    /// [`VaultDirectDeliveryError::Rejected`].
    fn deliver(
        &self,
        request: VaultDirectRequest<'_>,
        payload: LeasePayload,
    ) -> Result<(), VaultDirectDeliveryError>;
}

/// A newly issued lease receipt safe to return across the tool boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct VaultLeaseReceipt {
    /// Opaque handle understood only by the trusted host broker.
    pub vault_ref: VaultRef,
    /// Authoritative lease expiry in milliseconds since Unix epoch.
    pub expires_at_ms: u64,
}

impl fmt::Debug for VaultLeaseReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultLeaseReceipt")
            .field("vault_ref", &"<redacted>")
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

/// Errors produced by the Chief vault lease boundary.
#[derive(Debug)]
pub enum VaultRuntimeError {
    /// The requested secret name is not registered with this broker.
    SecretNotFound,
    /// The consumer identifier is empty or exceeds the protocol bound.
    InvalidConsumerAgentId,
    /// The supplied reference was not minted by this broker format.
    InvalidVaultRef,
    /// The trusted direct-delivery adapter rejected the operation.
    DirectDelivery(VaultDirectDeliveryError),
    /// The underlying lease manager rejected the operation.
    Lease(LeaseError),
}

impl fmt::Display for VaultRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SecretNotFound => f.write_str("secret not found"),
            Self::InvalidConsumerAgentId => f.write_str("invalid consumer agent identifier"),
            Self::InvalidVaultRef => f.write_str("invalid VaultRef"),
            Self::DirectDelivery(error) => write!(f, "{error}"),
            Self::Lease(error) => write!(f, "lease operation failed: {error}"),
        }
    }
}

impl std::error::Error for VaultRuntimeError {}

impl From<LeaseError> for VaultRuntimeError {
    fn from(value: LeaseError) -> Self {
        Self::Lease(value)
    }
}

impl From<VaultDirectDeliveryError> for VaultRuntimeError {
    fn from(value: VaultDirectDeliveryError) -> Self {
        Self::DirectDelivery(value)
    }
}

/// In-process vault actor boundary used by Chief host runtimes.
pub struct ChiefVaultRuntime {
    secrets: Mutex<HashMap<String, LeasePayload>>,
    leases: InMemoryLeaseManager,
}

impl Default for ChiefVaultRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ChiefVaultRuntime {
    /// Construct an empty vault runtime.
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
            leases: InMemoryLeaseManager::new(),
        }
    }

    /// Register or rotate a named secret while retaining it in zeroizing memory.
    pub fn register_secret(&self, name: impl Into<String>, payload: LeasePayload) {
        self.secrets
            .lock()
            .expect("vault secret mutex poisoned")
            .insert(name.into(), payload);
    }

    /// Issue a short-lived opaque reference for a named secret.
    ///
    /// The receipt is the canonical agent-facing `{ vault_ref,
    /// expires_at_ms }` shape. Secret bytes and decryption material never cross
    /// this boundary.
    pub fn request_lease(
        &self,
        secret_name: &str,
        ttl_ms: u64,
    ) -> Result<VaultLeaseReceipt, VaultRuntimeError> {
        let payload = self
            .secrets
            .lock()
            .expect("vault secret mutex poisoned")
            .get(secret_name)
            .cloned()
            .ok_or(VaultRuntimeError::SecretNotFound)?;
        let lease_id = self.leases.issue(payload, ttl_ms)?;
        let info = self.leases.lookup(&lease_id)?;
        Ok(VaultLeaseReceipt {
            vault_ref: VaultRef::trusted(format!("{VAULT_REF_PREFIX}{}", lease_id.as_hex())),
            expires_at_ms: info.expires_at_ms,
        })
    }

    /// Deliver a named secret to a trusted consumer without returning it.
    ///
    /// The registered secret is cloned directly into zeroizing payload storage
    /// and ownership is transferred to `delivery`. The caller receives only
    /// success or a bounded, secret-free error.
    pub fn request_direct(
        &self,
        request: VaultDirectRequest<'_>,
        delivery: &dyn VaultDirectDelivery,
    ) -> Result<(), VaultRuntimeError> {
        if request.consumer_agent_id.is_empty()
            || request.consumer_agent_id.len() > MAX_CONSUMER_AGENT_ID_BYTES
        {
            return Err(VaultRuntimeError::InvalidConsumerAgentId);
        }

        let payload = self
            .secrets
            .lock()
            .expect("vault secret mutex poisoned")
            .get(request.secret_name)
            .cloned()
            .ok_or(VaultRuntimeError::SecretNotFound)?;
        delivery.deliver(request, payload)?;
        Ok(())
    }

    /// Atomically resolve and revoke a lease inside a trusted host handler.
    ///
    /// Agent and model code must not call this boundary directly. Successful
    /// resolution returns zeroizing payload storage and makes the reference
    /// unusable for every subsequent request.
    pub fn consume(&self, vault_ref: &VaultRef) -> Result<LeasePayload, VaultRuntimeError> {
        let lease_id = lease_id(vault_ref)?;
        self.leases.consume(&lease_id).map_err(Into::into)
    }

    /// Revoke an outstanding lease before its TTL elapses.
    pub fn revoke(&self, vault_ref: &VaultRef) -> Result<(), VaultRuntimeError> {
        let lease_id = lease_id(vault_ref)?;
        self.leases.revoke(&lease_id).map_err(Into::into)
    }
}

fn lease_id(vault_ref: &VaultRef) -> Result<LeaseId, VaultRuntimeError> {
    vault_ref
        .as_str()
        .strip_prefix(VAULT_REF_PREFIX)
        .and_then(LeaseId::from_hex)
        .ok_or(VaultRuntimeError::InvalidVaultRef)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"chief-vault-runtime-secret";

    #[derive(Default)]
    struct RecordingDelivery {
        deliveries: Mutex<Vec<(String, Vec<u8>)>>,
        result: Mutex<Option<VaultDirectDeliveryError>>,
    }

    impl RecordingDelivery {
        fn rejecting(error: VaultDirectDeliveryError) -> Self {
            Self {
                deliveries: Mutex::new(Vec::new()),
                result: Mutex::new(Some(error)),
            }
        }
    }

    impl VaultDirectDelivery for RecordingDelivery {
        fn deliver(
            &self,
            request: VaultDirectRequest<'_>,
            payload: LeasePayload,
        ) -> Result<(), VaultDirectDeliveryError> {
            self.deliveries.lock().unwrap().push((
                request.consumer_agent_id.to_string(),
                payload.as_bytes().to_vec(),
            ));
            match *self.result.lock().unwrap() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    /// Build a descriptor for the common test case: an agent asking for one
    /// secret on behalf of one consumer.
    fn direct_request<'a>(
        secret_name: &'a str,
        consumer_agent_id: &'a str,
    ) -> VaultDirectRequest<'a> {
        VaultDirectRequest {
            requesting_agent_id: Some("agent:test"),
            requesting_user_id: Some("user:test"),
            session_id: Some("session:test"),
            secret_name,
            consumer_agent_id,
        }
    }

    #[test]
    fn opaque_reference_resolves_once_inside_host_boundary() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret("weather-api-key", LeasePayload::new(SECRET.to_vec()));

        let receipt = vault
            .request_lease("weather-api-key", 30_000)
            .expect("lease should be issued");
        assert!(receipt.vault_ref.as_str().starts_with(VAULT_REF_PREFIX));
        assert!(!receipt.vault_ref.as_str().contains("runtime-secret"));

        let payload = vault
            .consume(&receipt.vault_ref)
            .expect("trusted host should consume lease");
        assert_eq!(payload.as_bytes(), SECRET);
        assert!(vault.consume(&receipt.vault_ref).is_err());
    }

    #[test]
    fn lease_receipt_debug_redacts_bearer_capability() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret("weather-api-key", LeasePayload::new(SECRET.to_vec()));

        let receipt = vault
            .request_lease("weather-api-key", 30_000)
            .expect("lease should be issued");
        let debug = format!("{receipt:?}");

        assert!(debug.contains("vault_ref: \"<redacted>\""));
        assert!(debug.contains("expires_at_ms"));
        assert!(!debug.contains(receipt.vault_ref.as_str()));
    }

    #[test]
    fn revoked_and_unknown_references_fail_closed() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret("weather-api-key", LeasePayload::new(SECRET.to_vec()));
        let receipt = vault
            .request_lease("weather-api-key", 30_000)
            .expect("lease should be issued");
        vault
            .revoke(&receipt.vault_ref)
            .expect("revoke should work");

        assert!(vault.consume(&receipt.vault_ref).is_err());
        assert!(vault
            .consume(&VaultRef::trusted("raw-secret-or-random-handle"))
            .is_err());
        assert!(vault.request_lease("missing", 30_000).is_err());
    }

    #[test]
    fn direct_delivery_moves_secret_only_into_trusted_adapter() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret("browser-session", LeasePayload::new(SECRET.to_vec()));
        let delivery = RecordingDelivery::default();

        vault
            .request_direct(
                direct_request("browser-session", "browser-agent"),
                &delivery,
            )
            .expect("trusted delivery should succeed");

        let deliveries = delivery.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, "browser-agent");
        assert_eq!(deliveries[0].1, SECRET);
    }

    #[test]
    fn direct_delivery_fails_closed_before_or_at_adapter_boundary() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret("browser-session", LeasePayload::new(SECRET.to_vec()));
        let delivery = RecordingDelivery::default();

        assert!(matches!(
            vault.request_direct(direct_request("browser-session", ""), &delivery),
            Err(VaultRuntimeError::InvalidConsumerAgentId)
        ));
        assert!(matches!(
            vault.request_direct(direct_request("missing", "browser-agent"), &delivery),
            Err(VaultRuntimeError::SecretNotFound)
        ));
        assert!(delivery.deliveries.lock().unwrap().is_empty());

        let rejecting = RecordingDelivery::rejecting(VaultDirectDeliveryError::Rejected);
        let error = vault
            .request_direct(
                direct_request("browser-session", "browser-agent"),
                &rejecting,
            )
            .expect_err("adapter rejection must reach the host");
        assert!(matches!(
            error,
            VaultRuntimeError::DirectDelivery(VaultDirectDeliveryError::Rejected)
        ));
        assert!(!format!("{error:?}").contains("runtime-secret"));
        assert!(!error.to_string().contains("runtime-secret"));
    }
}
