//! Regression test for #17841: a `function` or `var` declared after a
//! `return` still hoists, so the dead-after-return drop must keep it.
//!
//! ```js
//! function f() { return h(); function h() { return 1; } }   // prints 1
//! function g() { x = 3; return x; var x; }                 // leaves global x alone
//! ```
//!
//! `closure-pass-fold-control-flow` dropped everything after a `return`, with
//! no hoisting guard. The first program then threw `ReferenceError: h is not
//! defined`. In the second, `g`'s local `x` became the global, so the program
//! printed 3 instead of 7. The drop now uses the same whitelist as
//! `closure-pass-dce` (`dead_tail_is_safe_to_drop`).
//!
//! For each input under `tests/regression/hoisted-after-return/input/`, at
//! SIMPLE and ADVANCED, the declaration survives and the output bytes are
//! pinned. When `node` is on `PATH`, the test also checks that the output
//! prints what the source prints. The expected bytes are closurec's own,
//! since no upstream output was captured.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const INPUT_DIR: &str = "tests/regression/hoisted-after-return/input";

/// `(fixture stem, the declaration that must survive, expected output)`.
/// SIMPLE and ADVANCED produce the same bytes for every case today.
const CASES: &[(&str, &str, &str)] = &[
    (
        "function_after_return",
        "function h(){return 1}",
        "function f(){return h();function h(){return 1}}console.log(f());",
    ),
    (
        "var_after_return",
        "var x}",
        "function g(){x=3;return x;var x}var x=7;g();console.log(x);",
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
fn declaration_after_return_survives() {
    for (stem, kept, expected) in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        for level in LEVELS {
            let actual = compile(&input, level);
            assert!(
                actual.contains(kept),
                "{stem} at {level}: `{kept}` was dropped after the return (#17841). Output: {actual}",
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
