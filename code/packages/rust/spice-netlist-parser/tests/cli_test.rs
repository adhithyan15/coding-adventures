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
