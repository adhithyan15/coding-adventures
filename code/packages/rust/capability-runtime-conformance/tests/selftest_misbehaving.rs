//! Live self-test 2: a pure package that misbehaves *after arming* must be
//! rejected by the real gate, by both rings, with the registered failure id.
//!
//! This is the gate-level counterpart of the per-category negative controls:
//! it proves a failing package yields a failing exit code and failing
//! evidence, not just that the machinery can detect syscalls.

#[cfg(target_os = "linux")]
mod live {
    use capability_runtime_conformance::sabotage;
    use capability_runtime_conformance::{
        run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
    };
    use std::path::PathBuf;

    const PURE_MANIFEST: &str = r#"{
      "version": 1,
      "package": "rust/rcg-selftest-bad",
      "capabilities": [],
      "justification": "Synthetic pure package that misbehaves, to self-test the gate."
    }"#;

    /// "Library" code that opens a file although the manifest is empty.
    fn misbehaving() -> Exercise {
        Box::new(|| {
            sabotage::attempt("fs-open", "selftest");
            vec![Check {
                name: "survived".into(),
                ok: true,
            }]
        })
    }

    pub fn run() {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("selftest-bad");
        let config = GateConfig {
            package: "rust/rcg-selftest-bad",
            manifest_json: PURE_MANIFEST,
            evidence_dir: Some(dir.clone()),
            prepare: misbehaving,
            options: PolicyOptions::default(),
            rings: Rings::MAIN_ONLY,
        };
        let code = run_package_gate(&config);
        let evidence = std::fs::read_to_string(dir.join("capability_runtime_evidence.json"))
            .expect("evidence file written");
        if evidence.contains("\"overall\": \"not-live\"") {
            // No usable strace here: honest not-live, nothing to assert.
            return;
        }
        assert_eq!(code, 1, "a misbehaving package must fail the gate");
        assert!(evidence.contains("\"overall\": \"fail\""), "{evidence}");
        assert!(
            evidence.contains("RCG001"),
            "enforcement must report SIGSYS: {evidence}"
        );
        assert!(
            evidence.contains("\"syscall\": \"openat\""),
            "observer must name the call: {evidence}"
        );
        assert!(evidence.contains("\"agreement\": true"));
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    live::run();
    #[cfg(not(target_os = "linux"))]
    println!("not-live: runtime capability gate is Linux-only (RCG010)");
}
