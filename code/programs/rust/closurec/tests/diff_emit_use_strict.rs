//! Integration test for the `tests/diff/emit-use-strict/` fixture and CCR-085 (#15920).
//!
//! Exercises CLOC11.18 / CCR-085 — `--emit_use_strict` — end-to-end through
//! the built binary. Asserts the emitted output starts with the
//! `'use strict';` directive when `--emit_use_strict` is passed, and strips
//! leading `"use strict"` / `'use strict'` directive prologues by default.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/emit-use-strict/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn compile_with_flags(source: &str, extra_flags: &[&str]) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-diff-strict-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");

    let mut cmd = Command::new(BINARY);
    cmd.args(extra_flags).arg("--js").arg(&path);
    let out = cmd.output().expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "{source:?} with flags {extra_flags:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("closurec output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

#[test]
fn emit_use_strict_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/emit-use-strict/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

#[test]
fn emit_use_strict_strips_top_level_directive_by_default() {
    for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
        let out = compile_with_flags("\"use strict\"; var x = 1;", &["--compilation_level", level]);
        assert!(
            !out.contains("use strict"),
            "expected 'use strict' stripped at {level}, got: {out:?}"
        );
        assert!(out.contains("var x=1") || out.is_empty(), "got: {out:?}");
    }
}

#[test]
fn emit_use_strict_reemits_single_quoted() {
    for level in ["WHITESPACE_ONLY", "SIMPLE"] {
        let out = compile_with_flags(
            "\"use strict\"; var x = 1;",
            &["--emit_use_strict", "--compilation_level", level],
        );
        assert!(
            out.starts_with("'use strict';"),
            "expected leading single-quoted 'use strict'; at {level}, got: {out:?}"
        );
    }
}

#[test]
fn emit_use_strict_preserves_non_use_strict_directive() {
    for level in ["WHITESPACE_ONLY", "SIMPLE"] {
        let out = compile_with_flags(
            "\"use asm\"; var x = 1;",
            &["--compilation_level", level],
        );
        assert!(
            out.contains("\"use asm\";"),
            "expected '\"use asm\";' preserved at {level}, got: {out:?}"
        );
    }
}

#[test]
fn emit_use_strict_strips_function_body_directive() {
    for level in ["WHITESPACE_ONLY", "SIMPLE"] {
        let out = compile_with_flags(
            "function f() { \"use strict\"; return 1; }",
            &["--compilation_level", level],
        );
        assert!(
            !out.contains("use strict"),
            "expected function-body 'use strict' stripped at {level}, got: {out:?}"
        );
        assert!(out.contains("function f()"), "got: {out:?}");
    }
}

#[test]
fn emit_use_strict_pretty_print() {
    let out = compile_with_flags(
        "var x = 1;",
        &["--emit_use_strict", "--formatting", "PRETTY_PRINT", "--compilation_level", "WHITESPACE_ONLY"],
    );
    assert!(
        out.starts_with("'use strict';\n"),
        "expected 'use strict';\\n in pretty print mode, got: {out:?}"
    );
}
