use std::fs;

use spice_netlist_parser::{run_netlist_json, CLI_RESULT_SCHEMA_VERSION};

#[test]
fn run_netlist_json_uses_the_shared_berkeley_cli_contract() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = &corpus["cases"][0];
    let payload: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    assert_eq!(payload["schemaVersion"], CLI_RESULT_SCHEMA_VERSION);
    assert_eq!(payload["title"], test_case["expected"]["title"]);
    let kinds = payload["analyses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|analysis| analysis["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(kinds, vec!["op", "dc", "ac", "tran", "tf"]);
    let record_counts = payload["analyses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|analysis| analysis["records"].as_array().unwrap().len())
        .collect::<Vec<_>>();
    assert_eq!(record_counts, vec![1, 2, 1, 1, 1]);
}
