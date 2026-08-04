use smart_home_core::BridgeId;
use smart_home_runtime::SmartHomeRuntime;
use smart_home_zwave_host::{ZWaveHost, ZWaveHostConfig, DEFAULT_BAUD_RATE};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "Usage: smart-home-zwave-host --port PATH [--bridge-id ID] [--baud-rate RATE]";

fn main() -> Result<(), Box<dyn Error>> {
    let mut port = None;
    let mut bridge_id = "zwave-controller".to_string();
    let mut baud_rate = DEFAULT_BAUD_RATE;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--port" => port = arguments.next(),
            "--bridge-id" => {
                bridge_id = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for --bridge-id\n{USAGE}"))?;
            }
            "--baud-rate" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for --baud-rate\n{USAGE}"))?;
                baud_rate = value
                    .parse()
                    .map_err(|_| format!("invalid baud rate `{value}`\n{USAGE}"))?;
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}").into()),
        }
    }
    let port = port.ok_or_else(|| format!("--port is required\n{USAGE}"))?;
    let config = ZWaveHostConfig::new(BridgeId::trusted(bridge_id), port).baud_rate(baud_rate);
    let host = ZWaveHost::open(config, SmartHomeRuntime::new(), unix_time_ms())?;
    let controller = host.controller();
    println!(
        "Z-Wave controller ready: version={}, home_id={:08x}, controller_node={:?}, nodes={}",
        controller.version.version,
        controller.memory_id.home_id.0,
        controller.memory_id.controller_node_id,
        controller.known_node_count()
    );
    Ok(())
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
