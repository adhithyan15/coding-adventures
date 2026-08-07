use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_fronius_local_integration::{
    discovery_record, scan_mdns_ipv4, FroniusClient, FroniusConfig, FroniusLanTransport,
    DEFAULT_PORT,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-fronius-local-integration: {error}");
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
            "usage: smart-home-fronius-local-integration discover | inspect <host> [port]"
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
    let config = FroniusConfig::new(
        BridgeId::trusted("fronius.cli"),
        format!("http://{host}:{port}"),
    )
    .map_err(|error| error.to_string())?;
    let mut client = FroniusClient::new(config, FroniusLanTransport::default())
        .map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "timestamp": snapshot.timestamp,
            "measurements": snapshot.measurements.iter().map(|measurement| json!({
                "id": measurement.id,
                "name": measurement.name,
                "value": measurement.value,
                "unit": measurement.unit,
                "scope": measurement.scope,
            })).collect::<Vec<_>>(),
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
