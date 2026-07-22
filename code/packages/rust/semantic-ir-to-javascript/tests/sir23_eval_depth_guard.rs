//! Empirical regression test for `Symbolic.evalTerm`'s own recursion-
//! depth guard (`MAX_EVAL_DEPTH`, `runtime.rs`) — SIR23 addendum item 1
//! ("`Symbolic.evalTerm` scaffold + arithmetic/comparison/logic
//! folding").
//!
//! `runtime.rs`'s own doc comment on `MAX_EVAL_DEPTH` records the full
//! measurement methodology and numbers; this file is the executable
//! proof that backs those numbers up, mirroring `derive-parser`'s own
//! `test_nesting_up_to_cap_still_parses` / `test_opt_in_cap_trips_
//! before_overflow_on_default_stack` discipline, retargeted from a Rust
//! worker-thread stack to a `node` subprocess's default V8 stack (this
//! guard runs in emitted JavaScript, not in this crate's own Rust code).
//!
//! Node is optional at test time; when unavailable every test here
//! degrades to a no-op rather than failing, mirroring every other
//! `node`-driven test in this crate (`tests/run_with_node.rs`, `tests/
//! sir23_symbolic.rs`).
//!
//! ## Why this calls `Symbolic.evalTerm` directly, bypassing the
//! compiled-program statement path
//!
//! `tests/sir23_symbolic.rs`'s own `print_on_deeply_nested_term_
//! truncates_instead_of_crashing_node` already proves the *statement-
//! level* wrapping (`emit.rs`'s `Stmt::ExprStmt` arm, this same
//! addendum item) round-trips through a real compiled SIR program. This
//! file is narrower and more direct: it measures/verifies `evalTerm`'s
//! *own* recursion behavior in isolation, so it builds the deep term and
//! calls `__Sir.Symbolic.evalTerm(...)` on it directly in a small
//! driver script appended after the (always fully self-contained, so
//! this works regardless of which SIR nodes the compiled module itself
//! used) inlined `__Sir` runtime — exactly the "bypass the compiled-
//! program path" approach the SIR23 addendum's own "Depth/DoS guard"
//! section calls for.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
};
use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sp() -> Span {
    Span::synthetic()
}

