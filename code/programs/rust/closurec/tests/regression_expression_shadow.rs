//! Regression test: a binding inside a function expression, arrow, class
//! expression or catch clause shadows a top-level `const`, so the const's
//! value must not be planted there (found in the security review of CCR-078,
//! #15863).
//!
//! ```js
//! const T = true;
//! (function (T) { if (T) console.log("a"); else console.log("b"); })(false);
//! // prints b; closurec used to emit `(function(T){console.log("a")})(!1)`
//! ```
//!
//! `closure-pass-inline-variables` counted declarations only at statement
//! level, so these expression-scoped bindings were invisible to its shadow
//! guard and every `T` was taken for the top-level one. It now counts binding
//! sites over the serialized AST (`count_binding_sites`).
//!
//! Each input under `tests/regression/expression-shadow/input/` is compiled
//! at SIMPLE and ADVANCED. The test checks the output still declares the
//! shadowing binding. When `node` is on `PATH`, it also checks that the
//! output prints what the source prints.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const INPUT_DIR: &str = "tests/regression/expression-shadow/input";

/// `(fixture stem, text the output must keep: the shadowing binding)`.
const CASES: &[(&str, &str)] = &[
    ("function_param", "function(T)"),
    ("function_local_var", "var T=!1"),
    ("arrow_param", "T=>"),
    ("named_function_expression", "function T()"),
    ("named_class_expression", "class T"),
    ("catch_param", "catch(T)"),
];

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
    String::from_utf8(out.stdout).expect("closurec output is UTF-8")
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
fn shadowing_binding_survives_and_behaviour_is_unchanged() {
    for (stem, kept) in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        let source_says = node_stdout(&[input.to_str().expect("utf-8 path")]);
        for level in ["SIMPLE", "ADVANCED"] {
            let compiled = compile(&input, level);
            assert!(
                compiled.contains(kept),
                "{stem} at {level}: lost the shadowing binding `{kept}`: {compiled}",
            );
            if let Some(expected) = &source_says {
                let got = node_stdout(&["-e", &compiled]).expect("node ran a moment ago");
                assert_eq!(&got, expected, "{stem} at {level} changed behaviour: {compiled}");
            }
        }
    }
}
