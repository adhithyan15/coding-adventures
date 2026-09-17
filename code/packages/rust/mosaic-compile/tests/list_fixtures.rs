//! #15428: list-typed story fixtures reach the browser backends, and
//! `--strict-fixtures` turns an unrenderable fixture into a failure.
//!
//! Before this, `pipeline_slot_values` dropped every list with a warning, so
//! `SegmentedControl` (whose content is `options : list<list<text>>`)
//! previewed with no options in every story while the story check passed.

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

fn scratch(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-list-fixtures-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("scratch directory");
    path
}

const FIXTURES: &str = r#"{
  "options": [["List", "List"], ["Board \"B\"", "Board, selected"]],
  "selected-index": 1,
  "disabled": false
}"#;

fn compile(backend: &str, dir: &Path, extra: &[&str]) -> Output {
    let src = repository_root().join("code/packages/mosaic/mosaic-pkg-toolkit/src");
    let fixtures = dir.join("fixtures.json");
    fs::write(&fixtures, FIXTURES).expect("write fixtures");
    let mut args: Vec<String> = [
        "--interface",
        src.join("SegmentedControl.mil").to_str().unwrap(),
        "--layout",
        src.join("SegmentedControl.mll").to_str().unwrap(),
        "--fixtures",
        fixtures.to_str().unwrap(),
        "--emit-project",
        "--backend",
        backend,
        "--output",
        dir.join(backend).join("SegmentedControl.out").to_str().unwrap(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
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
fn list_fixtures_reach_every_browser_backend_under_strict_mode() {
    let dir = scratch("browser");
    for backend in ["react", "html", "webcomponent"] {
        let result = compile(backend, &dir, &["--strict-fixtures"]);
        assert!(
            result.status.success(),
            "{backend} should accept a list fixture under --strict-fixtures:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let react = read(dir.join("react/src/main.tsx"));
    assert!(
        react.contains(r#"options: [["List", "List"], ["Board \"B\"", "Board, selected"]],"#),
        "react fallback props must carry the rows, escaped:\n{react}"
    );
    let web_component = read(dir.join("webcomponent/main.js"));
    assert!(
        web_component.contains(r#"fallback: [["List", "List"], ["Board \"B\"", "Board, selected"]]"#),
        "web component slot table must carry the rows:\n{web_component}"
    );
    let html = read(dir.join("html/main.js"));
    assert!(
        html.contains("\"Board, selected\"") && !html.contains("\"options\": []"),
        "html fallback props must carry the rows:\n{html}"
    );
}

#[test]
fn a_list_the_backend_cannot_render_fails_only_under_strict_mode() {
    let dir = scratch("native");
    let lenient = compile("swiftui", &dir, &[]);
    assert!(lenient.status.success(), "a dropped list is a warning by default");
    assert!(
        String::from_utf8_lossy(&lenient.stderr).contains("warning"),
        "the drop must still be reported"
    );

    let strict = compile("swiftui", &dir, &["--strict-fixtures"]);
    assert!(!strict.status.success(), "a dropped list must fail under --strict-fixtures");
    let stderr = String::from_utf8_lossy(&strict.stderr);
    assert!(
        stderr.contains("slot `options`") && stderr.contains("swiftui"),
        "the failure must name the slot and the backend: {stderr}"
    );
}

/// `--strict-style` was documented and read by `main.rs`, but missing from
/// the CLI spec, so passing it failed with "Unknown flag" (#15428 found it).
#[test]
fn strict_style_is_a_recognised_flag() {
    let dir = scratch("strict-style");
    let src = repository_root().join("code/packages/mosaic/mosaic-pkg-toolkit/src");
    let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
        .args([
            "--interface",
            src.join("Badge.mil").to_str().unwrap(),
            "--layout",
            src.join("Badge.mll").to_str().unwrap(),
            "--style",
            src.join("Badge.light.msl").to_str().unwrap(),
            "--backend",
            "html",
            "--output",
            dir.join("Badge.html").to_str().unwrap(),
            "--strict-style",
        ])
        .output()
        .expect("run mosaic-compile");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.contains("Unknown flag"), "{stderr}");
    assert!(result.status.success(), "{stderr}");
}
