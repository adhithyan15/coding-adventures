//! Strict structural parser for the first `warrant-lane-evidence-v1` profile.
//!
//! This module deliberately sits beside, rather than inside, the legacy Rust
//! evidence loader. It parses bounded bytes and joins them to the planned
//! consumer registry and the checked-in ct-compare corpus. It does not open
//! paths, run a subject binding, verify build/source/toolchain freshness, or
//! create an assurance report.

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use coding_adventures_sha256::sha256_hex;

use crate::evidence::MAX_MANIFEST_BYTES;
use crate::json::JsonValue;

const MAX_CORPUS_BYTES: usize = 32 * 1024 * 1024;
const MAX_SPEC_BYTES: usize = 4 * 1024 * 1024;
const MAX_TEST_CASES: u64 = 1 << 53;
const MAX_MANIFEST_ROWS: usize = 29;
const CORPUS_DIR: &str = "code/specs/fixtures/ct-compare-warrant-v1";
const CORPUS_FILE: &str = "code/specs/fixtures/ct-compare-warrant-v1/corpus.json";
const SPEC_FILE: &str = "code/specs/ct-compare.warrant";

/// A parse or structural-validation failure. Error text is intended for CI
/// diagnostics and deliberately does not echo complete untrusted input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneEvidenceError(pub String);

impl fmt::Display for LaneEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lane evidence error: {}", self.0)
    }
}

impl std::error::Error for LaneEvidenceError {}

type Obj = BTreeMap<String, JsonValue>;

/// A checked portable example row. Its expected result remains in the
/// canonical corpus; the row only records that the runner claims one case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExampleEntry {
    pub id: String,
    pub op: String,
    pub cases: u64,
    pub verdict: String,
}

/// A checked exhaustive row joined against one corpus suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExhaustiveEntry {
    pub id: String,
    pub op: String,
    pub claim: String,
    pub cases: u64,
    pub discarded: u64,
    pub calls: u64,
    pub inputs_sha256: String,
    pub expected_sha256: String,
    pub domain: String,
    pub verdict: String,
}

/// A checked negative-control row. `target` keeps the VER04h manifest key and
/// is compared with the corpus control's `target_kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NegativeControlEntry {
    pub id: String,
    pub op: String,
    pub fault: String,
    pub target: String,
    pub caught_by: Vec<String>,
    pub verdict: String,
}

/// The only test-entry kinds admitted by the first portable lane profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaneTestEntry {
    Example(ExampleEntry),
    Exhaustive(ExhaustiveEntry),
    NegativeControl(NegativeControlEntry),
}

impl LaneTestEntry {
    fn id(&self) -> &str {
        match self {
            Self::Example(row) => &row.id,
            Self::Exhaustive(row) => &row.id,
            Self::NegativeControl(row) => &row.id,
        }
    }
}

/// Typed representation of the closed v0 lane manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneEvidenceManifest {
    pub lane: String,
    pub package: String,
    pub module: String,
    pub spec: String,
    pub spec_sha256: String,
    pub corpus: String,
    pub corpus_sha256: String,
    pub cases_sha256: String,
    pub runner_name: String,
    pub runner_lane: String,
    pub runner_src_sha256: String,
    pub binding_sha256: String,
    pub source_sha256: String,
    pub toolchain_name: String,
    pub toolchain_version: String,
    pub evidence_seed: u64,
    pub faulted: bool,
    pub tests: Vec<LaneTestEntry>,
}

/// One language/package reservation in the per-corpus consumer registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneConsumer {
    pub lane: String,
    pub package: String,
    pub module: String,
    pub package_root: String,
    pub binding: String,
    pub runner_entry: String,
    pub evidence_manifest: String,
    pub build_files: Vec<String>,
    pub status: String,
}

/// The closed `warrant-lane-consumers-v1` registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneConsumerRegistry {
    pub corpus: String,
    pub consumers: Vec<LaneConsumer>,
    pub active_consumer_count: u64,
}

