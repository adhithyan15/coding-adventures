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

/// Find the newest jar under `root` whose file name satisfies `matches`.
///
/// Versions are not pinned here on purpose: this locates whatever the machine
/// already has rather than asserting a particular release, and skips the test
/// when it has none. Sources and javadoc jars are not the artifact.
///
/// When several versions match, the NEWEST wins, never the first one found.
/// `read_dir` order is arbitrary, and a Gradle cache routinely holds more than
/// one version of the same library. JNA is the case that bit: Compose brings
/// 5.19.1 and `kotlin-compiler-embeddable` (1.8.0 and older) brings 5.6.0, which predates
/// Apple-silicon support (5.7). Picking whichever the directory walk met first
/// made the macOS arm64 run fail with "did not load the conformance runtime"
/// on some runs and pass on others.
fn find_jar(root: &Path, matches: &dyn Fn(&str) -> bool, depth: usize) -> Option<PathBuf> {
    let mut found = Vec::new();
    collect_jars(root, matches, depth, &mut found);
    found
        .into_iter()
        .max_by(|a, b| jar_version(a).cmp(&jar_version(b)).then_with(|| a.cmp(b)))
}

fn collect_jars(
    root: &Path,
    matches: &dyn Fn(&str) -> bool,
    depth: usize,
    found: &mut Vec<PathBuf>,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jars(&path, matches, depth - 1, found);
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
            found.push(path);
        }
    }
}

/// The numeric version in a jar's file name, for ordering: `jna-5.19.1.jar`
/// reads as `[5, 19, 1]`, compared number by number, so 5.19 sorts above 5.6
/// (as a string it would not). The first run of dot-separated digits is the
/// version; a name with none sorts lowest.
fn jar_version(path: &Path) -> Vec<u64> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let Some(start) = name.find(|c: char| c.is_ascii_digit()) else {
        return Vec::new();
    };
    name[start..]
        .split('.')
        .map_while(|part| part.parse::<u64>().ok())
        .collect()
}

fn jar(prefix: &str, environment_override: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(environment_override) {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    // The version must follow the prefix directly. A bare `starts_with` on
    // `jna-` also matches `jna-platform-*.jar`, which is a different artifact
    // and does not carry `Native` -- the same over-broad-prefix trap that
    // `kotlin-stdlib-jdk8.jar` sets for `kotlin_stdlib` below.
    let owned = prefix.to_string();
    find_jar(
        &home.join(".gradle"),
        &move |name: &str| {
            name.strip_prefix(&owned)
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        },
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
        "runaway",
        "deferred",
    ] {
        let stdout = run(
            Command::new("java")
                .current_dir(&project)
                // On a load failure the host prints JNA's own reason only in
                // debug mode; without it the test can say only that loading
                // failed, never why.
                .args(["-Dmosaic.app.debug=1", "-cp", &run_classpath, "DriverKt"])
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
        "a runaway chain returns instead of spinning forever",
        "a runaway chain is reported rather than abandoned quietly",
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

/// The ordering `find_jar` relies on: numeric, not lexical, and independent of
/// directory order.
#[test]
fn the_newest_jar_version_wins() {
    let dir = std::env::temp_dir().join(format!("mosaic-jar-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["a/jna/5.19.1", "b/jna/5.6.0", "c/jna-platform/5.20.0"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    for file in [
        "a/jna/5.19.1/jna-5.19.1.jar",
        "b/jna/5.6.0/jna-5.6.0.jar",
        "c/jna-platform/5.20.0/jna-platform-5.20.0.jar",
    ] {
        std::fs::write(dir.join(file), b"").unwrap();
    }
    let jna = |name: &str| {
        name.strip_prefix("jna-")
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    let found = find_jar(&dir, &jna, 10).expect("a JNA jar");
    assert_eq!(found.file_name().unwrap(), "jna-5.19.1.jar");
    assert!(jar_version(Path::new("jna-5.19.1.jar")) > jar_version(Path::new("jna-5.6.0.jar")));
    std::fs::remove_dir_all(&dir).unwrap();
}
