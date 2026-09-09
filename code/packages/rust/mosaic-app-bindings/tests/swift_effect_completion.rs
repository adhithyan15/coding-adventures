//! The emitted SwiftUI host answers effects, proved by running it.
//!
//! Same argument as the Qt acceptance beside it: this crate's other tests
//! assert on the *text* of the emitted host, which cannot establish that it
//! compiles, let alone that it behaves. `Effect` rode the wire and was read by
//! no native host for as long as it existed.

#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mosaic-app-bindings sits inside the rust workspace")
        .to_path_buf()
}

fn tool_available(command: &str) -> bool {
    Command::new(command)
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

fn conformance_runtime() -> PathBuf {
    let root = workspace_root();
    let status = Command::new(cargo())
        .current_dir(&root)
        .args(["build", "-p", "mosaic-app-conformance"])
        .status()
        .expect("build the conformance runtime");
    assert!(status.success(), "conformance runtime failed to build");
    let library = root
        .join("target")
        .join("debug")
        .join("libmosaic_app_conformance.dylib");
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
fn the_emitted_swiftui_host_answers_effects() {
    if !tool_available("swiftc") || !tool_available("clang") {
        eprintln!("skipping SwiftUI effect acceptance: swiftc or clang unavailable");
        return;
    }

    let project = std::env::temp_dir().join(format!(
        "mosaic-swift-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&project).expect("create the driver project");

    // The host under test, exactly as a real package would receive it.
    let binding =
        mosaic_app_bindings::swift_runtime_binding_for_application("dev.mosaic.swift-effects");
    std::fs::write(project.join("MosaicRuntimeHost.swift"), &binding.host_swift).unwrap();
    std::fs::write(project.join("CMosaicRuntime.h"), &binding.header).unwrap();
    std::fs::write(project.join("CMosaicRuntime.c"), &binding.loader_c).unwrap();
    std::fs::write(
        project.join("module.modulemap"),
        "module CMosaicRuntime {\n    header \"CMosaicRuntime.h\"\n    export *\n}\n",
    )
    .unwrap();
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("swift_effect_driver")
            .join("main.swift"),
        project.join("main.swift"),
    )
    .expect("copy the driver");

    run(
        Command::new("clang").current_dir(&project).args([
            "-c",
            "CMosaicRuntime.c",
            "-o",
            "CMosaicRuntime.o",
        ]),
        "clang (C shim)",
    );
    run(
        Command::new("swiftc").current_dir(&project).args([
            "-o",
            "driver",
            "main.swift",
            "MosaicRuntimeHost.swift",
            "CMosaicRuntime.o",
            "-I",
            ".",
            "-Xcc",
            "-fmodule-map-file=module.modulemap",
        ]),
        "swiftc",
    );

    // Each host gets its own state file: they persist, and a shared path lets
    // one restore another's count, so an assertion reads a number its own host
    // never produced.
    let stdout = run(
        Command::new(project.join("driver"))
            .env("MOSAIC_APP_LIBRARY", conformance_runtime())
            .env("MOSAIC_PROBE_STATE_A", project.join("state-a.json"))
            .env("MOSAIC_PROBE_STATE_B", project.join("state-b.json"))
            .env("MOSAIC_PROBE_STATE_C", project.join("state-c.json"))
            .env("MOSAIC_PROBE_STATE_D", project.join("state-d.json")),
        "swift effect driver",
    );

    assert!(
        stdout.contains("all checks passed"),
        "driver did not report success:\n{stdout}"
    );
    // A driver that printed nothing would satisfy the line above by accident.
    for expected in [
        "a fresh app awaits nothing",
        "an unanswered await is failed, not dropped",
        "the app is told why, rather than just waiting",
        "an answered await is settled",
        "the handler's value reached the app",
        "the handler answered both effects of the batch",
        "a fully-answered chaining batch leaves nothing outstanding",
        "snapshot still works after a fully-answered chaining batch",
        "a partly-answered batch leaves nothing outstanding",
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
