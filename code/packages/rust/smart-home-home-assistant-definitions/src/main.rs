use smart_home_home_assistant_definitions::{
    collect_definitions, write_enriched_export_atomically, DefinitionCollectorConfig,
    DefinitionError,
};
use smart_home_home_assistant_migration::HomeAssistantExport;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("Home Assistant definition collection failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), DefinitionError> {
    let mut positional = Vec::new();
    let mut collected_at_ms = None;
    let mut timeout_ms = 30_000_u64;
    let mut max_response_bytes = 4 * 1024 * 1024_usize;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--collected-at-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                collected_at_ms = Some(value.parse::<u64>().map_err(|_| {
                    DefinitionError::Usage(
                        "--collected-at-ms must be an unsigned integer".to_string(),
                    )
                })?);
            }
            "--timeout-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                timeout_ms = value.parse::<u64>().map_err(|_| {
                    DefinitionError::Usage("--timeout-ms must be an unsigned integer".to_string())
                })?;
            }
            "--max-response-bytes" => {
                let value = arguments.next().ok_or_else(usage)?;
                max_response_bytes = value.parse::<usize>().map_err(|_| {
                    DefinitionError::Usage(
                        "--max-response-bytes must be an unsigned integer".to_string(),
                    )
                })?;
            }
            _ => positional.push(argument),
        }
    }
    if positional.len() != 4 {
        return Err(usage());
    }

    let topology_path = PathBuf::from(&positional[0]);
    let topology_bytes = fs::read(&topology_path).map_err(|error| DefinitionError::Io {
        operation: "read topology export",
        path: topology_path.clone(),
        message: error.to_string(),
    })?;
    let topology: HomeAssistantExport = serde_json::from_slice(&topology_bytes)
        .map_err(|error| DefinitionError::Decode(error.to_string()))?;
    let access_token = env::var("HOME_ASSISTANT_TOKEN")
        .map_err(|_| DefinitionError::Usage("HOME_ASSISTANT_TOKEN must be set".to_string()))?;
    let config = DefinitionCollectorConfig {
        websocket_url: positional[1].clone(),
        rest_base_url: positional[2].clone(),
        access_token,
        source_instance_id: topology.source_instance_id.clone(),
        collected_at_ms: collected_at_ms.unwrap_or(system_time_ms()?),
        io_timeout: Duration::from_millis(timeout_ms),
        max_response_bytes,
    };
    let output_path = PathBuf::from(&positional[3]);
    let enriched = collect_definitions(&topology, &config)?;
    write_enriched_export_atomically(&output_path, &enriched)?;
    let summary = enriched.definition_collection.summary;
    println!(
        "collected Home Assistant definitions {} with {}/{} scenes, {}/{} automations, and {} diagnostics",
        output_path.display(),
        summary.collected_scenes,
        summary.discovered_scenes,
        summary.collected_automations,
        summary.discovered_automations,
        summary.diagnostics,
    );
    Ok(())
}

fn system_time_ms() -> Result<u64, DefinitionError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            DefinitionError::Validation(format!("system clock is before epoch: {error}"))
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        DefinitionError::Validation("system time does not fit u64 milliseconds".to_string())
    })
}

fn usage() -> DefinitionError {
    DefinitionError::Usage(
        "usage: smart-home-collect-home-assistant-definitions <topology-export.json> <ws-url> <rest-base-url> <enriched-export.json> [--collected-at-ms <timestamp>] [--timeout-ms <milliseconds>] [--max-response-bytes <bytes>]"
            .to_string(),
    )
}
