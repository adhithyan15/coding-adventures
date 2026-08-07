//! Host-side broker for issuing opaque Chief of Staff secret leases.
//!
//! Model-facing tools receive only a [`VaultRef`]. Raw payload bytes remain in
//! the zeroizing lease manager and can only be consumed by a trusted host
//! handler.

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

/// A newly issued lease receipt safe to return across the tool boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultLeaseReceipt {
    /// Opaque handle understood only by the trusted host broker.
    pub vault_ref: VaultRef,
    /// Authoritative lease expiry in milliseconds since Unix epoch.
    pub expires_at_ms: u64,
}

/// Errors produced by the Chief vault lease boundary.
#[derive(Debug)]
pub enum VaultRuntimeError {
    /// The requested secret name is not registered with this broker.
    SecretNotFound,
    /// The supplied reference was not minted by this broker format.
    InvalidVaultRef,
    /// The underlying lease manager rejected the operation.
    Lease(LeaseError),
}

impl fmt::Display for VaultRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SecretNotFound => f.write_str("secret not found"),
            Self::InvalidVaultRef => f.write_str("invalid VaultRef"),
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

    /// Atomically resolve and revoke a lease inside a trusted host handler.
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
}
