use serde_json::json;
use smart_home_google_cast_discovery_integration::{
    discover, GoogleCastDiscoveryConfig, UdpGoogleCastMdnsTransport,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-google-cast-discovery-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() != ["discover"] {
        return Err("usage: smart-home-google-cast-discovery-integration discover".to_string());
    }
    let config = GoogleCastDiscoveryConfig::default();
    let report =
        discover(&config, &mut UdpGoogleCastMdnsTransport, 0).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "receivers": report.records.iter().map(|record| json!({
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
