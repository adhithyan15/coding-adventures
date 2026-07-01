//! End-to-end round-trip tests: JavaScript → SIR → JavaScript → `node`.
//!
//! Each test lowers a golden JS program to SIR with this crate, validates
//! the module, emits JavaScript with the merged `semantic-ir-to-javascript`
//! backend (a dev-dependency), writes it to a temp file, and **executes it
//! with `node`**, asserting the printed output.  This is the strongest
//! confirmation that the M4 lowering is faithful: the program runs.
//!
//! The tests are **gated on `node` availability** — if `node` is not on
//! `PATH` (CI image without Node, say) they no-op with a notice rather than
//! failing, so the suite stays green everywhere while still exercising the
//! real pipeline where Node exists.

use std::process::Command;

use javascript_to_semantic_ir::compile_source;

/// `true` iff a `node` interpreter is runnable on this machine.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lower `src` → SIR → JS, run it through `node`, and return trimmed stdout.
/// Panics (failing the test) on any lowering/validation/backend/runtime
/// error so a regression is loud.
fn run_via_node(name: &str, src: &str) -> String {
    let module = compile_source(src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact = semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    // Write to a unique temp file and execute it.
    let mut path = std::env::temp_dir();
    path.push(format!("sir19_e2e_{name}_{}.js", std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn factorial_runs_in_node() {
    if !node_available() {
        eprintln!("skipping factorial_runs_in_node: `node` not available");
        return;
    }
    // Tail if/else recursion — no early return.
    let out = run_via_node(
        "factorial",
        "function fact(n) { if (n <= 1) { return 1; } else { return n * fact(n - 1); } } \
         console.log(fact(5));",
    );
    assert_eq!(out, "120");
}

#[test]
fn fibonacci_runs_in_node() {
    if !node_available() {
        eprintln!("skipping fibonacci_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "fibonacci",
        "function fib(n) { if (n < 2) { return n; } else { return fib(n - 1) + fib(n - 2); } } \
         console.log(fib(10));",
    );
    assert_eq!(out, "55");
}

#[test]
fn closure_adder_runs_in_node() {
    if !node_available() {
        eprintln!("skipping closure_adder_runs_in_node: `node` not available");
        return;
    }
    // A closure capturing `x`, applied indirectly.
    let out = run_via_node(
        "closure_adder",
        "function makeAdder(x) { return (y) => x + y; } \
         let add5 = makeAdder(5); console.log(add5(3));",
    );
    assert_eq!(out, "8");
}

#[test]
fn array_sum_runs_in_node() {
    if !node_available() {
        eprintln!("skipping array_sum_runs_in_node: `node` not available");
        return;
    }
    // M5 golden: build an array, index it, mutate an element, sum it with a
    // counting `for` over `xs.length`.  Exercises SeqLit / SeqIndex / SeqLen
    // / SeqSet end to end.
    let out = run_via_node(
        "array_sum",
        "let xs = [1, 2, 3, 4]; \
         xs[0] = 10; \
         let total = 0; \
         for (let i = 0; i < xs.length; i++) { total = total + xs[i]; } \
         console.log(total);",
    );
    // [10, 2, 3, 4] sums to 19.
    assert_eq!(out, "19");
}

#[test]
fn object_get_set_runs_in_node() {
    if !node_available() {
        eprintln!("skipping object_get_set_runs_in_node: `node` not available");
        return;
    }
    // M5 golden: build an object, read a property, set a property, read the
    // updated value.  Exercises MapLit / MapGet / MapSet end to end.  Note
    // we print individual values, not the Map itself (which `console.log`
    // would render as `Map { … }`).
    let out = run_via_node(
        "object_get_set",
        "let acct = {balance: 100, owner: \"ada\"}; \
         acct.balance = acct.balance + 50; \
         acct[\"bonus\"] = 5; \
         console.log(acct.balance + acct[\"bonus\"]);",
    );
    // 100 + 50 + 5 = 155.
    assert_eq!(out, "155");
}

#[test]
fn default_params_run_in_node() {
    if !node_available() {
        eprintln!("skipping default_params_run_in_node: `node` not available");
        return;
    }
    // A default that references an earlier param (`b = a + 1`), exercised
    // both by a *partial* call that omits `b` (`f(5)` → default → 6) and a
    // full call that supplies it (`f(5, 10)` → 10).  Running this under node
    // proves the call-time + param-scope semantics end to end: the default
    // is evaluated at the call site against the actual `a`.
    let out = run_via_node(
        "default_params",
        "function f(a, b = a + 1) { return b; }\n\
         console.log(f(5));\n\
         console.log(f(5, 10));",
    );
    assert_eq!(out, "6\n10");
}

#[test]
fn mutual_recursion_runs_in_node() {
    if !node_available() {
        eprintln!("skipping mutual_recursion_runs_in_node: `node` not available");
        return;
    }
    // isEven / isOdd reference each other (DirectCall both ways).
    let out = run_via_node(
        "mutual",
        "function isEven(n) { if (n === 0) { return true; } else { return isOdd(n - 1); } } \
         function isOdd(n) { if (n === 0) { return false; } else { return isEven(n - 1); } } \
         console.log(isEven(10));",
    );
    // SIR `print` renders booleans Lisp-style (`#t`/`#f`).
    assert_eq!(out, "#t");
}

// ── C3: collection-method dispatch (`__method__`) executes ─────────────
//
// These prove the frontend's `__method__` lowering runs end-to-end: the JS
// backend routes the dispatch envelope to the runtime's `callMethod`, which
// invokes the JS-native array/string method (unwrapping any `Closure`
// callback).  The example in the C3 brief — `xs.push(4)` then `xs.length`
// → `4` — is the canonical case: a mutating method call plus a length read.

#[test]
fn array_push_length_runs_in_node() {
    if !node_available() {
        eprintln!("skipping array_push_length_runs_in_node: `node` not available");
        return;
    }
    // `xs.push(4)` lowers to __method__ dispatch → runtime `callMethod`;
    // `xs.length` is a property read (SeqLen).  After the push the length
    // is 4.
    let out = run_via_node(
        "array_push_length",
        "const xs = [1, 2, 3]; xs.push(4); console.log(xs.length);",
    );
    assert_eq!(out, "4");
}

#[test]
fn array_map_reduce_runs_in_node() {
    if !node_available() {
        eprintln!("skipping array_map_reduce_runs_in_node: `node` not available");
        return;
    }
    // Higher-order dispatch: `.map` with an arrow callback (lowered to a
    // MakeClosure argument, unwrapped by the runtime), then `.reduce` with a
    // two-param accumulator and an initial value.  [1,2,3] → *2 → [2,4,6] →
    // sum → 12.
    let out = run_via_node(
        "array_map_reduce",
        "const xs = [1, 2, 3]; \
         const doubled = xs.map(x => x * 2); \
         const total = doubled.reduce((a, b) => a + b, 0); \
         console.log(total);",
    );
    assert_eq!(out, "12");
}

// ── SECURITY (C3): reflective-gadget method names are refused ─────────
//
// `recv.method(args…)` lowers to a `__method__` dispatch whose method name is
// attacker-controlled.  A handful of JavaScript member names are reflective
// gadgets that would turn the backend's `recv[name]` lookup into arbitrary
// code execution — most dangerously `constructor`, which yields the
// `Function` constructor:
//
//     function id(x){ return x; }
//     let xs = [1];
//     xs.map( id.constructor("return …evil…") );   // RCE
//
// The frontend now refuses to *lower* those names, so the dangerous `StrLit`
// never enters SIR (protecting every backend).  These tests assert the
// gadget program is rejected at compile time — it never reaches node.  No
// `node` gate: the rejection happens in the node-independent lowering pass.

#[test]
fn constructor_gadget_is_rejected_at_lowering() {
    // The C3-brief RCE gadget: `id.constructor("…")` must not lower.
    let src = "function id(x){ return x; } \
               let xs = [1]; \
               xs.map(id.constructor(\"return 1\"));";
    let err = compile_source(src, "prog")
        .expect_err("`constructor` method dispatch must be refused, not lowered");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("constructor"),
        "error should name the refused gadget, got: {msg}"
    );
}

#[test]
fn proto_gadget_is_rejected_at_lowering() {
    // `obj.__proto__(…)` — a prototype-chain escape hatch — is refused too.
    let src = "let obj = {}; obj.__proto__(1);";
    let err = compile_source(src, "prog")
        .expect_err("`__proto__` method dispatch must be refused, not lowered");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("__proto__"),
        "error should name the refused gadget, got: {msg}"
    );
}

#[test]
fn apply_gadget_is_rejected_at_lowering() {
    // `fn.apply(…)` re-binds `this`; also on the denylist.
    let src = "function f(){ return 1; } f.apply(null);";
    let err = compile_source(src, "prog")
        .expect_err("`apply` method dispatch must be refused, not lowered");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("apply"),
        "error should name the refused gadget, got: {msg}"
    );
}

#[test]
fn array_filter_includes_runs_in_node() {
    if !node_available() {
        eprintln!("skipping array_filter_includes_runs_in_node: `node` not available");
        return;
    }
    // `.filter` (closure callback) then `.includes` (value membership).
    // [1,2,3,4] keep >2 → [3,4]; includes(4) → true (printed `#t`).
    let out = run_via_node(
        "array_filter_includes",
        "const xs = [1, 2, 3, 4]; \
         const big = xs.filter(x => x > 2); \
         console.log(big.includes(4));",
    );
    assert_eq!(out, "#t");
}
