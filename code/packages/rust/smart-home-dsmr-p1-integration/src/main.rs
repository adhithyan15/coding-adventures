use serde_json::json;
use smart_home_core::{AgentId, BridgeId, CapabilityGrant, CapabilityGrantId, PrivilegeTier};
use smart_home_dsmr_p1_integration::{DsmrP1Config, DsmrP1StreamSupervisor, SerialPortOpener};
use smart_home_runtime::SmartHomeRuntime;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-dsmr-p1-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [command, serial_path] = arguments.as_slice() else {
        return Err("usage: smart-home-dsmr-p1-integration inspect <serial-path>".to_string());
    };
    if command != "inspect" {
        return Err("usage: smart-home-dsmr-p1-integration inspect <serial-path>".to_string());
    }
    let observed_at_ms = now_ms();
    let principal = AgentId::trusted("agent:dsmr-p1-cli");
    let mut runtime = SmartHomeRuntime::new();
    let _ = runtime.registry_mut().upsert_capability_grant(
        CapabilityGrant::for_all_smart_home(
            CapabilityGrantId::trusted("grant:dsmr-p1-cli"),
            principal.clone(),
            PrivilegeTier::LowRisk,
            "local DSMR P1 inspection",
            observed_at_ms,
        )
        .with_expiry(observed_at_ms.saturating_add(60_000)),
    );
    let config = DsmrP1Config::new(BridgeId::trusted("dsmr.cli"), serial_path)
        .map_err(|error| error.to_string())?;
    let mut supervisor = DsmrP1StreamSupervisor::new(config, SerialPortOpener, observed_at_ms);
    let installed = supervisor
        .sample_and_install_authorized(&mut runtime, principal, observed_at_ms)
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "bridge_id": installed.bridge_id.as_str(),
            "device_id": installed.device_id.as_str(),
            "entity_ids": installed.entity_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            "sequence": installed.checkpoint.cursor.sequence,
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