/// Parse only the first ct-compare-v0 profile's lane manifest. Unknown fields,
/// alternate schema versions, non-portable kinds, numeric seeds, faulted
/// manifests, duplicate ids, and malformed hashes fail closed.
pub fn parse_lane_manifest(bytes: &[u8]) -> Result<LaneEvidenceManifest, LaneEvidenceError> {
    let root = parse_json(bytes, MAX_MANIFEST_BYTES as usize, "lane manifest")?;
    let object = object(&root, "lane manifest")?;
    exact_fields(
        object,
        &[
            "schema",
            "lane",
            "package",
            "module",
            "spec",
            "spec_sha256",
            "corpus",
            "corpus_sha256",
            "cases_sha256",
            "runner",
            "binding_sha256",
            "source_sha256",
            "toolchain",
            "evidence_seed",
            "faulted",
            "tests",
        ],
        "lane manifest",
    )?;
    if string(object, "schema", "lane manifest")? != "warrant-lane-evidence-v1" {
        return fail("unsupported lane manifest schema");
    }

    let lane = string(object, "lane", "lane manifest")?.to_owned();
    if !is_identifier(&lane) {
        return fail("lane must be a lowercase identifier");
    }
    let package = string(object, "package", "lane manifest")?.to_owned();
    if package.split('/').count() != 2 || !package.starts_with(&format!("{lane}/")) {
        return fail("package must be <lane>/<name> and match the lane");
    }
    let module = string(object, "module", "lane manifest")?.to_owned();
    if !is_identifier(&module) {
        return fail("module must be a lowercase identifier");
    }
    let spec = string(object, "spec", "lane manifest")?.to_owned();
    let spec_sha256 = digest_field(object, "spec_sha256", "lane manifest")?;
    let corpus = string(object, "corpus", "lane manifest")?.to_owned();
    let corpus_sha256 = digest_field(object, "corpus_sha256", "lane manifest")?;
    let cases_sha256 = digest_field(object, "cases_sha256", "lane manifest")?;

    let runner = object_field(object, "runner", "lane manifest")?;
    exact_fields(runner, &["name", "lane", "src_sha256"], "runner")?;
    let runner_name = string(runner, "name", "runner")?.to_owned();
    let runner_lane = string(runner, "lane", "runner")?.to_owned();
    if runner_lane != lane {
        return fail("runner lane does not match manifest lane");
    }
    let runner_src_sha256 = digest_field(runner, "src_sha256", "runner")?;
    let binding_sha256 = digest_field(object, "binding_sha256", "lane manifest")?;
    let source_sha256 = digest_field(object, "source_sha256", "lane manifest")?;

    let toolchain = object_field(object, "toolchain", "lane manifest")?;
    exact_fields(toolchain, &["name", "version"], "toolchain")?;
    let toolchain_name = string(toolchain, "name", "toolchain")?.to_owned();
    let toolchain_version = string(toolchain, "version", "toolchain")?.to_owned();
    let seed_text = string(object, "evidence_seed", "lane manifest")?;
    let evidence_seed = seed_text
        .parse::<u64>()
        .ok()
        .filter(|value| value.to_string() == seed_text)
        .ok_or_else(|| {
            LaneEvidenceError("evidence_seed must be a canonical u64 decimal string".into())
        })?;
    let faulted = boolean(object, "faulted", "lane manifest")?;
    if faulted {
        return fail("faulted lane manifests are not accepted");
    }
    let tests_value = field(object, "tests", "lane manifest")?;
    let tests_array = array(tests_value, "lane manifest tests")?;
    if tests_array.len() != MAX_MANIFEST_ROWS {
        return fail("ct-compare-v0 manifest must contain exactly 29 test rows");
    }
    let mut tests = Vec::with_capacity(tests_array.len());
    let mut ids = BTreeSet::new();
    for (index, value) in tests_array.iter().enumerate() {
        let row = parse_test_entry(value, index)?;
        if !ids.insert(row.id().to_owned()) {
            return fail("lane manifest contains a duplicate test id");
        }
        tests.push(row);
    }
    if !tests.windows(2).all(|pair| pair[0].id() < pair[1].id()) {
        return fail("lane manifest tests must be sorted by id");
    }

    Ok(LaneEvidenceManifest {
        lane,
        package,
        module,
        spec,
        spec_sha256,
        corpus,
        corpus_sha256,
        cases_sha256,
        runner_name,
        runner_lane,
        runner_src_sha256,
        binding_sha256,
        source_sha256,
        toolchain_name,
        toolchain_version,
        evidence_seed,
        faulted,
        tests,
    })
}

