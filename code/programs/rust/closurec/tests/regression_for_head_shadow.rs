//! Regression test for #16008: a `for`-head `let`/`const` that shadows an
//! outer binding must not get the outer binding deleted.
//!
//! ## The miscompile
//!
//! ```js
//! var x = 1;
//! for (let x of [7]) console.log(x);   // prints 7 — the loop's x
//! console.log(x);                       // prints 1 — the outer x
//! ```
//!
//! At ADVANCED, closurec used to emit
//! `for(let x of [7])console.log(x);console.log(x);` — the outer `var x = 1;`
//! was gone, so the last line threw `ReferenceError: x is not defined`.
//!
//! The scope analyzer registered the head's `let x` in the *enclosing* scope,
//! right beside the outer `var x`. The read after the loop resolved to the
//! loop binding, the outer `var` looked unreferenced, and remove-unused-vars
//! collected it. The analyzer now gives a lexical `for` head its own loop
//! scope (see `loop_scope_ctx` in `closure-scope-analyzer`).
//!
//! ## What each case pins
//!
//! The inputs under `tests/regression/for-head-shadow/input/` are the matrix from
//! the issue: `for` / `for-in` / `for-of`, `let` / `const` heads, `var` /
//! `const` outer bindings, plus the same shape inside a function body. For
//! each one, at SIMPLE and ADVANCED:
//!
//! 1. the outer declaration survives in the output (this is what failed
//!    before the fix), and
//! 2. the output is byte-for-byte what closurec emits today, so a later pass
//!    that starts deleting or renaming here shows up as a diff.
//!
//! When `node` is on `PATH` the test also runs the source and the output and
//! compares what they print, which is the issue's own acceptance check
//! ("the output runs"). Without `node` that step is skipped; the structural
//! check in (1) is what guards CI.
//!
//! These are not ladder rungs or `tests/diff/` fixtures: both of those claim
//! upstream provenance (the oracle manifest classifies every `tests/diff/`
//! directory), and no upstream bytes were captured for this shape. The
//! expected outputs here are closurec's own, so the inputs live under
//! `tests/regression/`.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const INPUT_DIR: &str = "tests/regression/for-head-shadow/input";

/// `(fixture stem, the outer declaration that must survive, expected output)`.
/// SIMPLE and ADVANCED produce the same bytes for every case today.
const CASES: &[(&str, &str, &str)] = &[
    (
        "for_of_let_var",
        "var x=1;",
        "var x=1;for(let x of [7])console.log(x);console.log(x);",
    ),
    (
        "for_of_const_var",
        "var x=1;",
        "var x=1;for(const x of [7])console.log(x);console.log(x);",
    ),
    (
        "for_in_let_var",
        "var x=1;",
        "var x=1;for(let x in {a:1})console.log(x);console.log(x);",
    ),
    (
        "for_let_var",
        "var x=1;",
        "var x=1;for(let x=0;x<1;x++)console.log(x);console.log(x);",
    ),
    (
        "for_of_let_const",
        "const x=1;",
        "const x=1;for(let x of [7])console.log(x);console.log(x);",
    ),
    (
        "in_function",
        "var x=1;",
        "function f(){var x=1;for(let x of [7])console.log(x);console.log(x)}f();",
    ),
];

const LEVELS: &[&str] = &["SIMPLE", "ADVANCED"];

fn compile(input: &Path, level: &str) -> String {
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(input)
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "{} at {level}: exit {:?}, stderr: {}",
        input.display(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("closurec output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

#[test]
fn outer_binding_shadowed_by_for_head_survives() {
    for (stem, outer_decl, expected) in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        for level in LEVELS {
            let actual = compile(&input, level);
            assert!(
                actual.contains(outer_decl),
                "{stem} at {level}: the outer `{outer_decl}` was deleted, so the read \
                 after the loop throws ReferenceError (#16008). Output: {actual}",
            );
            assert_eq!(&actual, expected, "{stem} at {level}: output changed");
        }
    }
}

/// Run `node` on a file or on `-e <code>` and return what it printed, or
/// `None` when `node` is not installed.
fn node_stdout(args: &[&str]) -> Option<String> {
    let out = Command::new("node").args(args).output().ok()?;
    assert!(
        out.status.success(),
        "node {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn compiled_output_prints_what_the_source_prints() {
    for (stem, _, _) in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        let Some(source_says) = node_stdout(&[input.to_str().expect("utf-8 path")]) else {
            eprintln!("node not on PATH; skipping the runtime comparison");
            return;
        };
        for level in LEVELS {
            let compiled = compile(&input, level);
            let compiled_says = node_stdout(&["-e", &compiled]).expect("node ran a moment ago");
            assert_eq!(compiled_says, source_says, "{stem} at {level} changed behaviour");
        }
    }
}
