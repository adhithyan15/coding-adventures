//! CCR-041 — the correlation-vector pass inventory is an *observation* of the
//! scheduler, never a second opinion about it.
//!
//! # The defect this pins shut
//!
//! `closurec` used to carry two hand-maintained constants, `SIMPLE_PASS_NAMES`
//! and `ADVANCED_PASS_NAMES`, listing the passes each compilation level runs.
//! They sat a few hundred lines away from the `pipeline.add(...)` calls that
//! actually register passes, and nothing tied the two together. They drifted:
//!
//! ```text
//!   registration (run.rs)                 SIMPLE_PASS_NAMES (run.rs)
//!   ---------------------                 --------------------------
//!   if advanced.is_some() {               "constant-fold"
//!       pipeline.add(InlinePass)          "fold-control-flow"
//!   }                                     "dce"
//!                                         "inline"     ← never ran at SIMPLE
//!                                         "inline-variables"
//!                                         "rename"
//! ```
//!
//! So every SIMPLE run emitted a provenance record naming a pass that did not
//! execute. For a compiler whose distinguishing feature is traceability, that
//! is worse than emitting nothing: a wrong answer is indistinguishable from a
//! right one to the tool consuming it.
//!
//! The fix is structural rather than corrective. `PassPipeline::run` already
//! reports the schedule it used in `PipelineOutput::execution_order`, so that
//! value is now threaded out and written straight into the trace. There is no
//! longer a second list that *could* drift — the gating lives in exactly one
//! place, and this site can only report what it did.
//!
//! # What these tests assert, and what they deliberately do not
//!
//! They assert the properties the fix is responsible for: a conditionally
//! registered pass appears in the trace exactly when it was registered, and
//! both levels agree with the scheduler. They do **not** assert that the
//! execution order matches the registration order — because it does not. The
//! scheduler's Kahn queue is FIFO, so a pass declaring no `depends_on` is
//! scheduled ahead of every dependent pass no matter where it was registered;
//! `rename` is registered eighth and executes second. That is a real defect in
//! `closure-pass-pipeline` (tracked separately), and it is one the old
//! constants were also concealing. Pinning the true order here is what keeps
//! it visible until it is fixed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Compile `source` at `level` with CV tracing on and return the `passes`
/// array recorded in the sidecar.
fn traced_passes(tag: &str, level: &str, source: &str) -> Vec<String> {
    // Same temp-dir discipline as `tests/conformance.rs`. A fixed name under a
    // shared `/tmp` is both a local-attacker hazard (another user pre-creates
    // the directory with `out.js.cv.json` symlinked somewhere we can write, and
    // a sticky-bit `/tmp` stops us removing it) and a straightforward collision
    // between parallel `cargo test` threads. pid + a monotonic counter makes
    // the path per-invocation distinct, and `create_dir` — not `create_dir_all`
    // — fails rather than adopting a directory somebody else already owns.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec_ccr041_{}_{}_{tag}_{level}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir(&dir).expect("create temp dir");
    let input = dir.join("in.js");
    let output = dir.join("out.js");
    let sidecar = dir.join("out.js.cv.json");
    std::fs::write(&input, source).expect("write input");

    let out = Command::new(BINARY)
        .arg("--js")
        .arg(&input)
        .arg("--js_output_file")
        .arg(&output)
        .arg("--compilation_level")
        .arg(level)
        .arg("--correlation_vector")
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "{level} compile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let body = std::fs::read_to_string(&sidecar).expect("read cv sidecar");
    let doc: serde_json::Value = serde_json::from_str(&body).expect("sidecar is json");
    let passes = find_passes(&doc).unwrap_or_else(|| panic!("no `passes` in sidecar: {body}"));
    let _ = std::fs::remove_dir_all(&dir);
    passes
}

/// The sidecar nests contributions; find the one carrying `meta.passes`.
fn find_passes(node: &serde_json::Value) -> Option<Vec<String>> {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(list)) =
                map.get("meta").and_then(|m| m.get("passes"))
            {
                return Some(
                    list.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                );
            }
            map.values().find_map(find_passes)
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_passes),
        _ => None,
    }
}

