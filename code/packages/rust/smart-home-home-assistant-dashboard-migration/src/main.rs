use smart_home_home_assistant_dashboard_migration::{
    migrate_live_dashboards, write_artifact_atomically, DashboardCollectorConfig,
    DashboardMigrationError,
};
use smart_home_home_assistant_migration::HomeAssistantExport;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("Home Assistant dashboard migration failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), DashboardMigrationError> {
    let mut positional = Vec::new();
    let mut collected_at_ms = None;
    let mut timeout_ms = 30_000_u64;
    let mut dry_run = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--collected-at-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                collected_at_ms = Some(value.parse::<u64>().map_err(|_| {
                    DashboardMigrationError::Usage(
                        "--collected-at-ms must be an unsigned integer".to_string(),
                    )
                })?);
            }
            "--timeout-ms" => {
                let value = arguments.next().ok_or_else(usage)?;
                timeout_ms = value.parse::<u64>().map_err(|_| {
                    DashboardMigrationError::Usage(
                        "--timeout-ms must be an unsigned integer".to_string(),
                    )
                })?;
            }
            "--dry-run" => dry_run = true,
            _ => positional.push(argument),
        }
    }
    if positional.len() != 3 {
        return Err(usage());
    }

    let topology_path = PathBuf::from(&positional[0]);
    let topology_bytes = fs::read(&topology_path).map_err(|error| DashboardMigrationError::Io {
        operation: "read topology export",
        path: topology_path.clone(),
        message: error.to_string(),
    })?;
    let topology: HomeAssistantExport = serde_json::from_slice(&topology_bytes)
        .map_err(|error| DashboardMigrationError::Decode(error.to_string()))?;
    let access_token = env::var("HOME_ASSISTANT_TOKEN").map_err(|_| {
        DashboardMigrationError::Usage("HOME_ASSISTANT_TOKEN must be set".to_string())
    })?;
    let config = DashboardCollectorConfig {
        websocket_url: positional[1].clone(),
        access_token,
        source_instance_id: topology.source_instance_id.clone(),
        collected_at_ms: collected_at_ms.unwrap_or(system_time_ms()?),
        io_timeout: Duration::from_millis(timeout_ms),
    };
    let artifact = migrate_live_dashboards(&topology, &config, dry_run)?;
    let output_path = PathBuf::from(&positional[2]);
    write_artifact_atomically(&output_path, &artifact)?;
    let summary = artifact.plan.summary;
    println!(
        "{} Home Assistant dashboards {} with {} views, {}/{} cards, and {} diagnostics",
        if dry_run { "planned" } else { "migrated" },
        output_path.display(),
        summary.views,
        summary.cards_migrated,
        summary.cards_discovered,
        summary.warnings + summary.errors,
    );
    Ok(())
}

fn system_time_ms() -> Result<u64, DashboardMigrationError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            DashboardMigrationError::Validation(format!("system clock is before epoch: {error}"))
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        DashboardMigrationError::Validation("system time does not fit u64 milliseconds".to_string())
    })
}

fn usage() -> DashboardMigrationError {
    DashboardMigrationError::Usage(
        "usage: smart-home-migrate-home-assistant-dashboards <topology-export.json> <ws-url> <dashboard-artifact.json> [--dry-run] [--collected-at-ms <timestamp>] [--timeout-ms <milliseconds>]"
            .to_string(),
    )
}
