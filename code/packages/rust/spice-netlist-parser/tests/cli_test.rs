use std::{
    env, fs,
    process::{self, Command},
};

#[test]
fn cli_runs_a_deck_file_as_json() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let deck = corpus["cases"][0]["deck"].as_str().unwrap();
    let path = env::temp_dir().join(format!("spice-netlist-parser-cli-{}.cir", process::id()));
    fs::write(&path, deck).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_spice-netlist-parser"))
        .args(["run", "--json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["title"], corpus["cases"][0]["expected"]["title"]);
}

#[test]
fn cli_inspects_the_complete_runnable_plan() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let deck = corpus["cases"][0]["deck"].as_str().unwrap();
    let path = env::temp_dir().join(format!(
        "spice-netlist-parser-inspect-{}.cir",
        process::id()
    ));
    fs::write(&path, deck).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_spice-netlist-parser"))
        .args(["inspect", "--json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["analyses"].as_array().unwrap().len(), 5);
    assert_eq!(payload["analyses"][4]["kind"], "tf");
}

#[test]
fn cli_reports_the_shared_stable_failure_code() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = &corpus["failureCases"][0];
    let path = env::temp_dir().join(format!(
        "spice-netlist-parser-cli-failure-{}.cir",
        process::id()
    ));
    fs::write(&path, test_case["deck"].as_str().unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_spice-netlist-parser"))
        .args(["run", "--json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();

    assert_eq!(
        output.status.code(),
        test_case["expected"]["exitStatus"]
            .as_i64()
            .map(|value| value as i32)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with(&format!(
        "{}: ",
        test_case["expected"]["diagnosticCode"].as_str().unwrap()
    )));
}
