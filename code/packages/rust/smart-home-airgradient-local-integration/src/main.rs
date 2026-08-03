use serde_json::json;
use smart_home_airgradient_local_integration::{
    discovery_record, AirGradientClient, AirGradientConfig, AirGradientLanTransport, DEFAULT_PORT,
};
use smart_home_core::BridgeId;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-airgradient-local-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, serial] if command == "discover" => discover(serial),
        [command, host] if command == "inspect" => inspect(host, DEFAULT_PORT),
        [command, host, port] if command == "inspect" => inspect(host, parse_port(port)?),
        _ => Err(
            "usage: smart-home-airgradient-local-integration discover <serial> | inspect <host> [port]"
                .to_string(),
        ),
    }
}

fn discover(serial: &str) -> Result<(), String> {
    let config = AirGradientConfig::from_serial(BridgeId::trusted("airgradient.cli"), serial)
        .map_err(|error| error.to_string())?;
    let mut client = AirGradientClient::new(config.clone(), AirGradientLanTransport::default())
        .map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    let record =
        discovery_record(&config, &snapshot, now_ms()).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "id": record.native_bridge_id,
            "name": record.display_name,
            "address": record.address,
            "model": record.hardware_model,
        })
    );
    Ok(())
}

fn inspect(host: &str, port: u16) -> Result<(), String> {
    let config = AirGradientConfig::new(
        BridgeId::trusted("airgradient.cli"),
        format!("http://{host}:{port}"),
    )
    .map_err(|error| error.to_string())?;
    let mut client = AirGradientClient::new(config, AirGradientLanTransport::default())
        .map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "device": {
                "serial": snapshot.device_info.serial,
                "model": snapshot.device_info.model,
                "firmware": snapshot.device_info.firmware,
            },
            "measurements": snapshot.measurements.iter().map(|measurement| json!({
                "id": measurement.id,
                "name": measurement.name,
                "value": measurement.value,
                "unit": measurement.unit,
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
