use std::collections::BTreeMap;
use std::path::Path;

use coding_adventures_ca_assurance_report::json::JsonValue;
use coding_adventures_ca_assurance_report::lane_evidence::{
    parse_consumer_registry, parse_lane_manifest, validate_lane_evidence, LaneEvidenceError,
};
use coding_adventures_sha256::sha256_hex;

const SPEC_REL: &str = "../../../specs/ct-compare.warrant";
const CORPUS_REL: &str = "../../../specs/fixtures/ct-compare-warrant-v1/corpus.json";
const CASES_REL: &str = "../../../specs/fixtures/ct-compare-warrant-v1/cases.tfv";
const CONSUMERS_REL: &str = "../../../specs/fixtures/ct-compare-warrant-v1/consumers.json";

struct Artifacts {
    spec: Vec<u8>,
    corpus: Vec<u8>,
    cases: Vec<u8>,
    consumers: Vec<u8>,
}

impl Artifacts {
    fn load() -> Self {
        Self {
            spec: read_fixture(SPEC_REL),
            corpus: read_fixture(CORPUS_REL),
            cases: read_fixture(CASES_REL),
            consumers: read_fixture(CONSUMERS_REL),
        }
    }

    fn active_registry(&self) -> Vec<u8> {
        let mut registry = parse_json(&self.consumers);
        let object = object_mut(&mut registry);
        set(object, "active_consumer_count", number(1));
        let consumers = match object.get_mut("consumers") {
            Some(JsonValue::Array(rows)) => rows,
            _ => panic!("checked-in consumer registry has an array"),
        };
        let consumer = object_mut(&mut consumers[0]);
        set(consumer, "status", string_value("active"));
        json_bytes(&registry)
    }

    fn synthetic_manifest(&self) -> Vec<u8> {
        let corpus = parse_json(&self.corpus);
        let corpus = object_ref(&corpus);
        let examples = match corpus.get("examples") {
            Some(JsonValue::Array(rows)) => rows,
            _ => panic!("corpus examples are an array"),
        };
        let suites = match corpus.get("suites") {
            Some(JsonValue::Array(rows)) => rows,
            _ => panic!("corpus suites are an array"),
        };
        let controls = match corpus.get("controls") {
            Some(JsonValue::Array(rows)) => rows,
            _ => panic!("corpus controls are an array"),
        };
        let mut tests = Vec::new();
        for row in examples {
            let row = object_ref(row);
            tests.push(object_value([
                ("id", string_value(read_string(row, "id"))),
                ("layer", string_value("portable")),
                ("kind", string_value("example")),
                ("op", string_value(read_string(row, "op"))),
                ("cases", number(1)),
                ("verdict", string_value("pass")),
            ]));
        }
        for row in suites {
            let row = object_ref(row);
            let subject_ops = match row.get("ops") {
                Some(JsonValue::Array(items)) => items,
                _ => panic!("corpus suite ops are an array"),
            };
            tests.push(object_value([
                ("id", string_value(read_string(row, "id"))),
                ("layer", string_value("portable")),
                ("kind", string_value("exhaustive")),
                (
                    "op",
                    string_value(subject_ops[0].as_str().expect("operation name")),
                ),
                ("claim", string_value(read_string(row, "claim"))),
                ("cases", number(read_number(row, "cases"))),
                ("discarded", number(read_number(row, "discarded"))),
                ("calls", number(read_number(row, "expected_calls"))),
                (
                    "inputs_sha256",
                    string_value(read_string(row, "inputs_sha256")),
                ),
                (
                    "expected_sha256",
                    string_value(read_string(row, "expected_sha256")),
                ),
                ("domain", string_value(read_string(row, "domain_text"))),
                ("verdict", string_value("pass")),
            ]));
        }
        for row in controls {
            let row = object_ref(row);
            let claims = match row.get("required_claims") {
                Some(JsonValue::Array(items)) => items,
                _ => panic!("control required_claims are an array"),
            };
            tests.push(object_value([
                ("id", string_value(read_string(row, "id"))),
                ("layer", string_value("portable")),
                ("kind", string_value("negctl")),
                ("fault", string_value(read_string(row, "fault"))),
                ("op", string_value(read_string(row, "op"))),
                ("target", string_value(read_string(row, "target_kind"))),
                (
                    "caught_by",
                    JsonValue::Array(vec![string_value(claims[0].as_str().expect("claim id"))]),
                ),
                ("verdict", string_value("pass")),
            ]));
        }
        tests.sort_by(|left, right| {
            read_string(object_ref(left), "id").cmp(read_string(object_ref(right), "id"))
        });
        json_bytes(&object_value([
            ("schema", string_value("warrant-lane-evidence-v1")),
            ("lane", string_value("python")),
            ("package", string_value("python/ct-compare")),
            ("module", string_value("ct_compare")),
            ("spec", string_value("code/specs/ct-compare.warrant")),
            ("spec_sha256", string_value(sha256_hex(&self.spec))),
            (
                "corpus",
                string_value("code/specs/fixtures/ct-compare-warrant-v1"),
            ),
            ("corpus_sha256", string_value(sha256_hex(&self.corpus))),
            ("cases_sha256", string_value(sha256_hex(&self.cases))),
            (
                "runner",
                object_value([
                    ("name", string_value("warrant-corpus")),
                    ("lane", string_value("python")),
                    ("src_sha256", string_value("a".repeat(64))),
                ]),
            ),
            ("binding_sha256", string_value("b".repeat(64))),
            ("source_sha256", string_value("c".repeat(64))),
            (
                "toolchain",
                object_value([
                    ("name", string_value("python")),
                    ("version", string_value("3.12.4")),
                ]),
            ),
            ("evidence_seed", string_value("0")),
            ("faulted", JsonValue::Bool(false)),
            ("tests", JsonValue::Array(tests)),
        ]))
    }
}