/// Parse the per-corpus consumer registry, including lexical path and derived
/// identity checks. It never opens any listed path or claims symlink safety.
pub fn parse_consumer_registry(bytes: &[u8]) -> Result<LaneConsumerRegistry, LaneEvidenceError> {
    let root = parse_json(bytes, MAX_MANIFEST_BYTES as usize, "consumer registry")?;
    let object = object(&root, "consumer registry")?;
    exact_fields(
        object,
        &[
            "schema_version",
            "profile",
            "corpus",
            "consumer_count",
            "active_consumer_count",
            "consumers",
        ],
        "consumer registry",
    )?;
    if number(object, "schema_version", "consumer registry")? != 1 {
        return fail("unsupported consumer registry schema_version");
    }
    if string(object, "profile", "consumer registry")? != "warrant-lane-consumers-v1" {
        return fail("unsupported consumer registry profile");
    }
    let corpus = string(object, "corpus", "consumer registry")?.to_owned();
    if corpus != CORPUS_FILE {
        return fail("consumer registry corpus path is not canonical");
    }
    let consumers_value = field(object, "consumers", "consumer registry")?;
    let consumers_array = array(consumers_value, "consumers")?;
    if consumers_array.is_empty() || consumers_array.len() > 18 {
        return fail("consumer registry must contain 1 to 18 consumers");
    }
    let declared_count = number(object, "consumer_count", "consumer registry")?;
    if declared_count != consumers_array.len() as u64 {
        return fail("consumer_count does not equal the registry row count");
    }
    let mut consumers = Vec::with_capacity(consumers_array.len());
    let mut packages = BTreeSet::new();
    let mut active_count = 0_u64;
    for (index, value) in consumers_array.iter().enumerate() {
        let consumer = parse_consumer(value, index)?;
        if !packages.insert(consumer.package.clone()) {
            return fail("consumer registry contains a duplicate package identity");
        }
        if consumer.status == "active" {
            active_count += 1;
        }
        consumers.push(consumer);
    }
    if !consumers
        .windows(2)
        .all(|pair| pair[0].package < pair[1].package)
    {
        return fail("consumer registry rows must be sorted by package");
    }
    let declared_active = number(object, "active_consumer_count", "consumer registry")?;
    if declared_active != active_count {
        return fail("active_consumer_count does not equal active registry rows");
    }

    Ok(LaneConsumerRegistry {
        corpus,
        consumers,
        active_consumer_count: active_count,
    })
}

/// Validate a typed manifest against the consumer registry and exact corpus
/// artifacts supplied by the caller. Hashes bind those bytes; no filesystem
/// access, runner execution, or build/source/toolchain freshness is performed.
pub fn validate_lane_evidence(
    manifest_bytes: &[u8],
    registry_bytes: &[u8],
    spec_bytes: &[u8],
    corpus_bytes: &[u8],
    cases_bytes: &[u8],
) -> Result<LaneEvidenceManifest, LaneEvidenceError> {
    let manifest = parse_lane_manifest(manifest_bytes)?;
    let registry = parse_consumer_registry(registry_bytes)?;
    validate_artifact_caps(spec_bytes, corpus_bytes, cases_bytes)?;
    let corpus = parse_corpus_profile(corpus_bytes)?;

    if manifest.module != "ct_compare"
        || manifest.spec != SPEC_FILE
        || manifest.corpus != CORPUS_DIR
        || registry.corpus != CORPUS_FILE
    {
        return fail("manifest identity is outside the ct-compare-v0 profile");
    }
    let spec_sha256 = sha256_hex(spec_bytes);
    if manifest.spec_sha256 != spec_sha256 || corpus.spec_sha256 != spec_sha256 {
        return fail("manifest, corpus, and supplied spec bytes have different digests");
    }
    if manifest.corpus_sha256 != sha256_hex(corpus_bytes) {
        return fail("corpus_sha256 does not match the supplied corpus bytes");
    }
    if manifest.cases_sha256 != sha256_hex(cases_bytes) {
        return fail("cases_sha256 does not match the supplied TFV bytes");
    }
    if corpus.spec != manifest.spec {
        return fail("corpus spec path does not match the manifest");
    }

    let consumer = registry
        .consumers
        .iter()
        .find(|row| row.package == manifest.package && row.lane == manifest.lane)
        .ok_or_else(|| LaneEvidenceError("manifest lane/package is not registered".into()))?;
    if consumer.status != "active" {
        return fail("planned consumer entries cannot authorize lane evidence");
    }
    if consumer.module != manifest.module {
        return fail("manifest module does not match the registered consumer");
    }
    if manifest.runner_lane != consumer.lane {
        return fail("runner lane does not match the registered consumer");
    }

    join_tests(&manifest.tests, &corpus)?;
    Ok(manifest)
}

