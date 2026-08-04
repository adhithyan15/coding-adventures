use serde_json::json;
use smart_home_core::{BridgeId, VaultRef};
use smart_home_reolink_integration::{
    discovery_record, ReolinkClient, ReolinkConfig, ReolinkCredentials, ReolinkLanTransport,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-reolink-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [command, base_url] = arguments.as_slice() else {
        return Err("usage: smart-home-reolink-integration inspect <base-url>".to_string());
    };
    if command != "inspect" {
        return Err("usage: smart-home-reolink-integration inspect <base-url>".to_string());
    }
    let username =
        env::var("REOLINK_USERNAME").map_err(|_| "REOLINK_USERNAME must be set".to_string())?;
    let password =
        env::var("REOLINK_PASSWORD").map_err(|_| "REOLINK_PASSWORD must be set".to_string())?;
    let credential_ref = env::var("REOLINK_CREDENTIAL_REF")
        .map_err(|_| "REOLINK_CREDENTIAL_REF must be set".to_string())?;
    let config = ReolinkConfig::new(
        BridgeId::trusted("reolink.cli"),
        base_url,
        VaultRef::new(credential_ref).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let credentials =
        ReolinkCredentials::new(username, password).map_err(|error| error.to_string())?;
    let mut client = ReolinkClient::new(config, credentials, ReolinkLanTransport::default());
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    let record = discovery_record(client.config(), &snapshot, now_ms())
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "id": record.native_bridge_id,
            "name": record.display_name,
            "model": record.hardware_model,
            "firmware_version": record.firmware_version,
            "address": record.address,
            "channels": snapshot.channels.iter().map(|channel| json!({
                "channel": channel.channel,
                "name": channel.name,
                "online": channel.online,
                "sleeping": channel.sleeping,
                "motion": channel.motion,
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
