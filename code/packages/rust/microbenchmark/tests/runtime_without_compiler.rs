use std::process::Command;

#[test]
fn installed_style_binary_reports_embedded_compiler_without_rustc_on_path() {
    let executable = env!("CARGO_BIN_EXE_array-microbenchmark");
    let output = Command::new(executable)
        .env("PATH", "")
        .env_remove("RUSTC")
        .args([
            "--pairs",
            "1",
            "--target-ns",
            "1",
            "--max-batch",
            "1",
            "--revision",
            "no-rustc-runtime-test",
        ])
        .output()
        .expect("run array-microbenchmark without a compiler on PATH");

    assert!(
        output.status.success(),
        "binary failed without rustc on PATH: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["revision"], "no-rustc-runtime-test");
    assert_eq!(report["runtime"], env!("CA_RUSTC_VERSION"));
    assert_eq!(report["pairs"].as_array().unwrap().len(), 1);
}
