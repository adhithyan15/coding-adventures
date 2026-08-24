use serde_json::json;
use smart_home_thread_border_agent_discovery_integration::{
    discover, ThreadBorderAgentDiscoveryConfig, UdpThreadBorderAgentMdnsTransport,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-thread-border-agent-discovery-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() != ["discover"] {
        return Err(
            "usage: smart-home-thread-border-agent-discovery-integration discover".to_string(),
        );
    }
    let config = ThreadBorderAgentDiscoveryConfig::default();
    let report = discover(&config, &mut UdpThreadBorderAgentMdnsTransport, 0)
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "border_agents": report.records.iter().map(|record| json!({
                "native_bridge_id": record.native_bridge_id,
                "display_name": record.display_name,
                "address": record.address,
                "hardware_model": record.hardware_model,
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
