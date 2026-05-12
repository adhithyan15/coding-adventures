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
//!
//! # Inject the fixture TSA rulebook into Arm A:
//! ADJ_DEMO_RULEBOOK_MODE=fixture cargo run -p adjudication-tsa-demo
//!
//! # Inject a rulebook from a file (e.g., one elicited via
//! # adjudication_rulebook::acquire_rulebook and persisted):
//! ADJ_DEMO_RULEBOOK_MODE=path/to/rulebook.txt cargo run -p adjudication-tsa-demo
//!
//! # Elicit a rulebook from the model's own weights, then inject it
//! # into Arm A's system prompt. This is the recursive use of the
//! # framework on itself.
//! ADJ_DEMO_RULEBOOK_MODE=elicit cargo run -p adjudication-tsa-demo
//!
//! # When using elicit-mode, also dump the elicited rulebook text
//! # to stdout so you can inspect what the model produced:
//! ADJ_DEMO_RULEBOOK_MODE=elicit ADJ_DEMO_DUMP_RULEBOOK=1 \
//!     cargo run -p adjudication-tsa-demo
//!
//! # Adversarial multi-model elicitation: elicit rulebooks from N
//! # independent models, concatenate with provenance tags, inject
//! # the merged text into Arm A. A rule cited at answer time can
//! # be traced back to the specific model that produced it.
//! ADJ_DEMO_RULEBOOK_MODE=adversarial:gemma4:latest,llama3.1:8b \
//!     cargo run -p adjudication-tsa-demo
//! ```
//!
//! The binary is intentionally environment-variable driven rather
//! than `clap`-flagged so the demo crate stays zero-dep beyond what
//! the framework already pulls in.

use std::time::Duration;

use adjudication_rulebook::{
    acquire_rulebook_adversarial, AcquireRulebookAdversarialRequest, PerModelOutcome,
};
use adjudication_tsa_demo::{
    acquire_demo_rulebook, fixture_tsa_rulebook, format_side_by_side, run_pipeline_arm,
    run_raw_arm, DemoConfig, IrMode, IrSourceTelemetry,
};
use llm_gateway::LlmClient;
use llm_primitives::{GatewayConfig, Role as PrimitiveRole};
use llm_provider_ollama::OllamaClient;

