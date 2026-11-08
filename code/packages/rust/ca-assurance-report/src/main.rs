//! # Assurance Report CLI (VER01 / VER15 / VER-P028)
//!
//! Evaluates package weakness matrix, generates `assurance_report.json`,
//! and verifies assurance reports fail-closed.

#![deny(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

use coding_adventures_ca_assurance_report::{
    generate_ct_compare_report, verify_assurance_report, AssuranceReport, CheckVerdict,
    WeaknessRegistry,
};

fn print_usage() {
    eprintln!("Usage: ca-assurance-report-rs [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --package-dir <DIR>    Package directory to audit (default: .)");
    eprintln!("  --generate             Generate and emit assurance_report.json");
    eprintln!("  --check <FILE>         Verify an existing assurance_report.json fail-closed");
    eprintln!("  --registry <FILE>      Path to weakness-registry-v1.json");
    eprintln!("  --commit <SHA>         Commit SHA to record in generated report");
    eprintln!("  --help                 Display this help message");
}

fn find_default_registry() -> WeaknessRegistry {
    // Try local relative path to specs
    let candidates = [
        PathBuf::from("code/specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../../specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../../../specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../specs/data/weakness-registry-v1.json"),
    ];

    for path in &candidates {
        if path.is_file() {
            if let Ok(reg) = WeaknessRegistry::load_from_file(path) {
                return reg;
            }
        }
    }

    // Fall back to embedded snapshot
    WeaknessRegistry::embedded()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut package_dir = PathBuf::from(".");
    let mut check_file: Option<PathBuf> = None;
    let mut generate = false;
    let mut registry_file: Option<PathBuf> = None;
    let mut commit_sha = "HEAD".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--package-dir" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --package-dir requires an argument");
                    exit(2);
                }
                package_dir = PathBuf::from(&args[i]);
            }
            "--check" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --check requires an argument");
                    exit(2);
                }
                check_file = Some(PathBuf::from(&args[i]));
            }
            "--generate" => {
                generate = true;
            }
            "--registry" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --registry requires an argument");
                    exit(2);
                }
                registry_file = Some(PathBuf::from(&args[i]));
            }
            "--commit" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --commit requires an argument");
                    exit(2);
                }
                commit_sha = args[i].clone();
            }
            "--help" | "-h" => {
                print_usage();
                exit(0);
            }
            other => {
                eprintln!("Error: unknown option '{other}'");
                print_usage();
                exit(2);
            }
        }
        i += 1;
    }

    let registry = if let Some(path) = registry_file {
        match WeaknessRegistry::load_from_file(&path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error loading registry from {}: {e}", path.display());
                exit(2);
            }
        }
    } else {
        find_default_registry()
    };

    if let Some(file_path) = check_file {
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading report file {}: {e}", file_path.display());
                exit(2);
            }
        };

        let report = match AssuranceReport::from_json_str(&content) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error parsing report JSON: {e}");
                exit(1);
            }
        };

        match verify_assurance_report(&report, &registry) {
            Ok(verdict) => {
                match verdict {
                    CheckVerdict::Pass => {
                        println!("ASSURANCE VERDICT: PASS [Package {}]", report.package);
                    }
                    CheckVerdict::ReportOnly(reason) => {
                        println!(
                            "ASSURANCE VERDICT: REPORT-ONLY [Package {}] ({})",
                            report.package, reason
                        );
                    }
                }
                exit(0);
            }
            Err(e) => {
                eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
                exit(1);
            }
        }
    }

    if generate {
        let target_file = package_dir.join("assurance_report.json");
        let report = match generate_ct_compare_report(&commit_sha, &registry) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error generating report: {e}");
                exit(1);
            }
        };

        // Self-check before writing
        if let Err(e) = verify_assurance_report(&report, &registry) {
            eprintln!("Generated report failed internal verification: {e}");
            exit(1);
        }

        let json_str = report.to_json_string();
        if let Err(e) = fs::write(&target_file, format!("{json_str}\n")) {
            eprintln!("Error writing {}: {e}", target_file.display());
            exit(2);
        }

        println!("Generated assurance report at {}", target_file.display());
        println!("Verdict: {}", report.verdict);
        exit(0);
    }

    print_usage();
    exit(2);
}
