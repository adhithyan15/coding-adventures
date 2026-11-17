// Shared by tests/wxg_evidence.rs, tests/wxg_vulnerable.rs and
// tests/wxg_wasteful.rs: run the generated suite of tests/fixtures/wxg.warrant
// once per process against whatever `harness::wxg_subject` is, and keep the
// outcomes, the TAP text and the manifest.  (Run once: every trip of the
// allocation observer abandons a worker thread, and the suite's own negative
// controls trip it on purpose, so repeated full runs in one process would spend
// the process-wide abandonment budget that honest tests need.)
//
// The including file defines `mod harness` (the subject plus the generated
// file) before it includes this one.

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
        coding_adventures_test_bootstrap::Verdict::Fail { failure_id, message } => format!("{failure_id}: {message}"),
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
        let out = std::env::temp_dir().join(format!("wxg_manifest_{}.json", std::process::id()));
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

pub fn outcome(id_part: &str) -> &'static Outcome {
    run_once()
        .outcomes
        .iter()
        .find(|o| o.id.contains(id_part))
        .unwrap_or_else(|| panic!("no test whose id contains {id_part:?}"))
}
