//! # ca-capability-analyzer CLI (VER08a / VER23 / VER24)
//!
//! Entry point for developer pre-commit verification, module generation,
//! and CI publication gating.

#![deny(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

use coding_adventures_ca_capability_analyzer::{
    analyze_package, check_module_on_disk, generate_certificate, verify_certificate,
    write_module_to_disk,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut dir = PathBuf::from(".");
    let mut verbose = false;
    let mut generate_module = false;
    let mut check_module = false;
    let mut emit_cert_path: Option<PathBuf> = None;
    let mut verify_cert_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --dir requires an argument");
                    exit(2);
                }
                dir = PathBuf::from(&args[i]);
            }
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--generate-module" => {
                generate_module = true;
            }
            "--check-module" => {
                check_module = true;
            }
            "--emit-cert" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --emit-cert requires a file path argument");
                    exit(2);
                }
                emit_cert_path = Some(PathBuf::from(&args[i]));
            }
            "--verify-cert" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --verify-cert requires a file path argument");
                    exit(2);
                }
                verify_cert_path = Some(PathBuf::from(&args[i]));
            }
            "--help" | "-h" => {
                print_help();
                exit(0);
            }
            unknown => {
                eprintln!("Error: unrecognized argument '{unknown}'");
                print_help();
                exit(2);
            }
        }
        i += 1;
    }

    if generate_module {
        match write_module_to_disk(&dir) {
            Ok(()) => {
                println!("Generated src/capabilities.rs successfully in {}", dir.display());
                exit(0);
            }
            Err(e) => {
                eprintln!("Error generating module: {e}");
                exit(2);
            }
        }
    }

    if check_module {
        match check_module_on_disk(&dir) {
            Ok(true) => {
                println!("PASS: src/capabilities.rs is up to date");
                exit(0);
            }
            Ok(false) => {
                eprintln!("FAIL: src/capabilities.rs is missing or differs from required_capabilities.json");
                exit(1);
            }
            Err(e) => {
                eprintln!("Error checking module: {e}");
                exit(2);
            }
        }
    }

    if let Some(cert_path) = emit_cert_path {
        match generate_certificate(&dir) {
            Ok(cert) => {
                let json_data = cert.to_json_string();
                if let Err(e) = fs::write(&cert_path, json_data) {
                    eprintln!("Error writing certificate {}: {e}", cert_path.display());
                    exit(2);
                }
                println!("Certificate emitted successfully to {}", cert_path.display());
                exit(0);
            }
            Err(e) => {
                eprintln!("Error generating certificate: {e}");
                exit(2);
            }
        }
    }

    if let Some(cert_path) = verify_cert_path {
        match verify_certificate(&dir, &cert_path) {
            Ok(res) => {
                println!(
                    "PASS: effect certificate for '{}' verified (label: {}, entry_points: {}, declared_effects: {})",
                    res.package, res.label, res.entry_points_count, res.declared_effects_count
                );
                exit(0);
            }
            Err(e) => {
                eprintln!("FAIL: effect certificate verification failed: {e}");
                exit(1);
            }
        }
    }

    match analyze_package(&dir, verbose) {
        Ok(report) => {
            print!("{}", report.format_text(verbose));
            exit(report.exit_code());
        }
        Err(err) => {
            eprintln!("Error: {err}");
            exit(2);
        }
    }
}

fn print_help() {
    println!("Usage: ca-capability-analyzer-rs [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dir <PATH>          Directory of the Rust package to analyze (default: .)");
    println!("  --verbose, -v         Show all detected capabilities and scanned files");
    println!("  --generate-module     Emit generated src/capabilities.rs for target package");
    println!("  --check-module        Verify on-disk src/capabilities.rs matches manifest");
    println!("  --emit-cert <FILE>    Emit canonical effect proof certificate to <FILE>");
    println!("  --verify-cert <FILE>  Verify effect certificate with small checker");
    println!("  --help, -h            Display this help message");
    println!();
    println!("Exit codes:");
    println!("  0: Success (all checks/verifications passed)");
    println!("  1: Capability violation, certificate mismatch, or module drift");
    println!("  2: Input error, malformed manifest JSON, or resource quota exceeded");
}