#[test]
fn synthetic_active_consumer_joins_the_checked_in_corpus_without_claiming_execution() {
    let artifacts = Artifacts::load();
    let manifest = artifacts.synthetic_manifest();
    let active_registry = artifacts.active_registry();

    let parsed = validate_lane_evidence(
        &manifest,
        &active_registry,
        &artifacts.spec,
        &artifacts.corpus,
        &artifacts.cases,
    )
    .expect("synthetic structural fixture joins corpus artifacts");
    assert_eq!(parsed.tests.len(), 29);
    assert!(
        parse_consumer_registry(&artifacts.consumers)
            .expect("checked-in registry parses")
            .consumers[0]
            .status
            == "planned"
    );
    assert!(error_contains(
        validate_lane_evidence(
            &manifest,
            &artifacts.consumers,
            &artifacts.spec,
            &artifacts.corpus,
            &artifacts.cases,
        ),
        "planned consumer"
    ));
}

#[test]
fn rejects_missing_duplicate_extra_and_unsorted_manifest_rows() {
    let artifacts = Artifacts::load();
    let registry = artifacts.active_registry();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let rows = tests_mut(&mut manifest);
    rows.pop();
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "29 test rows"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let rows = tests_mut(&mut manifest);
    let duplicate = rows[0].clone();
    rows.push(duplicate);
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "29 test rows"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let rows = tests_mut(&mut manifest);
    rows.swap(0, 1);
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "sorted by id"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let object = object_mut(&mut manifest);
    set(object, "unexpected", string_value("extra"));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "closed schema"
    ));
}

#[test]
fn rejects_example_exhaustive_and_digest_drift() {
    let artifacts = Artifacts::load();
    let registry = artifacts.active_registry();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let example = find_test_mut(&mut manifest, "warrant/ct_compare/example/");
    set(object_mut(example), "op", string_value("eq_u64"));
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "example identity"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let exhaustive = find_test_mut(&mut manifest, "warrant/ct_compare/exhaustive/");
    set(
        object_mut(exhaustive),
        "domain",
        string_value("Bytes x Bytes"),
    );
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "corpus suite"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let exhaustive = find_test_mut(&mut manifest, "warrant/ct_compare/exhaustive/");
    set(object_mut(exhaustive), "calls", number(0));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "must be positive"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let exhaustive = find_test_mut(&mut manifest, "warrant/ct_compare/exhaustive/");
    set(
        object_mut(exhaustive),
        "expected_sha256",
        string_value("f".repeat(64)),
    );
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "corpus suite"
    ));

    let mut changed_spec = artifacts.spec.clone();
    changed_spec.push(b' ');
    assert!(error_contains(
        validate_bytes(
            &artifacts.synthetic_manifest(),
            &registry,
            &changed_spec,
            &artifacts.corpus,
            &artifacts.cases
        ),
        "different digests"
    ));
}

