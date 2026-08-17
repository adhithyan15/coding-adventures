//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the TypeScript backend — hand-builds a module calling
//! each op directly (bypassing the frontend, since no frontend emits
//! these names yet), runs it with `node`, and asserts stdout/exit status.
//!
//! As with every other node proof in this crate (see `run_with_node.rs`'s
//! module doc-comment), the TypeScript backend emits genuine TypeScript
//! that bare `node` cannot parse, so we swap the `@coding-adventures/
//! sir-runtime-core`/`sir-runtime-exceptions` imports for inline stubs
//! and strip the fixed set of type annotations the emitter adds. The
//! stubs TRANSCRIBE the real `arithmetic.ts`/`runtime.ts`/exceptions
//! logic (not a fake shape check), so this proof genuinely exercises the
//! division behavior, not just that the emitted call site is present.
//! Skips gracefully when no `node` is on `PATH`.
//!
//! **`div_floor` is NOT Ruby-floor-faithful here — a documented, unfixed
//! limitation.** Every sibling backend's `div_floor` either aliases
//! already-floor-faithful logic, or (for the closest sibling,
//! `semantic-ir-to-javascript`) uses a boxed-float runtime tag to
//! dispatch floor-vs-true-divide correctly. This runtime's `Val` has NO
//! such tag (see `arithmetic.ts`'s `div` doc comment for the full
//! writeup), so `div_floor` is a bare alias for the pre-existing `div`,
//! which TRUNCATES instead of floors. The test below asserts the ACTUAL
//! (truncating) behavior, not the Ruby-correct one — this is deliberate,
//! not an oversight; flip it only alongside the value-tagging work that
//! doc comment describes as a prerequisite. `div_trunc`/`udiv_trunc`/
//! `div_true` have no such gap (see their own doc comments for why) and
//! are asserted against their fully-correct values.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    RescueClause, Span, Stmt,
};
use semantic_ir_to_typescript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    call(name, vec![a, b])
}
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
                expr,
            ],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

fn div_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "divprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::Floats,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// A minimal `__SirExc` runtime stub, mirroring `run_with_node.rs`'s own
/// `SIR_EXC_STUB` (kept local to this file rather than shared, since the
/// two test binaries don't share a module tree).
const SIR_EXC_STUB: &str = r#"const __SirExc = (() => {
  const ANCESTRY = { ZeroDivisionError: "StandardError", StandardError: "Exception" };
  class SirError extends Error {
    constructor(sirClass, message) {
      super(message == null ? sirClass : String(message));
      this.sirClass = sirClass;
    }
  }
  const raiseError = (c, m) => { throw new SirError(c ?? "RuntimeError", m); };
  const classOfThrown = (e) => (e instanceof SirError ? e.sirClass : "StandardError");
  const rescueMatches = (e, names) => {
    if (names.length === 0) return true;
    const actual = classOfThrown(e);
    let cur = actual;
    const seen = new Set();
    while (cur !== undefined && !seen.has(cur)) {
      if (names.includes(cur)) return true;
      seen.add(cur);
      cur = ANCESTRY[cur];
    }
    return false;
  };
  return { raiseError, rescueMatches, classOfThrown };
})();
"#;

/// A `__Sir` stub carrying `write` (for `print_stmt`'s output) plus the
/// SIR21 T3b-2 division family, TRANSCRIBING `arithmetic.ts`'s real
/// `div`/`truncDiv`/`trueDiv` (see their doc comments there for why
/// `div` truncates — a documented, unfixed limitation, not a stub
/// shortcut).
const SIR_DIV_STUB: &str = r#"const __Sir = {
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  write: (stream, terminator, unpackArrays, ...values) => {
    const out = stream === "stderr" ? process.stderr : process.stdout;
    if (terminator === "once") {
      out.write(values.map((v) => __Sir.toDisplay(v)).join(" ") + "\n");
      return null;
    }
    for (const v of values) { out.write(__Sir.toDisplay(v)); }
    return null;
  },
  div: (...args) => {
    if (args.length === 0) return 0;
    let acc = args[0];
    for (let i = 1; i < args.length; i++) {
      const d = args[i];
      if (d === 0) __SirExc.raiseError("ZeroDivisionError", "divided by 0");
      acc = Math.trunc(acc / d);
    }
    return acc;
  },
  truncDiv: (a, b) => {
    if (b === 0) __SirExc.raiseError("ZeroDivisionError", "divided by 0");
    return Math.trunc(a / b);
  },
  trueDiv: (a, b) => {
    if (b === 0) __SirExc.raiseError("ZeroDivisionError", "divided by 0");
    return a / b;
  },
};
"#;

