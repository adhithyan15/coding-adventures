use serde_json::json;
use smart_home_knxnet_ip_integration::{discover, KnxnetIpDiscoveryConfig, UdpKnxnetIpTransport};
use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-knxnet-ip-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let (local_interface, destination) = match arguments.as_slice() {
        [command, local_interface] if command == "discover" => {
            (parse_ipv4(local_interface)?, None)
        }
        [command, local_interface, destination] if command == "discover" => (
            parse_ipv4(local_interface)?,
            Some(SocketAddrV4::new(parse_ipv4(destination)?, knxnet_ip_protocol::KNXNET_IP_DEFAULT_PORT)),
        ),
        [command, local_interface, destination, port] if command == "discover" => (
            parse_ipv4(local_interface)?,
            Some(SocketAddrV4::new(parse_ipv4(destination)?, parse_port(port)?)),
        ),
        _ => {
            return Err(
                "usage: smart-home-knxnet-ip-integration discover <local-ipv4> [destination-ipv4] [port]"
                    .to_string(),
            )
        }
    };
    let mut config = KnxnetIpDiscoveryConfig::new(local_interface);
    if let Some(destination) = destination {
        config.destination = destination;
    }
    let report =
        discover(&config, &mut UdpKnxnetIpTransport, 0).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "local_endpoint": report.request_endpoint.to_string(),
            "destination": config.destination.to_string(),
            "interfaces": report.records.iter().map(|record| json!({
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
        .map_err(|_| "address must be IPv4".to_string())
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
