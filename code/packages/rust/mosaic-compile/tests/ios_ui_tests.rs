//! `mosaic-compile pkg --ios-ui-test` (UI89 §4.3): the flag reaches the
//! builder, repeats, and is refused outside an iOS app build.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("mosaic-compile lives at code/packages/rust/mosaic-compile")
        .to_path_buf()
}

fn temporary_directory(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-ios-ui-tests-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

/// A stand-in `.xcframework`: the builder checks for the directory and its
/// `Info.plist` and copies it; nothing here links it.
fn fake_xcframework(root: &Path) -> PathBuf {
    let framework = root.join("JournalRuntime.xcframework");
    fs::create_dir_all(framework.join("ios-arm64")).unwrap();
    fs::write(framework.join("Info.plist"), b"<plist/>").unwrap();
    fs::write(framework.join("ios-arm64/libjournal.a"), b"static-runtime").unwrap();
    framework
}

fn pkg(output: &Path, extra: &[&str]) -> Output {
    let journal = repository_root().join("code/programs/mosaic/journal-app");
    Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
        .args([
            "pkg",
            journal.to_str().unwrap(),
            "--backend",
            "swiftui",
            "--output",
            output.to_str().unwrap(),
            "--emit-project",
        ])
        .args(extra)
        .output()
        .expect("run mosaic-compile")
}

#[test]
fn repeated_ui_tests_reach_the_ios_project_and_its_scheme() {
    let scratch = temporary_directory("ok");
    let framework = fake_xcframework(&scratch);
    let first = scratch.join("JournalUiTests.swift");
    let second = scratch.join("JournalRestoreUiTests.swift");
    fs::write(&first, "import XCTest\n").unwrap();
    fs::write(&second, "import XCTest\n").unwrap();
    let output = scratch.join("out");

    let result = pkg(
        &output,
        &[
            "--runtime-library",
            framework.to_str().unwrap(),
            "--ios-ui-test",
            first.to_str().unwrap(),
            "--ios-ui-test",
            second.to_str().unwrap(),
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let swiftui = output.join("swiftui");
    for name in ["JournalUiTests.swift", "JournalRestoreUiTests.swift"] {
        assert!(swiftui.join("UITests").join(name).is_file(), "{name}");
    }
    let project = fs::read_to_string(swiftui.join("iOS/App.xcodeproj/project.pbxproj")).unwrap();
    assert!(
        project.contains("path = \"UITests/JournalUiTests.swift\";"),
        "{project}"
    );
    assert!(
        project.contains("path = \"UITests/JournalRestoreUiTests.swift\";"),
        "{project}"
    );
    assert!(swiftui
        .join("iOS/App.xcworkspace/xcshareddata/xcschemes/AppUITests.xcscheme")
        .is_file());
    fs::remove_dir_all(&scratch).ok();
}

#[test]
fn ui_tests_without_an_ios_app_build_are_refused() {
    let scratch = temporary_directory("refused");
    let test = scratch.join("JournalUiTests.swift");
    fs::write(&test, "import XCTest\n").unwrap();

    // No .xcframework runtime: a macOS SwiftPM shell, not an iOS app.
    let result = pkg(
        &scratch.join("out"),
        &["--ios-ui-test", test.to_str().unwrap()],
    );
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("--ios-ui-test"), "{stderr}");
    fs::remove_dir_all(&scratch).ok();
}
