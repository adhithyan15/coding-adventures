//! Integration test for the `tests/diff/js-output-file/` fixture.
//!
//! This is the CLOC11.03 counterpart of `diff_glob` — it drives
//! the actual built binary with `--js_output_file` pointing at a
//! freshly-generated path under a previously-nonexistent
//! directory, then asserts the file landed with the expected
//! content. Per [CLOC11 §3], the diff fixture's `expected.stdout`
//! captures what real `closure-compiler.jar` writes for the same
//! input.
//!
//! Two test cases:
//!   1. `--js_output_file out/dist/result.js` (parent dir doesn't
//!      exist) — exercises `write_output_file`'s auto-create.
//!   2. omitting `--js_output_file` — exercises stdout fallback.
//!
//! [CLOC11 §3]: ../../../specs/CLOC11-drop-in-closure-compat.md#3-strategy

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/js-output-file/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Unique temp dir under the system temp dir.
fn temp_dir(suffix: &str) -> std::path::PathBuf {
    let id = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let p = std::env::temp_dir().join(format!("closurec-cloc11-03-{id}-{nanos}-{suffix}"));
    std::fs::create_dir_all(&p).expect("create temp dir");
    p
}

#[test]
fn js_output_file_writes_to_disk_with_auto_create_parents() {
    let work = temp_dir("output");
    let out_path = work.join("build").join("dist").join("result.js");
    assert!(!out_path.parent().unwrap().exists(), "parent must not pre-exist");

    let mut flags = read_flags();
    flags.push("--js_output_file".to_string());
    flags.push(out_path.to_string_lossy().to_string());

    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "closurec exit code: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    // With --js_output_file set, stdout should be empty.
    assert!(
        out.stdout.is_empty(),
        "expected empty stdout when output-file is set; got: {:?}",
        String::from_utf8_lossy(&out.stdout),
    );

    // The output file landed and contains the expected content.
    let actual = std::fs::read_to_string(&out_path).expect("read out_path");
    let expected = std::fs::read_to_string("tests/diff/js-output-file/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
    );

    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn omitting_js_output_file_falls_back_to_stdout() {
    let flags = read_flags();
    // Same fixture but no --js_output_file → output should land
    // on stdout. Exercises CLOC11.01's stdout fallback path.
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(out.status.success());

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/js-output-file/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
    );
}
