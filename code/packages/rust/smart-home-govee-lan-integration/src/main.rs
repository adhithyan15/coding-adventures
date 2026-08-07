use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_govee_lan_integration::{
    discovery_record, scan_lan, GoveeClient, GoveeDeviceConfig, GoveeLanTransport, GoveeScanConfig,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-govee-lan-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => {
            let result = scan_lan(&GoveeScanConfig::default(), &mut GoveeLanTransport)
                .map_err(|error| error.to_string())?;
            let records = result
                .devices
                .iter()
                .filter_map(|device| {
                    discovery_record(device, now_ms()).ok().map(|record| {
                        json!({
                            "id": record.native_bridge_id,
                            "address": record.address,
                            "model": record.hardware_model,
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
        [command, host, device_id, sku] if command == "inspect" => inspect(host, device_id, sku),
        _ => Err(
            "usage: smart-home-govee-lan-integration discover | inspect <host> <device-id> <sku>"
                .to_string(),
        ),
    }
}

fn inspect(host: &str, device_id: &str, sku: &str) -> Result<(), String> {
    let config = GoveeDeviceConfig::new(BridgeId::trusted("govee-lan.cli"), host, device_id, sku)
        .map_err(|error| error.to_string())?;
    let mut client = GoveeClient::new(config, GoveeLanTransport);
    let status = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "on": status.on,
            "brightness": status.brightness,
            "color": status.color,
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
