use serde_json::json;
use smart_home_core::{AgentId, BridgeId, CapabilityGrant, CapabilityGrantId, PrivilegeTier};
use smart_home_kodi_jsonrpc_integration::{
    KodiClient, KodiConfig, KodiLanTransport, KodiRuntimeIntegration,
};
use smart_home_runtime::SmartHomeRuntime;
use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-kodi-jsonrpc-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [command, endpoint] = arguments.as_slice() else {
        return Err("usage: smart-home-kodi-jsonrpc-integration inspect <ip:port>".to_string());
    };
    if command != "inspect" {
        return Err("usage: smart-home-kodi-jsonrpc-integration inspect <ip:port>".to_string());
    }
    let endpoint = endpoint
        .parse::<SocketAddr>()
        .map_err(|_| "endpoint must be an IP literal and port".to_string())?;
    let config = KodiConfig::new(BridgeId::trusted("kodi.cli"), endpoint)
        .map_err(|error| error.to_string())?;
    let client = KodiClient::new(config, KodiLanTransport);
    let mut integration = KodiRuntimeIntegration::new(client);
    let mut runtime = SmartHomeRuntime::new();
    let principal = AgentId::trusted("operator:kodi-inspection-cli");
    let now_ms = now_ms();
    let _ = runtime.registry_mut().upsert_capability_grant(
        CapabilityGrant::for_all_smart_home(
            CapabilityGrantId::trusted("grant:kodi-inspection-cli"),
            principal.clone(),
            PrivilegeTier::LowRisk,
            "explicit local CLI inspection",
            now_ms,
        )
        .with_expiry(now_ms.saturating_add(60_000)),
    );
    let installed = integration
        .inspect_and_install_authorized(&mut runtime, principal, now_ms)
        .map_err(|error| error.to_string())?;
    let entity = runtime
        .registry()
        .entity(&installed.entity_id)
        .ok_or_else(|| "installed Kodi entity is missing".to_string())?;
    println!(
        "{}",
        json!({
            "bridge_id": installed.bridge_id.as_str(),
            "device_id": installed.device_id.as_str(),
            "entity_id": installed.entity_id.as_str(),
            "name": entity.name,
            "state": entity.state.as_ref().map(|state| &state.value),
        })
    );
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
