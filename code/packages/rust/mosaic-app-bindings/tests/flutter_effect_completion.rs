//! The emitted Flutter host answers effects, proved by running it.
//!
//! Same argument as the Qt, SwiftUI and Compose acceptances beside it: this
//! crate's other tests assert on the *text* of the emitted host, which cannot
//! establish that it compiles, let alone that it behaves. The Compose port of
//! this same change shipped three missing imports past every text assertion and
//! failed the first compiler invocation. `Effect` rode the wire and was read by
//! no native host for as long as it existed.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mosaic-app-bindings sits inside the rust workspace")
        .to_path_buf()
}

fn dart_available() -> bool {
    Command::new("dart")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

/// The conformance runtime, as a shared library the host loads through FFI.
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
fn the_emitted_flutter_host_answers_effects() {
    if !dart_available() {
        eprintln!("skipping Flutter effect acceptance: dart unavailable");
        return;
    }

    let project = std::env::temp_dir().join(format!(
        "mosaic-flutter-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&project).expect("create the driver project");
    std::fs::create_dir(project.join("lib")).unwrap();
    std::fs::create_dir(project.join("bin")).unwrap();

    // The host under test, exactly as a real package would receive it. Not
    // bundled: this resolves the runtime through `MOSAIC_APP_LIBRARY`, which is
    // the path whose seventh symbol is looked up leniently.
    let host = mosaic_app_bindings::flutter_runtime_binding_for_application(
        "dev.mosaic.flutter-effects",
        false,
    );
    std::fs::write(project.join("lib").join("mosaic_host.dart"), &host).unwrap();
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("flutter_effect_driver")
            .join("driver.dart"),
        project.join("bin").join("driver.dart"),
    )
    .expect("copy the driver");
    std::fs::write(
        project.join("pubspec.yaml"),
        "name: mosaic_flutter_effect_driver\n\
         description: Execution acceptance for the emitted Mosaic Flutter host.\n\
         publish_to: none\n\
         version: 0.0.0\n\n\
         environment:\n  sdk: '>=3.5.0 <4.0.0'\n\n\
         dependencies:\n  ffi: '>=2.1.0 <3.0.0'\n",
    )
    .unwrap();

    // Offline: the acceptance must not depend on pub.dev being reachable. `ffi`
    // is the only dependency and any Dart or Flutter checkout that has ever
    // resolved it has it cached; without it the test skips rather than fails.
    let resolve = Command::new("dart")
        .current_dir(&project)
        .args(["pub", "get", "--offline"])
        .output()
        .expect("run dart pub get");
    if !resolve.status.success() {
        eprintln!(
            "skipping Flutter effect acceptance: `dart pub get --offline` could not resolve \
             `ffi` from the local pub cache:\n{}",
            String::from_utf8_lossy(&resolve.stderr)
        );
        let _ = std::fs::remove_dir_all(&project);
        return;
    }

    let runtime = conformance_runtime();

    // Each case gets its own process AND its own state file: the host reads
    // MOSAIC_APP_STATE_PATH once at load, so cases sharing one file would let
    // one restore another's count, making an assertion read a number its own
    // host never produced.
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
            Command::new("dart")
                .current_dir(&project)
                .args(["run", "bin/driver.dart"])
                .env("MOSAIC_APP_LIBRARY", &runtime)
                .env("MOSAIC_PROBE_CASE", case)
                .env(
                    "MOSAIC_APP_STATE_PATH",
                    project.join(format!("{case}.json")),
                ),
            &format!("flutter effect driver ({case})"),
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
