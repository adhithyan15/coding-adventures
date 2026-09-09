//! The emitted Compose host answers effects, proved by running it.
//!
//! Same argument as the Qt and SwiftUI acceptances beside it: this crate's other
//! tests assert on the *text* of the emitted host, which cannot establish that
//! it compiles, let alone that it behaves. Three missing `kotlinx.serialization`
//! imports passed every one of those text assertions and failed the first
//! `kotlinc` invocation. `Effect` rode the wire and was read by no native host
//! for as long as it existed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mosaic-app-bindings sits inside the rust workspace")
        .to_path_buf()
}

fn tool_available(command: &str) -> bool {
    Command::new(command)
        .arg("-version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

/// The conformance runtime, as a shared library the host loads through JNA.
fn conformance_runtime() -> PathBuf {
    let root = workspace_root();
    let status = Command::new(cargo())
        .current_dir(&root)
        .args(["build", "-p", "mosaic-app-conformance"])
        .status()
        .expect("build the conformance runtime");
    assert!(status.success(), "conformance runtime failed to build");
    let stem = root.join("target").join("debug");
    for name in [
        "libmosaic_app_conformance.dylib",
        "libmosaic_app_conformance.so",
        "mosaic_app_conformance.dll",
    ] {
        let candidate = stem.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("missing conformance runtime under {stem:?}");
}

/// Find a jar under `root` whose file name satisfies `matches`.
///
/// Versions are not pinned here on purpose: this locates whatever the machine
/// already has rather than asserting a particular release, and skips the test
/// when it has none. Sources and javadoc jars are not the artifact.
fn find_jar(root: &Path, matches: &dyn Fn(&str) -> bool, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    let mut directories = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with(".jar")
            && !name.contains("-sources")
            && !name.contains("-javadoc")
            && matches(name)
        {
            return Some(path);
        }
    }
    for directory in directories {
        if let Some(found) = find_jar(&directory, matches, depth - 1) {
            return Some(found);
        }
    }
    None
}

fn jar(prefix: &str, environment_override: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(environment_override) {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let owned = prefix.to_string();
    find_jar(
        &home.join(".gradle"),
        &move |name: &str| name.starts_with(&owned),
        10,
    )
}

/// The Kotlin standard library, which `java` needs and `kotlinc` does not.
///
/// kotlinc puts the stdlib on its own compile classpath, so a host that
/// compiles cleanly still dies at startup with `NoClassDefFoundError:
/// kotlin/Result` unless the jar is passed explicitly here. It ships beside the
/// compiler rather than in the Gradle cache, so the compiler's own installation
/// is searched first.
fn kotlin_stdlib() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MOSAIC_KOTLIN_STDLIB_JAR") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    // `kotlin-stdlib-jdk8.jar` sits beside it and is not a substitute, so the
    // match is the bare name or a version-suffixed one, never another artifact
    // that merely starts with the same words.
    let matches = |name: &str| {
        name == "kotlin-stdlib.jar"
            || name
                .strip_prefix("kotlin-stdlib-")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    if let Some(home) = std::env::var_os("KOTLIN_HOME") {
        let candidate = PathBuf::from(home).join("lib").join("kotlin-stdlib.jar");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    for root in [
        home.join(".local")
            .join("share")
            .join("mise")
            .join("installs")
            .join("kotlin"),
        home.join(".sdkman").join("candidates").join("kotlin"),
        PathBuf::from("/opt/homebrew/opt/kotlin/libexec"),
        PathBuf::from("/usr/local/opt/kotlin/libexec"),
        home.join(".gradle"),
    ] {
        if let Some(found) = find_jar(&root, &matches, 10) {
            return Some(found);
        }
    }
    None
}

fn run(command: &mut Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|err| panic!("{what}: {err}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "{what} failed ({});\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status
    );
    stdout
}

#[test]
fn the_emitted_compose_host_answers_effects() {
    if !tool_available("kotlinc") || !tool_available("java") {
        eprintln!("skipping Compose effect acceptance: kotlinc or java unavailable");
        return;
    }
    let (Some(jna), Some(json), Some(core), Some(stdlib)) = (
        jar("jna-", "MOSAIC_JNA_JAR"),
        jar("kotlinx-serialization-json-jvm-", "MOSAIC_KOTLINX_JSON_JAR"),
        jar("kotlinx-serialization-core-jvm-", "MOSAIC_KOTLINX_CORE_JAR"),
        kotlin_stdlib(),
    ) else {
        eprintln!(
            "skipping Compose effect acceptance: JNA, kotlinx-serialization or kotlin-stdlib \
             jars not found; set MOSAIC_JNA_JAR, MOSAIC_KOTLINX_JSON_JAR, \
             MOSAIC_KOTLINX_CORE_JAR and MOSAIC_KOTLIN_STDLIB_JAR to run it"
        );
        return;
    };

    let project = std::env::temp_dir().join(format!(
        "mosaic-compose-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&project).expect("create the driver project");

    // The host under test, exactly as a real package would receive it.
    let host =
        mosaic_app_bindings::compose_jna_binding_for_application("dev.mosaic.compose-effects");
    std::fs::write(project.join("MosaicRuntimeHost.kt"), &host).unwrap();
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("compose_effect_driver")
            .join("Driver.kt"),
        project.join("Driver.kt"),
    )
    .expect("copy the driver");

    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = format!(
        "{}{separator}{}{separator}{}",
        jna.display(),
        json.display(),
        core.display()
    );

    run(
        Command::new("kotlinc").current_dir(&project).args([
            "-cp",
            &classpath,
            "MosaicRuntimeHost.kt",
            "Driver.kt",
            "-d",
            "out",
        ]),
        "kotlinc",
    );

    let runtime = conformance_runtime();
    let run_classpath = format!("out{separator}{classpath}{separator}{}", stdlib.display());

    // Each case gets its own process AND its own state file. The host reads
    // MOSAIC_APP_STATE_PATH once at load and the JVM cannot change its own
    // environment, so cases sharing one process would share one state file --
    // and one case would restore another's count, making an assertion read a
    // number its own host never produced.
    let mut transcript = String::new();
    for case in [
        "unhandled",
        "answered",
        "batch-both",
        "batch-mixed",
        "throwing",
        "unconvertible",
        "deferred",
    ] {
        let stdout = run(
            Command::new("java")
                .current_dir(&project)
                .args(["-cp", &run_classpath, "DriverKt"])
                .env("MOSAIC_APP_LIBRARY", &runtime)
                .env("MOSAIC_PROBE_CASE", case)
                .env(
                    "MOSAIC_APP_STATE_PATH",
                    project.join(format!("{case}.json")),
                ),
            &format!("compose effect driver ({case})"),
        );
        assert!(
            stdout.contains("case passed"),
            "case `{case}` did not report success:\n{stdout}"
        );
        transcript.push_str(&stdout);
    }

    // A driver that printed nothing would satisfy the line above by accident.
    for expected in [
        "a fresh app awaits nothing",
        "an unanswered await is failed, not dropped",
        "the app is told why, rather than just waiting",
        "a failed completion does not advance the app",
        "an answered await is settled",
        "the handler's value reached the app",
        "the handler answered both effects of the batch",
        "a fully-answered chaining batch leaves nothing outstanding",
        "snapshot still works after a fully-answered chaining batch",
        "a partly-answered batch leaves nothing outstanding",
        "snapshot still works after a partly-answered batch",
        "a handler that throws does not leave the effect pending",
        "the app is told the handler failed, and why",
        "snapshot survives a handler that threw",
        "an unconvertible effect result does not wedge persistence",
        "the app is told the result value could not be converted",
        "snapshot survives an unconvertible effect result",
        "deferring an effect nothing awaits is refused",
        "the handler was offered the effect",
        "a deferred effect stays outstanding rather than being failed",
        "snapshot is refused while a deferred effect is outstanding",
        "answering from another thread does not deadlock",
        "the late answer reached the UI as a props change",
        "answering a deferred effect settles it",
        "the deferred answer's value reached the app",
        "snapshot works again once the deferred effect is answered",
    ] {
        assert!(
            transcript.contains(expected),
            "missing check `{expected}`:\n{transcript}"
        );
    }
    assert!(
        !transcript.contains("VACUOUS"),
        "a check ran vacuously:\n{transcript}"
    );

    let _ = std::fs::remove_dir_all(&project);
}
