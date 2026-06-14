//! Clinical-note adjudication demo binary.
//!
//! ```text
//! ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 cargo run -p adjudication-clinical-demo
//! ```
//!
//! Env vars (mirror `adjudication-tsa-demo`):
//! - `ADJ_DEMO_MODEL`            — primary model (default `gemma4:latest`).
//! - `ADJ_DEMO_ADVERSARY_MODEL`  — second-family model for ADJ05.
//! - `ADJ_DEMO_ENDPOINT`         — Ollama endpoint (`http://127.0.0.1:11434` on macOS).
//! - `ADJ_DEMO_SOURCE`           — override the canonical assessment text.
//! - `ADJ_DEMO_CACHE_DIR`        — enable disk-persisted prompt cache.
//! - `ADJ_DEMO_TIMEOUT_SECS`     — per-call HTTP timeout (default 120s).
//! - `ADJ_DEMO_AUDIT=1`          — also dump the full ADJ07 audit trail.

use std::time::Duration;

use adjudication_clinical_demo::{
    format_side_by_side, run_pipeline_arm, run_raw_arm, DemoConfig,
};

fn main() {
    let cfg = config_from_env();

    println!(
        "Running clinical demo against {endpoint}\n\
         primary model: `{model}` (Extractor/Renderer/Nli/Plausibility)\n\
         adversary:     {adv}\n\
         cache:         {cache}\n\
         (override via ADJ_DEMO_{{ENDPOINT,MODEL,ADVERSARY_MODEL,SOURCE,CACHE_DIR}})\n",
        endpoint = cfg.endpoint,
        model = cfg.model,
        adv = cfg
            .adversary_model
            .as_deref()
            .map(|s| format!("`{s}` (Adversary)"))
            .unwrap_or_else(|| "(none; ADJ05 will Skip)".to_string()),
        cache = cfg
            .cache_dir
            .as_deref()
            .map(|s| format!("disk persistence at {s}"))
            .unwrap_or_else(|| "in-memory only".to_string()),
    );

    println!("[arm A] asking the raw model directly...");
    let raw = match run_raw_arm(&cfg) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "Arm A failed: {e}\n\nHint: is Ollama running? Try `ollama serve` and \
                 `ollama pull {model}` first.",
                model = cfg.model,
            );
            std::process::exit(1);
        }
    };

    println!("[arm B] running the structured pipeline...");
    let pipe = run_pipeline_arm(&cfg);

    println!("\n{}", format_side_by_side(&raw, &pipe));

    if std::env::var("ADJ_DEMO_AUDIT").is_ok() {
        match serde_json::to_string_pretty(&pipe.pipeline_output.audit_trail) {
            Ok(json) => println!("--- full audit trail (ADJ07-v1) ---\n{json}"),
            Err(e) => eprintln!("warning: failed to serialize audit trail: {e}"),
        }
    }
}

fn config_from_env() -> DemoConfig {
    let mut cfg = DemoConfig::default();
    if let Ok(v) = std::env::var("ADJ_DEMO_ENDPOINT") {
        cfg.endpoint = v;
    }
    if let Ok(v) = std::env::var("ADJ_DEMO_MODEL") {
        cfg.model = v;
    }
    if let Ok(v) = std::env::var("ADJ_DEMO_SOURCE") {
        cfg.source_text = v;
    }
    if let Ok(v) = std::env::var("ADJ_DEMO_ADVERSARY_MODEL") {
        cfg.adversary_model = if v.is_empty() { None } else { Some(v) };
    }
    if let Ok(v) = std::env::var("ADJ_DEMO_TIMEOUT_SECS") {
        if let Ok(n) = v.parse::<u64>() {
            cfg.timeout = Duration::from_secs(n);
        }
    }
    if let Ok(v) = std::env::var("ADJ_DEMO_CACHE_DIR") {
        cfg.cache_dir = if v.is_empty() { None } else { Some(v) };
    }
    cfg
}
