//! Offline verifier for the pinned `v20260915` minify refresh report.
//!
//! The test suite reads only checked-in bytes. It never downloads or executes
//! the upstream compiler.

use coding_adventures_sha256::sha256_hex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MANIFEST_PATH: &str = "tests/oracle/manifest.json";
const REPORT_PATH: &str = "tests/oracle/minify-v20260915-report.json";
const EXPECTED_SCHEMA_VERSION: u32 = 1;
const EXPECTED_COHORT_ID: &str = "minify-v20260915";
const EXPECTED_COMMAND_ID: &str = "closure-flags-file-v1";
const EXPECTED_FIXTURE_SET_ID: &str = "minify-verified-v20260915";
const EXPECTED_FIXTURE_COUNT: usize = 462;
const ACCEPTED_CHANGE: &str = "accepted_expected_update";
const DOCUMENTED_DECLINE: &str = "documented_decline";
const MAX_FIXTURE_BYTES: usize = 1_048_576;
const MAX_PROCESS_OUTPUT_BYTES: usize = 1_048_576;
const MAX_CAPTURE_BYTES: usize = 33_554_432;
const MAX_REPORT_BYTES: usize = 33_554_432;
const MAX_MANIFEST_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshReport {
    schema_version: u32,
    manifest_schema_version: u32,
    cohort_id: String,
    oracle: ReportOracle,
    capture_environment: ReportEnvironment,
    command_id: String,
    summary: Summary,
    fixtures: Vec<FixtureResult>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportOracle {
    project: String,
    release_tag: String,
    annotated_tag_object_sha: String,
    release_commit_sha: String,
    audited_source_commit_sha: String,
    artifact_size_bytes: u64,
    artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportEnvironment {
    java_version: String,
    locale: String,
    timezone: String,
    encoding: String,
    working_directory: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Summary {
    total: usize,
    equal: usize,
    changed: usize,
    declined: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Classification {
    Equal,
    Changed,
    Declined,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    disposition: String,
    reason_id: String,
    reason: String,
    issue_url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureResult {
    fixture: String,
    classification: Classification,
    flags_path: String,
    flags_sha256: String,
    input_path: String,
    input_sha256: String,
    baseline_expected_sha256: String,
    current_expected_sha256: String,
    oracle_stdout_sha256: String,
    oracle_stdout_bytes: usize,
    oracle_stdout_hex: String,
    oracle_stderr_sha256: String,
    oracle_stderr_bytes: usize,
    oracle_stderr_hex: String,
    exit_code: Option<i32>,
    review: Option<Review>,
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn checked_in_manifest() -> serde_json::Value {
    let path = package_root().join(MANIFEST_PATH);
    let bytes = read_file_bounded(&path, MAX_MANIFEST_BYTES, "oracle manifest")
        .expect("read oracle manifest");
    serde_json::from_slice(&bytes).expect("parse oracle manifest")
}

fn checked_in_report() -> RefreshReport {
    let path = package_root().join(REPORT_PATH);
    let bytes = read_file_bounded(&path, MAX_REPORT_BYTES, "minify refresh report")
        .expect("read minify refresh report");
    serde_json::from_slice(&bytes).expect("parse strict minify refresh report")
}

fn manifest_string<'a>(
    manifest: &'a serde_json::Value,
    pointer: &str,
    errors: &mut Vec<String>,
) -> Option<&'a str> {
    let value = manifest
        .pointer(pointer)
        .and_then(serde_json::Value::as_str);
    if value.is_none() {
        errors.push(format!("manifest string is missing: {pointer}"));
    }
    value
}

fn manifest_u64(
    manifest: &serde_json::Value,
    pointer: &str,
    errors: &mut Vec<String>,
) -> Option<u64> {
    let value = manifest
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64);
    if value.is_none() {
        errors.push(format!("manifest integer is missing: {pointer}"));
    }
    value
}

fn manifest_fixture_cohort(
    manifest: &serde_json::Value,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let Some(sets) = manifest
        .get("fixture_sets")
        .and_then(serde_json::Value::as_array)
    else {
        errors.push("manifest fixture_sets is missing".to_string());
        return BTreeSet::new();
    };
    let mut cohort = BTreeSet::new();
    let mut matching_sets = 0;
    for set in sets {
        let fixtures = set
            .get("fixtures")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .filter(|fixture| fixture.starts_with("minify_"))
            .collect::<Vec<_>>();
        if fixtures.is_empty() {
            continue;
        }
        matching_sets += 1;
        if set.get("id").and_then(serde_json::Value::as_str) != Some(EXPECTED_FIXTURE_SET_ID)
            || set.get("disposition").and_then(serde_json::Value::as_str) != Some("upstream_golden")
            || set.get("command").and_then(serde_json::Value::as_str) != Some(EXPECTED_COMMAND_ID)
            || set
                .pointer("/current_provenance/status")
                .and_then(serde_json::Value::as_str)
                != Some("verified_release")
            || set
                .pointer("/current_provenance/release")
                .and_then(serde_json::Value::as_str)
                != manifest
                    .pointer("/upstream/release/tag")
                    .and_then(serde_json::Value::as_str)
            || set
                .pointer("/current_provenance/evidence")
                .and_then(serde_json::Value::as_str)
                != Some(REPORT_PATH)
        {
            errors.push("manifest minify fixture set lacks verified report provenance".to_string());
        }
        for fixture in fixtures {
            if !cohort.insert(fixture.to_string()) {
                errors.push(format!("duplicate manifest minify fixture: {fixture}"));
            }
        }
    }
    if matching_sets != 1 {
        errors.push(format!(
            "expected one verified minify fixture set, found {matching_sets}"
        ));
    }
    cohort
}

fn validate_report(
    root: &Path,
    manifest: &serde_json::Value,
    report: &RefreshReport,
) -> Vec<String> {
    let mut errors = Vec::new();
    if report.schema_version != EXPECTED_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported report schema version: {}",
            report.schema_version
        ));
    }
    let manifest_schema = manifest
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as u32;
    if report.manifest_schema_version != manifest_schema {
        errors.push("report manifest schema version drifted".to_string());
    }
    if report.cohort_id != EXPECTED_COHORT_ID || report.command_id != EXPECTED_COMMAND_ID {
        errors.push("report cohort or command identity drifted".to_string());
    }
    validate_pins(manifest, report, &mut errors);

    let manifest_cohort = manifest_fixture_cohort(manifest, &mut errors);
    if manifest_cohort.len() != EXPECTED_FIXTURE_COUNT {
        errors.push(format!(
            "manifest minify cohort has {}, expected {EXPECTED_FIXTURE_COUNT}",
            manifest_cohort.len()
        ));
    }
    if report.fixtures.len() != EXPECTED_FIXTURE_COUNT {
        errors.push(format!(
            "report has {} fixtures, expected {EXPECTED_FIXTURE_COUNT}",
            report.fixtures.len()
        ));
    }
    if !report
        .fixtures
        .windows(2)
        .all(|pair| pair[0].fixture < pair[1].fixture)
    {
        errors.push("report fixtures are not strictly sorted".to_string());
    }
    let report_cohort: BTreeSet<_> = report
        .fixtures
        .iter()
        .map(|fixture| fixture.fixture.clone())
        .collect();
    if report_cohort != manifest_cohort {
        errors.push("report fixture census differs from manifest cohort".to_string());
    }

    let mut reason_ids = BTreeSet::new();
    let mut aggregate_evidence_bytes = 0_usize;
    for result in &report.fixtures {
        validate_fixture(
            root,
            result,
            &mut reason_ids,
            &mut aggregate_evidence_bytes,
            &mut errors,
        );
    }
    let actual = Summary {
        total: report.fixtures.len(),
        equal: report
            .fixtures
            .iter()
            .filter(|item| item.classification == Classification::Equal)
            .count(),
        changed: report
            .fixtures
            .iter()
            .filter(|item| item.classification == Classification::Changed)
            .count(),
        declined: report
            .fixtures
            .iter()
            .filter(|item| item.classification == Classification::Declined)
            .count(),
    };
    if report.summary.total != actual.total
        || report.summary.equal != actual.equal
        || report.summary.changed != actual.changed
        || report.summary.declined != actual.declined
    {
        errors.push("report summary does not match fixture classifications".to_string());
    }
    errors
}

