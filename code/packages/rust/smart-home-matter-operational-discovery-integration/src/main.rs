use serde_json::json;
use smart_home_matter_operational_discovery_integration::{
    discover, MatterOperationalDiscoveryConfig, UdpMatterOperationalMdnsTransport,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-matter-operational-discovery-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() != ["discover"] {
        return Err(
            "usage: smart-home-matter-operational-discovery-integration discover".to_string(),
        );
    }
    let report = discover(
        &MatterOperationalDiscoveryConfig::default(),
        &mut UdpMatterOperationalMdnsTransport,
        0,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "nodes": report.records.iter().map(|record| json!({
                "native_bridge_id": record.native_bridge_id,
                "display_name": record.display_name,
                "address": record.address,
                "pairing_requirement": record.pairing_requirement.as_str(),
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
