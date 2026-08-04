use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_wled_integration::{
    discovery_record, scan_mdns_ipv4, WledClient, WledDeviceConfig, WledLanTransport,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-wled-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => {
            let result = scan_mdns_ipv4(now_ms(), Duration::from_secs(2))
                .map_err(|error| error.to_string())?;
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
        [command, host] if command == "inspect" => inspect(host, 80),
        [command, host, port] if command == "inspect" => inspect(
            host,
            port.parse::<u16>()
                .map_err(|_| "port must be an unsigned 16-bit integer".to_string())?,
        ),
        _ => Err("usage: smart-home-wled-integration discover | inspect <host> [port]".to_string()),
    }
}

fn inspect(host: &str, port: u16) -> Result<(), String> {
    let config = WledDeviceConfig::new(BridgeId::trusted("wled.cli"), host)
        .map_err(|error| error.to_string())?
        .with_port(port);
    let mut client =
        WledClient::new(config, WledLanTransport::default()).map_err(|error| error.to_string())?;
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "name": snapshot.info.name,
            "version": snapshot.info.version,
            "brand": snapshot.info.brand,
            "product": snapshot.info.product,
            "led_count": snapshot.info.leds.count,
            "segments": snapshot.state.segments.iter().map(|segment| json!({
                "id": segment.id,
                "name": segment.name,
                "on": segment.on,
                "brightness": segment.brightness,
            })).collect::<Vec<_>>(),
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
