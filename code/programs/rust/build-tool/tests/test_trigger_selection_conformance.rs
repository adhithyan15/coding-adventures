use build_tool::graph_diff::{
    evaluate_test_trigger_selection, Edge, Evaluation, TestTriggerSelectionInput,
    MAX_TEST_TRIGGER_RELATIONS,
};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const FIXTURES: &[&str] = &[
    "test-trigger-selection-dev-cycle-outside-graph.json",
    "test-trigger-selection-empty.json",
    "test-trigger-selection-graph-cycle.json",
    "test-trigger-selection-one-hop.json",
    "test-trigger-selection-prerequisites-after-hop.json",
    "test-trigger-selection-unknown-relation.json",
];

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    input: FixtureInput,
    expected: FixtureExpected,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    options: FixtureOptions,
}

#[derive(Debug, Deserialize)]
struct FixtureOptions {
    packages: Vec<String>,
    edges: Vec<[String; 2]>,
    trigger_relations: Vec<[String; 2]>,
    changed_packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureExpected {
    outcome: String,
    #[serde(default)]
    result: FixtureResult,
    #[serde(default)]
    diagnostics: Vec<FixtureDiagnostic>,
}

#[derive(Debug, Default, Deserialize)]
struct FixtureResult {
    #[serde(default)]
    changed_packages: Vec<String>,
    #[serde(default)]
    affected_packages: Vec<String>,
    #[serde(default)]
    prerequisite_packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnostic {
    code: String,
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../specs/fixtures/build-tool-v1/cases")
}

fn fixture_names() -> Vec<String> {
    let mut names = fs::read_dir(fixture_root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.starts_with("test-trigger-selection-") && name.ends_with(".json"))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn edges(values: &[[String; 2]]) -> Vec<Edge> {
    values
        .iter()
        .map(|value| Edge::new(value[0].clone(), value[1].clone()))
        .collect()
}

fn input(
    packages: &[&str],
    graph_edges: &[(&str, &str)],
    trigger_relations: &[(&str, &str)],
    changed_packages: &[&str],
) -> TestTriggerSelectionInput {
    TestTriggerSelectionInput {
        packages: packages.iter().map(|value| (*value).to_owned()).collect(),
        edges: graph_edges
            .iter()
            .map(|(from, to)| Edge::new((*from).to_owned(), (*to).to_owned()))
            .collect(),
        trigger_relations: trigger_relations
            .iter()
            .map(|(from, to)| Edge::new((*from).to_owned(), (*to).to_owned()))
            .collect(),
        changed_packages: changed_packages
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

#[test]
fn dynamically_consumes_exact_test_trigger_fixture_roster() {
    assert_eq!(fixture_names(), FIXTURES);

    for name in FIXTURES {
        let fixture: Fixture = serde_json::from_slice(
            &fs::read(fixture_root().join(name)).expect("fixture must be readable"),
        )
        .expect("fixture must decode");
        let input = TestTriggerSelectionInput {
            packages: fixture.input.options.packages,
            edges: edges(&fixture.input.options.edges),
            trigger_relations: edges(&fixture.input.options.trigger_relations),
            changed_packages: fixture.input.options.changed_packages,
        };
        match evaluate_test_trigger_selection(&input) {
            Evaluation::Success(actual) => {
                assert_eq!(fixture.expected.outcome, "ok", "{}", fixture.id);
                assert_eq!(
                    actual.changed_packages, fixture.expected.result.changed_packages,
                    "{}",
                    fixture.id
                );
                assert_eq!(
                    actual.affected_packages, fixture.expected.result.affected_packages,
                    "{}",
                    fixture.id
                );
                assert_eq!(
                    actual.prerequisite_packages, fixture.expected.result.prerequisite_packages,
                    "{}",
                    fixture.id
                );
            }
            Evaluation::Failure(code) => {
                assert_eq!(fixture.expected.outcome, "error", "{}", fixture.id);
                assert_eq!(code, fixture.expected.diagnostics[0].code, "{}", fixture.id);
            }
        }
    }
}

#[test]
fn relation_ceiling_is_exact_and_precedes_relation_validation() {
    let packages = (0..129)
        .map(|index| format!("fixture/p-{index}"))
        .collect::<Vec<_>>();
    let mut relations = Vec::with_capacity(MAX_TEST_TRIGGER_RELATIONS + 1);
    'outer: for source in &packages {
        for target in &packages {
            if source == target {
                continue;
            }
            relations.push(Edge::new(source.clone(), target.clone()));
            if relations.len() == MAX_TEST_TRIGGER_RELATIONS {
                break 'outer;
            }
        }
    }
    let mut at_limit = TestTriggerSelectionInput {
        packages,
        edges: Vec::new(),
        trigger_relations: relations,
        changed_packages: Vec::new(),
    };
    assert!(matches!(
        evaluate_test_trigger_selection(&at_limit),
        Evaluation::Success(_)
    ));

    at_limit.trigger_relations.push(Edge::new(
        "fixture/unknown".to_owned(),
        "fixture/also-unknown".to_owned(),
    ));
    assert_eq!(
        evaluate_test_trigger_selection(&at_limit),
        Evaluation::Failure("TEST_TRIGGER_RELATION_LIMIT_EXCEEDED")
    );
}

#[test]
fn stable_domain_diagnostics_cover_invalid_inert_inputs() {
    let cases = [
        (
            input(&["INVALID"], &[], &[], &[]),
            "TEST_TRIGGER_PACKAGE_INVALID",
        ),
        (
            input(&["fixture/a", "fixture/a"], &[], &[], &[]),
            "TEST_TRIGGER_PACKAGE_DUPLICATE",
        ),
        (
            input(&["fixture/a"], &[("fixture/a", "fixture/b")], &[], &[]),
            "TEST_TRIGGER_GRAPH_EDGE_UNKNOWN",
        ),
        (
            input(&["fixture/a"], &[("fixture/a", "fixture/a")], &[], &[]),
            "TEST_TRIGGER_GRAPH_EDGE_SELF",
        ),
        (
            input(
                &["fixture/a", "fixture/b"],
                &[("fixture/a", "fixture/b"), ("fixture/a", "fixture/b")],
                &[],
                &[],
            ),
            "TEST_TRIGGER_GRAPH_EDGE_DUPLICATE",
        ),
        (
            input(&["fixture/a"], &[], &[("fixture/a", "fixture/a")], &[]),
            "TEST_TRIGGER_RELATION_SELF",
        ),
        (
            input(
                &["fixture/a", "fixture/b"],
                &[],
                &[("fixture/a", "fixture/b"), ("fixture/a", "fixture/b")],
                &[],
            ),
            "TEST_TRIGGER_RELATION_DUPLICATE",
        ),
        (
            input(&["fixture/a"], &[], &[], &["fixture/missing"]),
            "TEST_TRIGGER_CHANGED_UNKNOWN",
        ),
        (
            input(&["fixture/a"], &[], &[], &["fixture/a", "fixture/a"]),
            "TEST_TRIGGER_CHANGED_DUPLICATE",
        ),
    ];

    for (input, diagnostic) in cases {
        assert_eq!(
            evaluate_test_trigger_selection(&input),
            Evaluation::Failure(diagnostic)
        );
    }
}

#[test]
fn all_cardinality_preflights_fail_before_element_validation() {
    let package_over_limit = TestTriggerSelectionInput {
        packages: vec!["INVALID".to_owned(); 4_097],
        edges: Vec::new(),
        trigger_relations: Vec::new(),
        changed_packages: Vec::new(),
    };
    assert_eq!(
        evaluate_test_trigger_selection(&package_over_limit),
        Evaluation::Failure("TEST_TRIGGER_PACKAGE_LIMIT_EXCEEDED")
    );

    let edge_over_limit = TestTriggerSelectionInput {
        packages: Vec::new(),
        edges: vec![Edge::new("INVALID".to_owned(), "INVALID".to_owned()); 16_385],
        trigger_relations: Vec::new(),
        changed_packages: Vec::new(),
    };
    assert_eq!(
        evaluate_test_trigger_selection(&edge_over_limit),
        Evaluation::Failure("TEST_TRIGGER_GRAPH_EDGE_LIMIT_EXCEEDED")
    );

    let changed_over_limit = TestTriggerSelectionInput {
        packages: Vec::new(),
        edges: Vec::new(),
        trigger_relations: Vec::new(),
        changed_packages: vec!["INVALID".to_owned(); 4_097],
    };
    assert_eq!(
        evaluate_test_trigger_selection(&changed_over_limit),
        Evaluation::Failure("TEST_TRIGGER_CHANGED_LIMIT_EXCEEDED")
    );
}
