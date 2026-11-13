//! Live self-test 7: a broken observer must fail the gate (fail closed).
//!
//! If `strace` produced no trace for an observed run, "no violations seen"
//! would be a statement about nothing. The gate must instead fail with
//! `RCG013` (observer unavailable or trace unparsable), in non-strict mode
//! too, because this is a broken observer, not a platform that cannot run
//! the gate.
//!
//! The test puts a shim called `strace` first on PATH. The shim answers the
//! gate's start-up probe (`... -- /bin/true`) with the real strace, and then
//! either (a) runs every traced program with NO trace file, or (b) does that
//! only for the negative-control runs and lets the main run be traced
//! properly. Where there is no real strace the gate is honestly NOT-LIVE and
//! the test has nothing to prove.
//!
//! `harness = false`: the binary is also the child the gate re-executes.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use std::path::PathBuf;
use std::process::Command;

const PURE_MANIFEST: &str = r#"{
  "version": 1,
  "package": "rust/rcg-selftest-fail-closed",
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

fn out_dir(tag: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("selftest-fail-closed-{tag}"))
}

fn run_gate(tag: &str) -> i32 {
    let config = GateConfig {
        package: "rust/rcg-selftest-fail-closed",
        manifest_json: PURE_MANIFEST,
        evidence_dir: Some(out_dir(tag)),
        prepare: clean,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    run_package_gate(&config)
}

fn find_strace() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join("strace"))
        .find(|p| p.is_file())
}

/// Write the shim; `trace_main` says whether the main run keeps a real trace.
fn write_shim(dir: &std::path::Path, real: &std::path::Path, trace_main: bool) {
    std::fs::create_dir_all(dir).unwrap();
    let keep = if trace_main {
        r#"*"--rcg-child main-"*|*"/bin/true"*"#
    } else {
        r#"*"/bin/true"*"#
    };
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in\n  {keep}) exec {real} \"$@\" ;;\nesac\n\
         while [ \"$1\" != \"--\" ]; do shift; done\nshift\nexec \"$@\"\n",
        real = real.display()
    );
    let path = dir.join("strace");
    std::fs::write(&path, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(tag) = std::env::var_os("RCG_SELFTEST_ROLE") {
        std::process::exit(run_gate(&tag.to_string_lossy()));
    }
    if args.iter().any(|a| a.starts_with("--rcg-")) {
        std::process::exit(run_gate("child"));
    }
    if !cfg!(target_os = "linux") {
        return;
    }
    let Some(real) = find_strace() else {
        println!("no strace here: gate is NOT-LIVE, nothing to prove");
        return;
    };
    let exe = std::env::current_exe().unwrap();
    for (tag, trace_main) in [("no-trace-at-all", false), ("controls-only", true)] {
        let shim_dir = out_dir(tag).join("shim");
        write_shim(&shim_dir, &real, trace_main);
        let path = std::env::join_paths(
            std::iter::once(shim_dir.clone())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let out = Command::new(&exe)
            .env("PATH", path)
            .env("RCG_SELFTEST_ROLE", tag)
            .env_remove("RCG_STRICT") // non-strict: must still fail
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(!out.status.success(), "{tag}: gate passed blind:\n{text}");
        if tag == "no-trace-at-all" {
            assert!(
                text.contains("RCG013: observer produced no trace (main run"),
                "{tag}: main-run failure must carry RCG013:\n{text}"
            );
        } else {
            assert!(
                text.contains("RCG013: observer produced no trace (control fs-open"),
                "{tag}: control failure must carry RCG013:\n{text}"
            );
            assert!(
                !text.contains("RCG013: observer produced no trace (main run"),
                "{tag}: main run was traced properly:\n{text}"
            );
        }
    }
}
