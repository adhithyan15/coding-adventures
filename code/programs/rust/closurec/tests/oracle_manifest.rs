//! Offline verifier for the Closure Compiler oracle manifest.
//!
//! The manifest is a trust ledger, not a downloader. CI proves that every
//! differential fixture has exactly one reviewed disposition and that every
//! claimed upstream command resolves inside this package. It deliberately
//! never opens the network or executes Java: artifact acquisition is a
//! maintainer action whose byte length and SHA-256 are pinned in the ledger.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

const MANIFEST_PATH: &str = "tests/oracle/manifest.json";
const MINIFY_REPORT_PATH: &str = "tests/oracle/minify-v20260915-report.json";
const MINIFY_VERIFIED_SET_ID: &str = "minify-verified-v20260915";
const REFUSALS_REPORT_PATH: &str = "tests/oracle/upstream-refusals-v20260915.json";
const REFUSALS_SET_ID: &str = "non-minify-upstream-refuses-v20260915";
const SCHEMA_VERSION: u32 = 1;
const RELEASE: &str = "v20260915";
const RELEASE_TAG_OBJECT: &str = "72421c28d352e5dda9a111bec39c3d41af46f3a3";
const RELEASE_COMMIT: &str = "56007b2869ef6ce70b659b033459b8d8113101de";
const AUDIT_COMMIT: &str = "10ca677aff381d2c2e6e1b254ba32861e503173d";
const ARTIFACT_SHA256: &str = "9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d";
const ARTIFACT_SIZE: u64 = 14_976_538;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleManifest {
    schema_version: u32,
    upstream: Upstream,
    capture_environment: CaptureEnvironment,
    commands: Vec<CommandTemplate>,
    fixture_sets: Vec<FixtureSet>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Upstream {
    project: String,
    repository_url: String,
    license_spdx: String,
    license_url: String,
    release: Release,
    audited_source: AuditedSource,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    tag: String,
    annotated_tag_object_sha: String,
    commit_sha: String,
    artifact: Artifact,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    repository: String,
    group_id: String,
    artifact_id: String,
    version: String,
    url: String,
    pom_url: String,
    size_bytes: u64,
    sha256: String,
    embedded_build_jdk_spec: String,
    embedded_license_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditedSource {
    branch: String,
    commit_sha: String,
    observed_on: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureEnvironment {
    java_version: String,
    locale: String,
    timezone: String,
    encoding: String,
    working_directory: String,
    oracle_jar_placeholder: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandTemplate {
    id: String,
    kind: CommandKind,
    description: String,
    executable: String,
    working_directory: String,
    argv: Vec<String>,
    flags_path: Option<String>,
    input_root: Option<String>,
    matrix: Vec<CommandCase>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CommandKind {
    FlagsFile,
    Matrix,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct CommandCase {
    id: String,
    variables: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureSet {
    id: String,
    disposition: Disposition,
    command: Option<String>,
    current_provenance: CurrentProvenance,
    expectation: Expectation,
    local_boundary: Option<String>,
    default_harness: Option<String>,
    harnesses: BTreeMap<String, String>,
    fixtures: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    UpstreamGolden,
    /// The pinned oracle REFUSES these inputs **as invoked** — with the
    /// fixture's own `flags.txt`, verbatim — producing a nonzero exit, empty
    /// stdout and a named `JSC_` diagnostic, captured per fixture in the
    /// evidence report (CCR-081).
    ///
    /// The distinction that matters: an `UpstreamGolden` set claims upstream
    /// produced its bytes; this one claims that under the invocation the
    /// fixture records, upstream produced *nothing*, so the bytes in
    /// `expected.stdout` are our own behaviour on input upstream declines.
    /// Whether `closurec` should decline it too is CCR-075 (#15860) and is not
    /// settled here.
    ///
    /// **"As invoked" is the whole predicate — do not read it as "no invocation
    /// of upstream could produce this golden."** That stronger claim is also
    /// true of several fixtures here, the ES-module ones especially, but it
    /// quantifies over a hand search no script can re-run, so it lives on
    /// #15868 rather than in a file this gate trusts. The difference is not
    /// academic: `simple-importmeta` is in this set and upstream compiles it
    /// happily given `--chunk_output_type=ES_MODULES`. It qualifies because its
    /// own `flags.txt` omits that flag, and it stays until someone completes
    /// the invocation and recaptures the golden.
    ///
    /// So, deciding whether a thirteenth fixture belongs: run it as its
    /// `flags.txt` says and see whether upstream refuses. Do not reason about
    /// whether some other invocation might have worked — that question belongs
    /// on the issue, not here.
    UpstreamRefuses,
    MixedContract,
    LocalExtension,
    LocalContract,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentProvenance {
    status: ProvenanceStatus,
    release: Option<String>,
    evidence: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ProvenanceStatus {
    VerifiedRelease,
    DocumentedRelease,
    Unverified,
    Local,
}

/// The shape of `upstream-refusals-<release>.json`. Only the fields this
/// validator checks are modelled; `deny_unknown_fields` is deliberately NOT
/// used here, because the report also carries human-facing prose (`description`,
/// `java_version`) that should be free to grow without breaking the gate.
#[derive(Clone, Debug, Deserialize)]
struct RefusalReport {
    release: String,
    oracle_jar_sha256: String,
    java_version: String,
    /// Required to be non-empty exactly when `java_version` differs from the
    /// manifest's pinned capture JVM. A capture on another JVM is allowed — a
    /// maintainer may not have the pinned build — but it must be DECLARED, so
    /// the mismatch reaches a reviewer instead of sitting silently in an
    /// artifact the gate treats as authoritative.
    java_version_deviation: Option<String>,
    command: String,
    refusals: BTreeMap<String, RefusalEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct RefusalEntry {
    exit_status: i32,
    stdout_bytes: u64,
    diagnostic: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Expectation {
    ExpectedStdout,
    InlineHarness,
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_manifest() -> OracleManifest {
    let text =
        std::fs::read_to_string(package_root().join(MANIFEST_PATH)).expect("read oracle manifest");
    serde_json::from_str(&text).expect("parse strict oracle manifest")
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_release(value: &str) -> bool {
    value.len() == 9
        && value.starts_with('v')
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
}

fn is_iso_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn validate_existing_path(
    root: &Path,
    relative: &str,
    expected_directory: bool,
    label: &str,
    errors: &mut Vec<String>,
) {
    let _ = resolve_existing_path(root, relative, expected_directory, label, errors);
}

fn resolve_existing_path(
    root: &Path,
    relative: &str,
    expected_directory: bool,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<PathBuf> {
    if !is_safe_relative(relative) {
        errors.push(format!("{label} is not a safe relative path: {relative}"));
        return None;
    }

    let joined = root.join(relative);
    let expected_kind = if expected_directory {
        "directory"
    } else {
        "file"
    };
    if (expected_directory && !joined.is_dir()) || (!expected_directory && !joined.is_file()) {
        errors.push(format!(
            "{label} does not resolve to a {expected_kind}: {relative}"
        ));
        return None;
    }

    let Ok(canonical_root) = root.canonicalize() else {
        errors.push(format!(
            "cannot canonicalize package root: {}",
            root.display()
        ));
        return None;
    };
    let Ok(canonical_path) = joined.canonicalize() else {
        errors.push(format!("cannot canonicalize {label}: {relative}"));
        return None;
    };
    if !canonical_path.starts_with(&canonical_root) {
        errors.push(format!("{label} escapes the package root: {relative}"));
        return None;
    }
    Some(canonical_path)
}

fn placeholders(parts: &[String]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for part in parts {
        let mut rest = part.as_str();
        while let Some(start) = rest.find('{') {
            let after_start = &rest[start..];
            let Some(end) = after_start.find('}') else {
                found.insert(after_start.to_string());
                break;
            };
            found.insert(after_start[..=end].to_string());
            rest = &after_start[end + 1..];
        }
    }
    found
}

fn render_fixture_template(template: &str, fixture: &str, harness: &str) -> String {
    template
        .replace("{fixture}", fixture)
        .replace("{harness}", harness)
}

fn discover_fixture_directories(root: &Path, errors: &mut Vec<String>) -> BTreeSet<String> {
    let diff_root = root.join("tests/diff");
    let Ok(entries) = std::fs::read_dir(&diff_root) else {
        errors.push(format!("cannot read fixture root: {}", diff_root.display()));
        return BTreeSet::new();
    };

    let mut names = BTreeSet::new();
    for entry in entries {
        match entry {
            Ok(entry) => match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    names.insert(entry.file_name().to_string_lossy().into_owned());
                }
                Ok(_) => {}
                Err(error) => errors.push(format!(
                    "cannot inspect fixture entry {}: {error}",
                    entry.path().display()
                )),
            },
            Err(error) => errors.push(format!("cannot read fixture entry: {error}")),
        }
    }
    names
}

fn validate_pins(manifest: &OracleManifest, errors: &mut Vec<String>) {
    if manifest.schema_version != SCHEMA_VERSION {
        errors.push(format!(
            "unsupported schema version: got {}, expected {SCHEMA_VERSION}",
            manifest.schema_version
        ));
    }

    let upstream = &manifest.upstream;
    if upstream.project != "google/closure-compiler" {
        errors.push(format!("unexpected upstream project: {}", upstream.project));
    }
    if upstream.repository_url != "https://github.com/google/closure-compiler" {
        errors.push(format!(
            "unexpected upstream repository URL: {}",
            upstream.repository_url
        ));
    }
    if upstream.license_spdx != "Apache-2.0" {
        errors.push(format!(
            "unexpected SPDX license: {}",
            upstream.license_spdx
        ));
    }
    if upstream.license_url != "https://github.com/google/closure-compiler/blob/master/COPYING" {
        errors.push(format!("unexpected license URL: {}", upstream.license_url));
    }

    let release = &upstream.release;
    if release.tag != RELEASE {
        errors.push(format!("unexpected release: {}", release.tag));
    }
    if release.annotated_tag_object_sha != RELEASE_TAG_OBJECT
        || !is_lower_hex(&release.annotated_tag_object_sha, 40)
    {
        errors.push(format!(
            "invalid release tag object SHA: {}",
            release.annotated_tag_object_sha
        ));
    }
    if release.commit_sha != RELEASE_COMMIT || !is_lower_hex(&release.commit_sha, 40) {
        errors.push(format!(
            "invalid release commit SHA: {}",
            release.commit_sha
        ));
    }

    let artifact = &release.artifact;
    if artifact.repository != "Maven Central"
        || artifact.group_id != "com.google.javascript"
        || artifact.artifact_id != "closure-compiler"
        || artifact.version != RELEASE
    {
        errors.push("unexpected Maven artifact coordinates".to_string());
    }
    if artifact.url
        != "https://repo1.maven.org/maven2/com/google/javascript/closure-compiler/v20260915/closure-compiler-v20260915.jar"
        || artifact.pom_url
            != "https://repo1.maven.org/maven2/com/google/javascript/closure-compiler/v20260915/closure-compiler-v20260915.pom"
    {
        errors.push("unexpected Maven artifact URL".to_string());
    }
    if artifact.size_bytes != ARTIFACT_SIZE {
        errors.push(format!("unexpected artifact size: {}", artifact.size_bytes));
    }
    if artifact.sha256 != ARTIFACT_SHA256 || !is_lower_hex(&artifact.sha256, 64) {
        errors.push(format!("invalid artifact SHA-256: {}", artifact.sha256));
    }
    if artifact.embedded_build_jdk_spec != "21" {
        errors.push(format!(
            "unexpected embedded JDK spec: {}",
            artifact.embedded_build_jdk_spec
        ));
    }
    let license_paths: BTreeSet<_> = artifact
        .embedded_license_paths
        .iter()
        .map(String::as_str)
        .collect();
    let expected_license_paths = BTreeSet::from([
        "META-INF/LICENSE",
        "META-INF/LICENSE.txt",
        "META-INF/NOTICE.txt",
    ]);
    if license_paths != expected_license_paths
        || artifact.embedded_license_paths.len() != expected_license_paths.len()
    {
        errors.push("embedded license path inventory is incomplete or duplicated".to_string());
    }

    let audit = &upstream.audited_source;
    if audit.branch != "master"
        || audit.commit_sha != AUDIT_COMMIT
        || !is_lower_hex(&audit.commit_sha, 40)
        || !is_iso_date(&audit.observed_on)
    {
        errors.push("invalid audited-source pin".to_string());
    }

    let environment = &manifest.capture_environment;
    if environment.java_version != "21.0.12"
        || environment.locale != "C.UTF-8"
        || environment.timezone != "UTC"
        || environment.encoding != "UTF-8"
        || environment.working_directory != "code/programs/rust/closurec"
        || environment.oracle_jar_placeholder != "{oracle_jar}"
    {
        errors.push("capture environment is not the reviewed deterministic profile".to_string());
    }
}

fn validate_commands(
    root: &Path,
    commands: &[CommandTemplate],
    errors: &mut Vec<String>,
) -> BTreeMap<String, CommandKind> {
    let mut kinds = BTreeMap::new();
    for command in commands {
        if !is_slug(&command.id) {
            errors.push(format!("command id is not a slug: {}", command.id));
        }
        if kinds.insert(command.id.clone(), command.kind).is_some() {
            errors.push(format!("duplicate command id: {}", command.id));
        }
        if command.description.trim().is_empty() {
            errors.push(format!("command {} has an empty description", command.id));
        }
        if command.executable != "java" || command.working_directory != "{package_root}" {
            errors.push(format!(
                "command {} has an unexpected execution boundary",
                command.id
            ));
        }

        let actual_placeholders = placeholders(&command.argv);
        match command.kind {
            CommandKind::FlagsFile => {
                let expected =
                    BTreeSet::from(["{fixture_flags}".to_string(), "{oracle_jar}".to_string()]);
                if actual_placeholders != expected {
                    errors.push(format!("command {} has invalid placeholders", command.id));
                }
                if command.flags_path.as_deref() != Some("tests/diff/{fixture}/flags.txt")
                    || command.input_root.as_deref() != Some("tests/diff/{fixture}/input")
                    || !command.matrix.is_empty()
                {
                    errors.push(format!(
                        "flags-file command {} has invalid fields",
                        command.id
                    ));
                }
            }
            CommandKind::Matrix => {
                let expected = BTreeSet::from([
                    "{compilation_level}".to_string(),
                    "{fixture_input}".to_string(),
                    "{oracle_jar}".to_string(),
                ]);
                if actual_placeholders != expected {
                    errors.push(format!("command {} has invalid placeholders", command.id));
                }
                if command.flags_path.is_some()
                    || command.input_root.is_some()
                    || command.matrix.is_empty()
                {
                    errors.push(format!("matrix command {} has invalid fields", command.id));
                }
            }
        }

        let mut case_ids = BTreeSet::new();
        for case in &command.matrix {
            if !is_slug(&case.id) || !case_ids.insert(case.id.clone()) {
                errors.push(format!(
                    "command {} has an invalid or duplicate matrix id",
                    command.id
                ));
            }
            for (key, value) in &case.variables {
                if !is_slug(key) || value.trim().is_empty() {
                    errors.push(format!(
                        "command {} has an invalid matrix variable",
                        command.id
                    ));
                }
                if key == "fixture_input" {
                    validate_existing_path(
                        root,
                        value,
                        false,
                        &format!("command {} matrix input", command.id),
                        errors,
                    );
                }
            }
            let variable_placeholders: BTreeSet<_> = case
                .variables
                .keys()
                .map(|key| format!("{{{key}}}"))
                .collect();
            let required: BTreeSet<_> = actual_placeholders
                .iter()
                .filter(|placeholder| placeholder.as_str() != "{oracle_jar}")
                .cloned()
                .collect();
            if command.kind == CommandKind::Matrix && variable_placeholders != required {
                errors.push(format!(
                    "command {} matrix case {} does not bind every placeholder",
                    command.id, case.id
                ));
            }
        }

        let actual_argv: Vec<_> = command.argv.iter().map(String::as_str).collect();
        match command.id.as_str() {
            "closure-flags-file-v1" => {
                let expected_argv = vec![
                    "-Duser.language=en",
                    "-Duser.country=US",
                    "-Duser.timezone=UTC",
                    "-Dfile.encoding=UTF-8",
                    "-jar",
                    "{oracle_jar}",
                    "{fixture_flags}",
                ];
                if command.kind != CommandKind::FlagsFile
                    || actual_argv != expected_argv
                    || command.flags_path.as_deref() != Some("tests/diff/{fixture}/flags.txt")
                    || command.input_root.as_deref() != Some("tests/diff/{fixture}/input")
                    || !command.matrix.is_empty()
                {
                    errors.push(
                        "command closure-flags-file-v1 differs from the reviewed template"
                            .to_string(),
                    );
                }
            }
            "typed-pipeline-failure-matrix-v1" => {
                let expected_argv = vec![
                    "-Duser.language=en",
                    "-Duser.country=US",
                    "-Duser.timezone=UTC",
                    "-Dfile.encoding=UTF-8",
                    "-jar",
                    "{oracle_jar}",
                    "--compilation_level",
                    "{compilation_level}",
                    "--language_out",
                    "NO_TRANSPILE",
                    "--js",
                    "{fixture_input}",
                ];
                let expected_matrix = vec![
                    CommandCase {
                        id: "advanced-destructuring".to_string(),
                        variables: BTreeMap::from([
                            ("compilation_level".to_string(), "ADVANCED".to_string()),
                            (
                                "fixture_input".to_string(),
                                "tests/diff/typed-pipeline-failure/input/destructuring.js"
                                    .to_string(),
                            ),
                        ]),
                    },
                    CommandCase {
                        id: "advanced-malformed".to_string(),
                        variables: BTreeMap::from([
                            ("compilation_level".to_string(), "ADVANCED".to_string()),
                            (
                                "fixture_input".to_string(),
                                "tests/diff/typed-pipeline-failure/input/malformed.js".to_string(),
                            ),
                        ]),
                    },
                    CommandCase {
                        id: "simple-destructuring".to_string(),
                        variables: BTreeMap::from([
                            ("compilation_level".to_string(), "SIMPLE".to_string()),
                            (
                                "fixture_input".to_string(),
                                "tests/diff/typed-pipeline-failure/input/destructuring.js"
                                    .to_string(),
                            ),
                        ]),
                    },
                    CommandCase {
                        id: "simple-malformed".to_string(),
                        variables: BTreeMap::from([
                            ("compilation_level".to_string(), "SIMPLE".to_string()),
                            (
                                "fixture_input".to_string(),
                                "tests/diff/typed-pipeline-failure/input/malformed.js".to_string(),
                            ),
                        ]),
                    },
                ];
                if command.kind != CommandKind::Matrix
                    || actual_argv != expected_argv
                    || command.flags_path.is_some()
                    || command.input_root.is_some()
                    || command.matrix != expected_matrix
                {
                    errors.push(
                        "command typed-pipeline-failure-matrix-v1 differs from the reviewed template"
                            .to_string(),
                    );
                }
            }
            other => errors.push(format!("unreviewed command template id: {other}")),
        }
    }
    kinds
}

/// Cross-check an `UpstreamRefuses` set against its captured evidence file.
///
/// This is the whole point of the disposition. Saying "upstream refuses these"
/// in the manifest costs nothing and proves nothing; CCR-079 (#15866) is the
/// standing lesson that a ledger the gate trusts, whose values a human typed,
/// is an assertion wearing evidence's clothes. So the manifest's claim is only
/// as good as the report generated by `code/scripts/capture-upstream-refusals.sh`,
/// and this function makes the two agree or fails.
///
/// Reading a JSON file in the repository keeps the "never execute Java, never
/// touch the network" rule in this module's header intact: the Java ran when a
/// maintainer generated the report, exactly as the JAR download did. The path
/// read is the compile-time constant `REFUSALS_REPORT_PATH`, routed through
/// `resolve_existing_path` like every other evidence path — the manifest's own
/// `evidence` field is only ever compared against that constant for equality,
/// never used to build the path that gets opened, so a manifest cannot steer
/// this read.
fn validate_refusal_evidence(
    root: &Path,
    set: &FixtureSet,
    pinned_java_version: &str,
    errors: &mut Vec<String>,
) {
    let Some(path) =
        resolve_existing_path(root, REFUSALS_REPORT_PATH, false, "refusal evidence", errors)
    else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        errors.push(format!(
            "refusal evidence {REFUSALS_REPORT_PATH} is unreadable"
        ));
        return;
    };
    let Ok(report) = serde_json::from_str::<RefusalReport>(&text) else {
        errors.push(format!(
            "refusal evidence {REFUSALS_REPORT_PATH} does not parse"
        ));
        return;
    };
    errors.extend(check_refusal_report(set, &report, pinned_java_version));
}

/// The report-side half of [`validate_refusal_evidence`], as a pure function
/// over an already-parsed report.
///
/// It is split out for one reason. These checks read a file on disk, so a test
/// that mutates the *manifest* cannot reach them — and a review of this change
/// found exactly that: five of the six could be deleted with the suite still
/// green. The enforcement was real, but it was coming from
/// `refusal_evidence_on_disk_is_a_genuine_capture` asserting against the
/// committed artifact, not from the code whose doc comment advertised it.
/// Taking the report by reference lets `refusal_report_checks_reject_a_doctored_report`
/// feed synthetic ones and prove every branch fires.
///
/// | Check | The lie it catches |
/// |---|---|
/// | keys == fixtures, both ways | a fixture with no captured refusal, or a capture with no fixture |
/// | release matches the pin | evidence captured against some other Closure build |
/// | jar sha matches the pin | evidence captured against a JAR that is not the one we pin |
/// | command matches the set | evidence captured with an invocation the set does not declare |
/// | JVM matches, or deviates *and says so* | a capture on an undeclared JVM |
/// | `exit_status != 0` | a fixture that actually compiles, filed as a refusal |
/// | `stdout_bytes == 0` | upstream produced output after all, so a golden was possible |
/// | diagnostic is `JSC_*` | a crash or a harness error dressed up as a refusal |
fn check_refusal_report(
    set: &FixtureSet,
    report: &RefusalReport,
    pinned_java_version: &str,
) -> Vec<String> {
    let mut errors = Vec::new();

    if report.release != RELEASE {
        errors.push(format!(
            "refusal evidence pins release {} but the manifest pins {RELEASE}",
            report.release
        ));
    }
    if report.oracle_jar_sha256 != ARTIFACT_SHA256 {
        errors.push(
            "refusal evidence was captured against a JAR that is not the pinned artifact"
                .to_string(),
        );
    }
    if set.command.as_deref() != Some(report.command.as_str()) {
        errors.push(format!(
            "refusal evidence used command {} but fixture set {} declares {:?}",
            report.command, set.id, set.command
        ));
    }

    // A capture on an off-pin JVM is permitted, because a maintainer may not
    // have the pinned build to hand. Silence about it is not: an undeclared
    // mismatch is the difference between "we know this was captured elsewhere"
    // and an artifact that quietly is not what the manifest says it is. The
    // converse is checked too, so a stale deviation note cannot outlive the
    // deviation it describes.
    let deviation_declared = report
        .java_version_deviation
        .as_deref()
        .is_some_and(|note| !note.trim().is_empty());
    if report.java_version == pinned_java_version {
        if deviation_declared {
            errors.push(format!(
                "refusal evidence declares a JVM deviation but was captured on the pinned {pinned_java_version}"
            ));
        }
    } else if !deviation_declared {
        errors.push(format!(
            "refusal evidence was captured on Java {} rather than the pinned {pinned_java_version} without declaring the deviation",
            report.java_version
        ));
    }

    let captured: BTreeSet<&str> = report.refusals.keys().map(String::as_str).collect();
    let declared: BTreeSet<&str> = set.fixtures.iter().map(String::as_str).collect();
    for missing in declared.difference(&captured) {
        errors.push(format!(
            "fixture {missing} is in {} with no captured refusal",
            set.id
        ));
    }
    for extra in captured.difference(&declared) {
        errors.push(format!(
            "refusal evidence records {extra}, which is not in {}",
            set.id
        ));
    }

    for (fixture, entry) in &report.refusals {
        if entry.exit_status == 0 {
            errors.push(format!("captured refusal for {fixture} exited zero"));
        }
        if entry.stdout_bytes != 0 {
            errors.push(format!(
                "captured refusal for {fixture} wrote {} stdout bytes",
                entry.stdout_bytes
            ));
        }
        if !entry.diagnostic.starts_with("JSC_") {
            errors.push(format!(
                "captured refusal for {fixture} has a non-Closure diagnostic {}",
                entry.diagnostic
            ));
        }
    }

    errors
}

fn validate_provenance(set: &FixtureSet, errors: &mut Vec<String>) {
    let provenance = &set.current_provenance;
    match provenance.status {
        ProvenanceStatus::VerifiedRelease => {
            if set.id != MINIFY_VERIFIED_SET_ID
                || !set
                    .fixtures
                    .iter()
                    .all(|fixture| fixture.starts_with("minify_"))
            {
                errors.push(format!(
                    "fixture set {} makes an unsupported verified-release claim",
                    set.id
                ));
            }
            if provenance.release.as_deref() != Some(RELEASE) {
                errors.push(format!(
                    "fixture set {} does not use the pinned verified release",
                    set.id
                ));
            }
            if provenance.evidence.as_deref() != Some(MINIFY_REPORT_PATH) {
                errors.push(format!(
                    "fixture set {} does not use the strict minify refresh report",
                    set.id
                ));
            }
        }
        ProvenanceStatus::DocumentedRelease => {
            if !provenance.release.as_deref().is_some_and(is_release) {
                errors.push(format!(
                    "fixture set {} has an invalid documented release",
                    set.id
                ));
            }
            if provenance.evidence.as_deref().is_none_or(str::is_empty) {
                errors.push(format!("fixture set {} lacks provenance evidence", set.id));
            }
        }
        ProvenanceStatus::Unverified | ProvenanceStatus::Local => {
            if provenance.release.is_some() || provenance.evidence.is_some() {
                errors.push(format!(
                    "fixture set {} attaches release evidence to non-documented provenance",
                    set.id
                ));
            }
        }
    }
}

fn validate_manifest(root: &Path, manifest: &OracleManifest) -> Vec<String> {
    let mut errors = Vec::new();
    validate_pins(manifest, &mut errors);
    let commands = validate_commands(root, &manifest.commands, &mut errors);
    let discovered = discover_fixture_directories(root, &mut errors);

    let mut set_ids = BTreeSet::new();
    let mut classified: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut minify = BTreeSet::new();
    let mut used_commands = BTreeSet::new();

    for set in &manifest.fixture_sets {
        if !is_slug(&set.id) || !set_ids.insert(set.id.clone()) {
            errors.push(format!("invalid or duplicate fixture-set id: {}", set.id));
        }
        if set.fixtures.is_empty() {
            errors.push(format!("fixture set {} is empty", set.id));
        }
        if !set.fixtures.windows(2).all(|pair| pair[0] < pair[1]) {
            errors.push(format!("fixture set {} is not strictly sorted", set.id));
        }

        validate_provenance(set, &mut errors);
        match set.disposition {
            Disposition::UpstreamGolden => {
                if set.command.is_none()
                    || set.local_boundary.is_some()
                    || set.current_provenance.status == ProvenanceStatus::Local
                {
                    errors.push(format!(
                        "upstream fixture set {} has invalid boundaries",
                        set.id
                    ));
                }
            }
            Disposition::UpstreamRefuses => {
                // A real invocation exists and is what got refused, so `command`
                // is required exactly as for an upstream set. What separates the
                // two is the evidence: a captured record of the refusal, which
                // `validate_refusal_evidence` cross-checks against this set's
                // fixture list. `local_boundary` must say what is ours and why,
                // because the expected bytes are no longer upstream's.
                if set.id != REFUSALS_SET_ID {
                    errors.push(format!(
                        "fixture set {} makes an unsupported upstream-refusal claim",
                        set.id
                    ));
                }
                if set.command.is_none()
                    || set.local_boundary.as_deref().is_none_or(str::is_empty)
                    || set.current_provenance.status != ProvenanceStatus::DocumentedRelease
                {
                    errors.push(format!(
                        "upstream-refusal fixture set {} has invalid boundaries",
                        set.id
                    ));
                }
                if set.current_provenance.evidence.as_deref() != Some(REFUSALS_REPORT_PATH) {
                    errors.push(format!(
                        "fixture set {} does not use the captured refusal report",
                        set.id
                    ));
                }
                validate_refusal_evidence(
                    root,
                    set,
                    &manifest.capture_environment.java_version,
                    &mut errors,
                );
            }
            Disposition::MixedContract => {
                if set.command.is_none()
                    || set.local_boundary.as_deref().is_none_or(str::is_empty)
                    || set.current_provenance.status != ProvenanceStatus::DocumentedRelease
                {
                    errors.push(format!(
                        "mixed fixture set {} has invalid boundaries",
                        set.id
                    ));
                }
            }
            Disposition::LocalExtension | Disposition::LocalContract => {
                if set.command.is_some()
                    || set.local_boundary.as_deref().is_none_or(str::is_empty)
                    || set.current_provenance.status != ProvenanceStatus::Local
                {
                    errors.push(format!(
                        "local fixture set {} has invalid boundaries",
                        set.id
                    ));
                }
            }
        }

        let command_kind = set
            .command
            .as_ref()
            .and_then(|id| commands.get(id))
            .copied();
        if let Some(command) = &set.command {
            used_commands.insert(command.clone());
            if command_kind.is_none() {
                errors.push(format!(
                    "fixture set {} references unknown command {command}",
                    set.id
                ));
            }
        }

        if set.default_harness.is_some() == !set.harnesses.is_empty() {
            errors.push(format!(
                "fixture set {} must use exactly one harness mapping mode",
                set.id
            ));
        }
        let fixture_names: BTreeSet<_> = set.fixtures.iter().cloned().collect();
        let harness_names: BTreeSet<_> = set.harnesses.keys().cloned().collect();
        if set.default_harness.is_none() && fixture_names != harness_names {
            errors.push(format!(
                "fixture set {} has incomplete harness mappings",
                set.id
            ));
        }

        for fixture in &set.fixtures {
            if !is_slug(fixture) {
                errors.push(format!("fixture name is not a slug: {fixture}"));
            }
            classified
                .entry(fixture.clone())
                .or_default()
                .push(set.id.clone());
            if fixture.starts_with("minify_") {
                minify.insert(fixture.clone());
                if set.disposition != Disposition::UpstreamGolden
                    || set.default_harness.as_deref() != Some("tests/diff_minify.rs")
                {
                    errors.push(format!("minify fixture {fixture} has the wrong contract"));
                }
            }

            let fixture_dir = format!("tests/diff/{fixture}");
            validate_existing_path(root, &fixture_dir, true, "fixture directory", &mut errors);

            let harness = set
                .default_harness
                .as_deref()
                .or_else(|| set.harnesses.get(fixture).map(String::as_str))
                .unwrap_or("");
            validate_existing_path(root, harness, false, "fixture harness", &mut errors);

            if set.expectation == Expectation::ExpectedStdout {
                validate_existing_path(
                    root,
                    &format!("tests/diff/{fixture}/expected.stdout"),
                    false,
                    "expected stdout",
                    &mut errors,
                );
            }

            if let Some(evidence) = &set.current_provenance.evidence {
                let rendered = render_fixture_template(evidence, fixture, harness);
                if rendered.contains('{') || rendered.contains('}') {
                    errors.push(format!(
                        "fixture {fixture} has unresolved evidence placeholders"
                    ));
                } else {
                    let evidence_path = resolve_existing_path(
                        root,
                        &rendered,
                        false,
                        "provenance evidence",
                        &mut errors,
                    );
                    if let (Some(release), Some(evidence_path)) =
                        (&set.current_provenance.release, evidence_path)
                    {
                        match std::fs::read_to_string(evidence_path) {
                            Ok(contents) if !contents.contains(release) => errors.push(format!(
                                "provenance evidence for {fixture} does not contain {release}"
                            )),
                            Err(error) => errors.push(format!(
                                "cannot read provenance evidence for {fixture}: {error}"
                            )),
                            Ok(_) => {}
                        }
                    }
                }
            }

            if command_kind == Some(CommandKind::FlagsFile) {
                validate_existing_path(
                    root,
                    &format!("tests/diff/{fixture}/flags.txt"),
                    false,
                    "fixture flags",
                    &mut errors,
                );
                validate_existing_path(
                    root,
                    &format!("tests/diff/{fixture}/input"),
                    true,
                    "fixture input root",
                    &mut errors,
                );
            }
        }
    }

    for (fixture, memberships) in &classified {
        if memberships.len() != 1 {
            errors.push(format!(
                "fixture {fixture} is classified {} times: {}",
                memberships.len(),
                memberships.join(", ")
            ));
        }
        if !discovered.contains(fixture) {
            errors.push(format!("manifest references missing fixture: {fixture}"));
        }
    }
    for fixture in discovered.difference(&classified.keys().cloned().collect()) {
        errors.push(format!("fixture is missing from the manifest: {fixture}"));
    }

    let discovered_minify: BTreeSet<_> = discovered
        .iter()
        .filter(|name| name.starts_with("minify_"))
        .cloned()
        .collect();
    if minify != discovered_minify {
        errors.push("manifest minify cohort differs from runtime discovery".to_string());
    }

    for command in commands.keys() {
        if !used_commands.contains(command) {
            errors.push(format!("command is not used by any fixture set: {command}"));
        }
    }

    errors
}

fn assert_error(errors: &[String], needle: &str) {
    assert!(
        errors.iter().any(|error| error.contains(needle)),
        "expected an error containing {needle:?}, got:\n{}",
        errors.join("\n")
    );
}

#[test]
fn canonical_manifest_classifies_every_fixture_offline() {
    let manifest = read_manifest();
    let errors = validate_manifest(&package_root(), &manifest);
    assert!(
        errors.is_empty(),
        "oracle manifest errors:\n{}",
        errors.join("\n")
    );

    let fixture_count: usize = manifest
        .fixture_sets
        .iter()
        .map(|set| set.fixtures.len())
        .sum();
    let minify_count: usize = manifest
        .fixture_sets
        .iter()
        .flat_map(|set| &set.fixtures)
        .filter(|fixture| fixture.starts_with("minify_"))
        .count();
    // 626 pre-existing + 171 CCR-066 ladder rungs (57 rungs x 3 compilation
    // levels; tiers 1-7, plus the five rungs CCR-078 added — three for
    // local-scope propagation and two pinning top-level `const` in a CONDITION
    // position). This assertion is a tripwire, not a fact to be kept current
    // automatically: a fixture appearing without a reviewer noticing is the
    // thing it exists to prevent, so update it deliberately or not at all.
    assert_eq!(fixture_count, 797, "reviewed fixture inventory changed");
    assert_eq!(minify_count, 462, "reviewed minify cohort changed");
}

#[test]
fn strict_schema_rejects_unknown_top_level_and_nested_fields() {
    let text = std::fs::read_to_string(package_root().join(MANIFEST_PATH)).unwrap();
    let canonical: serde_json::Value = serde_json::from_str(&text).unwrap();

    let mut top_level = canonical.clone();
    top_level
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_string(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<OracleManifest>(top_level).is_err());

    let mut nested = canonical;
    nested["upstream"]["release"]["artifact"]
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_string(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<OracleManifest>(nested).is_err());
}

#[test]
fn validator_collects_independent_pin_and_mapping_errors() {
    let mut manifest = read_manifest();
    manifest.schema_version = 99;
    manifest.upstream.release.artifact.sha256 = "not-a-hash".to_string();
    manifest
        .upstream
        .release
        .artifact
        .embedded_license_paths
        .push("META-INF/LICENSE".to_string());
    let duplicate = manifest.fixture_sets[0].fixtures[0].clone();
    manifest.fixture_sets[1].fixtures.push(duplicate);

    let errors = validate_manifest(&package_root(), &manifest);
    assert!(
        errors.len() >= 3,
        "expected collected errors, got {errors:?}"
    );
    assert_error(&errors, "unsupported schema version");
    assert_error(&errors, "invalid artifact SHA-256");
    assert_error(
        &errors,
        "embedded license path inventory is incomplete or duplicated",
    );
    assert_error(&errors, "classified 2 times");
}

#[test]
fn validator_rejects_missing_and_stale_fixture_entries() {
    let mut manifest = read_manifest();
    let removed = manifest.fixture_sets[0].fixtures.remove(0);
    manifest.fixture_sets[1]
        .fixtures
        .push("fixture-that-does-not-exist".to_string());
    manifest.fixture_sets[1].fixtures.sort();

    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(
        &errors,
        &format!("fixture is missing from the manifest: {removed}"),
    );
    assert_error(
        &errors,
        "manifest references missing fixture: fixture-that-does-not-exist",
    );
}

#[test]
fn validator_rejects_unknown_commands_and_false_upstream_claims() {
    let mut manifest = read_manifest();
    manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == "typed-pipeline-current-v20260915")
        .unwrap()
        .command = Some("unknown-command".to_string());
    let local = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.disposition == Disposition::LocalExtension)
        .unwrap();
    local.command = Some("closure-flags-file-v1".to_string());
    local.current_provenance = CurrentProvenance {
        status: ProvenanceStatus::VerifiedRelease,
        release: Some(RELEASE.to_string()),
        evidence: Some(MINIFY_REPORT_PATH.to_string()),
    };

    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "references unknown command unknown-command");
    assert_error(
        &errors,
        "local fixture set correlation-vector-local-extensions",
    );
    assert_error(&errors, "unsupported verified-release claim");
    assert_error(
        &errors,
        "command is not used by any fixture set: typed-pipeline-failure-matrix-v1",
    );
}

#[test]
fn validator_rejects_escaping_paths_and_incomplete_matrix_bindings() {
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == "non-minify-unverified-stdout")
        .unwrap();
    let fixture = set.fixtures[0].clone();
    set.harnesses.insert(fixture, "../outside.rs".to_string());

    let flags_command = manifest
        .commands
        .iter_mut()
        .find(|command| command.kind == CommandKind::FlagsFile)
        .unwrap();
    flags_command
        .argv
        .insert(0, "-javaagent:unreviewed.jar".to_string());

    let matrix = manifest
        .commands
        .iter_mut()
        .find(|command| command.kind == CommandKind::Matrix)
        .unwrap();
    matrix.matrix[0].variables.remove("fixture_input");

    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "fixture harness is not a safe relative path");
    assert_error(&errors, "does not bind every placeholder");
    assert_error(
        &errors,
        "command closure-flags-file-v1 differs from the reviewed template",
    );
    assert_error(
        &errors,
        "command typed-pipeline-failure-matrix-v1 differs from the reviewed template",
    );
}

#[test]
fn rejected_evidence_paths_are_never_read() {
    let mut manifest = read_manifest();
    let set_index = manifest
        .fixture_sets
        .iter()
        .position(|set| set.current_provenance.status == ProvenanceStatus::DocumentedRelease)
        .unwrap();
    manifest.fixture_sets[set_index].current_provenance.evidence =
        Some("../Cargo.toml".to_string());

    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "provenance evidence is not a safe relative path");
    assert!(
        !errors
            .iter()
            .any(|error| error.contains("cannot read provenance evidence")),
        "rejected traversal path was read: {errors:?}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let unique = format!("oracle-manifest-evidence-{}", std::process::id());
        let outside = std::env::temp_dir().join(format!("{unique}.txt"));
        let link = package_root().join("target").join(&unique);
        std::fs::write(&outside, "outside package evidence must never be read").unwrap();
        symlink(&outside, &link).unwrap();

        manifest.fixture_sets[set_index].current_provenance.evidence =
            Some(format!("target/{unique}"));
        let errors = validate_manifest(&package_root(), &manifest);
        assert_error(&errors, "provenance evidence escapes the package root");
        assert!(
            !errors
                .iter()
                .any(|error| error.contains("does not contain")),
            "rejected symlink target was read: {errors:?}"
        );

        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(outside).unwrap();
    }
}

#[test]
fn refusal_evidence_must_agree_with_the_set_it_backs() {
    // The `upstream_refuses` disposition is only worth having if the manifest
    // cannot assert it unilaterally. Each mutation below is a way the manifest
    // and the captured report could drift apart; this test exists so that the
    // green run above means "the checks fired and found nothing" rather than
    // "the checks never ran".
    //
    // Mutating the manifest rather than the report keeps the on-disk evidence
    // untouched: the report is a capture, and a test that rewrites it would be
    // defeating the point of capturing it.

    // A fixture in the set with no captured refusal behind it.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == REFUSALS_SET_ID)
        .expect("the refusals set exists");
    set.fixtures.push("simple-yield".to_string());
    set.harnesses.insert(
        "simple-yield".to_string(),
        "tests/diff_simple_yield.rs".to_string(),
    );
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "simple-yield is in");
    assert_error(&errors, "with no captured refusal");

    // A fixture dropped from the set while the report still records it.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == REFUSALS_SET_ID)
        .unwrap();
    let dropped = set.fixtures.remove(0);
    set.harnesses.remove(&dropped);
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, &format!("refusal evidence records {dropped}"));

    // The set pointed at some other evidence file. The minify report is a real,
    // readable, correctly-shaped JSON artifact, so this proves the check is on
    // the *identity* of the evidence and not merely on it parsing.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == REFUSALS_SET_ID)
        .unwrap();
    set.current_provenance.evidence = Some(MINIFY_REPORT_PATH.to_string());
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "does not use the captured refusal report");

    // The disposition claimed by a set that is not the reviewed one. Without
    // this, any set could opt out of `upstream_golden` by relabelling itself.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == "non-minify-unverified-stdout")
        .unwrap();
    set.disposition = Disposition::UpstreamRefuses;
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "unsupported upstream-refusal claim");

    // Provenance downgraded to `unverified`, which is what the set used to
    // carry. An upstream-refusal claim without a documented release is exactly
    // the unevidenced assertion this disposition exists to prevent.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == REFUSALS_SET_ID)
        .unwrap();
    set.current_provenance = CurrentProvenance {
        status: ProvenanceStatus::Unverified,
        release: None,
        evidence: None,
    };
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "has invalid boundaries");

    // `local_boundary` emptied. The bytes are ours, not upstream's, and the set
    // has to say so in words a reader can act on.
    let mut manifest = read_manifest();
    let set = manifest
        .fixture_sets
        .iter_mut()
        .find(|set| set.id == REFUSALS_SET_ID)
        .unwrap();
    set.local_boundary = Some(String::new());
    let errors = validate_manifest(&package_root(), &manifest);
    assert_error(&errors, "has invalid boundaries");
}