fn parse_test_entry(value: &JsonValue, index: usize) -> Result<LaneTestEntry, LaneEvidenceError> {
    let label = format!("tests[{index}]");
    let row = object(value, &label)?;
    let kind = string(row, "kind", &label)?;
    match kind {
        "example" => {
            exact_fields(
                row,
                &["id", "layer", "kind", "op", "cases", "verdict"],
                &label,
            )?;
            common_portable_fields(row, &label)?;
            let cases = number(row, "cases", &label)?;
            if cases != 1 {
                return fail("each example row must record exactly one case");
            }
            let verdict = passing_verdict(row, &label)?;
            Ok(LaneTestEntry::Example(ExampleEntry {
                id: string(row, "id", &label)?.to_owned(),
                op: string(row, "op", &label)?.to_owned(),
                cases,
                verdict,
            }))
        }
        "exhaustive" => {
            exact_fields(
                row,
                &[
                    "id",
                    "layer",
                    "kind",
                    "op",
                    "claim",
                    "cases",
                    "discarded",
                    "calls",
                    "inputs_sha256",
                    "expected_sha256",
                    "domain",
                    "verdict",
                ],
                &label,
            )?;
            common_portable_fields(row, &label)?;
            let cases = positive_number(row, "cases", &label)?;
            let discarded = number(row, "discarded", &label)?;
            let calls = positive_number(row, "calls", &label)?;
            let inputs_sha256 = digest_field(row, "inputs_sha256", &label)?;
            let expected_sha256 = digest_field(row, "expected_sha256", &label)?;
            let verdict = passing_verdict(row, &label)?;
            Ok(LaneTestEntry::Exhaustive(ExhaustiveEntry {
                id: string(row, "id", &label)?.to_owned(),
                op: string(row, "op", &label)?.to_owned(),
                claim: string(row, "claim", &label)?.to_owned(),
                cases,
                discarded,
                calls,
                inputs_sha256,
                expected_sha256,
                domain: string(row, "domain", &label)?.to_owned(),
                verdict,
            }))
        }
        "negctl" => {
            exact_fields(
                row,
                &[
                    "id",
                    "layer",
                    "kind",
                    "fault",
                    "op",
                    "target",
                    "caught_by",
                    "verdict",
                ],
                &label,
            )?;
            common_portable_fields(row, &label)?;
            let caught_value = field(row, "caught_by", &label)?;
            let caught_array = array(caught_value, "caught_by")?;
            if caught_array.is_empty() {
                return fail("negative-control caught_by must be non-empty");
            }
            let mut caught_by = Vec::with_capacity(caught_array.len());
            let mut unique = BTreeSet::new();
            for item in caught_array {
                let claim = item
                    .as_str()
                    .ok_or_else(|| LaneEvidenceError("caught_by values must be strings".into()))?
                    .to_owned();
                if !unique.insert(claim.clone()) {
                    return fail("negative-control caught_by contains a duplicate claim id");
                }
                caught_by.push(claim);
            }
            let verdict = passing_verdict(row, &label)?;
            Ok(LaneTestEntry::NegativeControl(NegativeControlEntry {
                id: string(row, "id", &label)?.to_owned(),
                op: string(row, "op", &label)?.to_owned(),
                fault: string(row, "fault", &label)?.to_owned(),
                target: string(row, "target", &label)?.to_owned(),
                caught_by,
                verdict,
            }))
        }
        _ => fail("unsupported lane test kind; only example, exhaustive, and negctl are accepted"),
    }
}

fn parse_consumer(value: &JsonValue, index: usize) -> Result<LaneConsumer, LaneEvidenceError> {
    let label = format!("consumers[{index}]");
    let row = object(value, &label)?;
    exact_fields(
        row,
        &[
            "lane",
            "package",
            "module",
            "package_root",
            "binding",
            "runner_entry",
            "evidence_manifest",
            "build_files",
            "status",
        ],
        &label,
    )?;
    let lane = string(row, "lane", &label)?.to_owned();
    let package = string(row, "package", &label)?.to_owned();
    let parts: Vec<&str> = package.split('/').collect();
    if parts.len() != 2
        || parts[0] != lane
        || !is_identifier(parts[0])
        || !is_package_name(parts[1])
    {
        return fail("consumer package must be <lane>/<name> with matching lane");
    }
    let derived_root = format!("code/packages/{lane}/{}", parts[1]);
    let package_root = string(row, "package_root", &label)?.to_owned();
    if package_root != derived_root {
        return fail("consumer package_root does not match package identity");
    }
    validate_repo_path(&package_root)?;
    let module = string(row, "module", &label)?.to_owned();
    if !is_identifier(&module) {
        return fail("consumer module must be a lowercase identifier");
    }
    let binding = string(row, "binding", &label)?.to_owned();
    let runner_entry = string(row, "runner_entry", &label)?.to_owned();
    let evidence_manifest = string(row, "evidence_manifest", &label)?.to_owned();
    for path in [&binding, &runner_entry, &evidence_manifest] {
        validate_file_under_root(path, &package_root)?;
    }
    if evidence_manifest != format!("{package_root}/evidence/test-manifest.json") {
        return fail(
            "consumer evidence_manifest must be <package_root>/evidence/test-manifest.json",
        );
    }
    let build_files_value = field(row, "build_files", &label)?;
    let build_files_array = array(build_files_value, "build_files")?;
    if build_files_array.is_empty() || build_files_array.len() > 2 {
        return fail("consumer build_files must contain one or two paths");
    }
    let mut build_files = Vec::with_capacity(build_files_array.len());
    let mut unique_build_files = BTreeSet::new();
    for item in build_files_array {
        let path = item
            .as_str()
            .ok_or_else(|| LaneEvidenceError("build_files values must be strings".into()))?
            .to_owned();
        validate_file_under_root(&path, &package_root)?;
        if !unique_build_files.insert(path.clone()) {
            return fail("consumer build_files contains a duplicate path");
        }
        build_files.push(path);
    }
    let status = string(row, "status", &label)?.to_owned();
    if status != "planned" && status != "active" {
        return fail("consumer status must be planned or active");
    }
    Ok(LaneConsumer {
        lane,
        package,
        module,
        package_root,
        binding,
        runner_entry,
        evidence_manifest,
        build_files,
        status,
    })
}

