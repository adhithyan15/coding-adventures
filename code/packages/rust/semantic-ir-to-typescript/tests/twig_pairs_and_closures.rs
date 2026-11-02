//! # Twig pairs, function values and strict closures, run under `node`
//!
//! Twig source → `twig-to-semantic-ir` → this backend → `node`, for the
//! programs from issues #18460 and #18461:
//!
//! ```text
//! program                                                  before               now
//! (print (car (cons 1 2)))                                 refused: no `pairs`  1
//! (define (h x) (* x 3)) (define g h) (print (g 4))        "undefined global"   12
//! (define g (begin (lambda (x y) (+ x y)))) (print (g 4))  NaN                  arity error
//! ```
//!
//! Every `semantic-ir-to-*` backend carries a copy of this table, so a
//! program means the same thing whichever backend runs it.
//!
//! ## Running TypeScript without the npm packages
//!
//! Emitted TypeScript imports `@coding-adventures/sir-runtime-core` (and,
//! for a pair-using module, `@coding-adventures/sir-runtime-pairs`), npm
//! packages this Rust test cannot install.  As in `run_with_node.rs`, each
//! import line is swapped for an inline stub — [`SIR_STUB`] (`__Sir`) and
//! [`SIR_PAIRS_STUB`] (`__SirPairs`) — carrying exactly the helpers these
//! programs call, with the real packages' semantics, and the `: __Sir.Val`
//! annotations are stripped.  The arity check this file
//! cares about is emitted *inline* in the program, not in the runtime, so
//! the stub cannot mask it.  Node is optional: without it each case still
//! lowers and compiles, and only the execution check is skipped.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir_to_typescript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The `__Sir` helpers these programs reach, mirroring the real runtime.
const SIR_STUB: &str = r##"const __Sir = (() => {
  class Closure { constructor(fn) { this.fn = fn; } }
  const globals = new Map();
  const keyOf = (n) => (typeof n === "string" ? n : n.name);
  const toDisplay = (v) => (v === null ? "nil" : v === true ? "#t" : v === false ? "#f" : String(v));
  return {
    Closure,
    apply(c, args) {
      if (!(c instanceof Closure)) throw new TypeError("apply on non-closure");
      return c.fn(...args);
    },
    globalSet(name, value) { globals.set(keyOf(name), value); return value; },
    globalGet(name) {
      const k = keyOf(name);
      if (!globals.has(k)) throw new Error(`undefined global: ${k}`);
      return globals.get(k);
    },
    globalGetStatic(name) {
      if (!globals.has(name)) throw new Error(`undefined global: ${name}`);
      return globals.get(name);
    },
    intern(name) { return { name }; },
    truthy: (v) => v !== false && v !== null,
    add: (...a) => a.reduce((x, y) => x + y, 0),
    sub: (a, ...r) => (r.length === 0 ? -a : r.reduce((x, y) => x - y, a)),
    mul: (...a) => a.reduce((x, y) => x * y, 1),
    toDisplay,
    write(stream, terminator, unpackArrays, ...values) {
      process.stdout.write(values.map(toDisplay).join(""));
      return null;
    },
  };
})();
"##;

/// The `__SirPairs` helpers, mirroring `sir-runtime-pairs`.
const SIR_PAIRS_STUB: &str = r#"const __SirPairs = (() => {
  class Pair { constructor(car, cdr) { this.car = car; this.cdr = cdr; } }
  return {
    cons: (a, b) => new Pair(a, b),
    car: (p) => p.car,
    cdr: (p) => p.cdr,
    isPair: (v) => v instanceof Pair,
  };
})();
"#;

fn ts_to_runnable_js(ts: &str) -> String {
    ts.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_STUB,
    )
    .replace(
        "import * as __SirPairs from \"@coding-adventures/sir-runtime-pairs\";\n",
        SIR_PAIRS_STUB,
    )
    .replace(": __Sir.Val[]", "")
    .replace(": __Sir.Val", "")
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Lower and compile `src` — with **no** manifest patching, so a missing
/// feature declaration fails here — then run it under `node`.  `None` when
/// Node is unavailable.
fn run(src: &str, tag: &str) -> Option<Output> {
    let module = twig_to_semantic_ir::compile_source(src, "t")
        .unwrap_or_else(|e| panic!("lower `{src}`: {e}"));
    let artifact = compile(&module).unwrap_or_else(|e| panic!("compile `{src}`: {e:?}"));
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!(
        "sir_ts_twig_pc_{tag}_{}_{n}.js",
        std::process::id()
    ));
    std::fs::write(&path, ts_to_runnable_js(&artifact.source)).expect("write temp js");
    let out = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);
    Some(out)
}

/// What the emitted inline check throws when a strict closure is called
/// with the wrong number of arguments.
const ARITY_ERROR_TEXT: &str = "arity mismatch";

/// `(tag, program, expected stdout)` — programs that must run and print.
const PRINTS: &[(&str, &str, &str)] = &[
    // #18460: pair builtins pass validation and run.
    ("car", "(print (car (cons 1 2)))", "1"),
    ("cdr", "(print (cdr (cons 1 2)))", "2"),
    ("pair_yes", "(print (if (pair? (cons 1 2)) 1 0))", "1"),
    ("pair_no", "(print (if (pair? 5) 1 0))", "0"),
    // #18461 item 1: a lambda that appears only in callee position.
    (
        "callee_lambda",
        "(define (mk k) (lambda (a) (k a))) (print ((mk (lambda (a) 77)) 5))",
        "77",
    ),
    (
        "nested_callee_lambda",
        "(define (f g) ((lambda () ((lambda () (g 1)))))) (print (f (lambda (x) (+ x 41))))",
        "42",
    ),
    // #18461 item 2: a top-level function used as a value.
    (
        "fn_value",
        "(define (h x) (* x 3)) (define g h) (print (g 4))",
        "12",
    ),
    (
        "fn_argument",
        "(define (h x) (* x 3)) (define (app f v) (f v)) (print (app h 5))",
        "15",
    ),
    // #18461 item 5: a value define, read back.
    ("value_define", "(define x 5) (print x)", "5"),
];

/// `(tag, program)` — closures called with the wrong number of arguments
/// (#18461 item 4).  Each must stop with an arity error, not print.
const ARITY_ERRORS: &[(&str, &str)] = &[
    (
        "too_few",
        "(define g (begin (lambda (x y) (+ x y)))) (print (g 4))",
    ),
    ("too_many", "(print ((lambda (x) x) 1 2))"),
    (
        "fn_value_too_few",
        "(define (h x) x) (define g h) (print (g))",
    ),
];

/// Every row is checked and every failure collected before asserting, so one
/// run reports all the rows that fail — not just the first (see
/// `lessons.d/a-table-driven-test-that-stops-at-its-first-failing-case-cannot.md`).
#[test]
fn twig_programs_print_the_expected_values() {
    let mut failures = Vec::new();
    for (tag, src, want) in PRINTS {
        let Some(out) = run(src, tag) else { continue };
        let got = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            failures.push(format!("`{src}` exited non-zero:\n{stderr}"));
        } else if got.trim_end() != *want {
            failures.push(format!("`{src}` printed {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_closure_called_with_the_wrong_arity_raises() {
    let mut failures = Vec::new();
    for (tag, src) in ARITY_ERRORS {
        let Some(out) = run(src, tag) else { continue };
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() || !stderr.contains(ARITY_ERROR_TEXT) || !stdout.is_empty() {
            failures.push(format!(
                "`{src}` should stop with {ARITY_ERROR_TEXT:?}; exit {:?}, \
                 printed {stdout:?}, stderr:\n{stderr}",
                out.status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
