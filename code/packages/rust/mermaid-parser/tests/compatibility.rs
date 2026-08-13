use mermaid_parser::{
    detect_mermaid_type, parse_any_mermaid, parse_journey, parse_quadrant_chart,
    MERMAID_COMPATIBILITY_BASELINE,
};
use serde_json::Value;

const COMPATIBILITY_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/compatibility.json"
));
const QUADRANT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/quadrant-11.16.1-corpus.json"
));
const JOURNEY_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/journey-11.16.1-corpus.json"
));

#[test]
fn journey_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let journey = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "journey")
        .expect("journey family");
    assert_eq!(journey["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(JOURNEY_CORPUS).expect("journey corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"].as_array().expect("invalid corpus").is_empty());
}

#[test]
fn pinned_journey_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(JOURNEY_CORPUS).expect("journey corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_journey(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_journey(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn pinned_quadrant_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(QUADRANT_CORPUS).expect("quadrant corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_quadrant_chart(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_quadrant_chart(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn compatibility_manifest_matches_detector_and_dispatch() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    assert_eq!(
        manifest["upstream"]["version"].as_str(),
        Some(MERMAID_COMPATIBILITY_BASELINE)
    );

    let families = manifest["families"]
        .as_array()
        .expect("families must be an array");
    assert!(
        families.len() >= 32,
        "the Mermaid 11.16.1 baseline should enumerate every documented family"
    );

    for family in families {
        let id = family["id"].as_str().expect("family id must be a string");
        let smoke_source = family["smoke_source"]
            .as_str()
            .expect("family smoke_source must be a string");
        let detected = detect_mermaid_type(smoke_source)
            .unwrap_or_else(|error| panic!("failed to detect {id}: {error}"));
        assert_eq!(detected.canonical_id(), id);

        if matches!(family["status"].as_str(), Some("partial" | "full")) {
            parse_any_mermaid(smoke_source)
                .unwrap_or_else(|error| panic!("native family {id} must parse: {error}"));
            assert!(detected.has_native_pipeline());
        }
    }
}

#[test]
fn detection_skips_front_matter_directives_and_comments() {
    let source = r#"---
title: Example
---
%%{init: {"theme": "neutral"}}%%
%% comment
sequenceDiagram
Alice->>Bob: Hello
"#;

    let detected = detect_mermaid_type(source).expect("sequence diagram should be detected");
    assert_eq!(detected.canonical_id(), "sequence");
}

#[test]
fn recognized_but_unimplemented_family_is_not_reported_as_unknown() {
    let error = parse_any_mermaid("requirementDiagram\nrequirement test_req")
        .err()
        .expect("requirement support is not implemented yet");
    assert!(error.message.contains("recognized but not implemented"));
    assert!(error.message.contains(MERMAID_COMPATIBILITY_BASELINE));
}
