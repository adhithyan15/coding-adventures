//! Python closures that **name themselves or each other**, on every backend.
//!
//! A nested `def` that reads a variable of its enclosing function is lifted
//! to a top-level SIR function carrying that variable as a *capture*. Every
//! closure built for it (`MakeClosure`) has to thread the same captures.
//!
//! Issue #18468: the frontend used to record a nested function's capture list
//! only *after* lowering its body. So a body that named its own function saw
//! no captures and built a zero-capture closure for a one-capture function.
//! The C backend then read past the end of the closure's capture array (a NULL
//! `caps[0]`). A *call* by name was worse on every backend: it lowered to a
//! `DirectCall`, which passes the arguments but never the captures.
//!
//! Each program below is checked two ways:
//!
//! 1. **Structurally:** after lowering, every `MakeClosure` threads exactly as
//!    many captures as its target function declares. No `DirectCall` targets a
//!    function with captures. (The validator does not enforce the first rule
//!    yet, so this test does.)
//! 2. **Behaviourally:** the program prints the value CPython prints, on every
//!    backend whose toolchain is present.
//!
//! Each program makes a single `print` call. Repeated `print`s do not separate
//! lines on the C/Ruby backends, a known gap (see `conformance.rs`).

use semantic_ir::{Expr, Module, Visitor};
use sir_conformance::{lower_source_via, run_source_via, Frontend, RunOutcome, Target};

/// `(name, Python source, CPython's stdout)`.
const PROGRAMS: &[(&str, &str, &str)] = &[
    // The program from the issue: `go` names itself as a value (`h = go`)
    // while capturing `n`. `outer` then calls it by name.
    (
        "py_nested_self_value",
        "def outer(n):\n    def go(x):\n        h = go\n        return x + n\n    return go(1)\n\nprint(outer(5))\n",
        "6",
    ),
    // The self-reference is called: `h` is `go`'s own closure, rebuilt
    // inside `go` from the captures `go` was itself built with.
    // go(3) = 2 * go(2) = 4 * go(1) = 8 * go(0) = 8 * n = 40.
    (
        "py_nested_self_value_called",
        "def outer(n):\n    def go(x):\n        h = go\n        if x == 0:\n            return n\n        else:\n            return h(x - 1) * 2\n    return go(3)\n\nprint(outer(5))\n",
        "40",
    ),
    // Direct recursion through the name. go(3) = n + 3 = 8.
    (
        "py_nested_recursion",
        "def outer(n):\n    def go(x):\n        if x == 0:\n            return n\n        else:\n            return go(x - 1) + 1\n    return go(3)\n\nprint(outer(5))\n",
        "8",
    ),
    // Two sibling nested defs calling each other. `ev` refers to `od`
    // before `od`'s def is reached, which Python allows: the name is
    // looked up when `ev` runs. ev(4) -> od(3) -> ev(2) -> od(1) -> ev(0) = n.
    (
        "py_nested_mutual_recursion",
        "def outer(n):\n    def ev(x):\n        if x == 0:\n            return n\n        else:\n            return od(x - 1)\n    def od(x):\n        if x == 0:\n            return 0 - n\n        else:\n            return ev(x - 1)\n    return ev(4)\n\nprint(outer(5))\n",
        "5",
    ),
    // `inner` calls its uncle `go`, which captures `n`. Building `go`'s
    // closure inside `inner` needs `n`, so `inner` and `mid` both capture
    // it, though neither reads `n` itself.
    (
        "py_nested_uncle_call",
        "def outer(n):\n    def go(z):\n        return z + n\n    def mid(y):\n        def inner(z):\n            return go(z)\n        return inner(y)\n    return mid(1)\n\nprint(outer(10))\n",
        "11",
    ),
    // A top-level def that reads a `main` local captures it (top-level
    // statements are `main`'s body). Calling it by name has the same
    // problem as calling a nested def.
    (
        "py_toplevel_def_reads_main_local",
        "x = 5\ndef f():\n    return x\n\nprint(f())\n",
        "5",
    ),
];

/// Collects every closure construction and every direct call in a module.
#[derive(Default)]
struct Calls {
    /// `(target, number of captures threaded)` for each `MakeClosure`.
    closures: Vec<(String, usize)>,
    /// Target of each `DirectCall`.
    direct: Vec<String>,
}

impl Visitor for Calls {
    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        match e {
            Expr::MakeClosure {
                fn_name, captures, ..
            } => self.closures.push((fn_name.clone(), captures.len())),
            Expr::DirectCall { fn_name, .. } => self.direct.push(fn_name.clone()),
            _ => {}
        }
        semantic_ir::walker::walk_expr_default(self, e, depth);
    }
}

/// The number of captures `name` declares.
fn declared_captures(m: &Module, name: &str) -> usize {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("closure target `{name}` is not a function of the module"))
        .captures
        .len()
}

/// Every program lowers to SIR whose closures agree with their targets.
#[test]
fn closures_thread_exactly_their_targets_captures() {
    for (name, source, _) in PROGRAMS {
        let module = lower_source_via(name, source, Frontend::Python)
            .unwrap_or_else(|e| panic!("`{name}` failed to lower: {e}"));
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "`{name}` failed validation: {:?}",
            report.issues
        );

        let mut calls = Calls::default();
        calls.visit_module(&module);
        for (target, threaded) in &calls.closures {
            assert_eq!(
                *threaded,
                declared_captures(&module, target),
                "`{name}`: MakeClosure({target}) threads {threaded} capture(s), \
                 but `{target}` declares {}",
                declared_captures(&module, target),
            );
        }
        for target in &calls.direct {
            assert_eq!(
                declared_captures(&module, target),
                0,
                "`{name}`: DirectCall to `{target}`, which has captures \
                 (a direct call cannot pass them)",
            );
        }
    }
}

/// Every program prints CPython's output on every available backend.
#[test]
fn programs_print_cpythons_output_on_every_backend() {
    let mut ran = 0usize;
    for (name, source, expected) in PROGRAMS {
        for &target in Target::all() {
            match run_source_via(name, source, Frontend::Python, target) {
                RunOutcome::Ran(stdout) => {
                    assert_eq!(
                        stdout,
                        *expected,
                        "`{name}` on the {} backend printed {stdout:?}, expected {expected:?}",
                        target.tag(),
                    );
                    ran += 1;
                }
                RunOutcome::Skipped(why) => eprintln!("skip {name}/{}: {why}", target.tag()),
                RunOutcome::Failed(msg) => {
                    panic!("`{name}` failed on the {} backend:\n{msg}", target.tag())
                }
            }
        }
    }
    eprintln!("python nested closures: {ran} program/backend runs");
}
