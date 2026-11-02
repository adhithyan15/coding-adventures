use build_tool::graph_diff::{
    evaluate_diff_selection, evaluate_graph, DiffSelectionInput, Edge, Evaluation, GraphInput,
    PackageSpec,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const GRAPH_FIXTURES: &[&str] = &[
    "graph-canonical-edge-order.json",
    "graph-chain.json",
    "graph-cycle.json",
    "graph-diamond.json",
    "graph-empty.json",
    "graph-isolated.json",
    "graph-multiple-components.json",
    "graph-partial-cycle-no-output.json",
];

const DIFF_FIXTURES: &[&str] = &[
    "diff-selection-exact-build-fronts.json",
    "diff-selection-forced-package.json",
    "diff-selection-known-unmatched-near-build.json",
    "diff-selection-match-work-at-limit.json",
    "diff-selection-match-work-over-limit.json",
    "diff-selection-package-prefix.json",
    "diff-selection-repository-boundary.json",
    "diff-selection-shared-input-multiconsumer.json",
    "diff-selection-strict-glob-character-classes.json",
    "diff-selection-transitive.json",
    "diff-selection-unknown-all.json",
    "diff-selection-unknown-error.json",
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
    #[serde(default)]
    changed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureOptions {
    packages: Vec<Value>,
    edges: Vec<[String; 2]>,
    #[serde(default)]
    forced_packages: Vec<String>,
    #[serde(default)]
    unknown_path_policy: String,
    #[serde(default)]
    boundary_sha256: String,
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
    edges: Vec<[String; 2]>,
    #[serde(default)]
    levels: Vec<Vec<String>>,
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../specs/fixtures/build-tool-v1")
}

fn fixture_names(prefix: &str) -> Vec<String> {
    let mut names = fs::read_dir(fixture_root().join("cases"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.starts_with(prefix) && name.ends_with(".json"))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn load_fixture(name: &str) -> Fixture {
    let path = fixture_root().join("cases").join(name);
    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap()
}

fn edges(values: &[[String; 2]]) -> Vec<Edge> {
    values
        .iter()
        .map(|edge| Edge::new(edge[0].clone(), edge[1].clone()))
        .collect()
}

fn boundary() -> Value {
    let path = fixture_root().join("repository-source-input-boundary.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn dynamically_consumes_exact_eight_graph_fixtures() {
    assert_eq!(fixture_names("graph-"), GRAPH_FIXTURES);

    for name in GRAPH_FIXTURES {
        let fixture = load_fixture(name);
        let input = GraphInput {
            packages: fixture
                .input
                .options
                .packages
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect(),
            edges: edges(&fixture.input.options.edges),
        };
        match evaluate_graph(&input) {
            Evaluation::Success(actual) => {
                assert_eq!(fixture.expected.outcome, "ok", "{}", fixture.id);
                assert_eq!(
                    actual.edges,
                    edges(&fixture.expected.result.edges),
                    "{}",
                    fixture.id
                );
                assert_eq!(
                    actual.levels, fixture.expected.result.levels,
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
fn dynamically_consumes_exact_twelve_diff_selection_fixtures() {
    assert_eq!(fixture_names("diff-selection-"), DIFF_FIXTURES);

    for name in DIFF_FIXTURES {
        let fixture = load_fixture(name);
        let boundary = (!fixture.input.options.boundary_sha256.is_empty()).then(boundary);
        let input = DiffSelectionInput {
            packages: fixture
                .input
                .options
                .packages
                .iter()
                .map(|value| serde_json::from_value::<PackageSpec>(value.clone()).unwrap())
                .collect(),
            edges: edges(&fixture.input.options.edges),
            forced_packages: fixture.input.options.forced_packages.clone(),
            unknown_path_policy: fixture.input.options.unknown_path_policy.clone(),
            changed_paths: fixture.input.changed_paths.clone(),
            boundary_sha256: fixture.input.options.boundary_sha256.clone(),
            boundary,
        };
        match evaluate_diff_selection(&input) {
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
fn shared_input_selects_every_registered_consumer() {
    let fixture = load_fixture("diff-selection-shared-input-multiconsumer.json");
    let input = DiffSelectionInput {
        packages: fixture
            .input
            .options
            .packages
            .iter()
            .map(|value| serde_json::from_value::<PackageSpec>(value.clone()).unwrap())
            .collect(),
        edges: edges(&fixture.input.options.edges),
        forced_packages: Vec::new(),
        unknown_path_policy: "error".to_owned(),
        changed_paths: fixture.input.changed_paths,
        boundary_sha256: fixture.input.options.boundary_sha256,
        boundary: Some(boundary()),
    };
    let Evaluation::Success(result) = evaluate_diff_selection(&input) else {
        panic!("shared-input fixture must succeed");
    };
    assert_eq!(
        result.changed_packages,
        ["swift/conduit", "swift/sha256-native"]
    );
    assert!(!result.changed_packages.contains(&"swift/decoy".to_owned()));
}

#[test]
fn source_is_process_free_and_does_not_acquire_repository_authority() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph_diff.rs"))
            .unwrap();
    for forbidden in [
        "std::process",
        "std::fs",
        "std::env",
        "Command::",
        "read_to_string",
        "File::open",
    ] {
        assert!(
            !source.contains(forbidden),
            "found forbidden authority marker {forbidden}"
        );
    }
}
