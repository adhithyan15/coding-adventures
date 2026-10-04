//! The Flutter platform library behaves as the Compose one does, proved by
//! running it (UI87 §7.7).
//!
//! The text assertions in `src/lib.rs` pin the library's tables and messages
//! to the Compose library's; they cannot establish that the Dart compiles, let
//! alone that it behaves. This runs the headless conformance harness in
//! `conformance/flutter-platform-effects/` against the core library and the
//! runtime host exactly as `flutter_platform_effects` and
//! `flutter_runtime_binding` emit them: every open, save, refusal and routing
//! case, with a fake dialog and a fake host, on the plain Dart VM. The
//! `file_selector` dialogs need the Flutter engine and are compiled for real
//! by the Flutter CI lane, which also runs this harness against TaskApp.

use std::path::PathBuf;
use std::process::Command;

fn dart_available() -> bool {
    Command::new("dart")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn the_flutter_platform_library_passes_its_headless_conformance() {
    if !dart_available() {
        // CI's off-Linux job sets this: there a missing `dart` is a broken
        // runner, not a reason to pass without running anything.
        assert!(
            std::env::var_os("MOSAIC_REQUIRE_DART").is_none(),
            "MOSAIC_REQUIRE_DART is set but `dart` is not on PATH"
        );
        eprintln!("skipping Flutter platform-library conformance: dart unavailable");
        return;
    }

    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("flutter-platform-effects");
    let project = std::env::temp_dir().join(format!(
        "mosaic-flutter-platform-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(project.join("bin")).expect("create the harness project");
    std::fs::create_dir_all(project.join("lib")).expect("create the harness lib");

    // The committed harness, not a second copy of it.
    for file in ["pubspec.yaml", "bin/conformance.dart"] {
        std::fs::copy(harness.join(file), project.join(file))
            .unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }
    // The library and the host under test, as a generated project gets them.
    std::fs::write(
        project.join("lib/mosaic_platform_effects_core.dart"),
        mosaic_app_bindings::flutter_platform_effects().core,
    )
    .unwrap();
    std::fs::write(
        project.join("lib/mosaic_host.dart"),
        mosaic_app_bindings::flutter_runtime_binding(),
    )
    .unwrap();

    let run = |args: &[&str]| {
        Command::new("dart")
            .current_dir(&project)
            .args(args)
            .output()
            .expect("run dart")
    };
    let steps: [&[&str]; 3] = [
        &["pub", "get"],
        &["analyze", "--fatal-infos"],
        &["run", "bin/conformance.dart"],
    ];
    let mut last_stdout = String::new();
    for step in steps {
        let output = run(step);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() {
            let _ = std::fs::remove_dir_all(&project);
            panic!(
                "`dart {}` failed ({});\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
                step.join(" "),
                output.status
            );
        }
        last_stdout = stdout;
    }
    let _ = std::fs::remove_dir_all(&project);
    assert!(
        last_stdout.contains("Mosaic Flutter platform effects conformance passed"),
        "{last_stdout}"
    );
}
