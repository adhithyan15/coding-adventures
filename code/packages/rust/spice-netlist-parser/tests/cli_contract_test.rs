use std::fs;

use spice_netlist_parser::{run_netlist_json, CLI_ERROR_CODE, CLI_RESULT_SCHEMA_VERSION};

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

#[test]
fn shared_release_manifest_freezes_the_berkeley_v1_gate() {
    let grammar_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../grammars/spice/");
    let release: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(format!("{grammar_root}berkeley-v1-release-manifest.json")).unwrap(),
    )
    .unwrap();
    let core: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(format!("{grammar_root}berkeley-v1-op-corpus.json")).unwrap(),
    )
    .unwrap();
    let syntax: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(format!("{grammar_root}berkeley-v1-syntax-corpus.json")).unwrap(),
    )
    .unwrap();
    let cli: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(format!("{grammar_root}berkeley-v1-cli-corpus.json")).unwrap(),
    )
    .unwrap();
    let gate = &release["corpusGate"];

    assert_eq!(release["schemaVersion"], 1);
    assert_eq!(release["suite"], "berkeley-v1-release");
    assert!(
        core["cases"].as_array().unwrap().len()
            >= gate["minimumCoreCaseCount"].as_u64().unwrap() as usize
    );
    let analyses = core["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["analysis"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let devices = core["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["kind"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for required in gate["requiredAnalysisKinds"].as_array().unwrap() {
        assert!(analyses.contains(required.as_str().unwrap()));
    }
    for required in gate["requiredDeviceKinds"].as_array().unwrap() {
        assert!(devices.contains(required.as_str().unwrap()));
    }
    assert!(
        syntax["cases"].as_array().unwrap().len()
            >= gate["minimumSyntaxCaseCount"].as_u64().unwrap() as usize
    );
    let classifications = syntax["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["classification"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for required in gate["requiredSyntaxClassifications"].as_array().unwrap() {
        assert!(classifications.contains(required.as_str().unwrap()));
    }
    assert!(
        cli["cases"].as_array().unwrap().len()
            >= gate["minimumCliSuccessCaseCount"].as_u64().unwrap() as usize
    );
    assert!(
        cli["failureCases"].as_array().unwrap().len()
            >= gate["minimumCliFailureCaseCount"].as_u64().unwrap() as usize
    );
    assert_eq!(
        release["cli"]["result"]["schemaVersion"],
        CLI_RESULT_SCHEMA_VERSION
    );
    assert_eq!(release["cli"]["failureDiagnostic"]["code"], CLI_ERROR_CODE);
}
