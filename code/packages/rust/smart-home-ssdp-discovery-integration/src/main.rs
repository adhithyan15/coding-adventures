use serde_json::json;
use smart_home_ssdp_discovery_integration::{discover, SsdpDiscoveryConfig, UdpSsdpTransport};
use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-ssdp-discovery-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let (local_interface, destination, search_target) = match arguments.as_slice() {
        [command, local_interface] if command == "discover" => {
            (parse_ipv4(local_interface)?, None, None)
        }
        [command, local_interface, destination] if command == "discover" => (
            parse_ipv4(local_interface)?,
            Some(parse_destination(destination)?),
            None,
        ),
        [command, local_interface, destination, search_target] if command == "discover" => (
            parse_ipv4(local_interface)?,
            Some(parse_destination(destination)?),
            Some(search_target.clone()),
        ),
        _ => return Err(
            "usage: smart-home-ssdp-discovery-integration discover <local-ipv4-interface> [ipv4-destination:port] [search-target]"
                .to_string(),
        ),
    };
    let mut config = SsdpDiscoveryConfig::new(local_interface);
    if let Some(destination) = destination {
        config.destination = destination;
    }
    if let Some(search_target) = search_target {
        config.request.search_target = search_target;
    }
    let report = discover(&config, &mut UdpSsdpTransport, 0).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "local_interface": config.local_interface.to_string(),
            "destination": config.destination.to_string(),
            "search_target": config.request.search_target,
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
        .map_err(|_| "local interface must be an IPv4 address".to_string())
}

fn parse_destination(value: &str) -> Result<SocketAddrV4, String> {
    value
        .parse()
        .map_err(|_| "destination must be an IPv4 socket address".to_string())
}
