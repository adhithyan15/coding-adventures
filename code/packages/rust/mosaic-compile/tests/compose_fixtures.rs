//! EM-6 (#14704): the Compose pipeline writes a runnable desktop project with
//! `--emit-project`, and a story's `--fixtures` become that shell's values, as
//! on every other pipeline backend (#14459).
//!
//! Before this, Compose refused fixtures under `--strict-fixtures` and warned
//! about both flags otherwise (#16930), so a story could not preview natively
//! on Compose at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A scratch-name nonce that is unique for every call in this process.
///
/// The clock alone is not: macOS reports `SystemTime` in whole microseconds,
/// so two tests running in parallel threads could read the same "nanos" and
/// be handed the same directory, each seeing the other's files (this made
/// `compose_fixtures` fail about one run in three). The counter makes every
/// call distinct; the clock keeps names from a previous run apart.
fn unique_nonce() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    format!("{nanos}-{n}")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("mosaic-compile lives at code/packages/rust/mosaic-compile")
        .to_path_buf()
}

fn scratch() -> PathBuf {
    let nonce = unique_nonce();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-compose-fixtures-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("scratch directory");
    path
}

/// Compile the toolkit's Button into `dir`, with `fixtures` (if any) written
/// to a file and passed by `--fixtures`.
fn compile(dir: &Path, fixtures: Option<&str>, extra: &[&str]) -> Output {
    let src = repository_root().join("code/packages/mosaic/mosaic-pkg-toolkit/src");
    let mut args: Vec<String> = [
        "--interface",
        src.join("Button.mil").to_str().unwrap(),
        "--layout",
        src.join("Button.mll").to_str().unwrap(),
        "--backend",
        "compose",
        "--output",
        dir.join("Button.kt").to_str().unwrap(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if let Some(json) = fixtures {
        let path = dir.join("fixtures.json");
        fs::write(&path, json).expect("write fixtures");
        args.push("--fixtures".into());
        args.push(path.to_str().unwrap().into());
    }
    args.extend(extra.iter().map(|s| s.to_string()));
    Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
        .args(&args)
        .output()
        .expect("run mosaic-compile")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn strict_fixtures_reach_the_compose_project_shell() {
    let dir = scratch();
    let result = compile(
        &dir,
        Some(r#"{ "label": "Pay $5 \"now\"" }"#),
        &["--emit-project", "--strict-fixtures"],
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    assert!(!stderr.contains("warning"), "{stderr}");

    // The story value replaces the sample, escaped for a Kotlin literal: `$`
    // would otherwise start a string template.
    let main = read(dir.join("src/main/kotlin/MosaicAppShell.kt"));
    assert!(
        main.contains(r#"mosaicString(hostProps, "label", "Pay \$5 \"now\"")"#),
        "{main}"
    );
    assert!(!main.contains("Sample Label"), "{main}");
    assert!(read(dir.join("src/main/kotlin/Main.kt"))
        .contains("mosaic-compile --backend compose --emit-project"));

    // A runnable Gradle project: the build files, and the component copied
    // into the source set Gradle compiles.
    for file in ["settings.gradle.kts", "build.gradle.kts", "README.md"] {
        assert!(dir.join(file).is_file(), "{file} missing");
    }
    assert_eq!(
        read(dir.join("src/main/kotlin/Button.kt")),
        read(dir.join("Button.kt"))
    );
    for file in [
        "MosaicAppShell.kt",
        "MosaicPlatform.kt",
        "MosaicRuntimeHost.kt",
        "MosaicFileEffects.kt",
        "MosaicPlatformEffects.kt",
    ] {
        assert!(
            dir.join("src/main/kotlin").join(file).is_file(),
            "{file} missing"
        );
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_compose_shell_without_fixtures_shows_samples_and_is_deterministic() {
    let first = scratch();
    let second = scratch();
    for dir in [&first, &second] {
        let result = compile(dir, None, &["--emit-project"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let main = read(first.join("src/main/kotlin/MosaicAppShell.kt"));
    assert!(main.contains("Sample Label"), "{main}");
    for file in [
        "settings.gradle.kts",
        "build.gradle.kts",
        "README.md",
        "src/main/kotlin/Main.kt",
    ] {
        assert_eq!(read(first.join(file)), read(second.join(file)), "{file}");
    }
    assert_eq!(main, read(second.join("src/main/kotlin/MosaicAppShell.kt")));
    fs::remove_dir_all(&first).ok();
    fs::remove_dir_all(&second).ok();
}

#[test]
fn compose_without_emit_project_writes_only_the_component() {
    let dir = scratch();
    let result = compile(&dir, Some(r#"{ "label": "Save" }"#), &["--strict-fixtures"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    assert!(!stderr.contains("warning"), "{stderr}");
    assert!(dir.join("Button.kt").is_file());
    assert!(!dir.join("build.gradle.kts").exists());
    assert!(!dir.join("src").exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unrenderable_fixture_still_fails_under_strict_mode() {
    let dir = scratch();
    let result = compile(
        &dir,
        Some(r#"{ "label": { "not": "text" } }"#),
        &["--emit-project", "--strict-fixtures"],
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--strict-fixtures"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::remove_dir_all(&dir).ok();
}

/// Every scratch directory is a fresh one. A clock-only name repeated
/// whenever two calls landed in the same microsecond, which on macOS is a
/// tight loop, and parallel tests then shared a directory.
#[test]
fn scratch_names_never_repeat_within_a_process() {
    let names: std::collections::HashSet<String> = (0..1000).map(|_| unique_nonce()).collect();
    assert_eq!(names.len(), 1000);
}
