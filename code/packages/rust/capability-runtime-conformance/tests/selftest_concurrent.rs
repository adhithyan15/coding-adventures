//! Live self-test 6: two gates sharing one `evidence_dir` at the same time.
//!
//! Regression for the CI-only failure of `rust/gf256` (PR #19314). The Rust
//! adopters all pass `CARGO_TARGET_TMPDIR/capability-runtime`, which is one
//! directory per *workspace*, and the build tool runs independent packages
//! in parallel. The gate used fixed `rcg-scratch` / `rcg-container`
//! subdirectories and `remove_dir_all`'d them when it finished, so a gate
//! that finished first deleted the directory a slower gate was still writing
//! its strace traces into. `strace -o` then failed, every observed run came
//! back with no trace, and RCG009 reported io-uring / bpf / env-leak (whichever
//! controls were still to run) as "NOT detected".
//!
//! This binary launches itself twice, staggered so that the first gate ends
//! while the second is in the middle of its negative controls, against ONE
//! shared evidence directory. Both must pass. (Where strace is absent both
//! gates are honestly NOT-LIVE and exit 0, like the other self-tests.)
//!
//! `harness = false`: the binary is also the child the gate re-executes.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const PURE_MANIFEST: &str = r#"{
  "version": 1,
  "package": "rust/rcg-selftest-concurrent",
  "capabilities": [],
  "justification": "Synthetic pure package used to self-test the runtime gate."
}"#;

fn clean() -> Exercise {
    Box::new(|| {
        vec![Check {
            name: "sum".into(),
            ok: (0..1000u64).sum::<u64>() == 499_500,
        }]
    })
}

fn shared_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("selftest-concurrent")
}

fn run_gate() -> i32 {
    let config = GateConfig {
        package: "rust/rcg-selftest-concurrent",
        manifest_json: PURE_MANIFEST,
        evidence_dir: Some(shared_dir()),
        prepare: clean,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    run_package_gate(&config)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_gate_child = args.iter().any(|a| a.starts_with("--rcg-"));
    if is_gate_child || std::env::var_os("RCG_SELFTEST_ROLE").is_some() {
        std::process::exit(run_gate());
    }

    let exe = std::env::current_exe().expect("current_exe");
    let spawn = |tag: &str| {
        Command::new(&exe)
            .env("RCG_SELFTEST_ROLE", tag)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn gate")
    };
    let first = spawn("first");
    std::thread::sleep(Duration::from_millis(400));
    let second = spawn("second");
    for (name, child) in [("first", first), ("second", second)] {
        let out = child.wait_with_output().expect("wait");
        assert!(
            out.status.success(),
            "{name} concurrent gate failed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