fn validate_pins(manifest: &serde_json::Value, report: &RefreshReport, errors: &mut Vec<String>) {
    let string_pairs = [
        ("/upstream/project", report.oracle.project.as_str()),
        ("/upstream/release/tag", report.oracle.release_tag.as_str()),
        (
            "/upstream/release/annotated_tag_object_sha",
            report.oracle.annotated_tag_object_sha.as_str(),
        ),
        (
            "/upstream/release/commit_sha",
            report.oracle.release_commit_sha.as_str(),
        ),
        (
            "/upstream/audited_source/commit_sha",
            report.oracle.audited_source_commit_sha.as_str(),
        ),
        (
            "/upstream/release/artifact/sha256",
            report.oracle.artifact_sha256.as_str(),
        ),
        (
            "/capture_environment/java_version",
            report.capture_environment.java_version.as_str(),
        ),
        (
            "/capture_environment/locale",
            report.capture_environment.locale.as_str(),
        ),
        (
            "/capture_environment/timezone",
            report.capture_environment.timezone.as_str(),
        ),
        (
            "/capture_environment/encoding",
            report.capture_environment.encoding.as_str(),
        ),
        (
            "/capture_environment/working_directory",
            report.capture_environment.working_directory.as_str(),
        ),
    ];
    for (pointer, actual) in string_pairs {
        if manifest_string(manifest, pointer, errors) != Some(actual) {
            errors.push(format!("report pin differs from manifest: {pointer}"));
        }
    }
    if manifest_u64(manifest, "/upstream/release/artifact/size_bytes", errors)
        != Some(report.oracle.artifact_size_bytes)
    {
        errors.push("report artifact size differs from manifest".to_string());
    }
}