#[test]
fn rejects_control_target_and_caught_by_drift() {
    let artifacts = Artifacts::load();
    let registry = artifacts.active_registry();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let control = find_test_mut(&mut manifest, "warrant/ct_compare/negctl/");
    set(object_mut(control), "target", string_value("sample"));
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "negative-control"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let control = find_test_mut(&mut manifest, "warrant/ct_compare/negctl/");
    set(
        object_mut(control),
        "caught_by",
        JsonValue::Array(vec![string_value("eq_agrees")]),
    );
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "negative-control"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let control = find_test_mut(&mut manifest, "warrant/ct_compare/negctl/");
    set(
        object_mut(control),
        "caught_by",
        JsonValue::Array(Vec::new()),
    );
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "non-empty"
    ));
}

#[test]
fn rejects_faulted_partial_native_and_unsupported_entries() {
    let artifacts = Artifacts::load();
    let registry = artifacts.active_registry();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    set(object_mut(&mut manifest), "faulted", JsonValue::Bool(true));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "faulted"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    set(object_mut(&mut manifest), "shard", string_value("0/1"));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "closed schema"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let example = find_test_mut(&mut manifest, "warrant/ct_compare/example/");
    set(object_mut(example), "kind", string_value("property"));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "unsupported lane test kind"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let example = find_test_mut(&mut manifest, "warrant/ct_compare/example/");
    set(object_mut(example), "layer", string_value("native"));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "only portable"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let example = find_test_mut(&mut manifest, "warrant/ct_compare/example/");
    set(object_mut(example), "verdict", string_value("not-run"));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "verdict pass"
    ));
    assert!(error_contains(
        validate(&artifacts, &manifest, &registry),
        "verdict pass"
    ));
}

#[test]
fn rejects_consumer_identity_path_and_counter_drift() {
    let artifacts = Artifacts::load();
    let mut registry = parse_json(&artifacts.consumers);
    let object = object_mut(&mut registry);
    set(object, "consumer_count", number(2));
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "consumer_count"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    set(
        object_mut(&mut registry),
        "active_consumer_count",
        number(1),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "active_consumer_count"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    let consumer = first_consumer_mut(&mut registry);
    set(
        object_mut(consumer),
        "package_root",
        string_value("code/packages/python/other"),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "does not match package identity"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    let consumer = first_consumer_mut(&mut registry);
    set(
        object_mut(consumer),
        "binding",
        string_value("code/packages/python/ct-compare/../outside.py"),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "dotdot"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    let consumer = first_consumer_mut(&mut registry);
    set(
        object_mut(consumer),
        "runner_entry",
        string_value("/tmp/runner.py"),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "repository-relative"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    let consumer = first_consumer_mut(&mut registry);
    set(
        object_mut(consumer),
        "evidence_manifest",
        string_value("code/packages/python/ct-compare/test-manifest.json"),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "must be <package_root>"
    ));

    let mut registry = parse_json(&artifacts.consumers);
    let consumer = first_consumer_mut(&mut registry);
    set(
        object_mut(consumer),
        "build_files",
        JsonValue::Array(vec![string_value("code/packages/go/ct-compare/BUILD")]),
    );
    assert!(error_contains(
        parse_consumer_registry(&json_bytes(&registry)),
        "beneath its derived package_root"
    ));
}

#[test]
fn rejects_overflow_noncanonical_numeric_tokens_and_duplicate_keys() {
    let artifacts = Artifacts::load();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    set(
        object_mut(&mut manifest),
        "evidence_seed",
        string_value("18446744073709551616"),
    );
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "canonical u64"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    set(object_mut(&mut manifest), "evidence_seed", number(1));
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "must be a string"
    ));

    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    set(
        object_mut(&mut manifest),
        "evidence_seed",
        string_value("01"),
    );
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "canonical u64"
    ));

    let raw = br#"{"schema":"warrant-lane-evidence-v1","schema":"warrant-lane-evidence-v1"}"#;
    assert!(error_contains(parse_lane_manifest(raw), "duplicate key"));

    let mut duplicate_nested =
        String::from_utf8(artifacts.synthetic_manifest()).expect("UTF-8 manifest");
    duplicate_nested = duplicate_nested.replace(
        "\"name\": \"warrant-corpus\"",
        "\"name\": \"warrant-corpus\", \"name\": \"other\"",
    );
    assert!(error_contains(
        parse_lane_manifest(duplicate_nested.as_bytes()),
        "duplicate key"
    ));

    let mut decimal = String::from_utf8(artifacts.synthetic_manifest()).expect("UTF-8 manifest");
    decimal = decimal.replacen("\"cases\": 1,", "\"cases\": 1.0,", 1);
    assert!(error_contains(
        parse_lane_manifest(decimal.as_bytes()),
        "non-canonical integer"
    ));

    let artifacts = Artifacts::load();
    let mut raw_number = String::from_utf8(artifacts.consumers).expect("UTF-8 registry");
    raw_number = raw_number.replace(
        "\"consumer_count\": 1",
        "\"consumer_count\": 9007199254740993",
    );
    assert!(error_contains(
        parse_consumer_registry(raw_number.as_bytes()),
        "supported exact range"
    ));

    let artifacts = Artifacts::load();
    let mut manifest = parse_json(&artifacts.synthetic_manifest());
    let rows = tests_mut(&mut manifest);
    let mut too_many = rows.clone();
    too_many.push(too_many[0].clone());
    set(
        object_mut(&mut manifest),
        "tests",
        JsonValue::Array(too_many),
    );
    assert!(error_contains(
        parse_lane_manifest(&json_bytes(&manifest)),
        "exactly 29"
    ));
}

