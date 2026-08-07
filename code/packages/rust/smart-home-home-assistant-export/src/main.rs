use smart_home_home_assistant_export::{
    collect_export, write_export_atomically, CollectorError, HomeAssistantCollectorConfig,
};
use std::env;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("Home Assistant export failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CollectorError> {
    let mut positional = Vec::new();
    let mut exported_at_ms = None;
    let mut timeout_ms = 30_000;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--exported-at-ms" {
            let value = arguments.next().ok_or_else(usage)?;
            exported_at_ms = Some(value.parse::<u64>().map_err(|_| {
                CollectorError::Usage("--exported-at-ms must be an unsigned integer".to_string())
            })?);
        } else if argument == "--timeout-ms" {
            let value = arguments.next().ok_or_else(usage)?;
            timeout_ms = value.parse::<u64>().map_err(|_| {
                CollectorError::Usage("--timeout-ms must be an unsigned integer".to_string())
            })?;
        } else {
            positional.push(argument);
        }
    }
    if positional.len() != 3 {
        return Err(usage());
    }

    let access_token = env::var("HOME_ASSISTANT_TOKEN")
        .map_err(|_| CollectorError::Usage("HOME_ASSISTANT_TOKEN must be set".to_string()))?;
    let config = HomeAssistantCollectorConfig {
        websocket_url: positional[0].clone(),
        access_token,
        source_instance_id: positional[1].clone(),
        exported_at_ms: exported_at_ms.unwrap_or(system_time_ms()?),
        io_timeout: Duration::from_millis(timeout_ms),
    };
    let output_path = PathBuf::from(&positional[2]);
    let (export, summary) = collect_export(&config)?;
    write_export_atomically(&output_path, &export)?;
    println!(
        "collected Home Assistant export {} with {} areas, {} devices, {} entities ({} synthetic), and {} states",
        output_path.display(),
        summary.areas,
        summary.devices,
        summary.entities,
        summary.synthetic_entities,
        summary.states,
    );
    Ok(())
}

fn system_time_ms() -> Result<u64, CollectorError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            CollectorError::Config(format!("system clock is before epoch: {error}"))
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        CollectorError::Config("system time does not fit u64 milliseconds".to_string())
    })
}

fn usage() -> CollectorError {
    CollectorError::Usage(
        "usage: smart-home-export-home-assistant <ws-url> <source-instance-id> <export.json> [--exported-at-ms <timestamp>] [--timeout-ms <milliseconds>]"
            .to_string(),
    )
}
