//! Maintainer tool for regenerating the pinned Closure CLI surface audit.
//!
//! The upstream source is caller-supplied and must match the reviewed commit's
//! exact byte length and SHA-256. The tool never downloads source itself.

#[path = "../tests/cli_surface_support/mod.rs"]
mod cli_surface_support;

use cli_surface_support::{generate_report, verify_report};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn usage() -> &'static str {
    "usage: cli_surface_audit --upstream-source <path|-> --output <path>"
}

fn arguments() -> Result<(String, PathBuf), String> {
    let mut source = None;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--upstream-source" => {
                source = Some(
                    args.next()
                        .ok_or_else(|| "--upstream-source requires a value".to_string())?,
                );
            }
            "--output" => {
                output = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a value".to_string())?,
                ));
            }
            "--help" | "-h" => return Err(usage().to_string()),
            _ => return Err(format!("unknown argument {arg:?}\n{}", usage())),
        }
    }
    Ok((
        source.ok_or_else(|| usage().to_string())?,
        output.ok_or_else(|| usage().to_string())?,
    ))
}

fn read_source(path: &str) -> Result<Vec<u8>, String> {
    if path == "-" {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("read upstream source from stdin: {error}"))?;
        Ok(bytes)
    } else {
        fs::read(path).map_err(|error| format!("read upstream source {path:?}: {error}"))
    }
}

fn run() -> Result<(), String> {
    let (source_path, output_path) = arguments()?;
    let source = read_source(&source_path)?;
    let spec = fs::read("cli.spec.json").map_err(|error| format!("read cli.spec.json: {error}"))?;
    let report = generate_report(&source, &spec).map_err(|errors| errors.join("\n"))?;
    let verification = verify_report(&report, &spec);
    if !verification.is_empty() {
        return Err(verification.join("\n"));
    }
    let mut json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize audit report: {error}"))?;
    json.push('\n');
    fs::write(&output_path, json)
        .map_err(|error| format!("write audit report {}: {error}", output_path.display()))?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
