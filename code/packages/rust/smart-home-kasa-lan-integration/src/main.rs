use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_kasa_lan_integration::{
    discovery_record, scan_lan, KasaClient, KasaDeviceConfig, KasaLanTransport, KasaScanConfig,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-kasa-lan-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => discover(),
        [command, host] if command == "inspect" => inspect(host, None),
        [command, host, port] if command == "inspect" => inspect(host, Some(port)),
        _ => Err(
            "usage: smart-home-kasa-lan-integration discover | inspect <host> [port]".to_string(),
        ),
    }
}

fn discover() -> Result<(), String> {
    let result = scan_lan(&KasaScanConfig::default(), &mut KasaLanTransport)
        .map_err(|error| error.to_string())?;
    let devices = result
        .devices
        .iter()
        .map(|device| {
            discovery_record(device, now_ms())
                .map(|record| {
                    json!({
                        "id": record.native_bridge_id,
                        "name": record.display_name,
                        "model": record.hardware_model,
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

fn inspect(host: &str, port: Option<&String>) -> Result<(), String> {
    let mut config = KasaDeviceConfig::new(BridgeId::trusted("kasa.cli"), host)
        .map_err(|error| error.to_string())?;
    if let Some(port) = port {
        config = config.with_port(
            port.parse::<u16>()
                .map_err(|_| "port must be an unsigned 16-bit integer".to_string())?,
        );
    }
    let mut client = KasaClient::new(config, KasaLanTransport);
    let status = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "device_id": status.device_id,
            "alias": status.alias,
            "model": status.model,
            "kind": status.kind.as_str(),
            "on": status.on,
            "supports_brightness": status.supports_brightness,
            "supports_color": status.supports_color,
            "supports_color_temperature": status.supports_color_temperature,
            "brightness": status.brightness,
            "hue": status.hue,
            "saturation": status.saturation,
            "color_temperature_kelvin": status.color_temperature_kelvin,
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