fn parse_corpus_profile(bytes: &[u8]) -> Result<CorpusProfile, LaneEvidenceError> {
    let root = parse_json(bytes, MAX_CORPUS_BYTES, "Warrant corpus")?;
    let root_object = object(&root, "Warrant corpus")?;
    exact_fields(
        root_object,
        &[
            "controls",
            "examples",
            "faults",
            "generator",
            "models",
            "module",
            "ops",
            "profile",
            "schema_version",
            "spec",
            "spec_sha256",
            "suites",
            "types",
        ],
        "Warrant corpus",
    )?;
    if number(root_object, "schema_version", "Warrant corpus")? != 1
        || string(root_object, "profile", "Warrant corpus")? != "warrant-corpus-v1"
        || string(root_object, "module", "Warrant corpus")? != "ct_compare"
    {
        return fail("unsupported Warrant corpus profile");
    }
    let spec = string(root_object, "spec", "Warrant corpus")?.to_owned();
    let spec_sha256 = digest_field(root_object, "spec_sha256", "Warrant corpus")?;

    let examples = array(
        field(root_object, "examples", "Warrant corpus")?,
        "corpus examples",
    )?;
    if examples.len() != 12 {
        return fail("ct-compare-v0 corpus must contain exactly 12 examples");
    }
    let mut example_rows = BTreeMap::new();
    for (index, value) in examples.iter().enumerate() {
        let row = object(value, &format!("examples[{index}]"))?;
        exact_fields(
            row,
            &["args", "expected", "id", "op", "result_type", "width"],
            "corpus example",
        )?;
        let id = string(row, "id", "corpus example")?.to_owned();
        let op = string(row, "op", "corpus example")?.to_owned();
        if !id.starts_with("warrant/ct_compare/example/") || example_rows.insert(id, op).is_some() {
            return fail("corpus examples contain an invalid or duplicate id");
        }
    }

    let suites = array(
        field(root_object, "suites", "Warrant corpus")?,
        "corpus suites",
    )?;
    if suites.len() != 9 {
        return fail("ct-compare-v0 corpus must contain exactly 9 exhaustive suites");
    }
    let mut suite_rows = BTreeMap::new();
    for (index, value) in suites.iter().enumerate() {
        let row = object(value, &format!("suites[{index}]"))?;
        exact_fields(
            row,
            &[
                "admit",
                "binders",
                "cases",
                "claim",
                "discarded",
                "domain_text",
                "expected_calls",
                "expected_sha256",
                "id",
                "inputs_sha256",
                "kind",
                "lhs",
                "ops",
                "result_type",
                "rhs",
            ],
            "corpus suite",
        )?;
        let result_type = string(row, "result_type", "corpus suite")?;
        if string(row, "kind", "corpus suite")? != "exhaustive"
            || !matches!(result_type, "Bool" | "Result<Bytes,SelectError>")
        {
            return fail("ct-compare-v0 suite kind/result_type is unsupported");
        }
        let id = string(row, "id", "corpus suite")?.to_owned();
        if !id.starts_with("warrant/ct_compare/exhaustive/") {
            return fail("corpus suite id is outside the ct-compare profile");
        }
        let claim = string(row, "claim", "corpus suite")?.to_owned();
        let cases = positive_number(row, "cases", "corpus suite")?;
        let discarded = number(row, "discarded", "corpus suite")?;
        let calls = positive_number(row, "expected_calls", "corpus suite")?;
        if calls != cases {
            return fail("ct-compare-v0 requires one subject call per admitted case");
        }
        let op_values = array(field(row, "ops", "corpus suite")?, "corpus suite ops")?;
        if op_values.len() != 1 {
            return fail("ct-compare-v0 suites must name exactly one subject operation");
        }
        let op = op_values[0]
            .as_str()
            .ok_or_else(|| LaneEvidenceError("corpus suite op must be a string".into()))?
            .to_owned();
        let domain = string(row, "domain_text", "corpus suite")?.to_owned();
        let inputs_sha256 = digest_field(row, "inputs_sha256", "corpus suite")?;
        let expected_sha256 = digest_field(row, "expected_sha256", "corpus suite")?;
        let suite = CorpusSuite {
            op,
            claim,
            cases,
            discarded,
            calls,
            inputs_sha256,
            expected_sha256,
            domain,
        };
        if suite_rows.insert(id, suite).is_some() {
            return fail("corpus suites contain a duplicate id");
        }
    }

    let faults = array(
        field(root_object, "faults", "Warrant corpus")?,
        "corpus faults",
    )?;
    if faults.len() != 8 {
        return fail("ct-compare-v0 corpus must contain exactly 8 fault transformers");
    }
    let mut fault_ops = BTreeSet::new();
    for (index, value) in faults.iter().enumerate() {
        let row = object(value, &format!("faults[{index}]"))?;
        exact_fields(row, &["applies", "fault", "op"], "corpus fault")?;
        let fault = string(row, "fault", "corpus fault")?.to_owned();
        let op = string(row, "op", "corpus fault")?.to_owned();
        if !fault_ops.insert((op, fault)) {
            return fail("corpus faults contain a duplicate operation/fault pair");
        }
    }

    let controls = array(
        field(root_object, "controls", "Warrant corpus")?,
        "corpus controls",
    )?;
    if controls.len() != 8 {
        return fail("ct-compare-v0 corpus must contain exactly 8 negative controls");
    }
    let mut control_rows = BTreeMap::new();
    let mut control_fault_ops = BTreeSet::new();
    for (index, value) in controls.iter().enumerate() {
        let row = object(value, &format!("controls[{index}]"))?;
        exact_fields(
            row,
            &["fault", "id", "op", "required_claims", "target_kind"],
            "corpus control",
        )?;
        let id = string(row, "id", "corpus control")?.to_owned();
        let op = string(row, "op", "corpus control")?.to_owned();
        let fault = string(row, "fault", "corpus control")?.to_owned();
        let target_kind = string(row, "target_kind", "corpus control")?.to_owned();
        if target_kind != "exhaustive" || !fault_ops.contains(&(op.clone(), fault.clone())) {
            return fail("corpus control does not match a supported fault transformer");
        }
        if !control_fault_ops.insert((op.clone(), fault.clone())) {
            return fail("corpus controls contain a duplicate operation/fault pair");
        }
        let claim_values = array(
            field(row, "required_claims", "corpus control")?,
            "required_claims",
        )?;
        if claim_values.is_empty() {
            return fail("corpus control required_claims must be non-empty");
        }
        let mut required_claims = BTreeSet::new();
        for value in claim_values {
            let claim_id = value
                .as_str()
                .ok_or_else(|| LaneEvidenceError("required_claims values must be strings".into()))?
                .to_owned();
            if !suite_rows.contains_key(&claim_id) || !required_claims.insert(claim_id) {
                return fail("corpus control required_claims has an unknown or duplicate claim id");
            }
        }
        let control = CorpusControl {
            op,
            fault,
            target_kind,
            required_claims,
        };
        if control_rows.insert(id, control).is_some() {
            return fail("corpus controls contain a duplicate id");
        }
    }
    if control_rows.len() != fault_ops.len() || control_fault_ops != fault_ops {
        return fail("every ct-compare fault transformer must have a generated control");
    }

    Ok(CorpusProfile {
        spec,
        spec_sha256,
        examples: example_rows,
        suites: suite_rows,
        controls: control_rows,
    })
}

