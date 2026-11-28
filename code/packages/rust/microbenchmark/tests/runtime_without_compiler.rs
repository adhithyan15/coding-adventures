use std::process::{Command, Output};

const BENCH_ARGS: &[&str] = &["--pairs", "1", "--target-ns", "1", "--max-batch", "1"];

fn run_binary(extra_args: &[&str]) -> Output {
    let executable = env!("CARGO_BIN_EXE_array-microbenchmark");
    Command::new(executable)
        .env("PATH", "")
        .env_remove("RUSTC")
        .args(BENCH_ARGS)
        .args(extra_args)
        .output()
        .expect("run array-microbenchmark without a compiler on PATH")
}

#[test]
fn installed_style_binary_reports_unknown_when_no_compiler_label_is_supplied() {
    let output = run_binary(&["--revision", "no-rustc-runtime-test"]);
    assert!(
        output.status.success(),
        "binary failed without rustc on PATH: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["revision"], "no-rustc-runtime-test");
    assert_eq!(report["runtime"], "unknown");
    assert_eq!(report["pairs"].as_array().unwrap().len(), 1);
}

#[test]
fn installed_style_binary_records_only_the_explicit_compiler_label() {
    let label = "rustc 0.0.0-test (synthetic metadata fixture)";
    let output = run_binary(&["--rustc-version", label]);
    assert!(
        output.status.success(),
        "binary rejected explicit metadata: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["runtime"], label);
    assert_eq!(report["invocation"][7], "--rustc-version");
    assert_eq!(report["invocation"][8], label);
}

#[test]
fn installed_style_binary_rejects_invalid_compiler_labels() {
    for label in [
        "",
        "unknown",
        "rustc test-version",
        "rustc 1.0.0\nforged",
        "rustc 1.0.0\u{2028}forged",
    ] {
        let output = run_binary(&["--rustc-version", label]);
        assert!(!output.status.success(), "accepted invalid label {label:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--rustc-version"),
            "missing compiler-label diagnostic for {label:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
