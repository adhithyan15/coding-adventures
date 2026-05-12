//! TSA adjudication demo — runs both arms against a local Ollama
//! instance and prints a side-by-side comparison.
//!
//! ## Usage
//!
//! ```text
//! # Default: localhost:11434, model `gemma3:4b`, default TSA text.
//! cargo run -p adjudication-tsa-demo
//!
//! # Override model:
//! ADJ_DEMO_MODEL=llama3.1:8b cargo run -p adjudication-tsa-demo
//!
//! # Override endpoint (e.g., remote Ollama on the LAN):
//! ADJ_DEMO_ENDPOINT=http://192.168.1.42:11434 cargo run -p adjudication-tsa-demo
//!
//! # Custom source text:
//! ADJ_DEMO_SOURCE='1 carry-on bag, lithium battery.' cargo run -p adjudication-tsa-demo
//!
//! # Optional JSON dump of the full audit trail:
//! ADJ_DEMO_AUDIT=1 cargo run -p adjudication-tsa-demo
//! ```
//!
//! The binary is intentionally environment-variable driven rather
//! than `clap`-flagged so the demo crate stays zero-dep beyond what
//! the framework already pulls in.

use std::time::Duration;

use adjudication_tsa_demo::{
    format_side_by_side, run_pipeline_arm, run_raw_arm, DemoConfig,
};

fn main() {
    let cfg = config_from_env();

    println!(
        "Running TSA demo against {endpoint} with model `{model}`.\n\
         (set ADJ_DEMO_MODEL / ADJ_DEMO_ENDPOINT / ADJ_DEMO_SOURCE to override.)\n",
        endpoint = cfg.endpoint,
        model = cfg.model,
    );

    // --- Arm A ---
    println!("[arm A] asking the raw model directly...");
    let raw = match run_raw_arm(&cfg) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "Arm A failed: {e}\n\n\
                 Hint: is Ollama running? Try `ollama serve` and \
                 `ollama pull {model}` first.",
                model = cfg.model,
            );
            std::process::exit(1);
        }
    };

    // --- Arm B ---
    println!("[arm B] running the structured pipeline (this calls the model 1× per IR leaf for ADJ04)...");
    let pipe = run_pipeline_arm(&cfg);

    // --- side-by-side ---
    println!("\n{}", format_side_by_side(&raw, &pipe));

    // --- optional audit-trail dump ---
    if std::env::var("ADJ_DEMO_AUDIT").is_ok() {
        match serde_json::to_string_pretty(&pipe.pipeline_output.audit_trail) {
            Ok(json) => {
                println!("--- full audit trail (ADJ07-v1) ---\n{json}");
            }
            Err(e) => {
                eprintln!("warning: failed to serialize audit trail: {e}");
            }
        }
    }
}

fn config_from_env() -> DemoConfig {
    let mut cfg = DemoConfig::default();
    if let Ok(endpoint) = std::env::var("ADJ_DEMO_ENDPOINT") {
        cfg.endpoint = endpoint;
    }
    if let Ok(model) = std::env::var("ADJ_DEMO_MODEL") {
        cfg.model = model;
    }
    if let Ok(source) = std::env::var("ADJ_DEMO_SOURCE") {
        cfg.source_text = source;
    }
    if let Ok(timeout_s) = std::env::var("ADJ_DEMO_TIMEOUT_SECS") {
        if let Ok(n) = timeout_s.parse::<u64>() {
            cfg.timeout = Duration::from_secs(n);
        }
    }
    cfg
}