fn join_tests(tests: &[LaneTestEntry], corpus: &CorpusProfile) -> Result<(), LaneEvidenceError> {
    let mut examples = BTreeSet::new();
    let mut suites = BTreeSet::new();
    let mut controls = BTreeSet::new();
    for entry in tests {
        match entry {
            LaneTestEntry::Example(row) => {
                let expected_op = corpus.examples.get(&row.id).ok_or_else(|| {
                    LaneEvidenceError("manifest contains an unknown example id".into())
                })?;
                if expected_op != &row.op || !examples.insert(row.id.as_str()) {
                    return fail("manifest example identity or operation does not match corpus");
                }
            }
            LaneTestEntry::Exhaustive(row) => {
                let suite = corpus.suites.get(&row.id).ok_or_else(|| {
                    LaneEvidenceError("manifest contains an unknown exhaustive id".into())
                })?;
                if row.op != suite.op
                    || row.claim != suite.claim
                    || row.cases != suite.cases
                    || row.discarded != suite.discarded
                    || row.calls != suite.calls
                    || row.inputs_sha256 != suite.inputs_sha256
                    || row.expected_sha256 != suite.expected_sha256
                    || row.domain != suite.domain
                    || !suites.insert(row.id.as_str())
                {
                    return fail("manifest exhaustive row does not match its corpus suite");
                }
            }
            LaneTestEntry::NegativeControl(row) => {
                let control = corpus.controls.get(&row.id).ok_or_else(|| {
                    LaneEvidenceError("manifest contains an unknown negative-control id".into())
                })?;
                if row.op != control.op
                    || row.fault != control.fault
                    || row.target != control.target_kind
                    || row
                        .caught_by
                        .iter()
                        .any(|claim| !control.required_claims.contains(claim))
                    || !controls.insert(row.id.as_str())
                {
                    return fail("manifest negative-control row does not match its corpus control");
                }
            }
        }
    }
    if examples.len() != corpus.examples.len()
        || suites.len() != corpus.suites.len()
        || controls.len() != corpus.controls.len()
    {
        return fail("manifest omits corpus examples, exhaustive suites, or negative controls");
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CorpusSuite {
    op: String,
    claim: String,
    cases: u64,
    discarded: u64,
    calls: u64,
    inputs_sha256: String,
    expected_sha256: String,
    domain: String,
}

#[derive(Clone, Debug)]
struct CorpusControl {
    op: String,
    fault: String,
    target_kind: String,
    required_claims: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct CorpusProfile {
    spec: String,
    spec_sha256: String,
    examples: BTreeMap<String, String>,
    suites: BTreeMap<String, CorpusSuite>,
    controls: BTreeMap<String, CorpusControl>,
}

fn common_portable_fields(row: &Obj, label: &str) -> Result<(), LaneEvidenceError> {
    if string(row, "layer", label)? != "portable" {
        return fail("the ct-compare-v0 lane profile accepts only portable entries");
    }
    if string(row, "verdict", label)? != "pass" {
        return fail("lane manifest entries must record verdict pass");
    }
    Ok(())
}

fn passing_verdict(row: &Obj, label: &str) -> Result<String, LaneEvidenceError> {
    let verdict = string(row, "verdict", label)?.to_owned();
    if verdict != "pass" {
        return fail("lane manifest entries must record verdict pass");
    }
    Ok(verdict)
}

fn validate_artifact_caps(
    spec_bytes: &[u8],
    corpus_bytes: &[u8],
    cases_bytes: &[u8],
) -> Result<(), LaneEvidenceError> {
    if spec_bytes.len() > MAX_SPEC_BYTES {
        return fail("Warrant spec exceeds the 4 MiB lane-profile limit");
    }
    if corpus_bytes.len() > MAX_CORPUS_BYTES {
        return fail("Warrant corpus exceeds the 32 MiB lane-profile limit");
    }
    if cases_bytes.len() > MAX_CORPUS_BYTES {
        return fail("TFV cases exceed the 32 MiB lane-profile limit");
    }
    Ok(())
}

fn parse_json(bytes: &[u8], cap: usize, label: &str) -> Result<JsonValue, LaneEvidenceError> {
    if bytes.len() > cap {
        return fail(format!("{label} exceeds its {cap}-byte limit"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LaneEvidenceError(format!("{label} must be UTF-8")))?;
    validate_integer_tokens(bytes, label)?;
    JsonValue::parse(text)
        .map_err(|error| LaneEvidenceError(format!("invalid {label} JSON: {error}")))
}

/// The shared JSON value type stores numbers as f64. Profile counts are small,
/// but a large integer token can round to an apparently valid count before a
/// typed accessor sees it. Keep all numeric tokens canonical signed integers
/// before using that parser, preserving exact count semantics.
fn validate_integer_tokens(bytes: &[u8], label: &str) -> Result<(), LaneEvidenceError> {
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte == b'-' || byte.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && !matches!(
                    bytes[index],
                    b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t' | b':'
                )
            {
                index += 1;
            }
            let token = std::str::from_utf8(&bytes[start..index])
                .map_err(|_| LaneEvidenceError(format!("{label} contains a non-ASCII number")))?;
            let parsed = token
                .parse::<i64>()
                .ok()
                .filter(|value| value.to_string() == token)
                .ok_or_else(|| {
                    LaneEvidenceError(format!("{label} contains a non-canonical integer"))
                })?;
            if parsed < 0 {
                return fail(format!("{label} profile does not accept negative numbers"));
            }
            if parsed as u64 > MAX_TEST_CASES {
                return fail(format!("{label} integer exceeds the supported exact range"));
            }
            continue;
        }
        index += 1;
    }
    Ok(())
}

fn exact_fields(object: &Obj, required: &[&str], label: &str) -> Result<(), LaneEvidenceError> {
    if object.len() != required.len() || required.iter().any(|field| !object.contains_key(*field)) {
        return fail(format!("{label} fields do not match the closed schema"));
    }
    Ok(())
}

fn object<'a>(value: &'a JsonValue, label: &str) -> Result<&'a Obj, LaneEvidenceError> {
    value
        .as_object()
        .ok_or_else(|| LaneEvidenceError(format!("{label} must be an object")))
}

fn array<'a>(value: &'a JsonValue, label: &str) -> Result<&'a [JsonValue], LaneEvidenceError> {
    value
        .as_array()
        .ok_or_else(|| LaneEvidenceError(format!("{label} must be an array")))
}

