//! `_MosaicDialogHost` behaves as UI29-1 §3.3 says, proved by running it.
//!
//! The unit tests in `src/pipeline.rs` pin the helper's text -- a route of
//! its own, never `showDialog` -- but text cannot show that the dialog opens
//! once, closes once, or leaves other routes alone. This writes the helper
//! exactly as the emitter produces it into a throwaway Flutter package,
//! beside the committed widget tests in `conformance/dialog-host/`, and runs
//! `flutter test`.
//!
//! Without `flutter` on PATH the test skips, unless
//! `MOSAIC_REQUIRE_FLUTTER` is set: CI's Flutter lane sets it, so there a
//! missing SDK is a broken runner rather than a pass.

use std::path::{Path, PathBuf};
use std::process::Command;

fn flutter_available() -> bool {
    Command::new("flutter")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// The throwaway package, removed however the test ends -- a panic in any
/// step included.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(project: &Path, args: &[&str]) -> std::process::Output {
    Command::new("flutter")
        .args(args)
        .current_dir(project)
        .output()
        .unwrap_or_else(|err| panic!("run flutter {args:?}: {err}"))
}

#[test]
fn the_dialog_host_passes_its_widget_tests() {
    if !flutter_available() {
        assert!(
            std::env::var_os("MOSAIC_REQUIRE_FLUTTER").is_none(),
            "MOSAIC_REQUIRE_FLUTTER is set but `flutter` is not on PATH"
        );
        eprintln!("skipping the dialog host's widget tests: flutter unavailable");
        return;
    }

    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("dialog-host");
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "mosaic-dialog-host-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    )));
    let project = scratch.0.as_path();
    // A fresh directory, never one that is already there.
    std::fs::create_dir(project).expect("create the harness project");
    std::fs::create_dir(project.join("lib")).expect("create the harness lib");
    std::fs::create_dir(project.join("test")).expect("create the harness tests");

    // The committed harness, not a second copy of it.
    for file in ["pubspec.yaml", "test/dialog_host_test.dart"] {
        std::fs::copy(harness.join(file), project.join(file))
            .unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }
    // The helper as a generated file carries it, and a public name for the
    // tests: Dart keeps `_MosaicDialogHost` private to its own library.
    std::fs::write(
        project.join("lib/dialog_host.dart"),
        format!(
            "import 'package:flutter/material.dart';\n\n{}\ntypedef MosaicDialogHost = _MosaicDialogHost;\n",
            mosaic_emit_flutter::pipeline::emit_dialog_helper()
        ),
    )
    .expect("write the helper");

    // Only SDK packages: resolve offline when the cache has them, as on a
    // sandboxed machine, and online otherwise.
    let offline = run(project, &["pub", "get", "--offline"]);
    if !offline.status.success() {
        let online = run(project, &["pub", "get"]);
        assert!(
            online.status.success(),
            "flutter pub get failed:\n{}",
            String::from_utf8_lossy(&online.stderr)
        );
    }
    // The reporter named, not left to `flutter test`'s choice: under GitHub
    // Actions it picks a reporter whose summary never says "All tests
    // passed", which the check below relies on to know tests really ran.
    let test = run(project, &["test", "--no-pub", "--reporter", "expanded"]);
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&test.stdout),
        String::from_utf8_lossy(&test.stderr)
    );
    assert!(
        test.status.success(),
        "the dialog host's widget tests failed:\n{report}"
    );
    assert!(report.contains("All tests passed"), "{report}");
}
