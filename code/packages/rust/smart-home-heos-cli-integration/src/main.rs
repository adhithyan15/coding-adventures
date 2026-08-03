use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_heos_cli_integration::{
    discover_ssdp_ipv4, HeosClient, HeosConfig, HeosLanTransport, DEFAULT_PORT,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-heos-cli-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "discover" => discover(),
        [command, host] if command == "inspect" => inspect(host, DEFAULT_PORT),
        [command, host, port] if command == "inspect" => inspect(host, parse_port(port)?),
        _ => Err(
            "usage: smart-home-heos-cli-integration discover | inspect <host> [port]".to_string(),
        ),
    }
}

fn discover() -> Result<(), String> {
    let candidates =
        discover_ssdp_ipv4(Duration::from_secs(3), 32).map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!(candidates
            .iter()
            .map(|candidate| json!({
                "host": candidate.host,
                "location": candidate.location,
                "usn": candidate.usn,
                "server": candidate.server,
            }))
            .collect::<Vec<_>>())
    );
    Ok(())
}

fn inspect(host: &str, port: u16) -> Result<(), String> {
    let config = HeosConfig::new(BridgeId::trusted("heos.cli"), host)
        .map_err(|error| error.to_string())?
        .with_port(port);
    let mut client = HeosClient::new(config, HeosLanTransport::default());
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "players": snapshot.players.iter().map(|snapshot| json!({
                "pid": snapshot.player.pid,
                "name": snapshot.player.name,
                "model": snapshot.player.model,
                "version": snapshot.player.version,
                "network": snapshot.player.network,
                "group_id": snapshot.player.group_id,
                "play_state": snapshot.play_state,
                "volume": snapshot.volume,
                "muted": snapshot.muted,
                "now_playing": {
                    "type": snapshot.now_playing.media_type,
                    "song": snapshot.now_playing.song,
                    "station": snapshot.now_playing.station,
                    "album": snapshot.now_playing.album,
                    "artist": snapshot.now_playing.artist,
                    "image_url": snapshot.now_playing.image_url,
                },
            })).collect::<Vec<_>>(),
        })
    );
    Ok(())
}

fn parse_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| "port must be an unsigned 16-bit integer".to_string())
}