fn field<'a>(object: &'a Obj, key: &str, label: &str) -> Result<&'a JsonValue, LaneEvidenceError> {
    object
        .get(key)
        .ok_or_else(|| LaneEvidenceError(format!("{label} is missing {key}")))
}

fn object_field<'a>(parent: &'a Obj, key: &str, label: &str) -> Result<&'a Obj, LaneEvidenceError> {
    object(field(parent, key, label)?, &format!("{label}.{key}"))
}

fn string<'a>(object: &'a Obj, key: &str, label: &str) -> Result<&'a str, LaneEvidenceError> {
    let value = field(object, key, label)?
        .as_str()
        .ok_or_else(|| LaneEvidenceError(format!("{label}.{key} must be a string")))?;
    if value.is_empty() {
        return fail(format!("{label}.{key} must not be empty"));
    }
    Ok(value)
}

fn boolean(object: &Obj, key: &str, label: &str) -> Result<bool, LaneEvidenceError> {
    field(object, key, label)?
        .as_bool()
        .ok_or_else(|| LaneEvidenceError(format!("{label}.{key} must be a boolean")))
}

fn number(object: &Obj, key: &str, label: &str) -> Result<u64, LaneEvidenceError> {
    let value = field(object, key, label)?
        .as_i64()
        .ok_or_else(|| LaneEvidenceError(format!("{label}.{key} must be an integer")))?;
    if value < 0 || value as u64 > MAX_TEST_CASES {
        return fail(format!(
            "{label}.{key} is outside the supported exact integer range"
        ));
    }
    Ok(value as u64)
}