fn validate_fixture(
    root: &Path,
    result: &FixtureResult,
    reason_ids: &mut BTreeSet<String>,
    aggregate_evidence_bytes: &mut usize,
    errors: &mut Vec<String>,
) {
    let prefix = format!("fixture {}", result.fixture);
    for (label, value) in [
        ("flags", result.flags_sha256.as_str()),
        ("input", result.input_sha256.as_str()),
        (
            "baseline expected",
            result.baseline_expected_sha256.as_str(),
        ),
        ("current expected", result.current_expected_sha256.as_str()),
        ("oracle stdout", result.oracle_stdout_sha256.as_str()),
        ("oracle stderr", result.oracle_stderr_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            errors.push(format!("{prefix} {label} hash is not lowercase SHA-256"));
        }
    }
    let expected_flags = format!("tests/diff/{}/flags.txt", result.fixture);
    if result.flags_path != expected_flags || !is_safe_relative(Path::new(&result.flags_path)) {
        errors.push(format!("{prefix} flags path mismatch"));
        return;
    }
    let flags = match read_contained(root, &result.flags_path, root) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!("{prefix} {error}"));
            return;
        }
    };
    if sha256_hex(&flags) != result.flags_sha256 {
        errors.push(format!("{prefix} flags hash mismatch"));
    }
    let input_from_flags = match validated_input_from_flags(&result.fixture, &flags) {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("{prefix} {error}"));
            return;
        }
    };
    if result.input_path != input_from_flags || !is_safe_relative(Path::new(&result.input_path)) {
        errors.push(format!("{prefix} input path mismatch"));
        return;
    }
    let input_boundary = root.join(format!("tests/diff/{}/input", result.fixture));
    let input = match read_contained(root, &result.input_path, &input_boundary) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!("{prefix} {error}"));
            return;
        }
    };
    if sha256_hex(&input) != result.input_sha256 {
        errors.push(format!("{prefix} input hash mismatch"));
    }
    let expected_path = format!("tests/diff/{}/expected.stdout", result.fixture);
    let fixture_boundary = root.join(format!("tests/diff/{}", result.fixture));
    let expected = match read_contained(root, &expected_path, &fixture_boundary) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!("{prefix} {error}"));
            return;
        }
    };
    let current_hash = sha256_hex(&expected);
    if current_hash != result.current_expected_sha256 {
        errors.push(format!("{prefix} current expected hash mismatch"));
    }

    let stdout = decode_evidence_hex(
        &result.oracle_stdout_hex,
        result.oracle_stdout_bytes,
        "stdout",
    )
    .unwrap_or_else(|error| {
        errors.push(format!("{prefix} invalid stdout hex: {error}"));
        Vec::new()
    });
    let stderr = decode_evidence_hex(
        &result.oracle_stderr_hex,
        result.oracle_stderr_bytes,
        "stderr",
    )
    .unwrap_or_else(|error| {
        errors.push(format!("{prefix} invalid stderr hex: {error}"));
        Vec::new()
    });
    match result
        .oracle_stdout_bytes
        .checked_add(result.oracle_stderr_bytes)
        .and_then(|fixture_bytes| aggregate_evidence_bytes.checked_add(fixture_bytes))
    {
        Some(total) if total <= MAX_CAPTURE_BYTES => *aggregate_evidence_bytes = total,
        _ => errors.push(format!(
            "{prefix} exceeds the aggregate {MAX_CAPTURE_BYTES}-byte evidence limit"
        )),
    }
    if stdout.len() != result.oracle_stdout_bytes
        || sha256_hex(&stdout) != result.oracle_stdout_sha256
    {
        errors.push(format!("{prefix} stdout evidence mismatch"));
    }
    if stderr.len() != result.oracle_stderr_bytes
        || sha256_hex(&stderr) != result.oracle_stderr_sha256
    {
        errors.push(format!("{prefix} stderr evidence mismatch"));
    }

    match result.classification {
        Classification::Equal => {
            if result.exit_code != Some(0)
                || stdout != expected
                || result.baseline_expected_sha256 != current_hash
                || result.oracle_stdout_sha256 != current_hash
                || result.review.is_some()
            {
                errors.push(format!("{prefix} does not satisfy equal-result invariants"));
            }
        }
        Classification::Changed => {
            if result.exit_code != Some(0)
                || result.baseline_expected_sha256 == result.oracle_stdout_sha256
                || result.current_expected_sha256 != result.oracle_stdout_sha256
                || stdout != expected
            {
                errors.push(format!(
                    "{prefix} does not satisfy changed-result invariants"
                ));
            }
            validate_review(
                result.review.as_ref(),
                ACCEPTED_CHANGE,
                reason_ids,
                &prefix,
                errors,
            );
        }
        Classification::Declined => {
            if !matches!(result.exit_code, Some(code) if code != 0)
                || result.current_expected_sha256 != result.baseline_expected_sha256
            {
                errors.push(format!(
                    "{prefix} does not satisfy declined-result invariants"
                ));
            }
            validate_review(
                result.review.as_ref(),
                DOCUMENTED_DECLINE,
                reason_ids,
                &prefix,
                errors,
            );
        }
    }
}

