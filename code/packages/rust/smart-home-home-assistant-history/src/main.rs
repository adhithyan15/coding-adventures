use smart_home_home_assistant_history::{
    migrate_live_history, write_artifact_atomically, HistoryCollectorConfig, HistoryError,
};
use smart_home_home_assistant_migration::HomeAssistantExport;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("Home Assistant history migration failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), HistoryError> {
    let mut positional = Vec::new();
    let mut dry_run = false;
    let mut collected_at_ms = None;
    let mut batch_size = 100usize;
    let mut timeout_ms = 30_000u64;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dry-run" => dry_run = true,
            "--collected-at-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                collected_at_ms = Some(value.parse::<u64>().map_err(|_| {
                    HistoryError::Usage("--collected-at-ms must be an unsigned integer".to_string())
                })?);
            }
            "--batch-size" => {
                let value = arguments.next().ok_or_else(usage)?;
                batch_size = value.parse::<usize>().map_err(|_| {
                    HistoryError::Usage("--batch-size must be an unsigned integer".to_string())
                })?;
            }
            "--timeout-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                timeout_ms = value.parse::<u64>().map_err(|_| {
                    HistoryError::Usage("--timeout-ms must be an unsigned integer".to_string())
                })?;
            }
            _ => positional.push(argument),
        }
    }
    if positional.len() != 5 {
        return Err(usage());
    }

    let topology_path = PathBuf::from(&positional[0]);
    let topology_bytes = fs::read(&topology_path).map_err(|error| HistoryError::Io {
        operation: "read topology export",
        path: topology_path.clone(),
        message: error.to_string(),
    })?;
    let topology: HomeAssistantExport = serde_json::from_slice(&topology_bytes)
        .map_err(|error| HistoryError::Decode(error.to_string()))?;
    let access_token = env::var("HOME_ASSISTANT_TOKEN")
        .map_err(|_| HistoryError::Usage("HOME_ASSISTANT_TOKEN must be set".to_string()))?;
    let config = HistoryCollectorConfig {
        websocket_url: positional[1].clone(),
        access_token,
        source_instance_id: topology.source_instance_id.clone(),
        start_time: positional[2].clone(),
        end_time: positional[3].clone(),
        collected_at_ms: collected_at_ms.unwrap_or(system_time_ms()?),
        batch_size,
        io_timeout: Duration::from_millis(timeout_ms),
    };
    let output_path = PathBuf::from(&positional[4]);
    let artifact = migrate_live_history(&topology, &config, dry_run)?;
    write_artifact_atomically(&output_path, &artifact)?;
    println!(
        "{} Home Assistant history artifact {} with {} source states, {} D23 events, and {} diagnostics",
        if dry_run { "planned" } else { "applied" },
        output_path.display(),
        artifact.history_plan.summary.source_states,
        artifact.history_plan.summary.planned_events,
        artifact.history_plan.diagnostics.len(),
    );
    Ok(())
}

fn system_time_ms() -> Result<u64, HistoryError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            HistoryError::Validation(format!("system clock is before epoch: {error}"))
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        HistoryError::Validation("system time does not fit u64 milliseconds".to_string())
    })
}

fn usage() -> HistoryError {
    HistoryError::Usage(
        "usage: smart-home-import-home-assistant-history <topology-export.json> <ws-url> <start-rfc3339> <end-rfc3339> <artifact.json> [--dry-run] [--collected-at-ms <timestamp>] [--batch-size <count>] [--timeout-ms <milliseconds>]"
            .to_string(),
    )
}
