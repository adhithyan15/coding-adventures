//! # Capability Analyzer — Binary Entry Point
//!
//! This is the thin binary wrapper around the library. It collects
//! command-line arguments and delegates to [`ca_capability_analyzer::cli::run`].

use ca_capability_analyzer::cli;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(cli::run(&args));
}