fn positive_number(object: &Obj, key: &str, label: &str) -> Result<u64, LaneEvidenceError> {
    let value = number(object, key, label)?;
    if value == 0 {
        return fail(format!("{label}.{key} must be positive"));
    }
    Ok(value)
}

fn digest_field(object: &Obj, key: &str, label: &str) -> Result<String, LaneEvidenceError> {
    let value = string(object, key, label)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return fail(format!("{label}.{key} must be lowercase SHA-256 hex"));
    }
    Ok(value.to_owned())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn is_package_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_repo_path(value: &str) -> Result<(), LaneEvidenceError> {
    if value.starts_with('/') || value.contains('\\') || !value.is_ascii() {
        return fail("registry path must be ASCII, repository-relative, and slash-separated");
    }
    let components: Vec<&str> = value.split('/').collect();
    if components
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return fail("registry path contains an empty, dot, or dotdot component");
    }
    if components.iter().any(|part| {
        !part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._@+-".contains(&byte))
    }) {
        return fail("registry path contains a non-portable component");
    }
    Ok(())
}

fn validate_file_under_root(value: &str, root: &str) -> Result<(), LaneEvidenceError> {
    validate_repo_path(value)?;
    if !value.starts_with(&format!("{root}/")) {
        return fail("registry file path must be beneath its derived package_root");
    }
    Ok(())
}

fn fail<T>(message: impl Into<String>) -> Result<T, LaneEvidenceError> {
    Err(LaneEvidenceError(message.into()))
}
