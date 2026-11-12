// Shared by the WX-E evidence tests: run the generated suite of
// tests/fixtures/wxv.warrant once per process and keep the outcomes and the
// manifest.  The including file defines `mod harness` before including this one.

use std::sync::OnceLock;

use coding_adventures_test_bootstrap::run::{Options, Outcome};

pub struct RunResult {
    pub outcomes: Vec<Outcome>,
    pub failed: usize,
    pub manifest: String,
}

/// The message of a failed outcome (empty for a pass).
pub fn message(o: &Outcome) -> String {
    match &o.verdict {
        coding_adventures_test_bootstrap::Verdict::Fail { failure_id, message } => {
            format!("{failure_id}: {message}")
        }
        coding_adventures_test_bootstrap::Verdict::Pass => String::new(),
    }
}

pub fn run_once() -> &'static RunResult {
    static RUN: OnceLock<RunResult> = OnceLock::new();
    RUN.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| ()));
        let suite = harness::build_suite(harness::Fault::None);
        let mut tap = Vec::new();
        let (summary, outcomes) = suite
            .run_recorded(&Options::default(), &mut tap)
            .expect("the suite runs");
        std::panic::set_hook(previous);
        let out = std::env::temp_dir().join(format!("wxv_manifest_{}.json", std::process::id()));
        std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
        harness::write_manifest_for(&outcomes);
        std::env::remove_var("WARRANT_EVIDENCE_OUT");
        let manifest = std::fs::read_to_string(&out).unwrap_or_default();
        let _ = std::fs::remove_file(&out);
        RunResult {
            outcomes,
            failed: summary.failed,
            manifest,
        }
    })
}
