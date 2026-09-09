//! The emitted Qt host answers effects, proved by running it.
//!
//! This crate's other tests assert on the *text* of the emitted host. That
//! cannot tell you it compiles, and certainly not that it behaves -- and the
//! defect this covers is behavioural. `Effect` rode the wire and was read by no
//! native host for as long as it existed: an `await` was dropped and the app
//! waited forever, with nothing anywhere reporting it.
//!
//! So this emits the host, compiles it against Qt Core, links it to the real
//! conformance runtime, and runs it.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mosaic-app-bindings sits inside the rust workspace")
        .to_path_buf()
}

/// Whether the build this test performs can actually run here.
///
/// Checks for the Qt6 **Core** CMake package specifically, rather than for the
/// presence of a Qt tool. `qmake` on `PATH` says a Qt exists; it does not say
/// CMake can find the component this build requires, and a guard that conflates
/// them turns "unavailable" into a red test instead of a skip.
fn qt_core_available() -> bool {
    if Command::new("cmake")
        .arg("--version")
        .output()
        .map(|out| !out.status.success())
        .unwrap_or(true)
    {
        return false;
    }
    let probe = std::env::temp_dir().join(format!("mosaic-qt-core-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&probe);
    if std::fs::create_dir_all(&probe).is_err() {
        return false;
    }
    let ok = std::fs::write(
        probe.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.19)\n\
         project(probe LANGUAGES CXX)\n\
         find_package(Qt6 REQUIRED COMPONENTS Core)\n",
    )
    .is_ok()
        && Command::new("cmake")
            .arg("-S")
            .arg(&probe)
            .arg("-B")
            .arg(probe.join("build"))
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
    let _ = std::fs::remove_dir_all(&probe);
    ok
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

/// Build the conformance runtime and return the cdylib the host will load.
fn conformance_runtime() -> PathBuf {
    let root = workspace_root();
    let status = Command::new(cargo())
        .current_dir(&root)
        .args(["build", "-p", "mosaic-app-conformance"])
        .status()
        .expect("build the conformance runtime");
    assert!(status.success(), "conformance runtime failed to build");

    let name = if cfg!(target_os = "macos") {
        "libmosaic_app_conformance.dylib"
    } else {
        "libmosaic_app_conformance.so"
    };
    let library = root.join("target").join("debug").join(name);
    assert!(
        library.is_file(),
        "missing conformance runtime at {library:?}"
    );
    library
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
fn the_emitted_qt_host_answers_effects() {
    if !qt_core_available() {
        eprintln!("skipping Qt effect-completion acceptance: CMake or Qt6 Core unavailable");
        return;
    }

    let project = std::env::temp_dir().join(format!(
        "mosaic-qt-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&project).expect("create the driver project");

    // The host under test, exactly as a real package would receive it.
    let binding =
        mosaic_app_bindings::qt_runtime_binding_for_application("dev.mosaic.qt-effect-acceptance");
    std::fs::write(project.join("MosaicHost.h"), &binding.header).expect("write header");
    std::fs::write(project.join("MosaicHost.cpp"), &binding.source).expect("write source");

    let driver = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("qt_effect_driver");
    for file in ["main.cpp", "CMakeLists.txt"] {
        std::fs::copy(driver.join(file), project.join(file))
            .unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }

    let build = project.join("build");
    run(
        Command::new("cmake")
            .arg("-S")
            .arg(&project)
            .arg("-B")
            .arg(&build),
        "cmake configure",
    );
    run(
        Command::new("cmake").arg("--build").arg(&build),
        "cmake build",
    );

    let executable = [
        build.join("mosaic_qt_effect_driver"),
        build.join("Debug").join("mosaic_qt_effect_driver"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .expect("locate the built driver");

    // Each host gets its own state file. They persist, and a shared path would
    // let one host restore the other's count -- an assertion reading a number
    // the host under test never produced.
    let stdout = run(
        Command::new(&executable)
            .env("MOSAIC_APP_LIBRARY", conformance_runtime())
            .env("MOSAIC_APP_STATE_PATH", project.join("state-a.json"))
            .env("MOSAIC_PROBE_STATE_B", project.join("state-b.json"))
            .env("MOSAIC_PROBE_STATE_C", project.join("state-c.json"))
            .env("MOSAIC_PROBE_STATE_D", project.join("state-d.json"))
            .env("MOSAIC_PROBE_STATE_E", project.join("state-e.json"))
            .env("MOSAIC_PROBE_STATE_F", project.join("state-f.json")),
        "qt effect driver",
    );

    assert!(
        stdout.contains("all checks passed"),
        "driver did not report success:\n{stdout}"
    );
    // A driver that printed nothing would satisfy the line above only by
    // accident; pin the individual checks it must have run.
    for expected in [
        "a fresh app awaits nothing",
        "an unanswered await is failed, not left outstanding",
        "the app is told why, rather than just waiting",
        "an answered await is settled",
        "the handler's value reached the app",
        "a partly-answered batch leaves nothing outstanding",
        "snapshot still works after a partly-answered batch",
        "a non-integral effect id is refused, not truncated",
        "a negative effect id is refused, not wrapped",
        "a bounded-out settle leaves nothing outstanding",
        "snapshot still works after the nesting bound fires",
        "the handler answered both effects of the batch",
        "a fully-answered chaining batch leaves nothing outstanding",
        "snapshot still works after a fully-answered chaining batch",
    ] {
        assert!(
            stdout.contains(expected),
            "missing check `{expected}`:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains("VACUOUS"),
        "a check ran vacuously:\n{stdout}"
    );

    let _ = std::fs::remove_dir_all(&project);
}
