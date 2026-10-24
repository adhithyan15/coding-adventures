//! Regression test for #17907: closurec must never print a statement that was
//! not a Use Strict Directive in the source as one in the output.
//!
//! ECMA-262 §11.2.1 makes a body strict only when its directive prologue holds
//! the exact token `"use strict"` or `'use strict'`. Each of these is *not*
//! one:
//!
//! ```js
//! "use\x20strict";    // has an escape
//! `use strict`;       // a template, not a string literal
//! ("use strict");     // parenthesised
//! "use strict", 1;    // part of a larger expression
//! ```
//!
//! closurec printed every one of them as `"use strict";`, which made sloppy
//! code strict. In strict code `this` in a plain call is `undefined`, so each
//! probe below flipped from printing `false` to printing `true`.
//!
//! The fix has two halves:
//!
//! - The AST printer (SIMPLE and ADVANCED) drops every `"use strict"`-valued
//!   statement in a program or function body's directive prologue (see
//!   `without_use_strict` in `closure-emitter`).
//! - WHITESPACE_ONLY keeps an escape in a `use strict` string whose source
//!   spelling had one, so it stays inert.
//!
//! Expected bytes are closurec's own. Where upstream v20260915 differs, the
//! row says how. When `node` is on `PATH`, the test also checks that each
//! output prints what its source prints.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Prints `true` only if `t`'s body runs in strict mode.
const PROBE: &str = "function t(){return this===undefined}console.log(t())";

/// `(source prefix, WHITESPACE_ONLY output, SIMPLE output)`. Each source is
/// `prefix + PROBE`.
const CASES: &[(&str, &str, &str)] = &[
    // Upstream strips this at both levels (it treats it as a directive).
    (
        r#""use\x20strict";"#,
        r#""use\x20strict";function t(){return this===undefined}console.log(t());"#,
        "function t(){return this===void 0}console.log(t());",
    ),
    // Upstream SIMPLE prints `"use strict";…` here: it folds the template to
    // a string after stripping directives, and so makes the program strict.
    (
        "`use strict`;",
        "`use strict`;function t(){return this===undefined}console.log(t());",
        "function t(){return this===void 0}console.log(t());",
    ),
    // Upstream strips this at both levels.
    (
        r#"("use strict");"#,
        r#"("use strict");function t(){return this===undefined}console.log(t());"#,
        "function t(){return this===void 0}console.log(t());",
    ),
    // Upstream SIMPLE prints `"use strict";1;…`, which is strict.
    (
        r#""use strict",1;"#,
        r#""use strict",1;function t(){return this===undefined}console.log(t());"#,
        "1;function t(){return this===void 0}console.log(t());",
    ),
];

/// Function bodies are directive prologues too. `(source, SIMPLE output)`;
/// the SIMPLE outputs match upstream.
const FUNCTION_CASES: &[(&str, &str)] = &[
    (
        r#"function f(){("use strict");return this===undefined}console.log(f())"#,
        "function f(){return this===void 0}console.log(f());",
    ),
    (
        r#"var g=function(){"use\x20strict";return this===undefined};console.log(g())"#,
        "var g=function(){return this===void 0};console.log(g());",
    ),
];

/// A `"use strict"` after another directive (`"a";`) is still inside the
/// prologue. SIMPLE flattens and folds these into a bare `"use strict"`
/// statement there, which upstream then prints as a live directive (its
/// SIMPLE output for each is `"a";"use strict";…`, which is strict). We drop
/// it. `(source prefix, SIMPLE output)`; each source is `prefix + PROBE`.
const LATER_IN_PROLOGUE: &[(&str, &str)] = &[
    (
        r#""a";if(1){"use strict"}"#,
        r#""a";function t(){return this===void 0}console.log(t());"#,
    ),
    (
        r#""a";{"use strict"}"#,
        r#""a";function t(){return this===void 0}console.log(t());"#,
    ),
    (
        r#""a";1&&"use strict";"#,
        r#""a";function t(){return this===void 0}console.log(t());"#,
    ),
    (
        r#""a";("use strict");"#,
        r#""a";function t(){return this===void 0}console.log(t());"#,
    ),
    (
        r#""a";"use strict",1;"#,
        r#""a";1;function t(){return this===void 0}console.log(t());"#,
    ),
];

fn compile(source: &str, level: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-use-strict-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{source:?} at {level}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("closurec output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

/// Run `node -e code` and return what it printed, or `None` when node isn't
/// installed.
fn node_stdout(code: &str) -> Option<String> {
    let out = Command::new("node").args(["-e", code]).output().ok()?;
    assert!(
        out.status.success(),
        "node -e {code:?} failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Every source the test compiles.
fn all_sources() -> Vec<String> {
    CASES
        .iter()
        .map(|(prefix, _, _)| format!("{prefix}{PROBE}"))
        .chain(FUNCTION_CASES.iter().map(|(s, _)| s.to_string()))
        .chain(
            LATER_IN_PROLOGUE
                .iter()
                .map(|(prefix, _)| format!("{prefix}{PROBE}")),
        )
        .collect()
}

#[test]
fn non_directives_are_never_printed_as_use_strict() {
    for (prefix, whitespace_only, simple) in CASES {
        let source = format!("{prefix}{PROBE}");
        assert_eq!(
            &compile(&source, "WHITESPACE_ONLY"),
            whitespace_only,
            "{source:?}"
        );
        assert_eq!(&compile(&source, "SIMPLE"), simple, "{source:?}");
    }
    for (source, simple) in FUNCTION_CASES {
        assert_eq!(&compile(source, "SIMPLE"), simple, "{source:?}");
    }
    for (prefix, simple) in LATER_IN_PROLOGUE {
        let source = format!("{prefix}{PROBE}");
        assert_eq!(&compile(&source, "SIMPLE"), simple, "{source:?}");
    }
}

#[test]
fn no_level_opens_its_output_with_use_strict() {
    for source in all_sources() {
        for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
            let out = compile(&source, level);
            // A whole statement at the very start. Function bodies and
            // later prologue positions are covered by the node comparison
            // below, which sees strictness directly.
            let opens = ["\"use strict\";", "\"use strict\"}"]
                .iter()
                .any(|d| out.starts_with(d));
            assert!(
                !opens,
                "{source:?} at {level} gained a Use Strict Directive: {out}"
            );
        }
    }
}

#[test]
fn compiled_output_prints_what_the_source_prints() {
    for source in all_sources() {
        let Some(source_says) = node_stdout(&source) else {
            eprintln!("node not on PATH; skipping the runtime comparison");
            return;
        };
        for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
            let compiled = compile(&source, level);
            let compiled_says = node_stdout(&compiled).expect("node ran a moment ago");
            assert_eq!(
                compiled_says, source_says,
                "{source:?} at {level}: {compiled}"
            );
        }
    }
}
