//! #15428: list-typed story fixtures reach the browser backends, and
//! `--strict-fixtures` turns an unrenderable fixture into a failure.
//!
//! Before this, `pipeline_slot_values` dropped every list with a warning, so
//! `SegmentedControl` (whose content is its `options` list) previewed with no
//! options in every story while the story check passed.
//!
//! `options` was `list<list<text>>` until toolkit 0.15 and is `list<text>`
//! since UI86, so these tests now drive the flat form through the real
//! component. The nested form is still covered by `mosmodel-compiler`'s
//! `parse_list_fixture` tests and each emitter's literal tests.

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
  "options": ["List", "Board \"B\""],
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
        react.contains(r#"options: ["List", "Board \"B\""],"#),
        "react fallback props must carry the labels, escaped:\n{react}"
    );
    let web_component = read(dir.join("webcomponent/main.js"));
    assert!(
        web_component.contains(r#"fallback: ["List", "Board \"B\""]"#),
        "web component slot table must carry the labels:\n{web_component}"
    );
    let html = read(dir.join("html/main.js"));
    assert!(
        html.contains(r#""Board \"B\"""#) && !html.contains("\"options\": []"),
        "html fallback props must carry the labels:\n{html}"
    );
}

#[test]
fn list_fixtures_reach_every_native_backend_under_strict_mode() {
    let dir = scratch("native");
    for backend in ["swiftui", "qt", "flutter", "xaml"] {
        let result = compile(backend, &dir, &["--strict-fixtures"]);
        assert!(
            result.status.success(),
            "{backend} should accept a list fixture under --strict-fixtures:
{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

/// A value no backend renders (an object) is still a warning by default and
/// a failure under --strict-fixtures, naming the slot.
#[test]
fn an_unrenderable_fixture_fails_only_under_strict_mode() {
    let dir = scratch("object");
    let src = repository_root().join("code/packages/mosaic/mosaic-pkg-toolkit/src");
    let fixtures = dir.join("fixtures.json");
    fs::write(&fixtures, r#"{ "options": { "not": "a list" } }"#).unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec![
            "--interface".to_string(),
            src.join("SegmentedControl.mil").to_str().unwrap().to_string(),
            "--layout".to_string(),
            src.join("SegmentedControl.mll").to_str().unwrap().to_string(),
            "--fixtures".to_string(),
            fixtures.to_str().unwrap().to_string(),
            "--backend".to_string(),
            "react".to_string(),
            "--output".to_string(),
            dir.join("out.tsx").to_str().unwrap().to_string(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));
        Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
            .args(&args)
            .output()
            .expect("run mosaic-compile")
    };
    let lenient = run(&[]);
    assert!(lenient.status.success(), "an unrenderable value is a warning by default");
    assert!(String::from_utf8_lossy(&lenient.stderr).contains("warning"));

    let strict = run(&["--strict-fixtures"]);
    assert!(!strict.status.success(), "it must fail under --strict-fixtures");
    assert!(
        String::from_utf8_lossy(&strict.stderr).contains("slot `options`"),
        "the failure must name the slot"
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