fn main() {
    let mut cfg = config_from_env();
    let rb_mode = std::env::var("ADJ_DEMO_RULEBOOK_MODE").unwrap_or_default();
    let elicit_requested = rb_mode == "elicit";
    let adversarial_models: Option<Vec<String>> = if let Some(rest) =
        rb_mode.strip_prefix("adversarial:")
    {
        let models: Vec<String> = rest
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if models.is_empty() {
            None
        } else {
            Some(models)
        }
    } else {
        None
    };

    // If the user asked for ADJ_DEMO_RULEBOOK_MODE=elicit, run the
    // rulebook-acquisition phase before Arm A. The acquired rulebook
    // text gets stashed in `cfg.rulebook_text` so `run_raw_arm`
    // injects it as the system prompt. This is the recursive use of
    // the framework on itself.
    if elicit_requested {
        println!(
            "[stage 0] eliciting TSA rulebook from `{model}` via \
             adjudication_rulebook::acquire_rulebook ...",
            model = cfg.model,
        );
        let ollama = OllamaClient::new(cfg.model.clone())
            .with_endpoint(cfg.endpoint.clone())
            .with_timeout(cfg.timeout);
        let gw = GatewayConfig::new()
            .with_client(PrimitiveRole::RuleExtractor, Box::new(ollama.clone()))
            .with_client(PrimitiveRole::Extractor, Box::new(ollama));
        match acquire_demo_rulebook(&cfg, &gw) {
            Ok(rb) => {
                let validation_summary = if rb.validation_passed {
                    "OK".to_string()
                } else {
                    format!(
                        "FAILED ({})",
                        rb.validation_error
                            .as_deref()
                            .unwrap_or("(no diagnostic)")
                    )
                };
                println!(
                    "[stage 0] elicited {bytes} bytes of rulebook text from \
                     `{model}` (trust={trust}, validation={validation})",
                    bytes = rb.source_text.len(),
                    model = cfg.model,
                    trust = rb.trust.as_str(),
                    validation = validation_summary,
                );
                if std::env::var("ADJ_DEMO_DUMP_RULEBOOK").is_ok() {
                    println!(
                        "[stage 0] elicited rulebook source_text:\n\
                         ----- BEGIN RULEBOOK -----\n\
                         {text}\n\
                         ----- END RULEBOOK -----",
                        text = rb.source_text,
                    );
                }
                cfg.rulebook_text = Some(rb.source_text);
            }
            Err(e) => {
                // Print to BOTH stdout and stderr. Benchmark tooling
                // often captures only stdout; the failure must not be
                // silent. Surfacing the typed error to stdout matches
                // the success-path log line so log-grepping benchmark
                // scripts can detect either outcome.
                let msg = format!(
                    "[stage 0] rulebook elicitation FAILED: {e}\n\
                     [stage 0] continuing without rulebook (Arm A will \
                     run the v0.7 baseline behaviour)."
                );
                println!("{msg}");
                eprintln!("{msg}");
            }
        }
    }

    // Adversarial multi-model elicitation. For each model in the
    // comma-separated list, build an OllamaClient + GatewayConfig
    // and call adjudication_rulebook::acquire_rulebook_adversarial.
    // The merged provenance-tagged rulebook text becomes Arm A's
    // injected prompt — a rule cited at answer time can be traced
    // back to which model produced it.
    if let Some(models) = adversarial_models.as_ref() {
        println!(
            "[stage 0] adversarial elicitation across {n} models: {list}",
            n = models.len(),
            list = models.join(", "),
        );
        let mut gateways: Vec<(String, GatewayConfig)> = Vec::with_capacity(models.len());
        for m in models {
            let ollama: Box<dyn LlmClient> = Box::new(
                OllamaClient::new(m.clone())
                    .with_endpoint(cfg.endpoint.clone())
                    .with_timeout(cfg.timeout),
            );
            // Second client for Extractor role (decompose_text). We
            // can't .clone() Box<dyn LlmClient>, so construct a
            // sibling OllamaClient with the same config.
            let extractor: Box<dyn LlmClient> = Box::new(
                OllamaClient::new(m.clone())
                    .with_endpoint(cfg.endpoint.clone())
                    .with_timeout(cfg.timeout),
            );
            let gw = GatewayConfig::new()
                .with_client(PrimitiveRole::RuleExtractor, ollama)
                .with_client(PrimitiveRole::Extractor, extractor);
            gateways.push((m.clone(), gw));
        }
        let req = AcquireRulebookAdversarialRequest {
            document_id_prefix: "rulebook-tsa-adversarial".to_string(),
            domain: "tsa-declaration".to_string(),
            scope: Some("carry-on baggage".to_string()),
            as_of: "2026-05-12".to_string(),
            language_hint: None,
        };
        let adv = acquire_rulebook_adversarial(&req, gateways);
        println!(
            "[stage 0] adversarial elicit: {ok}/{total} models succeeded ({fail} failed)",
            ok = adv.successful_count,
            total = adv.per_model.len(),
            fail = adv.failed_count,
        );
        for outcome in &adv.per_model {
            match outcome {
                PerModelOutcome::Acquired { model_label, rulebook } => {
                    let validation_summary = if rulebook.validation_passed {
                        "OK".to_string()
                    } else {
                        format!(
                            "FAILED ({})",
                            rulebook.validation_error.as_deref().unwrap_or("(no diagnostic)")
                        )
                    };
                    println!(
                        "[stage 0]   ✓ `{model_label}`: {bytes} bytes (validation={validation})",
                        bytes = rulebook.source_text.len(),
                        validation = validation_summary,
                    );
                }
                PerModelOutcome::Failed { model_label, error_summary } => {
                    let msg = format!("[stage 0]   ✗ `{model_label}` FAILED: {error_summary}");
                    println!("{msg}");
                    eprintln!("{msg}");
                }
            }
        }
        if std::env::var("ADJ_DEMO_DUMP_RULEBOOK").is_ok() && !adv.merged_source_text.is_empty() {
            println!(
                "[stage 0] merged adversarial rulebook (provenance-tagged):\n\
                 ----- BEGIN RULEBOOK -----\n\
                 {text}\n\
                 ----- END RULEBOOK -----",
                text = adv.merged_source_text,
            );
        }
        if !adv.merged_source_text.is_empty() {
            cfg.rulebook_text = Some(adv.merged_source_text);
        } else {
            let msg = "[stage 0] adversarial elicit: ALL models failed; \
                       continuing without rulebook.";
            println!("{msg}");
            eprintln!("{msg}");
        }
    }

    println!(
        "Running TSA demo against {endpoint}\n\
         primary model: `{model}` (Extractor/Renderer/Nli/Plausibility)\n\
         adversary:     {adv}\n\
         IR mode:       {mode:?}\n\
         cache:         {cache}\n\
         (override via ADJ_DEMO_{{ENDPOINT,MODEL,ADVERSARY_MODEL,SOURCE,IR_MODE,CACHE_DIR}})\n",
        endpoint = cfg.endpoint,
        model = cfg.model,
        adv = cfg
            .adversary_model
            .as_deref()
            .map(|s| format!("`{s}` (Adversary)"))
            .unwrap_or_else(|| "(none; ADJ05 will Skip)".to_string()),
        mode = cfg.ir_mode,
        cache = cfg
            .cache_dir
            .as_deref()
            .map(|s| format!("disk persistence at {s}"))
            .unwrap_or_else(|| "in-memory only".to_string()),
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
    if let Ok(dir) = std::env::var("ADJ_DEMO_CACHE_DIR") {
        cfg.cache_dir = if dir.is_empty() { None } else { Some(dir) };
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
    // Rulebook-injection mode for Arm A. Three values:
    //   - `none` (default) — Arm A receives no rulebook; the model
    //     relies on its training data, exposing the hallucination
    //     baseline captured in ADJ12.
    //   - `fixture` — inject the canonical hardcoded TSA rulebook
    //     (`fixture_tsa_rulebook()`). Deterministic, fast.
    //   - `<path>` — read the rulebook text from that file. Useful
    //     for testing with rulebooks produced by
    //     `adjudication-rulebook::acquire_rulebook` and saved to
    //     disk.
    if let Ok(mode) = std::env::var("ADJ_DEMO_RULEBOOK_MODE") {
        cfg.rulebook_text = match mode.as_str() {
            "" | "none" => None,
            "fixture" => Some(fixture_tsa_rulebook()),
            // `elicit` and `adversarial:...` are handled later in
            // `main()` — they need a gateway, which isn't available
            // at config-load time. Leave rulebook_text = None here;
            // main() fills it in after the corresponding orchestrator
            // runs.
            "elicit" => None,
            mode if mode.starts_with("adversarial:") => None,
            path => match std::fs::read_to_string(path) {
                Ok(text) => Some(text),
                Err(e) => {
                    eprintln!(
                        "warning: ADJ_DEMO_RULEBOOK_MODE={path:?} did not \
                         read as a file ({e}); running without rulebook."
                    );
                    None
                }
            },
        };
    }
    cfg
}
