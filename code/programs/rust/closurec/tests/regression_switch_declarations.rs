//! Regression test for #17879: DCE keeps the declarations a `switch` body
//! shares across its cases.
//!
//! ## The miscompile
//!
//! A switch body is one scope. A `var` in any case is hoisted to the
//! enclosing function, and a `function`/`class` in any case is visible to
//! every case. `closure-pass-dce` handled each case as if it stood alone:
//!
//! - The drop-after-terminator loop truncated a case's dead tail without the
//!   hoisting guard (`tail_is_safe_to_truncate`) that the block path uses.
//! - The constant-discriminant collapse replaced the switch with the matched
//!   case and threw the other cases away, declarations included.
//!
//! Either way the function's local `x` disappeared, and `typeof x` read the
//! global:
//!
//! ```text
//! var x='glob'; function f(){switch(2){case 1: var x=1; break; case 2: return typeof x}}
//!   source: "undefined"     before: "string"
//! ```
//!
//! ## What this test checks
//!
//! Each case compiles at SIMPLE and ADVANCED. When `node` is on `PATH`, the
//! output must print exactly what the source prints. The `function`/`class`
//! forms of this bug also need #17877, which makes a declaration in a case
//! clause a declaration; the DCE unit tests cover them on the AST directly.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[&str] = &[
    // A dead `var` after the case's `return` is still the function's local.
    "var x='glob'; function f(){switch(1){case 1: return typeof x; var x=1}} console.log(f())",
    // A `var` in a case the constant switch never selects.
    "var x='glob'; function f(){switch(2){case 1: var x=1; break; case 2: return typeof x}} console.log(f())",
    // No case matches at all; the `var` still hoists.
    "var x='glob'; function f(){switch(3){case 1: var x=1}; return typeof x} console.log(f())",
    // A dead `var` after `break` keeps an assignment local.
    "var x='glob'; function f(){switch(1){case 1: x=2; break; var x}; return x} console.log(f(), x)",
    // Ordinary switches still compile.
    "function f(k){switch(k){case 1: return 'a'; case 2: return 'b'; default: return 'c'}} console.log(f(2))",
];

fn compile(source: &str, level: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-switch-decl-{}-{n}-{}",
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
        "{level} {source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// Run `program` under node: `(succeeded, stdout, stderr)`, or `None`
/// without node.
fn run_node(program: &str) -> Option<(bool, String, String)> {
    let run = Command::new("node").args(["-e", program]).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    ))
}

#[test]
fn switch_declarations_print_what_the_source_prints() {
    for source in CASES {
        let outputs: Vec<(&str, String)> =
            ["SIMPLE", "ADVANCED"].iter().map(|l| (*l, compile(source, l))).collect();
        let Some((ok, expected, err)) = run_node(source) else {
            eprintln!("node not on PATH; skipping the runtime check");
            return;
        };
        assert!(ok, "source {source:?} must run: {err}");
        for (level, out) in outputs {
            let (ok, got, err) = run_node(&out).expect("node ran once already");
            assert!(ok, "{level} {source:?} -> {out:?} throws: {err}");
            assert_eq!(got, expected, "{level} {source:?} -> {out:?}");
        }
    }
}
