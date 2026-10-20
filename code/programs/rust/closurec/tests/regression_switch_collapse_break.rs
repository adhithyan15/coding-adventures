//! Regression test for #17842: collapsing a constant `switch` must not
//! retarget a nested `break`.
//!
//! ```js
//! while (c--) { switch (1) { case 1: if (x) break; b(); break; } d(); }
//! ```
//!
//! The inner `break` exits the `switch`. closurec's DCE collapsed the switch
//! into its case body, so the `break` exited the `while` instead and `d()`
//! never ran. Outside a loop the collapsed output didn't even parse. The
//! collapse is now declined while any non-trailing `break` still targets the
//! switch (`has_break_targeting_enclosing`).
//!
//! Upstream rewrites the case (`x||b()`) instead of keeping the switch. That
//! is smaller output closurec doesn't produce yet, but both run the same.
//! When `node` is on `PATH`, the test checks that the output prints what the
//! source prints.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const INPUT_DIR: &str = "tests/regression/switch-collapse-break/input";
const CASES: &[&str] = &["break_in_loop", "break_in_function"];

fn compile(input: &Path, level: &str) -> String {
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(input)
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "{} at {level}: {}",
        input.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// Run `node` and return what it printed, or `None` when it isn't installed.
fn node_stdout(args: &[&str]) -> Option<String> {
    let out = Command::new("node").args(args).output().ok()?;
    assert!(
        out.status.success(),
        "node {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn nested_break_keeps_its_switch() {
    for stem in CASES {
        let input = Path::new(INPUT_DIR).join(format!("{stem}.js"));
        let source_says = node_stdout(&[input.to_str().expect("utf-8 path")]);
        for level in ["SIMPLE", "ADVANCED"] {
            let compiled = compile(&input, level);
            // At SIMPLE `x` is open-world, so the switch must survive. At
            // ADVANCED `x` is propagated and the whole switch may legitimately
            // fold away. The runtime check below covers both.
            if level == "SIMPLE" {
                assert!(compiled.contains("switch"), "{stem} at {level}: switch collapsed: {compiled}");
            }
            if let Some(expected) = &source_says {
                let got = node_stdout(&["-e", &compiled]).expect("node ran a moment ago");
                assert_eq!(&got, expected, "{stem} at {level} changed behaviour: {compiled}");
            }
        }
    }
}
