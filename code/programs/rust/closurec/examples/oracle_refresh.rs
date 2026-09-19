//! Explicit maintainer tool for refreshing the pinned minify oracle report.
//!
//! This program never downloads an oracle. It verifies a caller-supplied JAR,
//! validates the exact 462-fixture command surface, runs the pinned compiler,
//! and writes the deterministic report consumed by the offline test suite.

use coding_adventures_sha256::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MANIFEST_PATH: &str = "tests/oracle/manifest.json";
const REPORT_PATH: &str = "tests/oracle/minify-v20260915-report.json";
const COMMAND_ID: &str = "closure-flags-file-v1";
const COHORT_ID: &str = "minify-v20260915";
const EXPECTED_FIXTURE_COUNT: usize = 462;
const MAX_WORKERS: usize = 4;
const MAX_FIXTURE_BYTES: usize = 1_048_576;
const MAX_PROCESS_OUTPUT_BYTES: u64 = 1_048_576;
const MAX_COHORT_BYTES: u64 = 67_108_864;
const MAX_CAPTURE_BYTES: u64 = 33_554_432;
const MAX_REPORT_BYTES: usize = 33_554_432;
const MAX_MANIFEST_BYTES: usize = 1_048_576;
const MAX_JAVA_LAUNCHER_BYTES: usize = 67_108_864;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60);
const EXPECTED_PROJECT: &str = "google/closure-compiler";
const EXPECTED_RELEASE: &str = "v20260915";
const EXPECTED_TAG_OBJECT: &str = "72421c28d352e5dda9a111bec39c3d41af46f3a3";
const EXPECTED_RELEASE_COMMIT: &str = "56007b2869ef6ce70b659b033459b8d8113101de";
const EXPECTED_AUDIT_COMMIT: &str = "10ca677aff381d2c2e6e1b254ba32861e503173d";
const EXPECTED_ARTIFACT_SIZE: u64 = 14_976_538;
const EXPECTED_ARTIFACT_SHA256: &str =
    "9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d";
const EXPECTED_ARTIFACT_URL: &str = "https://repo1.maven.org/maven2/com/google/javascript/closure-compiler/v20260915/closure-compiler-v20260915.jar";
const EXPECTED_JAVA_VERSION: &str = "21.0.12";
const EXPECTED_COMMAND_ARGV: &[&str] = &[
    "-Duser.language=en",
    "-Duser.country=US",
    "-Duser.timezone=UTC",
    "-Dfile.encoding=UTF-8",
    "-jar",
    "{oracle_jar}",
    "{fixture_flags}",
];

#[derive(Debug)]
struct Arguments {
    oracle_jar: PathBuf,
    java: PathBuf,
}

#[derive(Debug)]
struct JavaIdentity {
    path: PathBuf,
    size: u64,
    sha256: String,
}