#[test]
fn refusal_evidence_on_disk_is_a_genuine_capture() {
    // Guards the evidence file itself rather than the manifest's pointer to it.
    // Every entry must be a real refusal — nonzero exit, no stdout, a Closure
    // diagnostic — and the capture must be of the pinned JAR at the pinned
    // release. A report that satisfied the manifest cross-check while recording
    // successful runs would make the disposition meaningless.
    let text = std::fs::read_to_string(package_root().join(REFUSALS_REPORT_PATH))
        .expect("read the refusal evidence");
    let report: RefusalReport = serde_json::from_str(&text).expect("parse the refusal evidence");

    assert_eq!(report.release, RELEASE);
    assert_eq!(report.oracle_jar_sha256, ARTIFACT_SHA256);
    assert_eq!(report.command, "closure-flags-file-v1");
    assert_eq!(
        report.refusals.len(),
        12,
        "reviewed refusal cohort changed; update deliberately or not at all"
    );

    for (fixture, entry) in &report.refusals {
        assert_ne!(entry.exit_status, 0, "{fixture} exited zero");
        assert_eq!(entry.stdout_bytes, 0, "{fixture} produced stdout");
        assert!(
            entry.diagnostic.starts_with("JSC_"),
            "{fixture} has diagnostic {}",
            entry.diagnostic
        );
    }

    // Six diagnostic families appear in the report. Only JSC_PARSE_ERROR is
    // pinned by name here, because it is the one that carries an argument:
    // `simple-try-catch` and
    // `simple-newtarget` are JSC_PARSE_ERROR — upstream rejecting these as
    // *syntax*, not as policy — which is the sharpest evidence on CCR-075
    // (#15860) and the reason that issue should not be read as purely a
    // question of matching upstream's taste.
    let parse_errors: Vec<&str> = report
        .refusals
        .iter()
        .filter(|(_, entry)| entry.diagnostic == "JSC_PARSE_ERROR")
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(parse_errors, vec!["simple-newtarget", "simple-try-catch"]);
}

