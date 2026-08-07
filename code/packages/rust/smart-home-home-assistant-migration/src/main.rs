use smart_home_home_assistant_migration::{
    migrate_export_bytes, write_artifact_atomically, MigrationError,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("Home Assistant migration failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), MigrationError> {
    let mut positional = Vec::new();
    let mut dry_run = false;
    for argument in env::args().skip(1) {
        if argument == "--dry-run" {
            dry_run = true;
        } else {
            positional.push(argument);
        }
    }
    if positional.len() != 2 {
        return Err(MigrationError::Usage(
            "usage: smart-home-import-home-assistant <export.json> <artifact.json> [--dry-run]"
                .to_string(),
        ));
    }

    let input_path = PathBuf::from(&positional[0]);
    let output_path = PathBuf::from(&positional[1]);
    let input = fs::read(&input_path).map_err(|error| MigrationError::Io {
        operation: "read",
        path: input_path,
        message: error.to_string(),
    })?;
    let artifact = migrate_export_bytes(&input, dry_run)?;
    write_artifact_atomically(&output_path, &artifact)?;

    println!(
        "{} Home Assistant migration artifact {} with {} devices, {} entities, {} scenes, {} automations, and {} diagnostics",
        if dry_run { "planned" } else { "applied" },
        output_path.display(),
        artifact.plan.summary.devices,
        artifact.plan.summary.entities,
        artifact.plan.summary.scenes,
        artifact.plan.summary.automations,
        artifact.plan.diagnostics.len(),
    );
    Ok(())
}