impl JavaIdentity {
    fn resolve(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("--java must name an absolute executable path".to_string());
        }
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("canonicalize Java executable {}: {error}", path.display()))?;
        let metadata = fs::metadata(&canonical)
            .map_err(|error| format!("stat Java executable {}: {error}", canonical.display()))?;
        if !metadata.is_file() {
            return Err(format!(
                "Java executable is not a file: {}",
                canonical.display()
            ));
        }
        let bytes = read_file_bounded(&canonical, MAX_JAVA_LAUNCHER_BYTES, "Java executable")?;
        Ok(Self {
            path: canonical,
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        })
    }

    fn verify_unchanged(&self) -> Result<(), String> {
        let bytes = read_file_bounded(&self.path, MAX_JAVA_LAUNCHER_BYTES, "Java executable")?;
        let actual_hash = sha256_hex(&bytes);
        if bytes.len() as u64 != self.size || actual_hash != self.sha256 {
            return Err("Java executable changed during oracle capture".to_string());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct PreparedFixture {
    fixture: String,
    flags: Vec<String>,
    flags_path: String,
    flags_bytes: Vec<u8>,
    input_path: String,
    input_bytes: Vec<u8>,
    expected: Vec<u8>,
}

#[derive(Debug)]
struct PrivateSnapshots {
    root: PathBuf,
    jar: PathBuf,
    outputs: PathBuf,
}

impl Drop for PrivateSnapshots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl PrivateSnapshots {
    fn create(fixtures: &[PreparedFixture], jar_bytes: &[u8]) -> Result<Self, String> {
        let root = create_private_root()?;
        let result = (|| {
            let jar = root.join("closure-compiler.jar");
            write_new_file(&jar, jar_bytes, "oracle JAR snapshot")?;
            let outputs = root.join("process-output");
            create_private_directory(&outputs)
                .map_err(|error| format!("create {}: {error}", outputs.display()))?;
            for fixture in fixtures {
                let relative = Path::new(&fixture.input_path);
                if !is_safe_relative(relative) {
                    return Err(format!(
                        "{}: unsafe preflight input path {}",
                        fixture.fixture, fixture.input_path
                    ));
                }
                let destination = root.join(relative);
                let parent = destination
                    .parent()
                    .ok_or_else(|| format!("{}: snapshot input has no parent", fixture.fixture))?;
                fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
                write_new_file(&destination, &fixture.input_bytes, "fixture input snapshot")?;
            }
            Ok(Self {
                root: root.clone(),
                jar,
                outputs,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&root);
        }
        result
    }
}

#[derive(Debug)]
struct ProcessOutcome {
    success: bool,
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct CaptureContext<'a> {
    java: &'a Path,
    snapshot_root: &'a Path,
    jar: &'a Path,
    outputs: &'a Path,
    command: &'a OracleCommand,
    environment: &'a CaptureEnvironment,
    capture_bytes: &'a AtomicU64,
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.child.as_mut().expect("live child").try_wait()
    }

    fn mark_reaped(&mut self) {
        self.child = None;
    }

    fn terminate(&mut self) -> Result<(), String> {
        let child = self.child.as_mut().expect("live child");
        if let Err(kill_error) = child.kill() {
            return match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    Ok(())
                }
                Ok(None) => Err(format!("kill subprocess: {kill_error}")),
                Err(wait_error) => Err(format!(
                    "kill subprocess: {kill_error}; poll after kill failure: {wait_error}"
                )),
            };
        }
        child
            .wait()
            .map_err(|error| format!("reap killed subprocess: {error}"))?;
        self.child = None;
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        if child.kill().is_ok() {
            let _ = child.wait();
        }
    }
}

fn create_private_root() -> Result<PathBuf, String> {
    let parent = env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..100_u32 {
        let candidate = parent.join(format!(
            "closurec-oracle-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match create_private_directory(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("create private execution root: {error}"));
            }
        }
    }
    Err("could not create a unique private execution root".to_string())
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

fn create_new_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn write_new_file(path: &Path, bytes: &[u8], label: &str) -> Result<(), String> {
    let mut file = create_new_file(path)
        .map_err(|error| format!("create {label} {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("write {label} {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("sync {label} {}: {error}", path.display()))
}

fn java_command(java: &Path) -> Command {
    let mut command = Command::new(java);
    for variable in [
        "JAVA_TOOL_OPTIONS",
        "_JAVA_OPTIONS",
        "JDK_JAVA_OPTIONS",
        "CLASSPATH",
    ] {
        command.env_remove(variable);
    }
    command
}

fn run_bounded(
    mut command: Command,
    output_directory: &Path,
    stem: &str,
) -> Result<ProcessOutcome, String> {
    if !is_single_safe_component(stem) {
        return Err(format!("unsafe process-output stem: {stem}"));
    }
    let stdout_path = output_directory.join(format!("{stem}.stdout"));
    let stderr_path = output_directory.join(format!("{stem}.stderr"));
    let stdout_file = create_new_file(&stdout_path)
        .map_err(|error| format!("create {}: {error}", stdout_path.display()))?;
    let stderr_file = create_new_file(&stderr_path)
        .map_err(|error| format!("create {}: {error}", stderr_path.display()))?;
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let child = command
        .spawn()
        .map_err(|error| format!("spawn subprocess: {error}"))?;
    let mut child = ChildGuard::new(child);
    let started = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| format!("poll subprocess: {error}"))?
        {
            Some(status) => break status,
            None => {
                let stdout_size = fs::metadata(&stdout_path)
                    .map_err(|error| format!("stat {}: {error}", stdout_path.display()))?
                    .len();
                let stderr_size = fs::metadata(&stderr_path)
                    .map_err(|error| format!("stat {}: {error}", stderr_path.display()))?
                    .len();
                if stdout_size > MAX_PROCESS_OUTPUT_BYTES || stderr_size > MAX_PROCESS_OUTPUT_BYTES
                {
                    child.terminate()?;
                    return Err(format!(
                        "subprocess output exceeded {MAX_PROCESS_OUTPUT_BYTES} bytes"
                    ));
                }
                if started.elapsed() >= PROCESS_TIMEOUT {
                    child.terminate()?;
                    return Err(format!(
                        "subprocess exceeded {} seconds",
                        PROCESS_TIMEOUT.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
    };
    child.mark_reaped();
    let code = status
        .code()
        .ok_or_else(|| "subprocess terminated without an exit code".to_string())?;
    for path in [&stdout_path, &stderr_path] {
        let size = fs::metadata(path)
            .map_err(|error| format!("stat {}: {error}", path.display()))?
            .len();
        if size > MAX_PROCESS_OUTPUT_BYTES {
            return Err(format!(
                "subprocess output exceeded {MAX_PROCESS_OUTPUT_BYTES} bytes"
            ));
        }
    }
    let output_limit = usize::try_from(MAX_PROCESS_OUTPUT_BYTES)
        .map_err(|_| "process output limit does not fit this platform".to_string())?;
    let stdout = read_file_bounded(&stdout_path, output_limit, "subprocess stdout")?;
    let stderr = read_file_bounded(&stderr_path, output_limit, "subprocess stderr")?;
    Ok(ProcessOutcome {
        success: status.success(),
        code,
        stdout,
        stderr,
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleManifest {
    schema_version: u32,
    upstream: Upstream,
    capture_environment: CaptureEnvironment,
    commands: Vec<OracleCommand>,
    fixture_sets: Vec<FixtureSet>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
struct Upstream {
    project: String,
    repository_url: String,
    license_spdx: String,
    license_url: String,
    release: Release,
    audited_source: AuditedSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    tag: String,
    annotated_tag_object_sha: String,
    commit_sha: String,
    artifact: Artifact,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
struct AuditedSource {
    branch: String,
    commit_sha: String,
    observed_on: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
struct CaptureEnvironment {
    java_version: String,
    locale: String,
    timezone: String,
    encoding: String,
    working_directory: String,
    oracle_jar_placeholder: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
struct OracleCommand {
    id: String,
    kind: String,
    description: String,
    executable: String,
    working_directory: String,
    argv: Vec<String>,
    flags_path: Option<String>,
    input_root: Option<String>,
    matrix: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Full strict schema: capture uses only the execution-relevant fields.
struct FixtureSet {
    id: String,
    disposition: String,
    command: Option<String>,
    current_provenance: serde_json::Value,
    expectation: String,
    local_boundary: Option<String>,
    default_harness: Option<String>,
    harnesses: BTreeMap<String, String>,
    fixtures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReportEnvironment {
    java_version: String,
    locale: String,
    timezone: String,
    encoding: String,
    working_directory: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Summary {
    total: usize,
    equal: usize,
    changed: usize,
    declined: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Classification {
    Equal,
    Changed,
    Declined,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Review {
    disposition: String,
    reason_id: String,
    reason: String,
    issue_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

fn main() {
    if let Err(error) = run() {
        eprintln!("oracle refresh failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let java = JavaIdentity::resolve(&arguments.java)?;
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .map_err(|error| format!("canonicalize package root: {error}"))?;
    let manifest = read_manifest(&package_root)?;
    validate_trusted_manifest(&manifest)?;
    let command = select_command(&manifest)?;
    let fixtures = select_and_validate_cohort(&package_root, &manifest)?;
    let prepared = preflight_all(&package_root, fixtures)?;
    let jar_bytes = verify_oracle_jar(&arguments.oracle_jar, &manifest)?;
    let previous = read_previous_report(&package_root)?;
    let snapshots = PrivateSnapshots::create(&prepared, &jar_bytes)?;
    verify_java(
        &java.path,
        &manifest.capture_environment.java_version,
        &snapshots,
        "java-version-before",
    )?;

    let queue = Arc::new(Mutex::new(VecDeque::from(prepared)));
    let results = Arc::new(Mutex::new(Vec::<Result<FixtureResult, String>>::new()));
    let capture_bytes = Arc::new(AtomicU64::new(0));
    let workers = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(MAX_WORKERS);
    let java_ref = java.path.as_path();
    let snapshot_root_ref = snapshots.root.as_path();
    let jar_ref = snapshots.jar.as_path();
    let outputs_ref = snapshots.outputs.as_path();
    let environment_ref = &manifest.capture_environment;

    thread::scope(|scope| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let capture_bytes = Arc::clone(&capture_bytes);
            scope.spawn(move || {
                let context = CaptureContext {
                    java: java_ref,
                    snapshot_root: snapshot_root_ref,
                    jar: jar_ref,
                    outputs: outputs_ref,
                    command,
                    environment: environment_ref,
                    capture_bytes: &capture_bytes,
                };
                loop {
                    let fixture = queue.lock().expect("queue lock").pop_front();
                    let Some(fixture) = fixture else { break };
                    let result = capture_fixture(&context, fixture);
                    results.lock().expect("results lock").push(result);
                }
            });
        }
    });

    java.verify_unchanged()?;
    verify_java(
        &java.path,
        &manifest.capture_environment.java_version,
        &snapshots,
        "java-version-after",
    )?;

    let mut captured = Vec::new();
    let mut errors = Vec::new();
    for result in Arc::try_unwrap(results)
        .map_err(|_| "capture result handles remain live".to_string())?
        .into_inner()
        .map_err(|_| "capture result lock was poisoned".to_string())?
    {
        match result {
            Ok(item) => captured.push(item),
            Err(error) => errors.push(error),
        }
    }
    if !errors.is_empty() {
        errors.sort();
        return Err(errors.join("\n"));
    }
    captured.sort_by(|left, right| left.fixture.cmp(&right.fixture));
    preserve_reviewed_history(&mut captured, previous.as_ref());

    let report = RefreshReport {
        schema_version: 1,
        manifest_schema_version: manifest.schema_version,
        cohort_id: COHORT_ID.to_string(),
        oracle: ReportOracle {
            project: manifest.upstream.project.clone(),
            release_tag: manifest.upstream.release.tag.clone(),
            annotated_tag_object_sha: manifest.upstream.release.annotated_tag_object_sha.clone(),
            release_commit_sha: manifest.upstream.release.commit_sha.clone(),
            audited_source_commit_sha: manifest.upstream.audited_source.commit_sha.clone(),
            artifact_size_bytes: manifest.upstream.release.artifact.size_bytes,
            artifact_sha256: manifest.upstream.release.artifact.sha256.clone(),
        },
        capture_environment: ReportEnvironment {
            java_version: manifest.capture_environment.java_version.clone(),
            locale: manifest.capture_environment.locale.clone(),
            timezone: manifest.capture_environment.timezone.clone(),
            encoding: manifest.capture_environment.encoding.clone(),
            working_directory: manifest.capture_environment.working_directory.clone(),
        },
        command_id: COMMAND_ID.to_string(),
        summary: summarize(&captured),
        fixtures: captured,
    };
    let mut json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize refresh report: {error}"))?;
    json.push('\n');
    if json.len() > MAX_REPORT_BYTES {
        return Err(format!(
            "serialized refresh report exceeds {MAX_REPORT_BYTES} bytes"
        ));
    }
    let report_path = write_report_atomically(&package_root, json.as_bytes())?;
    println!(
        "wrote {}: total={} equal={} changed={} declined={}",
        report_path.display(),
        report.summary.total,
        report.summary.equal,
        report.summary.changed,
        report.summary.declined
    );
    Ok(())
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut arguments = env::args_os().skip(1);
    let mut oracle_jar = None;
    let mut java = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--oracle-jar") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--oracle-jar requires a path".to_string())?;
                if oracle_jar.replace(PathBuf::from(value)).is_some() {
                    return Err("--oracle-jar may appear only once".to_string());
                }
            }
            Some("--java") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--java requires a path".to_string())?;
                if java.replace(PathBuf::from(value)).is_some() {
                    return Err("--java may appear only once".to_string());
                }
            }
            Some(other) => return Err(format!("unknown argument: {other}")),
            None => return Err("arguments must be valid Unicode".to_string()),
        }
    }
    Ok(Arguments {
        oracle_jar: oracle_jar.ok_or_else(|| "missing required --oracle-jar PATH".to_string())?,
        java: java.ok_or_else(|| "missing required --java ABSOLUTE_PATH".to_string())?,
    })
}

fn read_manifest(package_root: &Path) -> Result<OracleManifest, String> {
    let path = package_root.join(MANIFEST_PATH);
    let bytes = read_file_bounded(&path, MAX_MANIFEST_BYTES, "oracle manifest")?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn validate_trusted_manifest(manifest: &OracleManifest) -> Result<(), String> {
    let release = &manifest.upstream.release;
    let artifact = &release.artifact;
    let environment = &manifest.capture_environment;
    if manifest.schema_version != 1
        || manifest.upstream.project != EXPECTED_PROJECT
        || release.tag != EXPECTED_RELEASE
        || release.annotated_tag_object_sha != EXPECTED_TAG_OBJECT
        || release.commit_sha != EXPECTED_RELEASE_COMMIT
        || manifest.upstream.audited_source.commit_sha != EXPECTED_AUDIT_COMMIT
        || artifact.version != EXPECTED_RELEASE
        || artifact.url != EXPECTED_ARTIFACT_URL
        || artifact.size_bytes != EXPECTED_ARTIFACT_SIZE
        || artifact.sha256 != EXPECTED_ARTIFACT_SHA256
        || environment.java_version != EXPECTED_JAVA_VERSION
        || environment.locale != "C.UTF-8"
        || environment.timezone != "UTC"
        || environment.encoding != "UTF-8"
        || environment.working_directory != "code/programs/rust/closurec"
        || environment.oracle_jar_placeholder != "{oracle_jar}"
    {
        return Err("oracle manifest differs from the executable trust pins".to_string());
    }
    Ok(())
}

fn verify_oracle_jar(path: &Path, manifest: &OracleManifest) -> Result<Vec<u8>, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("canonicalize oracle JAR {}: {error}", path.display()))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("stat oracle JAR {}: {error}", canonical.display()))?;
    if !metadata.is_file() {
        return Err(format!("oracle JAR is not a file: {}", canonical.display()));
    }
    let artifact = &manifest.upstream.release.artifact;
    if metadata.len() != artifact.size_bytes {
        return Err(format!(
            "oracle JAR size mismatch: expected {}, got {}",
            artifact.size_bytes,
            metadata.len()
        ));
    }
    let artifact_limit = usize::try_from(artifact.size_bytes)
        .map_err(|_| "oracle JAR size does not fit this platform".to_string())?;
    let bytes = read_file_bounded(&canonical, artifact_limit, "oracle JAR")?;
    if bytes.len() as u64 != artifact.size_bytes {
        return Err(format!(
            "oracle JAR changed while being read: expected {} bytes, got {}",
            artifact.size_bytes,
            bytes.len()
        ));
    }
    let actual = sha256_hex(&bytes);
    if actual != artifact.sha256 {
        return Err(format!(
            "oracle JAR SHA-256 mismatch: expected {}, got {actual}",
            artifact.sha256
        ));
    }
    Ok(bytes)
}

fn verify_java(
    java: &Path,
    expected: &str,
    snapshots: &PrivateSnapshots,
    output_stem: &str,
) -> Result<(), String> {
    let mut command = java_command(java);
    command
        .arg("-version")
        .current_dir(&snapshots.root)
        .stdin(Stdio::null());
    let output = run_bounded(command, &snapshots.outputs, output_stem)
        .map_err(|error| format!("run {} -version: {error}", java.display()))?;
    if !output.success {
        return Err(format!("{} -version failed", java.display()));
    }
    let combined = [output.stdout, output.stderr].concat();
    let text = String::from_utf8_lossy(&combined);
    let version = text
        .split('"')
        .nth(1)
        .ok_or_else(|| format!("could not parse Java version from {text:?}"))?;
    if version != expected {
        return Err(format!(
            "Java version mismatch: expected {expected}, got {version}"
        ));
    }
    Ok(())
}

fn select_command(manifest: &OracleManifest) -> Result<&OracleCommand, String> {
    let matches: Vec<_> = manifest
        .commands
        .iter()
        .filter(|command| command.id == COMMAND_ID)
        .collect();
    if matches.len() != 1 {
        return Err(format!("expected exactly one {COMMAND_ID} command"));
    }
    let command = matches[0];
    let expected_argv: Vec<String> = EXPECTED_COMMAND_ARGV
        .iter()
        .map(|argument| (*argument).to_string())
        .collect();
    if command.kind != "flags_file"
        || command.executable != "java"
        || command.working_directory != "{package_root}"
        || command.argv != expected_argv
        || command.flags_path.as_deref() != Some("tests/diff/{fixture}/flags.txt")
        || command.input_root.as_deref() != Some("tests/diff/{fixture}/input")
        || !command.matrix.is_empty()
    {
        return Err(format!("{COMMAND_ID} no longer has the reviewed shape"));
    }
    Ok(command)
}

fn select_and_validate_cohort(
    package_root: &Path,
    manifest: &OracleManifest,
) -> Result<Vec<String>, String> {
    let mut fixtures = BTreeSet::new();
    for set in &manifest.fixture_sets {
        for fixture in &set.fixtures {
            if fixture.starts_with("minify_") {
                if !is_single_safe_component(fixture) {
                    return Err(format!("unsafe minify fixture name: {fixture}"));
                }
                if set.disposition != "upstream_golden"
                    || set.command.as_deref() != Some(COMMAND_ID)
                    || set.expectation != "expected_stdout"
                {
                    return Err(format!(
                        "fixture {fixture} is outside the reviewed minify cohort"
                    ));
                }
                if !fixtures.insert(fixture.clone()) {
                    return Err(format!("duplicate minify fixture: {fixture}"));
                }
            }
        }
    }
    if fixtures.len() != EXPECTED_FIXTURE_COUNT {
        return Err(format!(
            "expected {EXPECTED_FIXTURE_COUNT} minify fixtures, found {}",
            fixtures.len()
        ));
    }
    let mut discovered = BTreeSet::new();
    for entry in fs::read_dir(package_root.join("tests/diff"))
        .map_err(|error| format!("read tests/diff: {error}"))?
    {
        let entry = entry.map_err(|error| format!("read tests/diff entry: {error}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "tests/diff contains a non-Unicode entry name".to_string())?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("inspect tests/diff/{name}: {error}"))?;
        if name.starts_with("minify_") {
            if !kind.is_dir() {
                return Err(format!("minify entry is not a directory: {name}"));
            }
            discovered.insert(name);
        }
    }
    if fixtures != discovered {
        return Err("manifest minify cohort differs from directory discovery".to_string());
    }
    Ok(fixtures.into_iter().collect())
}

fn preflight_all(
    package_root: &Path,
    fixtures: Vec<String>,
) -> Result<Vec<PreparedFixture>, String> {
    let mut prepared = Vec::with_capacity(fixtures.len());
    let mut errors = Vec::new();
    let mut cohort_bytes = 0_u64;
    for fixture in fixtures {
        match preflight_fixture(package_root, fixture) {
            Ok(item) => {
                let item_bytes = item
                    .flags_bytes
                    .len()
                    .checked_add(item.input_bytes.len())
                    .and_then(|total| total.checked_add(item.expected.len()))
                    .map(|total| total as u64);
                match item_bytes.and_then(|size| cohort_bytes.checked_add(size)) {
                    Some(total) if total <= MAX_COHORT_BYTES => {
                        cohort_bytes = total;
                        prepared.push(item);
                    }
                    _ => errors.push(format!("preflight cohort exceeds {MAX_COHORT_BYTES} bytes")),
                }
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(prepared)
    } else {
        errors.sort();
        Err(errors.join("\n"))
    }
}

fn preflight_fixture(package_root: &Path, fixture: String) -> Result<PreparedFixture, String> {
    let fixture_root_path = package_root.join("tests/diff").join(&fixture);
    let fixture_root = fixture_root_path
        .canonicalize()
        .map_err(|error| format!("{fixture}: canonicalize fixture root: {error}"))?;
    if !fixture_root.starts_with(package_root) || !fixture_root.is_dir() {
        return Err(format!("{fixture}: fixture root escapes the package"));
    }

    let flags_path = fixture_root_path.join("flags.txt");
    let flags_bytes =
        read_bounded_contained_file(&flags_path, &fixture_root, "flags", MAX_FIXTURE_BYTES)?;
    let flags_text = std::str::from_utf8(&flags_bytes)
        .map_err(|error| format!("{fixture}: flags.txt is not UTF-8: {error}"))?;
    let flags: Vec<String> = flags_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    let (input_path, input_relative) =
        validate_flags(package_root, &fixture_root, &fixture, &flags)?;
    let input_root = fixture_root_path.join("input");
    let input_bytes =
        read_bounded_contained_file(&input_path, &input_root, "input", MAX_FIXTURE_BYTES)?;
    let expected_path = fixture_root_path.join("expected.stdout");
    let expected = read_bounded_contained_file(
        &expected_path,
        &fixture_root,
        "expected stdout",
        MAX_FIXTURE_BYTES,
    )?;

    Ok(PreparedFixture {
        flags_path: format!("tests/diff/{fixture}/flags.txt"),
        fixture,
        flags,
        flags_bytes,
        input_path: input_relative,
        input_bytes,
        expected,
    })
}

fn capture_fixture(
    context: &CaptureContext<'_>,
    prepared: PreparedFixture,
) -> Result<FixtureResult, String> {
    let mut arguments = Vec::new();
    let process_jar = process_compatible_path(context.jar);
    for argument in &context.command.argv {
        match argument.as_str() {
            "{oracle_jar}" => arguments.push(process_jar.as_os_str().to_owned()),
            "{fixture_flags}" => arguments.extend(prepared.flags.iter().map(Into::into)),
            value if value.contains('{') || value.contains('}') => {
                return Err(format!(
                    "{}: unsupported command placeholder {value}",
                    prepared.fixture
                ));
            }
            value => arguments.push(value.into()),
        }
    }
    let mut process = java_command(context.java);
    process
        .args(arguments)
        .current_dir(context.snapshot_root)
        .env("LC_ALL", &context.environment.locale)
        .env("LANG", &context.environment.locale)
        .env("TZ", &context.environment.timezone)
        .stdin(Stdio::null());
    let output = run_bounded(process, context.outputs, &prepared.fixture)
        .map_err(|error| format!("{}: execute oracle: {error}", prepared.fixture))?;
    let evidence_bytes = (output.stdout.len() as u64)
        .checked_add(output.stderr.len() as u64)
        .ok_or_else(|| format!("{}: capture size overflow", prepared.fixture))?;
    context
        .capture_bytes
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            current
                .checked_add(evidence_bytes)
                .filter(|total| *total <= MAX_CAPTURE_BYTES)
        })
        .map_err(|_| {
            format!(
                "{}: aggregate oracle output exceeds {MAX_CAPTURE_BYTES} bytes",
                prepared.fixture
            )
        })?;
    let classification = if output.success {
        if output.stdout == prepared.expected {
            Classification::Equal
        } else {
            Classification::Changed
        }
    } else {
        Classification::Declined
    };
    let expected_hash = sha256_hex(&prepared.expected);
    Ok(FixtureResult {
        fixture: prepared.fixture,
        classification,
        flags_path: prepared.flags_path,
        flags_sha256: sha256_hex(&prepared.flags_bytes),
        input_path: prepared.input_path,
        input_sha256: sha256_hex(&prepared.input_bytes),
        baseline_expected_sha256: expected_hash.clone(),
        current_expected_sha256: expected_hash,
        oracle_stdout_sha256: sha256_hex(&output.stdout),
        oracle_stdout_bytes: output.stdout.len(),
        oracle_stdout_hex: hex(&output.stdout),
        oracle_stderr_sha256: sha256_hex(&output.stderr),
        oracle_stderr_bytes: output.stderr.len(),
        oracle_stderr_hex: hex(&output.stderr),
        exit_code: Some(output.code),
        review: None,
    })
}

fn validate_flags(
    package_root: &Path,
    fixture_root: &Path,
    fixture: &str,
    flags: &[String],
) -> Result<(PathBuf, String), String> {
    if flags.len() != 4 {
        return Err(format!("{fixture}: expected exactly two flag/value pairs"));
    }
    let mut pairs = BTreeMap::new();
    for pair in flags.as_chunks::<2>().0 {
        if !matches!(pair[0].as_str(), "--compilation_level" | "--js") {
            return Err(format!("{fixture}: forbidden oracle flag {}", pair[0]));
        }
        if pairs.insert(pair[0].as_str(), pair[1].as_str()).is_some() {
            return Err(format!("{fixture}: duplicate oracle flag {}", pair[0]));
        }
    }
    if pairs.get("--compilation_level") != Some(&"WHITESPACE_ONLY") {
        return Err(format!(
            "{fixture}: compilation level is not WHITESPACE_ONLY"
        ));
    }
    let relative = pairs
        .get("--js")
        .ok_or_else(|| format!("{fixture}: missing --js"))?;
    if relative.contains('\\') || !is_safe_relative(Path::new(relative)) {
        return Err(format!("{fixture}: unsafe --js path {relative}"));
    }
    let input_root = fixture_root
        .join("input")
        .canonicalize()
        .map_err(|error| format!("{fixture}: canonicalize input root: {error}"))?;
    let input = package_root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("{fixture}: canonicalize --js path {relative}: {error}"))?;
    if !fixture_root.starts_with(package_root)
        || !input_root.starts_with(fixture_root)
        || !input_root.starts_with(package_root)
        || !input.starts_with(&input_root)
        || !input.starts_with(fixture_root)
        || !input.starts_with(package_root)
        || !input.is_file()
    {
        return Err(format!("{fixture}: --js path escapes its input directory"));
    }
    Ok((input, relative.to_string()))
}

fn is_single_safe_component(value: &str) -> bool {
    let path = Path::new(value);
    is_safe_relative(path) && path.components().count() == 1
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(windows)]
fn process_compatible_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(not(windows))]
fn process_compatible_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

fn read_contained_file(path: &Path, boundary: &Path, label: &str) -> Result<Vec<u8>, String> {
    let boundary = boundary.canonicalize().map_err(|error| {
        format!(
            "canonicalize {label} boundary {}: {error}",
            boundary.display()
        )
    })?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("canonicalize {label} {}: {error}", path.display()))?;
    if !canonical.starts_with(&boundary) || !canonical.is_file() {
        return Err(format!("{label} path escapes boundary: {}", path.display()));
    }
    read_file_bounded(&canonical, MAX_FIXTURE_BYTES, label)
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
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| format!("{label} byte limit overflow"))?;
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(read_limit as u64)
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

fn read_bounded_contained_file(
    path: &Path,
    boundary: &Path,
    label: &str,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let bytes = read_contained_file(path, boundary, label)?;
    if bytes.len() > limit {
        return Err(format!(
            "{label} {} exceeds the {limit}-byte limit",
            path.display()
        ));
    }
    Ok(bytes)
}

fn read_previous_report(package_root: &Path) -> Result<Option<RefreshReport>, String> {
    let path = validated_report_path(package_root)?;
    let Some(_) = existing_regular_report(&path, package_root)? else {
        return Ok(None);
    };
    let bytes = read_file_bounded(&path, MAX_REPORT_BYTES, "existing refresh report")?;
    let report = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse existing {}: {error}", path.display()))?;
    Ok(Some(report))
}

fn validated_report_path(package_root: &Path) -> Result<PathBuf, String> {
    let path = package_root.join(REPORT_PATH);
    let parent = path
        .parent()
        .ok_or_else(|| "report path has no parent".to_string())?
        .canonicalize()
        .map_err(|error| format!("canonicalize report directory: {error}"))?;
    if !parent.starts_with(package_root) || !parent.is_dir() {
        return Err("report directory escapes the package root".to_string());
    }
    Ok(parent.join(
        path.file_name()
            .ok_or_else(|| "report path has no file name".to_string())?,
    ))
}

fn existing_regular_report(path: &Path, package_root: &Path) -> Result<Option<()>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("stat report {}: {error}", path.display())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "report destination is not a regular non-symlink file: {}",
            path.display()
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("canonicalize report {}: {error}", path.display()))?;
    let expected_parent = path
        .parent()
        .ok_or_else(|| "report path has no parent".to_string())?;
    if !canonical.starts_with(package_root) || canonical.parent() != Some(expected_parent) {
        return Err(format!(
            "report destination escapes the package: {}",
            path.display()
        ));
    }
    Ok(Some(()))
}

fn write_report_atomically(package_root: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = validated_report_path(package_root)?;
    let _ = existing_regular_report(&path, package_root)?;
    let parent = path.parent().expect("validated report parent");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temporary = None;
    for attempt in 0..100_u32 {
        let candidate = parent.join(format!(
            ".minify-v20260915-report-{}-{nonce}-{attempt}.tmp",
            std::process::id()
        ));
        match create_new_file(&candidate) {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("create report temporary file: {error}"));
            }
        }
    }
    let (temporary_path, mut file) =
        temporary.ok_or_else(|| "could not create report temporary file".to_string())?;
    let write_result = (|| {
        file.write_all(bytes)
            .map_err(|error| format!("write report temporary file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("sync report temporary file: {error}"))?;
        drop(file);
        let _ = existing_regular_report(&path, package_root)?;
        replace_report_file(&temporary_path, &path)
            .map_err(|error| format!("replace report {}: {error}", path.display()))?;
        sync_report_directory(parent)?;
        Ok::<(), String>(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result?;
    Ok(path)
}

#[cfg(unix)]
fn replace_report_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_report_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    if fs::symlink_metadata(destination)
        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return fs::rename(temporary, destination);
    }
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let temporary_wide: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    #[link(name = "Kernel32")]
    extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut c_void,
            reserved: *mut c_void,
        ) -> i32;
    }
    let replaced = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            temporary_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_report_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync report directory {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn sync_report_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn preserve_reviewed_history(results: &mut [FixtureResult], previous: Option<&RefreshReport>) {
    let Some(previous) = previous else { return };
    let by_fixture: BTreeMap<_, _> = previous
        .fixtures
        .iter()
        .map(|result| (result.fixture.as_str(), result))
        .collect();
    for result in results {
        let Some(old) = by_fixture.get(result.fixture.as_str()) else {
            continue;
        };
        let same_capture = old.flags_sha256 == result.flags_sha256
            && old.input_sha256 == result.input_sha256
            && old.oracle_stdout_sha256 == result.oracle_stdout_sha256
            && old.oracle_stderr_sha256 == result.oracle_stderr_sha256
            && old.exit_code == result.exit_code;
        if same_capture && old.classification != Classification::Equal {
            result.classification = old.classification;
            result.baseline_expected_sha256 = old.baseline_expected_sha256.clone();
            result.review = old.review.clone();
        }
    }
}

fn summarize(fixtures: &[FixtureResult]) -> Summary {
    Summary {
        total: fixtures.len(),
        equal: fixtures
            .iter()
            .filter(|item| item.classification == Classification::Equal)
            .count(),
        changed: fixtures
            .iter()
            .filter(|item| item.classification == Classification::Changed)
            .count(),
        declined: fixtures
            .iter()
            .filter(|item| item.classification == Classification::Declined)
            .count(),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> OracleManifest {
        read_manifest(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("read canonical manifest")
    }

    #[test]
    fn executable_rejects_manifest_artifact_pin_drift() {
        let mut manifest = manifest();
        manifest.upstream.release.artifact.sha256 = "0".repeat(64);
        let error = validate_trusted_manifest(&manifest).expect_err("reject changed artifact pin");
        assert!(error.contains("trust pins"));
    }

    #[test]
    fn executable_rejects_unreviewed_jvm_arguments() {
        let mut manifest = manifest();
        let command = manifest
            .commands
            .iter_mut()
            .find(|command| command.id == COMMAND_ID)
            .expect("flags-file command");
        command
            .argv
            .insert(0, "-javaagent:unreviewed.jar".to_string());
        let error = select_command(&manifest).expect_err("reject unreviewed JVM argument");
        assert!(error.contains("reviewed shape"));
    }

    #[test]
    fn java_subprocess_removes_injection_environment() {
        let command = java_command(Path::new("java"));
        let removed: BTreeSet<_> = command
            .get_envs()
            .filter_map(|(name, value)| {
                value.is_none().then(|| name.to_string_lossy().into_owned())
            })
            .collect();
        for variable in [
            "JAVA_TOOL_OPTIONS",
            "_JAVA_OPTIONS",
            "JDK_JAVA_OPTIONS",
            "CLASSPATH",
        ] {
            assert!(removed.contains(variable), "{variable} was not removed");
        }
    }

    #[test]
    fn report_reader_rejects_a_symlink_destination() {
        let root = create_private_root().expect("create test root");
        let oracle_directory = root.join("tests/oracle");
        fs::create_dir_all(&oracle_directory).expect("create oracle directory");
        let target = root.join("outside-report.json");
        fs::write(&target, b"{}\n").expect("write symlink target");
        let report = root.join(REPORT_PATH);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &report).expect("create report symlink");
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_file(&target, &report) {
            let _ = fs::remove_dir_all(&root);
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314)
            {
                return;
            }
            panic!("create report symlink: {error}");
        }
        let canonical_root = root.canonicalize().expect("canonicalize test root");
        let error = read_previous_report(&canonical_root).expect_err("reject report symlink");
        let _ = fs::remove_dir_all(&root);
        assert!(error.contains("non-symlink"));
    }

    #[cfg(windows)]
    #[test]
    fn java_receives_process_compatible_windows_paths() {
        assert_eq!(
            process_compatible_path(Path::new(r"\\?\C:\oracle\compiler.jar")),
            PathBuf::from(r"C:\oracle\compiler.jar")
        );
        assert_eq!(
            process_compatible_path(Path::new(r"\\?\UNC\server\share\compiler.jar")),
            PathBuf::from(r"\\server\share\compiler.jar")
        );
    }
}