/// Build a report that passes every check, as a base for doctoring.
fn healthy_refusal_report() -> RefusalReport {
    let text = std::fs::read_to_string(package_root().join(REFUSALS_REPORT_PATH))
        .expect("read the refusal evidence");
    serde_json::from_str(&text).expect("parse the refusal evidence")
}

fn refusals_set() -> FixtureSet {
    read_manifest()
        .fixture_sets
        .into_iter()
        .find(|set| set.id == REFUSALS_SET_ID)
        .expect("the refusals set exists")
}

#[test]
fn refusal_report_checks_reject_a_doctored_report() {
    // These branches read the report on disk, so the manifest-mutating test
    // above cannot reach them — and a review of this change proved it: five of
    // the six could be deleted with the whole suite still green. The real
    // enforcement was coming from `refusal_evidence_on_disk_is_a_genuine_capture`
    // asserting against the committed artifact, which catches a hand-edited
    // file but leaves the validator's own branches unprotected against a
    // refactor. `check_refusal_report` exists so they can be fed synthetic
    // reports, and this test is what makes each one load-bearing.
    let set = refusals_set();
    let pinned = read_manifest().capture_environment.java_version;

    // Sanity: the committed artifact passes cleanly. Without this, every
    // assertion below could be firing on some unrelated pre-existing error.
    let clean = check_refusal_report(&set, &healthy_refusal_report(), &pinned);
    assert!(clean.is_empty(), "healthy report should pass: {clean:?}");

    // Evidence captured against a different Closure release.
    let mut report = healthy_refusal_report();
    report.release = "v20250101".to_string();
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "pins release v20250101 but the manifest pins",
    );

    // Evidence captured against a JAR that is not the pinned artifact. This is
    // the check that stops someone regenerating the report with a patched
    // compiler that refuses more than the real one does.
    let mut report = healthy_refusal_report();
    report.oracle_jar_sha256 = "0".repeat(64);
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "not the pinned artifact",
    );

    // Evidence captured with an invocation the set does not declare.
    let mut report = healthy_refusal_report();
    report.command = "typed-pipeline-failure-matrix-v1".to_string();
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "but fixture set",
    );

    // An off-pin JVM with no deviation note: the silent mismatch this field
    // exists to prevent.
    let mut report = healthy_refusal_report();
    report.java_version = "17.0.1".to_string();
    report.java_version_deviation = None;
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "without declaring the deviation",
    );

    // Whitespace is not a declaration.
    let mut report = healthy_refusal_report();
    report.java_version = "17.0.1".to_string();
    report.java_version_deviation = Some("   ".to_string());
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "without declaring the deviation",
    );

    // The converse, so a deviation note cannot outlive the deviation: once
    // someone recaptures on the pinned JVM, a leftover note is an error rather
    // than a harmless stale string.
    let mut report = healthy_refusal_report();
    report.java_version = pinned.clone();
    report.java_version_deviation = Some("stale note".to_string());
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "declares a JVM deviation but was captured on the pinned",
    );

    // A fixture that actually compiles, filed as a refusal.
    let mut report = healthy_refusal_report();
    report
        .refusals
        .get_mut("simple-with")
        .unwrap()
        .exit_status = 0;
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "captured refusal for simple-with exited zero",
    );

    // Upstream produced output after all, which would mean a golden WAS
    // possible and the disposition is wrong for that fixture.
    let mut report = healthy_refusal_report();
    report
        .refusals
        .get_mut("simple-super")
        .unwrap()
        .stdout_bytes = 17;
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "wrote 17 stdout bytes",
    );

    // A crash or harness error dressed up as a Closure refusal.
    let mut report = healthy_refusal_report();
    report
        .refusals
        .get_mut("simple-newtarget")
        .unwrap()
        .diagnostic = "OutOfMemoryError".to_string();
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "non-Closure diagnostic OutOfMemoryError",
    );

    // A capture with no corresponding fixture in the set.
    let mut report = healthy_refusal_report();
    report.refusals.insert(
        "simple-regex".to_string(),
        report.refusals["simple-with"].clone(),
    );
    assert_error(
        &check_refusal_report(&set, &report, &pinned),
        "refusal evidence records simple-regex",
    );
}