/// The regression itself: `inline` is registered under `if advanced.is_some()`,
/// so a SIMPLE trace must not claim it.
#[test]
fn simple_does_not_report_the_advanced_only_inline_pass() {
    let passes = traced_passes(
        "inline",
        "SIMPLE",
        "function f(a){return a+1;} var r = f(2);\n",
    );
    assert!(
        !passes.iter().any(|p| p == "inline"),
        "SIMPLE reported a pass that is only registered for ADVANCED: {passes:?}"
    );
}

/// The other half — the fix must not have simply deleted the name everywhere.
/// ADVANCED does register `inline`, so ADVANCED must still report it.
#[test]
fn advanced_does_report_the_inline_pass_it_registers() {
    let passes = traced_passes(
        "inline",
        "ADVANCED",
        "function f(a){return a+1;} var r = f(2);\n",
    );
    assert!(
        passes.iter().any(|p| p == "inline"),
        "ADVANCED registers InlinePass but did not report it: {passes:?}"
    );
}

/// The three other ADVANCED-gated passes travel with `inline`: absent at
/// SIMPLE (open-world — removing a global another script may read is a
/// miscompile), present at ADVANCED.
#[test]
fn closed_world_passes_are_advanced_only_in_the_trace() {
    let src = "function unused(){return 1;} var dead = 2; var live = 3; console.log(live);\n";
    let simple = traced_passes("closed", "SIMPLE", src);
    let advanced = traced_passes("closed", "ADVANCED", src);

    for gated in ["remove-unused-vars", "treeshake", "rename-globals"] {
        assert!(
            !simple.iter().any(|p| p == gated),
            "SIMPLE must not report closed-world pass `{gated}`: {simple:?}"
        );
        assert!(
            advanced.iter().any(|p| p == gated),
            "ADVANCED must report closed-world pass `{gated}`: {advanced:?}"
        );
    }
}

/// `rename-properties` is the conditional-within-conditional case: registered
/// only at ADVANCED *and* only when `--externs` supplied the property
/// boundary. Without externs it must be absent even at ADVANCED.
#[test]
fn rename_properties_is_absent_without_the_externs_property_boundary() {
    let passes = traced_passes("props", "ADVANCED", "var o = {k: 1}; console.log(o.k);\n");
    assert!(
        !passes.iter().any(|p| p == "rename-properties"),
        "rename-properties must not appear without --externs: {passes:?}"
    );
}

/// Every reported name must be a pass the build actually has. A typo or a
/// stale name in the trace is the class of bug CCR-041 removed, so it is worth
/// one assertion that the inventory stays a closed set.
#[test]
fn every_reported_pass_is_a_known_pass() {
    let known = [
        "constant-fold",
        "fold-control-flow",
        "dce",
        "inline",
        "inline-variables",
        "remove-unused-vars",
        "treeshake",
        "rename",
        "rename-globals",
        "rename-properties",
    ];
    for level in ["SIMPLE", "ADVANCED"] {
        for pass in traced_passes("known", level, "var x = 1 + 2; console.log(x);\n") {
            assert!(
                known.contains(&pass.as_str()),
                "{level} reported unknown pass `{pass}`"
            );
        }
    }
}

/// The scheduler's real order, pinned. This is NOT the registration order and
/// is not meant to be: it is what `closure-pass-pipeline` currently executes,
/// and the point of CCR-041 is that the trace says so out loud. When the
/// scheduler's FIFO tie-breaking is fixed so registration order is honoured,
/// this test should fail — and updating it is how that fix proves it changed
/// the observable schedule.
#[test]
fn trace_pins_the_schedulers_real_order() {
    let src = "var x = 1 + 2; console.log(x);\n";
    assert_eq!(
        traced_passes("order", "SIMPLE", src),
        vec![
            "constant-fold",
            "rename",
            "fold-control-flow",
            "dce",
            "inline-variables",
        ],
        "SIMPLE schedule changed"
    );
    assert_eq!(
        traced_passes("order", "ADVANCED", src),
        vec![
            "constant-fold",
            "rename",
            "rename-globals",
            "fold-control-flow",
            "dce",
            "inline",
            "inline-variables",
            "treeshake",
            "remove-unused-vars",
        ],
        "ADVANCED schedule changed"
    );
}
