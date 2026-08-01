use serde_json::json;
use smart_home_onvif_integration::{
    scan_ws_discovery, ws_discovery_ipv4_destination, OnvifClient, OnvifCredentials,
    OnvifLanTransport, DEFAULT_MAX_DISCOVERY_RESPONSES,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-onvif-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => {
            let report = scan_ws_discovery(
                ws_discovery_ipv4_destination(),
                Duration::from_secs(2),
                DEFAULT_MAX_DISCOVERY_RESPONSES,
            )
            .map_err(|error| error.to_string())?;
            let matches = report
                .matches
                .iter()
                .map(|matched| {
                    json!({
                        "endpoint_reference": matched.endpoint_reference,
                        "xaddrs": matched.xaddrs,
                        "scopes": matched.scopes,
                        "source": matched.source.to_string(),
                    })
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                json!({"matches": matches, "failure_count": report.failures.len()})
            );
            Ok(())
        }
        [command, endpoint] if command == "inspect" => {
            let username =
                env::var("ONVIF_USERNAME").map_err(|_| "ONVIF_USERNAME is required".to_string())?;
            let password =
                env::var("ONVIF_PASSWORD").map_err(|_| "ONVIF_PASSWORD is required".to_string())?;
            let credentials =
                OnvifCredentials::new(username, password).map_err(|error| error.to_string())?;
            let mut client = OnvifClient::new(OnvifLanTransport::default());
            let snapshot = client
                .inspect_camera(endpoint, &credentials)
                .map_err(|error| error.to_string())?;
            let profiles = snapshot
                .profiles
                .iter()
                .map(|profile| {
                    json!({
                        "token": profile.token,
                        "name": profile.name,
                        "encoding": profile.encoding,
                        "width": profile.width,
                        "height": profile.height,
                        "frame_rate_limit": profile.frame_rate_limit,
                        "snapshot_available": profile.has_snapshot_uri(),
                        "stream_available": profile.has_stream_uri(),
                    })
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                json!({
                    "manufacturer": snapshot.device_information.manufacturer,
                    "model": snapshot.device_information.model,
                    "firmware_version": snapshot.device_information.firmware_version,
                    "serial_number": snapshot.device_information.serial_number,
                    "profile_count": profiles.len(),
                    "profiles": profiles,
                })
            );
            Ok(())
        }
        _ => Err(
            "usage: smart-home-onvif-integration discover | inspect <device-service-url>"
                .to_string(),
        ),
    }
}