fn validate_review(
    review: Option<&Review>,
    disposition: &str,
    reason_ids: &mut BTreeSet<String>,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let Some(review) = review else {
        errors.push(format!("{prefix} lacks a required review"));
        return;
    };
    if review.disposition != disposition
        || !is_reason_id(&review.reason_id)
        || review.reason.trim().is_empty()
        || !is_issue_url(&review.issue_url)
        || !reason_ids.insert(review.reason_id.clone())
    {
        errors.push(format!("{prefix} has an invalid or duplicate review"));
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_reason_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_issue_url(value: &str) -> bool {
    value
        .strip_prefix("https://github.com/adhithyan15/coding-adventures/issues/")
        .is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn validated_input_from_flags(fixture: &str, bytes: &[u8]) -> Result<String, String> {
    let text =
        std::str::from_utf8(bytes).map_err(|error| format!("flags are not UTF-8: {error}"))?;
    let flags: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if flags.len() != 4 {
        return Err("flags do not contain exactly two pairs".to_string());
    }
    let pairs: BTreeMap<_, _> = flags
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    if pairs.len() != 2
        || pairs.get("--compilation_level") != Some(&"WHITESPACE_ONLY")
        || !pairs.contains_key("--js")
    {
        return Err("flags differ from the reviewed WHITESPACE_ONLY shape".to_string());
    }
    let input = pairs["--js"];
    let expected_prefix = format!("tests/diff/{fixture}/input/");
    if input.contains('\\')
        || !input.starts_with(&expected_prefix)
        || !is_safe_relative(Path::new(input))
    {
        return Err("--js path escapes the fixture input directory".to_string());
    }
    Ok(input.to_string())
}

fn read_contained(root: &Path, relative: &str, boundary: &Path) -> Result<Vec<u8>, String> {
    if !is_safe_relative(Path::new(relative)) {
        return Err(format!("unsafe evidence path: {relative}"));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("canonicalize package root: {error}"))?;
    let canonical_boundary = boundary
        .canonicalize()
        .map_err(|error| format!("canonicalize evidence boundary: {error}"))?;
    let canonical = root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("canonicalize evidence path {relative}: {error}"))?;
    if !canonical.starts_with(&canonical_root)
        || !canonical.starts_with(&canonical_boundary)
        || !canonical.is_file()
    {
        return Err(format!("evidence path escapes its boundary: {relative}"));
    }
    read_file_bounded(&canonical, MAX_FIXTURE_BYTES, "fixture evidence")
}

fn read_file_bounded(path: &Path, limit: usize, label: &str) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("open {label} {}: {error}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|error| format!("stat {label} {}: {error}", path.display()))?
        .len();
    if size > limit as u64 {
        return Err(format!(
            "{label} {} exceeds the {limit}-byte limit",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!(
            "{label} {} grew beyond the {limit}-byte limit while being read",
            path.display()
        ));
    }
    Ok(bytes)
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn decode_lower_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("value is not even-length lowercase hexadecimal".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("ASCII hex pair");
            u8::from_str_radix(text, 16).map_err(|error| error.to_string())
        })
        .collect()
}

fn decode_evidence_hex(value: &str, declared_bytes: usize, label: &str) -> Result<Vec<u8>, String> {
    if declared_bytes > MAX_PROCESS_OUTPUT_BYTES {
        return Err(format!(
            "{label} evidence exceeds the {MAX_PROCESS_OUTPUT_BYTES}-byte limit"
        ));
    }
    let encoded_bytes = declared_bytes
        .checked_mul(2)
        .ok_or_else(|| format!("{label} encoded length overflow"))?;
    if value.len() != encoded_bytes {
        return Err(format!(
            "{label} hexadecimal length does not match its declared byte length"
        ));
    }
    decode_lower_hex(value)
}

fn assert_invalid(report: &RefreshReport, expected_fragment: &str) {
    let errors = validate_report(&package_root(), &checked_in_manifest(), report);
    assert!(
        errors.iter().any(|error| error.contains(expected_fragment)),
        "missing {expected_fragment:?} in {errors:#?}"
    );
}

#[test]
fn checked_in_refresh_report_is_complete_and_verified() {
    let errors = validate_report(
        &package_root(),
        &checked_in_manifest(),
        &checked_in_report(),
    );
    assert!(
        errors.is_empty(),
        "refresh report violations:\n{}",
        errors.join("\n")
    );
}

#[test]
fn strict_report_schema_rejects_unknown_fields() {
    let report_path = package_root().join(REPORT_PATH);
    let report_bytes = read_file_bounded(&report_path, MAX_REPORT_BYTES, "minify refresh report")
        .expect("read report");
    let mut value: serde_json::Value =
        serde_json::from_slice(&report_bytes).expect("parse report value");
    value
        .as_object_mut()
        .expect("report object")
        .insert("surprise".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<RefreshReport>(value).is_err());
}

#[test]
fn verifier_rejects_missing_fixture_and_false_summary() {
    let mut report = checked_in_report();
    report.fixtures.pop();
    assert_invalid(&report, "report has 461 fixtures");
    assert_invalid(&report, "report summary does not match");
}

#[test]
fn verifier_rejects_tampered_raw_bytes() {
    let mut report = checked_in_report();
    report.fixtures[0].oracle_stdout_hex.push_str("00");
    assert_invalid(&report, "stdout evidence mismatch");
}

#[test]
fn verifier_rejects_unreviewed_change() {
    let mut report = checked_in_report();
    report.fixtures[0].classification = Classification::Changed;
    report.fixtures[0].baseline_expected_sha256 = "0".repeat(64);
    assert_invalid(&report, "lacks a required review");
}

#[test]
fn verifier_rejects_signal_terminated_decline() {
    let mut report = checked_in_report();
    report.fixtures[0].classification = Classification::Declined;
    report.fixtures[0].exit_code = None;
    assert_invalid(&report, "does not satisfy declined-result invariants");
}

#[test]
fn verifier_rejects_declared_output_over_the_capture_limit() {
    let mut report = checked_in_report();
    report.fixtures[0].oracle_stdout_bytes = MAX_PROCESS_OUTPUT_BYTES + 1;
    assert_invalid(&report, "stdout evidence exceeds");
}

#[test]
fn verifier_rejects_escaped_paths_before_reading() {
    let mut report = checked_in_report();
    report.fixtures[0].flags_path = "../../outside-flags.txt".to_string();
    assert_invalid(&report, "flags path mismatch");
}

#[test]
fn verifier_rejects_oracle_pin_drift() {
    let mut report = checked_in_report();
    report.oracle.release_tag = "v20990101".to_string();
    assert_invalid(&report, "report pin differs from manifest");
}

#[test]
fn verifier_rejects_malformed_sha256_text() {
    let mut report = checked_in_report();
    report.fixtures[0].input_sha256 = "A".repeat(64);
    assert_invalid(&report, "input hash is not lowercase SHA-256");
}

#[test]
fn review_identifiers_and_issue_urls_are_strict() {
    assert!(is_reason_id("legacy-octal-warning"));
    assert!(!is_reason_id("Legacy_Octal"));
    assert!(is_issue_url(
        "https://github.com/adhithyan15/coding-adventures/issues/15571"
    ));
    assert!(!is_issue_url(
        "https://github.com/adhithyan15/coding-adventures/issues/not-a-number"
    ));
}
