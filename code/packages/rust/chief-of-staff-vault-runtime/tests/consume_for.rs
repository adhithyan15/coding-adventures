//! VLT06 P8 and P9: a lease is redeemable only by the agent it was issued to,
//! and a secret only for its provisioned destinations — and a refusal never
//! consumes the lease, so its rightful holder still has it.

use std::sync::Arc;
use std::thread;

use chief_of_staff_vault_runtime::{
    AllowedAgents, ChiefVaultRuntime, SecretPolicy, VaultDeliveryMode, VaultLeaseRequest,
    VaultRuntimeError,
};
use coding_adventures_vault_leases::LeasePayload;

const WEATHER: &str = "api.weather.gov:443";

fn policy(destinations: &[&str]) -> SecretPolicy {
    SecretPolicy {
        privilege_tier: 1,
        allowed_agents: AllowedAgents::only(["weather-host", "other-host"]),
        allowed_mode: VaultDeliveryMode::Leased,
        rotated_at_ms: 1,
        allowed_destinations: destinations.iter().map(|d| d.to_string()).collect(),
    }
}

fn vault(destinations: &[&str]) -> ChiefVaultRuntime {
    let vault = ChiefVaultRuntime::new();
    vault.register_secret(
        "weather-key",
        LeasePayload::new(b"k3y-value".to_vec()),
        policy(destinations),
    );
    vault
}

fn lease_for(vault: &ChiefVaultRuntime, agent: Option<&str>) -> smart_home_core::VaultRef {
    vault
        .request_lease(VaultLeaseRequest {
            requesting_agent_id: agent,
            secret_name: "weather-key",
            ttl_ms: 60_000,
        })
        .expect("lease")
        .vault_ref
}

#[test]
fn the_issuing_agent_redeems_for_a_provisioned_destination() {
    let vault = vault(&[WEATHER]);
    let reference = lease_for(&vault, Some("weather-host"));
    let payload = vault
        .consume_for(&reference, "weather-host", WEATHER)
        .expect("redeem");
    assert_eq!(payload.as_bytes(), b"k3y-value");
    // Single use, and the index slot is released.
    assert!(vault
        .consume_for(&reference, "weather-host", WEATHER)
        .is_err());
    assert_eq!(vault.tracked_lease_count("weather-key"), 0);
}

#[test]
fn another_agent_cannot_redeem_a_leaked_reference_and_does_not_burn_it() {
    let vault = vault(&[WEATHER]);
    let reference = lease_for(&vault, Some("weather-host"));
    assert!(matches!(
        vault.consume_for(&reference, "other-host", WEATHER),
        Err(VaultRuntimeError::AgentNotPermitted)
    ));
    // The refusal left the lease in place for its rightful holder.
    assert!(vault
        .consume_for(&reference, "weather-host", WEATHER)
        .is_ok());
}

#[test]
fn an_unprovisioned_destination_is_refused_without_consuming() {
    let vault = vault(&[WEATHER]);
    let reference = lease_for(&vault, Some("weather-host"));
    for elsewhere in [
        "evil.example:443",
        "api.weather.gov:8443",
        "API.weather.gov:443",
    ] {
        assert!(matches!(
            vault.consume_for(&reference, "weather-host", elsewhere),
            Err(VaultRuntimeError::DestinationNotPermitted)
        ));
    }
    assert!(vault
        .consume_for(&reference, "weather-host", WEATHER)
        .is_ok());
}

#[test]
fn a_secret_with_no_destinations_is_never_released_for_a_request() {
    // `SecretPolicy::unrestricted` and every version-1 record land here.
    let vault = vault(&[]);
    let reference = lease_for(&vault, Some("weather-host"));
    assert!(matches!(
        vault.consume_for(&reference, "weather-host", WEATHER),
        Err(VaultRuntimeError::DestinationNotPermitted)
    ));
    assert!(SecretPolicy::unrestricted(0)
        .allowed_destinations
        .is_empty());
}

#[test]
fn a_lease_issued_without_an_attested_agent_is_never_redeemable_this_way() {
    let vault = ChiefVaultRuntime::new();
    let mut open = policy(&[WEATHER]);
    open.allowed_agents = AllowedAgents::Any;
    vault.register_secret(
        "weather-key",
        LeasePayload::new(b"k3y-value".to_vec()),
        open,
    );
    let reference = lease_for(&vault, None);
    assert!(matches!(
        vault.consume_for(&reference, "weather-host", WEATHER),
        Err(VaultRuntimeError::AgentNotPermitted)
    ));
}

#[test]
fn malformed_and_unknown_references_are_refused() {
    let vault = vault(&[WEATHER]);
    assert!(matches!(
        vault.consume_for(
            &smart_home_core::VaultRef::trusted("nonsense"),
            "weather-host",
            WEATHER
        ),
        Err(VaultRuntimeError::InvalidVaultRef)
    ));
    let foreign = lease_for(&vault, Some("weather-host"));
    vault.revoke(&foreign).unwrap();
    assert!(vault
        .consume_for(&foreign, "weather-host", WEATHER)
        .is_err());
}

#[test]
fn rotation_revokes_leases_redeemable_through_consume_for() {
    let vault = vault(&[WEATHER]);
    let reference = lease_for(&vault, Some("weather-host"));
    vault.register_secret(
        "weather-key",
        LeasePayload::new(b"rotated".to_vec()),
        policy(&[WEATHER]),
    );
    assert!(vault
        .consume_for(&reference, "weather-host", WEATHER)
        .is_err());
}

#[test]
fn concurrent_redemptions_release_the_payload_exactly_once() {
    let vault = Arc::new(vault(&[WEATHER]));
    let reference = lease_for(&vault, Some("weather-host"));
    let winners: usize = (0..8)
        .map(|_| {
            let vault = Arc::clone(&vault);
            let reference = reference.clone();
            thread::spawn(move || {
                vault
                    .consume_for(&reference, "weather-host", WEATHER)
                    .is_ok()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|handle| usize::from(handle.join().unwrap()))
        .sum();
    assert_eq!(winners, 1);
}
