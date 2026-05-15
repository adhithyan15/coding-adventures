//! adj_pr6_bench — single-cell driver for the ADJ25 / ADJ26 foundation
//! bench.
//!
//! The Python harness `scripts/adj_pr6_foundation_bench.py` iterates
//! the 8-declaration × 5-model matrix and shells out to this binary
//! per cell. Each cell receives source text + a model name + an
//! endpoint via env vars, runs `decompose_hierarchical` against a
//! real Ollama gateway, runs the per-level coverage check on the
//! produced IR, and emits one JSON record to stdout.
//!
//! Env-var contract (all required unless marked):
//!
//!   ADJ_PR6_SOURCE          — source text to decompose
//!   ADJ_PR6_MODEL           — ollama model id (e.g. "gemma4:latest")
//!   ADJ_PR6_ENDPOINT        — ollama endpoint (e.g. "http://127.0.0.1:11434")
//!   ADJ_PR6_TIMEOUT_SECS    — per-call timeout, defaults to 300
//!   ADJ_PR6_MAX_RETRIES     — per-parent retry budget, defaults to 3
//!   ADJ_PR6_DOCUMENT_ID     — stable doc id, defaults to "doc-bench"
//!
//! Output: one JSON object on stdout. Schema:
//!
//! ```json
//! {
//!   "model": "gemma4:latest",
//!   "source": "1 carry-on bag, matches.",
//!   "wallclock_secs": 42.3,
//!   "total_llm_calls": 7,
//!   "retry_calls": 1,
//!   "ir_summary": {
//!     "node_count": 14,
//!     "edge_count": 13,
//!     "kinds_present": ["Document", "Sentence", "Phrase", ...]
//!   },
//!   "per_level_coverage": [
//!     { "level": "DocumentToSentence", "passed": true, "gap_count": 0 },
//!     ...
//!   ],
//!   "correlation_completeness": "pass" | { "missing": "..." },
//!   "error": null
//! }
//! ```
//!
//! Errors are reported in the `error` field; the binary always
//! exits 0 so the harness can capture the JSON even on failure.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use adjudication_coverage::{
    check_hierarchical_coverage, DecompLevel, Document as CovDocument,
    HierarchicalCoverageResult,
};
use adjudication_ir::check_correlation_completeness;
use adjudication_pipeline::{
    decompose_hierarchical, HierarchicalDecomposeError, HierarchicalDecomposeRequest,
    PerLevelRetryBudget, DEFAULT_MAX_RETRIES_PER_PARENT,
};
use llm_primitives::{GatewayConfig, Role};
use llm_provider_ollama::OllamaClient;
use serde_json::json;

fn main() {
    let source = match std::env::var("ADJ_PR6_SOURCE") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            emit_error("missing ADJ_PR6_SOURCE env var");
            return;
        }
    };
    let model = match std::env::var("ADJ_PR6_MODEL") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            emit_error("missing ADJ_PR6_MODEL env var");
            return;
        }
    };
    let endpoint = match std::env::var("ADJ_PR6_ENDPOINT") {
        Ok(s) if !s.is_empty() => s,
        _ => "http://127.0.0.1:11434".to_string(),
    };
    let timeout_secs: u64 = std::env::var("ADJ_PR6_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    // ADJ29: per-level retry budgets. The harness keeps a single
    // `ADJ_PR6_MAX_RETRIES` env knob for uniform budgets (back-compat
    // with prior bench runs) AND adds four per-level overrides
    // (`ADJ_PR6_MAX_RETRIES_<LEVEL>`) so a careful-decomposition bench
    // can give deeper levels more headroom. When the per-level
    // overrides are unset, the default budgets (3/4/5/8) apply.
    fn lookup(var: &str, default: usize) -> usize {
        std::env::var(var).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
    }
    let max_retries = if std::env::var("ADJ_PR6_MAX_RETRIES").is_ok() {
        // Uniform mode: every level gets the same cap from the
        // single legacy env knob.
        PerLevelRetryBudget::uniform(lookup(
            "ADJ_PR6_MAX_RETRIES",
            DEFAULT_MAX_RETRIES_PER_PARENT,
        ))
    } else {
        // Per-level mode: take defaults (3/4/5/8) unless individual
        // overrides are set.
        let d = PerLevelRetryBudget::default();
        PerLevelRetryBudget {
            document_to_sentence: lookup("ADJ_PR6_MAX_RETRIES_DOC_SENT", d.document_to_sentence),
            sentence_to_phrase: lookup("ADJ_PR6_MAX_RETRIES_SENT_PHRASE", d.sentence_to_phrase),
            phrase_to_claim: lookup("ADJ_PR6_MAX_RETRIES_PHRASE_CLAIM", d.phrase_to_claim),
            fact_to_typed_component: lookup(
                "ADJ_PR6_MAX_RETRIES_FACT_TYPED",
                d.fact_to_typed_component,
            ),
        }
    };
    let document_id = std::env::var("ADJ_PR6_DOCUMENT_ID")
        .unwrap_or_else(|_| "doc-bench".to_string());

    let client = OllamaClient::new(model.clone())
        .with_endpoint(endpoint.clone())
        .with_timeout(Duration::from_secs(timeout_secs));
    let gateway = GatewayConfig::new().with_client(Role::Extractor, Box::new(client));

    let req = HierarchicalDecomposeRequest {
        document_id,
        source_text: source.clone(),
        max_retries_per_parent: max_retries,
    };

    let start = Instant::now();
    let now_clock = || chrono_iso_now();
    let outcome = decompose_hierarchical(&req, &gateway, now_clock);
    let elapsed = start.elapsed().as_secs_f64();

    match outcome {
        Ok(out) => {
            let cov_doc = CovDocument {
                id: out.ir_document.document_id.clone(),
                normalized_text: source.clone(),
            };
            let cov = check_hierarchical_coverage(&cov_doc, &out.ir_document);
            let cov_summary = summarise_coverage(&cov);
            let kinds_present = collect_kinds(&out.ir_document);
            let correlation = match check_correlation_completeness(&out.ir_document) {
                Ok(_) => json!("pass"),
                Err(e) => json!({ "missing": e.to_string() }),
            };
            let record = json!({
                "model": model,
                "source": source,
                "wallclock_secs": elapsed,
                "total_llm_calls": out.total_llm_calls,
                "retry_calls": out.retry_calls,
                "ir_summary": {
                    "node_count": out.ir_document.nodes.len(),
                    "edge_count": out.ir_document.edges.len(),
                    "kinds_present": kinds_present,
                },
                "per_level_coverage": cov_summary,
                "correlation_completeness": correlation,
                "error": serde_json::Value::Null,
            });
            println!("{}", record);
        }
        Err(e) => {
            let kind = match &e {
                HierarchicalDecomposeError::Primitive { level, .. } => {
                    format!("primitive_at_{:?}", level)
                }
                HierarchicalDecomposeError::UnparseableResponse { level, .. } => {
                    format!("unparseable_at_{:?}", level)
                }
                HierarchicalDecomposeError::CoverageUnresolved { gaps } => {
                    format!("coverage_unresolved({} gap(s))", gaps.len())
                }
            };
            let record = json!({
                "model": model,
                "source": source,
                "wallclock_secs": elapsed,
                "error": {
                    "kind": kind,
                    "message": e.to_string(),
                }
            });
            println!("{}", record);
        }
    }
}

