//! Regression test for #17836: a `let`/`const` inside a `switch` case that
//! shadows an outer binding must not get the outer binding deleted.
//!
//! ## The miscompile
//!
//! ```js
//! var x = 1;
//! switch (1) { case 1: let x = 2; console.log(x); }   // prints 2
//! console.log(x);                                      // prints 1
//! ```
//!
//! At ADVANCED, closurec used to emit
//! `switch(1){case 1:let x=2;console.log(x)}console.log(x);` — the outer
//! `var x = 1;` was gone and the last line threw `ReferenceError`.
//!
//! The scope analyzer walked the case block in the *enclosing* scope, so the
//! case's `let x` sat beside the outer `var x`, the read after the switch
//! resolved to it, and remove-unused-vars collected the outer `var`. The case
//! block now gets its own Block scope (ECMAScript §14.12.4), the same shape as
//! the #16008 loop-scope fix (`regression_for_head_shadow.rs`).
//!
//! For each input under `tests/regression/switch-case-shadow/input/`, at SIMPLE
//! and ADVANCED, the outer declaration survives and the output bytes are
//! pinned. When `node` is on `PATH` the test also checks the output prints what
//! the source prints. The expected bytes are closurec's own (no upstream
//! capture), so these inputs live outside `tests/diff/`.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const INPUT_DIR: &str = "tests/regression/switch-case-shadow/input";

/// `(fixture stem, the outer declaration that must survive, expected output)`.
/// SIMPLE and ADVANCED produce the same bytes for every case today.
const CASES: &[(&str, &str, &str)] = &[
    (
        "let_var",
        "var x=1;",
        "var x=1;switch(1){case 1:let x=2;console.log(x)}console.log(x);",
    ),
    (
        "const_var",
        "var x=1;",
        "var x=1;switch(1){case 1:const x=2;console.log(x)}console.log(x);",
    ),
    (
        "let_const_default",
        "const x=1;",
        "const x=1;switch(2){case 1:let x=2;console.log(x);break;default:console.log(\"d\")}console.log(x);",
    ),
    (
        "in_function",
        "var x=1;",
        "function f(){var x=1;switch(1){case 1:let x=2;console.log(x)}console.log(x)}f();",
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
fn outer_binding_shadowed_in_a_switch_case_survives() {
    for (stem, outer_decl, expected) in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        for level in LEVELS {
            let actual = compile(&input, level);
            assert!(
                actual.contains(outer_decl),
                "{stem} at {level}: the outer `{outer_decl}` was deleted, so the read \
                 after the switch throws ReferenceError (#17836). Output: {actual}",
            );
            assert_eq!(&actual, expected, "{stem} at {level}: output changed");
        }
    }
}

/// Run `node` and return what it printed, or `None` when it isn't installed.
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