fn ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_DIV_STUB,
    );
    js = js.replace(
        "import * as __SirExc from \"@coding-adventures/sir-runtime-exceptions\";\n",
        SIR_EXC_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(" as __Sir.Val[]", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Compile a `div_module`, transform to JS, run under `node`, and return
/// trimmed stdout, or `None` to skip when no usable `node` is present.
fn run(module: &Module, tag: &str) -> Option<String> {
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let artifact = compile(module).expect("compile to typescript");
    let js = ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_div_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        js,
    );
    Some(
        String::from_utf8(output.stdout)
            .expect("utf8 stdout")
            .replace("\r\n", "\n"),
    )
}

// ── div_floor: DOCUMENTED to truncate, not floor (see module doc) ────────

#[test]
fn div_floor_currently_truncates_not_floors_documented_limitation() {
    let m = div_module(vec![
        print_stmt(bin("div_floor", ilit(7), ilit(2))),
        // Ruby-correct would be -4 (floor); this runtime gives -3
        // (truncate) — see the module doc comment and `div`'s own doc
        // comment in `arithmetic.ts` for why.
        print_stmt(bin("div_floor", ilit(-7), ilit(2))),
    ]);
    if let Some(out) = run(&m, "floor") {
        assert_eq!(out, "3\n-3\n");
    }
}

// ── div_trunc/udiv_trunc: fully correct — truncation needs no int/float tag ──

#[test]
fn div_trunc_truncates_toward_zero() {
    let m = div_module(vec![
        print_stmt(bin("div_trunc", ilit(7), ilit(2))),
        print_stmt(bin("div_trunc", ilit(-7), ilit(2))),
        print_stmt(bin("div_trunc", ilit(7), ilit(-2))),
        print_stmt(bin("div_trunc", ilit(-7), ilit(-2))),
    ]);
    if let Some(out) = run(&m, "trunc") {
        assert_eq!(out, "3\n-3\n-3\n3\n");
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    let m = div_module(vec![print_stmt(bin("udiv_trunc", ilit(7), ilit(2)))]);
    if let Some(out) = run(&m, "udiv") {
        assert_eq!(out, "3\n");
    }
}

// ── div_true: fully correct — always true-divides, no tag needed ─────────

#[test]
fn div_true_always_true_divides_even_on_integer_operands() {
    let m = div_module(vec![
        print_stmt(bin("div_true", ilit(7), ilit(2))),
        print_stmt(bin("div_true", ilit(-7), ilit(2))),
        // No boxed-float tag in this runtime (see module doc comment), so
        // an integral float result prints as a bare "2", not "2.0" —
        // unlike every sibling backend. A real, documented divergence,
        // not a test bug.
        print_stmt(bin("div_true", ilit(6), ilit(3))),
    ]);
    if let Some(out) = run(&m, "true") {
        assert_eq!(out, "3.5\n-3.5\n2\n");
    }
}

// ── zero-divisor: proves the fault raises AND is rescue-catchable ────────

fn rescue(types: &[&str], binding: Option<&str>, body: Vec<Stmt>) -> RescueClause {
    RescueClause {
        exception_types: types.iter().map(|t| (*t).to_string()).collect(),
        binding: binding.map(|b| b.to_string()),
        body,
        span: s(),
    }
}

/// Wrap `fault` in `begin <fault>; rescue ZeroDivisionError => e; print
/// marker; end`, run it, and assert the process exits 0 printing only the
/// marker — proving the op both RAISES the typed error and that a Ruby
/// `rescue ZeroDivisionError` catches it (matches `run_with_node.rs`'s T2
/// `assert_typed_rescue_catches`-style pattern, rather than depending on
/// Node's uncaught-exception stderr formatting).
fn assert_zero_divisor_raises_and_is_caught(op: &str) {
    let fault = bin(op, ilit(7), ilit(0));
    let tc = Stmt::TryCatch {
        body: vec![print_stmt(fault)],
        rescues: vec![rescue(&["ZeroDivisionError"], Some("e"), vec![print_stmt(ilit(1))])],
        ensure_body: None,
        span: s(),
    };
    let m = Module {
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::Floats,
            Feature::Exceptions,
        ]),
        ..div_module(vec![tc])
    };
    if let Some(out) = run(&m, op) {
        assert_eq!(out, "1\n", "[{op}] expected only the rescue marker; got {out:?}");
    }
}

#[test]
fn zero_divisor_raises_zero_division_error_for_every_op() {
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        assert_zero_divisor_raises_and_is_caught(op);
    }
}
