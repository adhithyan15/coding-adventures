//! Regression test for #18122: SIMPLE and ADVANCED double backslashes in import/export
//! module specifiers (`./a\x2ejs` → `./a\\x2ejs`).
//!
//! ## The miscompile
//!
//! At `SIMPLE` and `ADVANCED` compilation levels, string escape sequences in ES module
//! specifiers (`import` declarations, side-effect imports, and `export ... from` re-exports)
//! previously retained their source escape sequences in `StringLiteral.value` because
//! `javascript-parser/src/bridge.rs::import_source` copied the raw token text without
//! decoding it via `decode_string_escapes`.
//!
//! When `closure-emitter` re-emitted the string via `emit_string`, its quote-and-escape
//! pass re-escaped every backslash in `s.value`, doubling them (e.g. `./a\x2ejs` became
//! the literal file path `./a\\x2ejs`).
//!
//! The fix: `import_source` decodes escapes via `decode_string_escapes(&t.value)`,
//! ensuring `StringLiteral.value` contains the decoded string, matching expression
//! string literals and object property keys.
//!
//! When `node` is on `PATH`, the test validates the output with `node --input-type=module --check`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

struct TestCase {
    name: &'static str,
    source: &'static str,
    expected_simple: &'static str,
    expected_ws: &'static str,
}

const CASES: &[TestCase] = &[
    TestCase {
        name: "import_default_hex_escape",
        source: "import a from \"./a\\x2ejs\";",
        expected_simple: "import a from\"./a.js\";",
        expected_ws: "import a from\"./a.js\";",
    },
    TestCase {
        name: "side_effect_import_hex_escape",
        source: "import \"./a\\x2ejs\";",
        expected_simple: "import\"./a.js\";",
        expected_ws: "import\"./a.js\";",
    },
    TestCase {
        name: "export_named_reexport_hex_escape",
        source: "export { b } from \"./c\\x41\";",
        expected_simple: "export{b}from\"./cA\";",
        expected_ws: "export{b}from\"./cA\";",
    },
    TestCase {
        name: "export_all_reexport_hex_escape",
        source: "export * from \"./c\\x41\";",
        expected_simple: "export*from\"./cA\";",
        expected_ws: "export*from\"./cA\";",
    },
    TestCase {
        name: "import_unicode_escape",
        source: "import x from \"./mod\\u0065\\x2ejs\";",
        expected_simple: "import x from\"./mode.js\";",
        expected_ws: "import x from\"./mode.js\";",
    },
    TestCase {
        name: "import_namespace_hex_escape",
        source: "import * as ns from \"./ns\\x2ejs\";",
        expected_simple: "import*as ns from\"./ns.js\";",
        expected_ws: "import*as ns from\"./ns.js\";",
    },
    TestCase {
        name: "import_named_multiple_specifiers",
        source: "import { foo, bar as baz } from \"./lib\\x2futils\\x2ejs\";",
        expected_simple: "import{foo,bar as baz}from\"./lib/utils.js\";",
        expected_ws: "import{foo,bar as baz}from\"./lib/utils.js\";",
    },
    TestCase {
        name: "import_escaped_backslash_not_quadrupled",
        source: "import a from \"./dir\\\\file\";",
        expected_simple: "import a from\"./dir\\\\file\";",
        expected_ws: "import a from\"./dir\\\\file\";",
    },
    TestCase {
        name: "mixed_imports_and_exports",
        source: "import a from \"./a\\x2ejs\";\nexport { b } from \"./c\\x41\";",
        expected_simple: "import a from\"./a.js\";export{b}from\"./cA\";",
        expected_ws: "import a from\"./a.js\";export{b}from\"./cA\";",
    },
];

fn run_closurec(source: &str, level: &str) -> (bool, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-specifier-escapes-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, source).expect("write tempfile");

    let out = Command::new(BINARY)
        .arg("-O")
        .arg(level)
        .arg("--js")
        .arg(&path)
        .output()
        .expect("run closurec");

    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (out.status.success(), stdout, stderr)
}

fn node_parses_module(code: &str) -> Option<bool> {
    static NEXT_NODE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT_NODE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-node-check-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("test.mjs");
    if std::fs::write(&path, code).is_err() {
        let _ = std::fs::remove_dir_all(&dir);
        return None;
    }
    let check = Command::new("node").arg("--check").arg(&path).output();
    let _ = std::fs::remove_dir_all(&dir);
    match check {
        Ok(out) => Some(out.status.success()),
        Err(_) => None,
    }
}

#[test]
fn test_module_specifier_escapes_at_simple_level() {
    for case in CASES {
        let (ok, stdout, stderr) = run_closurec(case.source, "SIMPLE");
        assert!(
            ok,
            "SIMPLE failed for case {}:\nsource: {:?}\nstderr: {}",
            case.name, case.source, stderr
        );
        let trimmed = stdout.trim();
        assert_eq!(
            trimmed, case.expected_simple,
            "SIMPLE mismatch for case {}:\nsource: {:?}\ngot: {:?}\nexpected: {:?}",
            case.name, case.source, trimmed, case.expected_simple
        );

        if let Some(parses) = node_parses_module(trimmed) {
            assert!(
                parses,
                "node --input-type=module --check rejected SIMPLE output for {}: {:?}",
                case.name, trimmed
            );
        }
    }
}

#[test]
fn test_module_specifier_escapes_at_advanced_level() {
    for case in CASES {
        let (ok, stdout, stderr) = run_closurec(case.source, "ADVANCED");
        assert!(
            ok,
            "ADVANCED failed for case {}:\nsource: {:?}\nstderr: {}",
            case.name, case.source, stderr
        );
        let trimmed = stdout.trim();
        assert_eq!(
            trimmed, case.expected_simple,
            "ADVANCED mismatch for case {}:\nsource: {:?}\ngot: {:?}\nexpected: {:?}",
            case.name, case.source, trimmed, case.expected_simple
        );

        if let Some(parses) = node_parses_module(trimmed) {
            assert!(
                parses,
                "node --input-type=module --check rejected ADVANCED output for {}: {:?}",
                case.name, trimmed
            );
        }
    }
}

#[test]
fn test_module_specifier_escapes_at_whitespace_only_level() {
    for case in CASES {
        let (ok, stdout, stderr) = run_closurec(case.source, "WHITESPACE_ONLY");
        assert!(
            ok,
            "WHITESPACE_ONLY failed for case {}:\nsource: {:?}\nstderr: {}",
            case.name, case.source, stderr
        );
        let trimmed = stdout.trim();
        assert_eq!(
            trimmed, case.expected_ws,
            "WHITESPACE_ONLY mismatch for case {}:\nsource: {:?}\ngot: {:?}\nexpected: {:?}",
            case.name, case.source, trimmed, case.expected_ws
        );

        if let Some(parses) = node_parses_module(trimmed) {
            assert!(
                parses,
                "node --input-type=module --check rejected WHITESPACE_ONLY output for {}: {:?}",
                case.name, trimmed
            );
        }
    }
}
