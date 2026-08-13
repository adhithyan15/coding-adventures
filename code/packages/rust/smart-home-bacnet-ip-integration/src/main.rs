use serde_json::json;
use smart_home_bacnet_ip_integration::{discover, BacnetIpDiscoveryConfig, UdpBacnetIpTransport};
use std::env;
use std::net::Ipv4Addr;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-bacnet-ip-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let (destination, port) = match arguments.as_slice() {
        [command, destination] if command == "discover" => (
            parse_ipv4(destination)?,
            bacnet_protocol::BACNET_IP_DEFAULT_PORT,
        ),
        [command, destination, port] if command == "discover" => {
            (parse_ipv4(destination)?, parse_port(port)?)
        }
        _ => {
            return Err(
                "usage: smart-home-bacnet-ip-integration discover <ipv4-destination> [port]"
                    .to_string(),
            )
        }
    };
    let mut config = BacnetIpDiscoveryConfig::new(destination);
    config.destination.set_port(port);
    let report =
        discover(&config, &mut UdpBacnetIpTransport, 0).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "destination": config.destination.to_string(),
            "devices": report.records.iter().map(|record| json!({
                "native_bridge_id": record.native_bridge_id,
                "display_name": record.display_name,
                "address": record.address,
                "metadata": record.metadata.iter().map(|item| json!({
                    "key": item.key,
                    "value": item.value,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "failures": report.failures,
        })
    );
    Ok(())
}

fn parse_ipv4(value: &str) -> Result<Ipv4Addr, String> {
    value
        .parse()
        .map_err(|_| "destination must be an IPv4 address".to_string())
}

fn parse_port(value: &str) -> Result<u16, String> {
    let port = value
        .parse::<u16>()
        .map_err(|_| "port must be an unsigned 16-bit integer".to_string())?;
    if port == 0 {
        Err("port must be non-zero".to_string())
    } else {
        Ok(port)
    }
}