/// A trivial, otherwise-empty module used ONLY to obtain the inlined
/// `__Sir` runtime prelude (`compile`'s `RUNTIME` blob is unconditional
/// — every compiled artifact carries the FULL runtime regardless of
/// which SIR nodes the module actually uses, per this backend's own
/// "self-contained" design, `src/lib.rs`'s own module doc). The driver
/// script appended below calls `__Sir.Symbolic.evalTerm` directly, so
/// this module's own (empty) `main` body is never exercised.
fn runtime_prelude() -> String {
    let module = Module {
        name: "eval_depth_probe".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::SymbolicExpr,
            Feature::PatternMatching,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::IntLit { value: 0, span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };
    compile(&module).expect("compile runtime-prelude probe module").source
}

/// Build a right-nested `Add(1, Add(1, Add(1, …)))` term `n` levels deep
/// via `n` real, runtime `__Sir.Symbolic.apply` calls in a plain `for`
/// loop (NOT a hand-written giant static literal — the whole point,
/// mirrored from `tests/sir23_symbolic.rs`'s own deep-nesting test, is
/// that a tiny driver script can still build an arbitrarily deep runtime
/// VALUE), call `__Sir.Symbolic.evalTerm` on it DIRECTLY, and print
/// either the folded result or the literal string `"DEPTH_LIMIT"` if
/// `evalTerm` returned the depth-limit sentinel. Returns `node`'s
/// trimmed stdout, or `None` when `node` is unavailable.
///
/// A clean process exit (`output.status.success()`) is asserted
/// unconditionally: if the guard were ever missing or mis-wired, `node`
/// itself would crash with an uncaught `RangeError: Maximum call stack
/// size exceeded` well before printing anything, which is exactly the
/// failure mode this test exists to catch.
fn run_eval_at_depth(n: u32) -> Option<String> {
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping eval-depth-guard check for n={n}");
        return None;
    }
    let mut source = runtime_prelude();
    source.push_str(&format!(
        r#"
(function () {{
  let acc = __Sir.Symbolic.int(1);
  for (let i = 0; i < {n}; i++) {{
    acc = __Sir.Symbolic.apply(__Sir.Symbolic.sym("Add"), [__Sir.Symbolic.int(1), acc]);
  }}
  const result = __Sir.Symbolic.evalTerm(acc);
  if (result !== null && typeof result === "object" && result.kind === "depth-limit") {{
    console.log("DEPTH_LIMIT");
  }} else {{
    console.log(__Sir.Symbolic.toDisplayString(result));
  }}
}})();
"#
    ));

    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_eval_depth_{n}_{}.js", std::process::id()));
    std::fs::write(&path, &source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node crashed evaluating a {n}-level-deep term instead of returning the depth-limit \
         sentinel cleanly (MAX_EVAL_DEPTH guard missing or broken):\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Boundary point one: nesting exactly at `MAX_EVAL_DEPTH` (2000 —
/// `runtime.rs`'s own doc comment) folds to a genuine value, not the
/// depth-limit sentinel — the cap must not be so tight it rejects
/// ordinary, well within-budget input.
#[test]
fn eval_term_at_the_cap_folds_to_a_real_value() {
    if let Some(stdout) = run_eval_at_depth(2000) {
        // 2000 additions of 1 onto a base value of 1 folds to 2001.
        assert_eq!(stdout, "2001", "expected the 2000-deep Add chain to fold cleanly");
    }
}

/// Boundary point two: nesting one level past `MAX_EVAL_DEPTH` returns
/// the depth-limit sentinel — checked by `Symbolic.unwrap` the same way
/// `replaceAll`/`replaceRepeated`'s own cap is — rather than crashing.
#[test]
fn eval_term_one_past_the_cap_returns_the_sentinel_not_a_crash() {
    if let Some(stdout) = run_eval_at_depth(2001) {
        assert_eq!(stdout, "DEPTH_LIMIT");
    }
}

/// A term FAR deeper than the empirically measured ~2800-level bare-
/// stack crash floor (`runtime.rs`'s `MAX_EVAL_DEPTH` doc comment) —
/// confirms the guard trips on EVERY recursive descent (bailing out near
/// the very top of a 200,000-level-deep term), not merely once at some
/// fixed point, so an arbitrarily deep runtime value built by a
/// compiled program never gets a chance to overflow the native stack
/// regardless of how deep it actually is. This is the direct sibling of
/// `tests/sir23_symbolic.rs`'s `print_on_deeply_nested_term_truncates_
/// instead_of_crashing_node`, for `evalTerm` instead of `toDisplayString`.
#[test]
fn eval_term_far_past_the_cap_still_returns_the_sentinel_not_a_crash() {
    if let Some(stdout) = run_eval_at_depth(200_000) {
        assert_eq!(stdout, "DEPTH_LIMIT");
    }
}

/// Regression test for a security-review finding on this same item:
/// `comparisonHandler`'s `Equal`/`NotEqual` structural-equality fallback
/// (`runtime.rs`) calls the pre-existing `termEquals` — a plain recursive
/// tree-equality check with, before this fix, NO depth cap of its own,
/// unlike every other whole-tree-walking function in this file
/// (`toDisplayString`/`walkOnce`/`replaceRepeatedTerm`). Every
/// PRE-EXISTING call site (the pattern matcher, the rewrite engine's
/// fixed-point check) only ever compares terms already implicitly
/// bounded by a `MAX_TERM_DEPTH`-capped traversal elsewhere — but
/// `comparisonHandler`'s operands can be an arbitrarily deep symbolic
/// tree a compiled program built at runtime with NO fold available (an
/// unrecognized head, like the `"F"` used here, has no `HANDLERS` entry,
/// so `evalApply`'s "no handler matched" fallthrough just rebuilds the
/// arg-evaluated term at essentially its original depth — it does NOT
/// fold it away).
///
/// Chosen depth, `1000`, is deliberate, not arbitrary: it sits strictly
/// BETWEEN `termEquals`'s own new cap (`MAX_TERM_DEPTH = 512`) and
/// `evalTerm`'s own, unrelated cap (`MAX_EVAL_DEPTH = 2000`). This
/// isolates the exact gap the fix closes — deep enough that `termEquals`
/// (reusing `MAX_TERM_DEPTH`) trips ITS OWN new guard partway through
/// (at depth 513, long before it would naturally finish comparing all
/// 1000 levels), yet shallow enough that evaluating EACH operand
/// (`evalTerm`'s own, separate, already-existing cap on the ARGUMENT
/// evaluation that runs before `comparisonHandler` is ever reached)
/// completes normally rather than hitting `MAX_EVAL_DEPTH` first and
/// masking whether `termEquals`'s own fix actually works. (A depth past
/// 2000, tried first, does NOT exercise this path at all: `evalApply`'s
/// own arg-evaluation loop would return `evalTerm`'s depth-limit
/// sentinel for the `Equal(...)` call's OPERANDS before `comparisonHandler`
/// is ever invoked — a different, already-covered guard, not this one.)
/// A clean, non-crashing process exit is the pass condition; the exact
/// folded/unfolded VALUE is secondary — past `MAX_TERM_DEPTH`,
/// `termEquals`'s own new cap conservatively returns `false`, so this
/// prints an unevaluated `Equal(...)` term rather than `True` (the
/// correct, safe, documented trade-off — see `termEquals`'s own updated
/// doc comment in `runtime.rs` — not a bug).
#[test]
fn comparison_of_two_deep_unfoldable_trees_does_not_crash_term_equals() {
    if !node_available() {
        eprintln!(
            "note: `node` unavailable — skipping comparison_of_two_deep_unfoldable_trees_does_not_crash_term_equals"
        );
        return;
    }
    let mut source = runtime_prelude();
    source.push_str(
        r#"
(function () {
  function buildChain(n) {
    let acc = __Sir.Symbolic.sym("x");
    for (let i = 0; i < n; i++) {
      acc = __Sir.Symbolic.apply(__Sir.Symbolic.sym("F"), [acc]);
    }
    return acc;
  }
  const left = buildChain(1000);
  const right = buildChain(1000);
  const cmp = __Sir.Symbolic.apply(__Sir.Symbolic.sym("Equal"), [left, right]);
  const result = __Sir.Symbolic.evalTerm(cmp);
  if (result !== null && typeof result === "object" && result.kind === "depth-limit") {
    console.log("DEPTH_LIMIT");
  } else {
    console.log("NO_CRASH");
  }
})();
"#,
    );

    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!(
        "sir_eval_termequals_depth_{}.js",
        std::process::id()
    ));
    std::fs::write(&path, &source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node crashed comparing two 1000-deep unfoldable trees with Equal instead of \
         terminating cleanly (termEquals's own depth guard missing or broken):\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "NO_CRASH",
        "expected a clean, non-crashing result (whether Equal folds, stays unevaluated, or hits \
         evalTerm's own depth-limit sentinel, all are acceptable outcomes here -- only a raw \
         node crash is not)"
    );
}
