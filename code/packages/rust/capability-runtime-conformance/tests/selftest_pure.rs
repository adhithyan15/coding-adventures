//! Live self-test 1: a synthetic *pure* package with a clean armed phase
//! passes the real gate (real seccomp, real Landlock, real strace).
//!
//! `harness = false`: the binary is also the child the gate re-executes, so
//! it needs control of `main`. On non-Linux the gate must report NOT-LIVE, and
//! this test asserts exactly that.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use std::path::PathBuf;

const PURE_MANIFEST: &str = r#"{
  "version": 1,
  "package": "rust/rcg-selftest-pure",
  "capabilities": [],
  "justification": "Synthetic pure package used to self-test the runtime gate."
}"#;

fn clean() -> Exercise {
    let data: Vec<u64> = (0..1000).collect();
    Box::new(move || {
        let sum: u64 = data.iter().sum();
        vec![Check {
            name: "sum".into(),
            ok: sum == 499_500,
        }]
    })
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("selftest-pure");
    let config = GateConfig {
        package: "rust/rcg-selftest-pure",
        manifest_json: PURE_MANIFEST,
        evidence_dir: Some(dir.clone()),
        prepare: clean,
        options: PolicyOptions::default(),
        rings: Rings::MAIN_ONLY,
    };
    let code = run_package_gate(&config);
    assert_eq!(code, 0, "a clean pure package must pass (or be not-live)");

    let evidence = std::fs::read_to_string(dir.join("capability_runtime_evidence.json"))
        .expect("evidence file written");
    if cfg!(target_os = "linux") {
        // strace may be absent on a bare Linux box: then the honest result is
        // not-live, which is also exit 0 unless RCG_STRICT is set.
        if evidence.contains("\"overall\": \"not-live\"") {
            assert!(evidence.contains("RCG013"), "{evidence}");
            return;
        }
        assert!(evidence.contains("\"overall\": \"pass\""), "{evidence}");
        assert!(evidence.contains("\"manifest_sha256\""));
        assert!(evidence.contains("\"agreement\": true"));
        assert!(evidence.contains("\"seccomp_enforced\": true"));
        assert!(evidence.contains("skipped by configuration"));
    } else {
        assert!(evidence.contains("\"overall\": \"not-live\""), "{evidence}");
        assert!(evidence.contains("RCG010"));
    }
}
