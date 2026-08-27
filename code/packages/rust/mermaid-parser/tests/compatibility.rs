use std::collections::BTreeSet;

use mermaid_parser::{
    detect_mermaid_type, parse_any_mermaid, parse_gantt, parse_gitgraph, parse_journey, parse_pie,
    parse_quadrant_chart, parse_requirement_diagram, parse_sankey, parse_xychart,
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
const REQUIREMENT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/requirement-11.16.1-corpus.json"
));
const GITGRAPH_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gitgraph-11.16.1-corpus.json"
));
const PIE_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/pie-11.16.1-corpus.json"
));
const SANKEY_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/sankey-11.16.1-corpus.json"
));
const XYCHART_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/xychart-11.16.1-corpus.json"
));
const GANTT_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gantt-11.16.1-corpus.json"
));
const GANTT_VISUAL_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../grammars/mermaid/gantt-11.16.1-visual-corpus.json"
));

#[test]
fn pinned_gantt_supported_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(GANTT_CORPUS).expect("gantt corpus must be JSON");
    assert_eq!(corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf"));
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_gantt(source).unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(parse_gantt(source).is_err(), "invalid upstream fixture {id} unexpectedly parsed");
    }
}

#[test]
fn pinned_gantt_visual_corpus_covers_every_valid_fixture() {
    let syntax: Value = serde_json::from_str(GANTT_CORPUS).expect("Gantt corpus must be JSON");
    let visual: Value =
        serde_json::from_str(GANTT_VISUAL_CORPUS).expect("Gantt visual corpus must be JSON");
    assert_eq!(visual["upstream_commit"], syntax["upstream_commit"]);

    let valid = syntax["valid"].as_array().expect("valid corpus array");
    let expected = valid
        .iter()
        .map(|fixture| fixture["id"].as_str().expect("fixture id"))
        .collect::<BTreeSet<_>>();
    let actual = visual["fixtures"]
        .as_array()
        .expect("visual fixture array")
        .iter()
        .map(|id| id.as_str().expect("visual fixture id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), valid.len(), "visual fixture ids must be unique");
}

#[test]
fn xychart_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let xychart = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "xychart")
        .expect("xychart family");
    assert_eq!(xychart["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(XYCHART_CORPUS).expect("xychart corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_xychart_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(XYCHART_CORPUS).expect("xychart corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_xychart(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_xychart(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn sankey_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let sankey = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "sankey")
        .expect("sankey family");
    assert_eq!(sankey["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(SANKEY_CORPUS).expect("sankey corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_sankey_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(SANKEY_CORPUS).expect("sankey corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_sankey(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_sankey(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn pie_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let pie = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "pie")
        .expect("pie family");
    assert_eq!(pie["status"].as_str(), Some("full"));

    let corpus: Value = serde_json::from_str(PIE_CORPUS).expect("pie corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_pie_corpus_matches_upstream_acceptance() {
    let corpus: Value = serde_json::from_str(PIE_CORPUS).expect("pie corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_pie(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_pie(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn gitgraph_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let gitgraph = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "gitgraph")
        .expect("gitgraph family");
    assert_eq!(gitgraph["status"].as_str(), Some("full"));

    let corpus: Value =
        serde_json::from_str(GITGRAPH_CORPUS).expect("gitgraph corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"]
        .as_array()
        .expect("invalid corpus")
        .is_empty());
}

#[test]
fn pinned_gitgraph_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(GITGRAPH_CORPUS).expect("gitgraph corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_gitgraph(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_gitgraph(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

#[test]
fn requirement_full_status_is_backed_by_the_pinned_corpus() {
    let manifest: Value =
        serde_json::from_str(COMPATIBILITY_MANIFEST).expect("compatibility manifest must be JSON");
    let requirement = manifest["families"]
        .as_array()
        .expect("families array")
        .iter()
        .find(|family| family["id"] == "requirement")
        .expect("requirement family");
    assert_eq!(requirement["status"].as_str(), Some("full"));

    let corpus: Value =
        serde_json::from_str(REQUIREMENT_CORPUS).expect("requirement corpus must be JSON");
    assert!(!corpus["valid"].as_array().expect("valid corpus").is_empty());
    assert!(!corpus["invalid"].as_array().expect("invalid corpus").is_empty());
}

#[test]
fn pinned_requirement_corpus_matches_upstream_acceptance() {
    let corpus: Value =
        serde_json::from_str(REQUIREMENT_CORPUS).expect("requirement corpus must be JSON");
    assert_eq!(
        corpus["upstream_commit"].as_str(),
        Some("7ecca0cd7f1658ef74f4e7e91f925724ef403bbf")
    );
    for fixture in corpus["valid"].as_array().expect("valid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        parse_requirement_diagram(source)
            .unwrap_or_else(|error| panic!("valid upstream fixture {id} failed: {error}"));
    }
    for fixture in corpus["invalid"].as_array().expect("invalid corpus array") {
        let id = fixture["id"].as_str().expect("fixture id");
        let source = fixture["source"].as_str().expect("fixture source");
        assert!(
            parse_requirement_diagram(source).is_err(),
            "invalid upstream fixture {id} unexpectedly parsed"
        );
    }
}

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
    let error = parse_any_mermaid("mindmap\nroot((mindmap))")
        .err()
        .expect("mindmap support is not implemented yet");
    assert!(error.message.contains("recognized but not implemented"));
    assert!(error.message.contains(MERMAID_COMPATIBILITY_BASELINE));
}
