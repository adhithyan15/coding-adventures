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
    format_side_by_side, run_pipeline_arm, run_raw_arm, DemoConfig, IrMode, IrSourceTelemetry,
};

fn main() {
    let cfg = config_from_env();

    println!(
        "Running TSA demo against {endpoint}\n\
         primary model: `{model}` (Extractor/Renderer/Nli/Plausibility)\n\
         adversary:     {adv}\n\
         IR mode:       {mode:?}\n\
         (override via ADJ_DEMO_{{ENDPOINT,MODEL,ADVERSARY_MODEL,SOURCE,IR_MODE}})\n",
        endpoint = cfg.endpoint,
        model = cfg.model,
        adv = cfg
            .adversary_model
            .as_deref()
            .map(|s| format!("`{s}` (Adversary)"))
            .unwrap_or_else(|| "(none; ADJ05 will Skip)".to_string()),
        mode = cfg.ir_mode,
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

    // --- optional audit-trail + LLM-IR dump ---
    if std::env::var("ADJ_DEMO_AUDIT").is_ok() {
        if let IrSourceTelemetry::LlmExtracted {
            raw_ir_json,
            converter_warnings,
            ..
        } = &pipe.ir_source
        {
            println!("--- LLM-extracted IR (raw decompose_text output) ---");
            println!("{raw_ir_json}");
            if !converter_warnings.is_empty() {
                println!("\n--- JSON-to-IR converter warnings ---");
                for w in converter_warnings {
                    println!("  - {w}");
                }
            }
            println!();
        }
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
    if let Ok(adv) = std::env::var("ADJ_DEMO_ADVERSARY_MODEL") {
        cfg.adversary_model = if adv.is_empty() { None } else { Some(adv) };
    }
    if let Ok(n) = std::env::var("ADJ_DEMO_MAX_CLARIFY_ATTEMPTS") {
        if let Ok(parsed) = n.parse::<usize>() {
            cfg.max_clarification_attempts = parsed;
        }
    }
    if let Ok(mode) = std::env::var("ADJ_DEMO_IR_MODE") {
        cfg.ir_mode = match mode.to_ascii_lowercase().as_str() {
            "llm" | "llm-extracted" | "llm_extracted" | "llmextracted" => IrMode::LlmExtracted,
            "hand" | "hand-built" | "hand_built" | "handbuilt" | "fixture" => IrMode::HandBuilt,
            other => {
                eprintln!(
                    "warning: ADJ_DEMO_IR_MODE={other:?} not recognized; \
                     using HandBuilt. Accepted values: 'hand', 'llm'."
                );
                IrMode::HandBuilt
            }
        };
    }
    cfg
}
