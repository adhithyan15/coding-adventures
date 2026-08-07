use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_sonos_upnp_integration::{
    discover_ssdp_ipv4, SonosClient, SonosConfig, SonosLanTransport,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-sonos-upnp-integration: {error}");
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
            let config = SonosConfig::new(BridgeId::trusted("sonos.cli"), setup_url)
                .map_err(|error| error.to_string())?;
            let mut client = SonosClient::new(config, SonosLanTransport::default());
            let snapshot = client.inspect().map_err(|error| error.to_string())?;
            println!(
                "{}",
                json!({
                    "name": snapshot.device.friendly_name,
                    "model": snapshot.device.model_name,
                    "serial_number": snapshot.device.serial_number,
                    "firmware_version": snapshot.device.firmware_version,
                    "room_name": snapshot.device.room_name,
                    "transport_state": snapshot.transport_state,
                    "volume": snapshot.volume,
                    "muted": snapshot.muted,
                    "track_uri": snapshot.track_uri,
                    "track_title": snapshot.track_title,
                    "track_artist": snapshot.track_artist,
                })
            );
            Ok(())
        }
        _ => Err(
            "usage: smart-home-sonos-upnp-integration discover | inspect <setup-url>".to_string(),
        ),
    }
}