fn emit_error(msg: &str) {
    let record = json!({ "error": { "kind": "harness_error", "message": msg } });
    println!("{}", record);
}

/// ISO-8601 timestamp without bringing in chrono — uses
/// `std::time::SystemTime` and a hand-rolled formatter so the
/// audit-trail discipline (every dialogue turn timestamped) is
/// honoured without adding a heavy dep. Format:
/// `YYYY-MM-DDTHH:MM:SSZ` (seconds resolution is enough for replay).
fn chrono_iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = epoch_secs_to_utc(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// Convert Unix epoch seconds → broken-down UTC. Uses the standard
/// civil-from-days algorithm (Howard Hinnant's date library). Bounded
/// to a 64-bit input; no overflow on any realistic timestamp.
fn epoch_secs_to_utc(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;
    // 719_468 = days from 0000-03-01 to 1970-01-01. We shift to a
    // calendar starting March so leap-day fits at the end of the year.
    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe.wrapping_sub(doe / 1_460).wrapping_add(doe / 36_524).wrapping_sub(doe / 146_096)) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, hour, minute, second)
}

fn summarise_coverage(result: &HierarchicalCoverageResult) -> serde_json::Value {
    let mut per_level: Vec<(DecompLevel, usize)> = vec![
        (DecompLevel::DocumentToSentence, 0),
        (DecompLevel::SentenceToPhrase, 0),
        (DecompLevel::PhraseToClaim, 0),
        (DecompLevel::FactToTypedComponent, 0),
    ];
    let mut flatten_count = 0usize;
    match result {
        HierarchicalCoverageResult::Pass => {}
        HierarchicalCoverageResult::Fail { gaps } => {
            for gap in gaps {
                // Flattening violations are reported under the
                // FactToTypedComponent level by check_hierarchical_coverage;
                // count them separately for clarity.
                if matches!(
                    gap.kind,
                    adjudication_coverage::HierarchicalGapKind::FlattenedAtom { .. }
                ) {
                    flatten_count += 1;
                    continue;
                }
                for (lvl, count) in per_level.iter_mut() {
                    if *lvl == gap.level {
                        *count += 1;
                    }
                }
            }
        }
    }
    let entries: Vec<serde_json::Value> = per_level
        .iter()
        .map(|(lvl, count)| {
            json!({
                "level": format!("{:?}", lvl),
                "passed": *count == 0,
                "gap_count": *count,
            })
        })
        .collect();
    json!({
        "by_level": entries,
        "flattening_gaps": flatten_count,
        "overall_pass": matches!(result, HierarchicalCoverageResult::Pass),
    })
}

fn collect_kinds(doc: &adjudication_ir::IRDocument) -> Vec<String> {
    let mut s: BTreeSet<String> = BTreeSet::new();
    for n in &doc.nodes {
        s.insert(format!("{:?}", n.kind));
    }
    s.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970_01_01() {
        let (y, m, d, h, mi, s) = epoch_secs_to_utc(0);
        assert_eq!((y, m, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_one_day_is_1970_01_02() {
        let (y, m, d, _, _, _) = epoch_secs_to_utc(86_400);
        assert_eq!((y, m, d), (1970, 1, 2));
    }

    #[test]
    fn summarise_coverage_counts_per_level() {
        let result = HierarchicalCoverageResult::Pass;
        let v = summarise_coverage(&result);
        assert_eq!(v["overall_pass"], serde_json::Value::Bool(true));
        assert_eq!(v["flattening_gaps"], serde_json::Value::Number(0.into()));
        assert_eq!(v["by_level"].as_array().unwrap().len(), 4);
    }
}