#[test]
fn rejects_malformed_corpus_cardinalities() {
    let artifacts = Artifacts::load();
    let registry = artifacts.active_registry();
    for (field, expected_error) in [
        ("examples", "exactly 12 examples"),
        ("suites", "exactly 9 exhaustive suites"),
        ("faults", "exactly 8 fault transformers"),
        ("controls", "exactly 8 negative controls"),
    ] {
        let mut corpus = parse_json(&artifacts.corpus);
        let rows = match object_mut(&mut corpus).get_mut(field) {
            Some(JsonValue::Array(rows)) => rows,
            _ => panic!("corpus {field} is an array"),
        };
        rows.pop();
        let corpus_bytes = json_bytes(&corpus);
        let mut manifest = parse_json(&artifacts.synthetic_manifest());
        set(
            object_mut(&mut manifest),
            "corpus_sha256",
            string_value(sha256_hex(&corpus_bytes)),
        );
        assert!(error_contains(
            validate_lane_evidence(
                &json_bytes(&manifest),
                &registry,
                &artifacts.spec,
                &corpus_bytes,
                &artifacts.cases,
            ),
            expected_error
        ));
    }
}

#[test]
fn rejects_oversized_inputs_before_parsing() {
    let oversized_manifest = vec![b' '; 16 * 1024 * 1024 + 1];
    assert!(error_contains(
        parse_lane_manifest(&oversized_manifest),
        "byte limit"
    ));

    let artifacts = Artifacts::load();
    let manifest = artifacts.synthetic_manifest();
    let registry = artifacts.active_registry();
    let oversized_spec = vec![b' '; 4 * 1024 * 1024 + 1];
    assert!(error_contains(
        validate_lane_evidence(
            &manifest,
            &registry,
            &oversized_spec,
            &artifacts.corpus,
            &artifacts.cases,
        ),
        "4 MiB"
    ));
    drop(oversized_spec);

    let oversized_corpus = vec![b' '; 32 * 1024 * 1024 + 1];
    assert!(error_contains(
        validate_lane_evidence(
            &manifest,
            &registry,
            &artifacts.spec,
            &oversized_corpus,
            &artifacts.cases,
        ),
        "32 MiB"
    ));
    drop(oversized_corpus);

    let oversized_cases = vec![b' '; 32 * 1024 * 1024 + 1];
    assert!(error_contains(
        validate_lane_evidence(
            &manifest,
            &registry,
            &artifacts.spec,
            &artifacts.corpus,
            &oversized_cases,
        ),
        "32 MiB"
    ));
}

