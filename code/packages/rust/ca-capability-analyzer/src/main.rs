//! # ca-capability-analyzer CLI (VER08a)
//!
//! Entry point for developer pre-commit verification and CI publication gating.

#![deny(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::exit;

use coding_adventures_ca_capability_analyzer::analyze_package;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut dir = PathBuf::from(".");
    let mut verbose = false;

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
    println!("  --dir <PATH>    Directory of the Rust package to analyze (default: .)");
    println!("  --verbose, -v   Show all detected capabilities and scanned files");
    println!("  --help, -h      Display this help message");
    println!();
    println!("Exit codes:");
    println!("  0: All detected capabilities declared; no unexempt banned constructs");
    println!("  1: One or more capability or banned construct violations found");
    println!("  2: Input error, malformed manifest JSON, or resource quota exceeded");
}
