use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_lifx_lan_integration::{
    discovery_record, scan_lan, LifxClient, LifxDeviceConfig, LifxLanTransport, LifxScanConfig,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-lifx-lan-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => discover(),
        [command, host, serial] if command == "inspect" => inspect(host, serial, None),
        [command, host, serial, port] if command == "inspect" => inspect(host, serial, Some(port)),
        _ => Err(
            "usage: smart-home-lifx-lan-integration discover | inspect <host> <serial> [port]"
                .to_string(),
        ),
    }
}

fn discover() -> Result<(), String> {
    let result = scan_lan(&LifxScanConfig::default(), &mut LifxLanTransport)
        .map_err(|error| error.to_string())?;
    let devices = result
        .devices
        .iter()
        .map(|device| {
            discovery_record(device, now_ms())
                .map(|record| {
                    json!({
                        "id": record.native_bridge_id,
                        "address": record.address,
                    })
                })
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        json!({"devices": devices, "failure_count": result.failures.len()})
    );
    Ok(())
}

fn inspect(host: &str, serial: &str, port: Option<&String>) -> Result<(), String> {
    let mut config = LifxDeviceConfig::new(BridgeId::trusted("lifx.cli"), host, serial)
        .map_err(|error| error.to_string())?;
    if let Some(port) = port {
        config = config.with_port(
            port.parse::<u16>()
                .map_err(|_| "port must be an unsigned 16-bit integer".to_string())?,
        );
    }
    let mut client = LifxClient::new(config, LifxLanTransport);
    let status = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "on": status.on,
            "label": status.label,
            "hue": status.color.hue,
            "saturation": status.color.saturation,
            "brightness": status.color.brightness,
            "kelvin": status.color.kelvin,
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
