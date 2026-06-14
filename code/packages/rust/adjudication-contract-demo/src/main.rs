//! Contract-clause adjudication demo binary.

use std::time::Duration;

use adjudication_contract_demo::{
    format_side_by_side, run_pipeline_arm, run_raw_arm, DemoConfig,
};

fn main() {
    let cfg = config_from_env();
    println!(
        "Running contract demo against {endpoint}\n\
         primary model: `{model}`\n\
         adversary:     {adv}\n\
         cache:         {cache}\n",
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
                "Arm A failed: {e}\n\nHint: is Ollama running? Try `ollama pull {model}`.",
                model = cfg.model
            );
            std::process::exit(1);
        }
    };
    println!("[arm B] running the structured pipeline...");
    let pipe = run_pipeline_arm(&cfg);
    println!("\n{}", format_side_by_side(&raw, &pipe));
    if std::env::var("ADJ_DEMO_AUDIT").is_ok() {
        if let Ok(json) = serde_json::to_string_pretty(&pipe.pipeline_output.audit_trail) {
            println!("--- full audit trail (ADJ07-v1) ---\n{json}");
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
