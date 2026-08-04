use serde_json::json;
use smart_home_core::{BridgeId, VaultRef};
use smart_home_nanoleaf_local_integration::{
    discovery_record, scan_mdns_ipv4, NanoleafClient, NanoleafConfig, NanoleafCredentials,
    NanoleafLanTransport, NanoleafPairingClient, DEFAULT_PORT,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-nanoleaf-local-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => discover(),
        [command, host] if command == "pair" => pair(host, DEFAULT_PORT),
        [command, host, port] if command == "pair" => pair(host, parse_port(port)?),
        [command, host] if command == "inspect" => inspect(host, DEFAULT_PORT),
        [command, host, port] if command == "inspect" => inspect(host, parse_port(port)?),
        _ => Err(
            "usage: smart-home-nanoleaf-local-integration discover | pair <host> [port] | inspect <host> [port]"
                .to_string(),
        ),
    }
}

fn discover() -> Result<(), String> {
    let result =
        scan_mdns_ipv4(now_ms(), Duration::from_secs(2)).map_err(|error| error.to_string())?;
    let records = result
        .advertisements
        .iter()
        .filter_map(|advertisement| {
            discovery_record(advertisement).ok().map(|record| {
                json!({
                    "id": record.native_bridge_id,
                    "name": record.display_name,
                    "address": record.address,
                    "pairing": record.pairing_requirement.as_str(),
                })
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        json!({"devices": records, "failure_count": result.failures.len()})
    );
    Ok(())
}

fn pair(host: &str, port: u16) -> Result<(), String> {
    let config = config(host, port)?;
    let mut client = NanoleafPairingClient::new(config, NanoleafLanTransport::default())
        .map_err(|error| error.to_string())?;
    let credentials = client.pair().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({"auth_token": credentials.expose_for_storage()})
    );
    Ok(())
}

fn inspect(host: &str, port: u16) -> Result<(), String> {
    let token = env::var("NANOLEAF_AUTH_TOKEN")
        .map_err(|_| "NANOLEAF_AUTH_TOKEN must contain the paired local token".to_string())?;
    let mut client = NanoleafClient::new(
        config(host, port)?,
        NanoleafCredentials::new(token).map_err(|error| error.to_string())?,
        NanoleafLanTransport::default(),
    )
    .map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "name": snapshot.name,
            "serial_number": snapshot.serial_number,
            "manufacturer": snapshot.manufacturer,
            "model": snapshot.model,
            "firmware_version": snapshot.firmware_version,
            "on": snapshot.state.on.value,
            "brightness": snapshot.state.brightness.value,
            "hue": snapshot.state.hue.value,
            "saturation": snapshot.state.sat.value,
            "color_temperature_kelvin": snapshot.state.ct.value,
            "color_mode": snapshot.state.color_mode,
        })
    );
    Ok(())
}

fn config(host: &str, port: u16) -> Result<NanoleafConfig, String> {
    let credential_ref =
        env::var("NANOLEAF_CREDENTIAL_REF").unwrap_or_else(|_| "vault:nanoleaf/cli".to_string());
    NanoleafConfig::new(
        BridgeId::trusted("nanoleaf.cli"),
        format!("http://{host}:{port}"),
        VaultRef::new(credential_ref).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn parse_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| "port must be an unsigned 16-bit integer".to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
