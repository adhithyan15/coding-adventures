use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_wemo_upnp_integration::{
    discover_ssdp_ipv4, WemoClient, WemoConfig, WemoLanTransport,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-wemo-upnp-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => {
            let candidates = discover_ssdp_ipv4(Duration::from_secs(3), 32)
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
        [command, setup_url] if command == "inspect" => {
            let config = WemoConfig::new(BridgeId::trusted("wemo.cli"), setup_url)
                .map_err(|error| error.to_string())?;
            let mut client = WemoClient::new(config, WemoLanTransport::default());
            let snapshot = client.inspect().map_err(|error| error.to_string())?;
            println!(
                "{}",
                json!({
                    "name": snapshot.device.friendly_name,
                    "model": snapshot.device.model_name,
                    "serial_number": snapshot.device.serial_number,
                    "firmware_version": snapshot.device.firmware_version,
                    "on": snapshot.on,
                    "light_command_supported": snapshot.device.supports_light_commands(),
                })
            );
            Ok(())
        }
        _ => Err(
            "usage: smart-home-wemo-upnp-integration discover | inspect <setup-url>".to_string(),
        ),
    }
}
