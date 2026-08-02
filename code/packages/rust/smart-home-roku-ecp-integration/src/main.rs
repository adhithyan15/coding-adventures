use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_roku_ecp_integration::{
    discover_ssdp_ipv4, RokuClient, RokuConfig, RokuLanTransport,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-roku-ecp-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => {
            let candidates = discover_ssdp_ipv4(Duration::from_secs(2), 32)
                .map_err(|error| error.to_string())?;
            println!(
                "{}",
                json!(candidates
                    .iter()
                    .map(|candidate| json!({
                        "location": candidate.location,
                        "usn": candidate.usn,
                        "server": candidate.server,
                    }))
                    .collect::<Vec<_>>())
            );
            Ok(())
        }
        [command, base_url] if command == "inspect" => {
            let config = RokuConfig::new(BridgeId::trusted("roku.cli"), base_url)
                .map_err(|error| error.to_string())?;
            let mut client = RokuClient::new(config, RokuLanTransport::default());
            let snapshot = client.inspect().map_err(|error| error.to_string())?;
            println!(
                "{}",
                json!({
                    "name": snapshot.device.name,
                    "model": snapshot.device.model,
                    "serial_number": snapshot.device.serial_number,
                    "software_version": snapshot.device.software_version,
                    "power_mode": snapshot.device.power_mode,
                    "active_app": snapshot.active_app.as_ref().map(|app| json!({
                        "id": app.id,
                        "name": app.name,
                        "version": app.version,
                    })),
                    "installed_apps": snapshot.apps.len(),
                })
            );
            Ok(())
        }
        _ => {
            Err("usage: smart-home-roku-ecp-integration discover | inspect <base-url>".to_string())
        }
    }
}
