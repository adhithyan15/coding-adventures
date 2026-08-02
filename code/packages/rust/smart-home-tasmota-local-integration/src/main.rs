use serde_json::json;
use smart_home_core::{BridgeId, VaultRef};
use smart_home_tasmota_local_integration::{
    discovery_record, scan_mdns_ipv4, TasmotaClient, TasmotaConfig, TasmotaCredentials,
    TasmotaLanTransport, DEFAULT_PORT,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-tasmota-local-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => discover(),
        [command, host] if command == "inspect" => inspect(host, DEFAULT_PORT),
        [command, host, port] if command == "inspect" => inspect(host, parse_port(port)?),
        _ => Err(
            "usage: smart-home-tasmota-local-integration discover | inspect <host> [port]"
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

fn inspect(host: &str, port: u16) -> Result<(), String> {
    let mut config = TasmotaConfig::new(
        BridgeId::trusted("tasmota.cli"),
        format!("http://{host}:{port}"),
    )
    .map_err(|error| error.to_string())?;
    let credentials = match (
        env::var("TASMOTA_USERNAME").ok(),
        env::var("TASMOTA_PASSWORD").ok(),
    ) {
        (None, None) => None,
        (Some(username), Some(password)) => {
            let credential_ref = env::var("TASMOTA_CREDENTIAL_REF")
                .unwrap_or_else(|_| "vault:tasmota/cli".to_string());
            config = config.with_credentials(
                VaultRef::new(credential_ref).map_err(|error| error.to_string())?,
            );
            Some(TasmotaCredentials::new(username, password).map_err(|error| error.to_string())?)
        }
        _ => return Err("TASMOTA_USERNAME and TASMOTA_PASSWORD must be set together".to_string()),
    };
    let mut client = TasmotaClient::new(config, credentials, TasmotaLanTransport::default())
        .map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "name": snapshot.device_name,
            "module": snapshot.module,
            "firmware_version": snapshot.firmware_version,
            "hostname": snapshot.hostname,
            "mac_address": snapshot.mac_address,
            "outputs": snapshot.outputs.iter().map(|output| json!({
                "index": output.index,
                "on": output.on,
                "is_light": output.is_light,
                "brightness": output.brightness,
                "hue": output.hue,
                "saturation": output.saturation,
                "color_temperature_mirek": output.color_temperature_mirek,
            })).collect::<Vec<_>>(),
            "sensors": snapshot.sensors.iter().map(|sensor| &sensor.name).collect::<Vec<_>>(),
        })
    );
    Ok(())
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