#[test]
fn binds_raw_spec_corpus_and_tfv_bytes_to_manifest_hashes() {
    let artifacts = Artifacts::load();
    let manifest = artifacts.synthetic_manifest();
    let registry = artifacts.active_registry();

    let mut changed_corpus = artifacts.corpus.clone();
    changed_corpus.push(b' ');
    assert!(error_contains(
        validate_bytes(
            &manifest,
            &registry,
            &artifacts.spec,
            &changed_corpus,
            &artifacts.cases
        ),
        "corpus_sha256"
    ));

    let mut changed_cases = artifacts.cases.clone();
    changed_cases.push(b'\n');
    assert!(error_contains(
        validate_bytes(
            &manifest,
            &registry,
            &artifacts.spec,
            &artifacts.corpus,
            &changed_cases
        ),
        "cases_sha256"
    ));
}

fn validate(
    artifacts: &Artifacts,
    manifest: &JsonValue,
    registry: &[u8],
) -> Result<(), LaneEvidenceError> {
    validate_bytes(
        &json_bytes(manifest),
        registry,
        &artifacts.spec,
        &artifacts.corpus,
        &artifacts.cases,
    )
}

fn validate_bytes(
    manifest: &[u8],
    registry: &[u8],
    spec: &[u8],
    corpus: &[u8],
    cases: &[u8],
) -> Result<(), LaneEvidenceError> {
    validate_lane_evidence(manifest, registry, spec, corpus, cases).map(|_| ())
}

fn error_contains<T>(result: Result<T, LaneEvidenceError>, expected: &str) -> bool {
    result
        .err()
        .is_some_and(|error| error.to_string().contains(expected))
}

fn read_fixture(relative: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read(path).expect("checked-in Warrant fixture is readable")
}

fn parse_json(bytes: &[u8]) -> JsonValue {
    JsonValue::parse(std::str::from_utf8(bytes).expect("fixture JSON is UTF-8"))
        .expect("fixture JSON parses")
}

fn json_bytes(value: &JsonValue) -> Vec<u8> {
    value.to_pretty_string(2).into_bytes()
}

fn object_value<const N: usize>(entries: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn object_ref(value: &JsonValue) -> &BTreeMap<String, JsonValue> {
    value.as_object().expect("JSON object")
}

fn object_mut(value: &mut JsonValue) -> &mut BTreeMap<String, JsonValue> {
    match value {
        JsonValue::Object(object) => object,
        _ => panic!("JSON object"),
    }
}

fn set(object: &mut BTreeMap<String, JsonValue>, key: &str, value: JsonValue) {
    object.insert(key.to_owned(), value);
}

fn tests_mut(manifest: &mut JsonValue) -> &mut Vec<JsonValue> {
    match object_mut(manifest).get_mut("tests") {
        Some(JsonValue::Array(rows)) => rows,
        _ => panic!("manifest tests are an array"),
    }
}

fn find_test_mut<'a>(manifest: &'a mut JsonValue, prefix: &str) -> &'a mut JsonValue {
    tests_mut(manifest)
        .iter_mut()
        .find(|row| read_string(object_ref(row), "id").starts_with(prefix))
        .expect("matching synthetic test row")
}

fn first_consumer_mut(registry: &mut JsonValue) -> &mut JsonValue {
    match object_mut(registry).get_mut("consumers") {
        Some(JsonValue::Array(rows)) => &mut rows[0],
        _ => panic!("consumer rows are an array"),
    }
}

fn read_string<'a>(object: &'a BTreeMap<String, JsonValue>, key: &str) -> &'a str {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .expect("string field")
}

fn read_number(object: &BTreeMap<String, JsonValue>, key: &str) -> u64 {
    object
        .get(key)
        .and_then(JsonValue::as_i64)
        .and_then(|value| u64::try_from(value).ok())
        .expect("integer field")
}

fn string_value(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

fn number(value: u64) -> JsonValue {
    JsonValue::Number(value as f64)
}
