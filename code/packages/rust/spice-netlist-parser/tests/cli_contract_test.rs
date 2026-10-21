use std::fs;

use spice_netlist_parser::{
    inspect_netlist_json, run_netlist_json, CLI_ERROR_CODE, CLI_INSPECTION_SCHEMA_VERSION,
    CLI_RESULT_SCHEMA_VERSION,
};

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
fn run_netlist_json_preserves_both_nested_dc_axes() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "nested-dc-sweep")
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    assert_eq!(payload["analyses"][0]["kind"], "dc");
    assert_eq!(
        payload["analyses"][0]["records"],
        test_case["expected"]["records"]
    );
}

#[test]
fn run_netlist_json_expands_global_temperature_cards() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "global-temperature-op")
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    let analyses = payload["analyses"].as_array().unwrap();
    assert_eq!(
        analyses
            .iter()
            .map(|analysis| analysis["index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 1]
    );
    assert_eq!(
        analyses
            .iter()
            .map(|analysis| analysis["temperatureKelvin"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![300.15, 348.15]
    );
    assert_eq!(
        payload["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|output| output["temperatureKelvin"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![300.15, 348.15]
    );
    assert_eq!(
        payload["measures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|measure| measure["temperatureKelvin"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![300.15, 348.15]
    );
    assert_eq!(
        payload["measures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|measure| measure["analysis"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["op", "op"]
    );
    assert_eq!(
        payload["measures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|measure| measure["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["temperature_output", "temperature_output"]
    );
}

#[test]
fn run_netlist_json_exposes_deck_output_artifacts() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "deck-output-artifacts")
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    assert_eq!(payload["outputs"], test_case["expected"]["outputs"]);
    assert_eq!(payload["measures"], test_case["expected"]["measures"]);
}

#[test]
fn run_netlist_json_exposes_raw_output_artifacts() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "raw-output-artifacts")
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    assert_eq!(payload["rawfiles"], test_case["expected"]["rawfiles"]);
    assert_eq!(payload["wrdata"], test_case["expected"]["wrdata"]);
}

#[test]
fn inspection_api_exposes_the_complete_runnable_plan() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = &corpus["cases"][0];
    let payload: serde_json::Value =
        serde_json::from_str(&inspect_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();

    assert_eq!(payload["schemaVersion"], CLI_INSPECTION_SCHEMA_VERSION);
    assert_eq!(
        payload["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|analysis| analysis["index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        test_case["expected"]["analysisIndexes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|index| index.as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        payload["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|analysis| analysis["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        test_case["expected"]["analysisKinds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kind| kind.as_str().unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn engine_backed_analysis_cards_are_inspectable_and_runnable() {
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../grammars/spice/berkeley-v1-cli-corpus.json"
    );
    let corpus: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus_path).unwrap()).unwrap();
    let test_case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "engine-backed-analysis-plan")
        .unwrap();
    let inspection: serde_json::Value =
        serde_json::from_str(&inspect_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();
    let result: serde_json::Value =
        serde_json::from_str(&run_netlist_json(test_case["deck"].as_str().unwrap()).unwrap())
            .unwrap();
    let expected = &test_case["expected"];

    assert_eq!(
        inspection["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|analysis| analysis["index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        expected["analysisIndexes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|index| index.as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    let expected_kinds = expected["analysisKinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        inspection["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|analysis| analysis["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_kinds
    );
    assert_eq!(
        result["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|analysis| analysis["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_kinds
    );
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
    for required in gate["requiredCoreKinds"].as_array().unwrap() {
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
