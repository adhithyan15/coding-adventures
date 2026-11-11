//! Live self-test 5: the full gate with every ring (negative controls and the
//! Docker ring, which reports NOT-LIVE where no daemon is reachable) on a
//! synthetic pure package. Exercises the per-control orchestration that the
//! other self-tests skip.
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
  "package": "rust/rcg-selftest-controls",
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
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("selftest-controls");
    let config = GateConfig {
        package: "rust/rcg-selftest-controls",
        manifest_json: PURE_MANIFEST,
        evidence_dir: Some(dir.clone()),
        prepare: clean,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
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
        assert!(!evidence.contains("skipped by configuration"), "{evidence}");
        assert!(evidence.contains("\"time-vdso\""), "{evidence}");
    } else {
        assert!(evidence.contains("\"overall\": \"not-live\""), "{evidence}");
        assert!(evidence.contains("RCG010"));
    }
}
