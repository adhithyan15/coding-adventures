//! SIR23 execution proof, Tier A pattern matcher (Phase A Slice 4):
//! `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` on the Ruby backend —
//! hand-builds a module calling each node directly (bypassing the
//! frontend, since no frontend targets this backend for SIR23 yet), emits
//! Ruby, runs it with a real `ruby` interpreter, and asserts stdout.
//! Mirrors `sir22_array.rs`'s pattern; skips (does not fail) when no
//! `ruby` is on `PATH`.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir23_symbolic.rs` — Tier A cases only (`replace_repeated`,
//! `replace_all`, typed-blank matching, the depth-limit and rewrite-cycle
//! DoS guards). The JS reference's remaining tests (`assign`/`define`/
//! `if`/elementary-function/differentiation cases) all exercise
//! `evalTerm` — Tier B, explicitly out of scope for this slice — so they
//! have no analogue here.

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
};

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn sym(name: &str) -> Expr {
    Expr::SymSymbol { name: name.into(), span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn sym_apply(head: Expr, args: Vec<Expr>) -> Expr {
    Expr::SymApply { head: Box::new(head), args, span: s() }
}
fn blank() -> Expr {
    Expr::SymPatternBlank { head: None, span: s() }
}
fn blank_typed(head: &str) -> Expr {
    Expr::SymPatternBlank { head: Some(Box::new(sym(head))), span: s() }
}
fn named(name: &str, pattern: Expr) -> Expr {
    Expr::SymPatternNamed { name: name.into(), pattern: Box::new(pattern), span: s() }
}
fn rule(lhs: Expr, rhs: Expr, delayed: bool) -> Expr {
    Expr::SymRule { lhs: Box::new(lhs), rhs: Box::new(rhs), delayed, span: s() }
}
fn replace_all(expr: Expr, rules: Vec<Expr>, repeated: bool) -> Expr {
    Expr::SymReplaceAll { expr: Box::new(expr), rules, repeated, span: s() }
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
fn let_binding(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}

const SYMBOLIC_FEATURES: &[Feature] =
    &[Feature::SymbolicExpr, Feature::PatternMatching, Feature::Rationals];

fn symbolic_module(stmts: Vec<Stmt>) -> Module {
    let mut features = vec![Feature::ConsoleIO, Feature::Strings];
    features.extend_from_slice(SYMBOLIC_FEATURES);
    Module {
        name: "sir23symbolic".into(),
        manifest: FeatureManifest::from_features(&features),
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

/// Run emitted Ruby, returning stdout, or `None` to skip when no `ruby` is
/// on `PATH`. Unique temp-file names per call (PID + a monotonic counter)
/// — see `sir22_array.rs::run_array_program`'s identical rationale.
fn run_symbolic_program(m: &Module) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    let source = semantic_ir_to_ruby::compile(m).expect("ruby emit").source;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sir_ruby_symbolic_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path).ok()?.write_all(source.as_bytes()).ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok()?;
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "emitted ruby exited non-zero:\n{}\n--- source ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

#[test]
fn replace_repeated_reduces_nested_add_zero_to_bare_symbol() {
    // Rule: x_ + 0 -> x_ (Wolfram `x_ + 0 :> x_`, held as `RuleDelayed`
    // here so the RHS is exactly the same pattern-bound `x_` node).
    let x_pat = named("x", blank());
    let r = rule(sym_apply(sym("Add"), vec![x_pat.clone(), ilit(0)]), x_pat, true);

    // expr: Add(Add(z, 0), 0) — both `+ 0`s should fire, to a fixed point.
    let inner = sym_apply(sym("Add"), vec![sym("z"), ilit(0)]);
    let expr = sym_apply(sym("Add"), vec![inner, ilit(0)]);

    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], true))]);
    if let Some(stdout) = run_symbolic_program(&m) {
        assert_eq!(stdout.trim_end(), "z");
    }
}

#[test]
fn replace_all_single_pass_does_not_retry_at_same_position() {
    // `/.` (single pass): a -> b applied to Pair(a, a) fires once at EACH
    // occurrence of `a` (bottom-up, one visit per node), not repeatedly.
    let r = rule(sym("a"), sym("b"), false);
    let expr = sym_apply(sym("Pair"), vec![sym("a"), sym("a")]);
    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], false))]);
    if let Some(stdout) = run_symbolic_program(&m) {
        assert_eq!(stdout.trim_end(), "Pair(b, b)");
    }
}

#[test]
fn typed_blank_matches_only_constrained_head() {
    // f(x_Integer) -> x_ matched against f(5) and f(z): only the
    // Integer-headed argument matches; the Symbol one is left unchanged
    // by replaceAll's "no match, no rewrite" fallthrough.
    let x_pat = named("x", blank_typed("Integer"));
    let r = rule(sym_apply(sym("f"), vec![x_pat.clone()]), x_pat, false);
    let e_int_term = sym_apply(sym("f"), vec![ilit(5)]);
    let e_sym_term = sym_apply(sym("f"), vec![sym("z")]);
    let m = symbolic_module(vec![
        print_stmt(replace_all(e_int_term, vec![r.clone()], false)),
        print_stmt(replace_all(e_sym_term, vec![r], false)),
    ]);
    if let Some(stdout) = run_symbolic_program(&m) {
        let lines: Vec<&str> = stdout.trim_end().lines().collect();
        assert_eq!(lines, vec!["5", "f(z)"]);
    }
}

#[test]
fn a_rational_term_prints_reduced() {
    // sir_sym_rational reduces numer/denom by their gcd at construction
    // time, mirroring the JS reference — 6/8 must print as "3/4".
    let r = Expr::SymRational { numer: 6, denom: 8, span: s() };
    let m = symbolic_module(vec![print_stmt(r)]);
    if let Some(stdout) = run_symbolic_program(&m) {
        assert_eq!(stdout.trim_end(), "3/4");
    }
}

#[test]
fn depth_limit_guard_raises_a_ruby_error_instead_of_crashing() {
    // A runtime-built term nested past SIR_SYM_MAX_TERM_DEPTH (512) must
    // raise a clean, catchable error from `sir_sym_unwrap`, not overflow
    // the native Ruby stack. Built via a REAL compiled `for`-loop in the
    // emitted program (600 runtime firings of `Wrap(acc)`, not a hand-
    // built 600-node static AST — mirrors the JS reference's own
    // `print_on_deeply_nested_term_truncates_instead_of_crashing_node`),
    // then run through `replaceAll` with an empty rule set (no rule ever
    // fires, so every level of the walk is exercised).
    let stmts = vec![
        let_binding("acc", sym("leaf")),
        Stmt::ForRange {
            var: "i".into(),
            start: ilit(0),
            stop: ilit(600),
            step: ilit(1),
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "acc".into(),
                    scope: Scope::Local,
                    value: sym_apply(sym("Wrap"), vec![local("acc")]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "__sys_write__".into(),
                args: vec![
                    Expr::StrLit { value: "stdout".into(), span: s() },
                    Expr::StrLit { value: "once".into(), span: s() },
                    Expr::BoolLit { value: false, span: s() },
                    replace_all(local("acc"), vec![], false),
                ],
                effects: EffectSet::PURE.with(Effect::MayPrint),
                span: s(),
            },
            span: s(),
        },
    ];
    let mut m = symbolic_module(stmts);
    let mut features: Vec<Feature> = SYMBOLIC_FEATURES.to_vec();
    features.extend_from_slice(&[
        Feature::ConsoleIO,
        Feature::Strings,
        Feature::Loops,
        Feature::MutableBindings,
    ]);
    m.manifest = FeatureManifest::from_features(&features);

    let source = semantic_ir_to_ruby::compile(&m).expect("ruby emit").source;
    if std::process::Command::new("ruby").arg("--version").output().is_err() {
        return; // skip: no ruby on PATH
    }
    let dir = std::env::temp_dir();
    let path = dir.join(format!("sir_ruby_symbolic_depth_{}.rb", std::process::id()));
    std::fs::write(&path, &source).expect("write temp ruby");
    let out = std::process::Command::new("ruby").arg(&path).output().expect("spawn ruby");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a non-zero exit from the depth-limit raise, got success:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("sir-runtime-symbolic: depth-limit"),
        "expected a depth-limit error, got:\n{stderr}"
    );
}

#[test]
fn deep_rule_rhs_reports_depth_limit_error_not_a_crash_even_with_a_shallow_target() {
    // Regression test (/security-review finding, follow-up fix after
    // PR #12128 shipped without this guard): `sir_sym_match_pattern`/
    // `sir_sym_substitute` originally carried NO depth cap at all, on the
    // (unverified) assumption inherited from the JS reference that a
    // rule's `lhs`/`rhs` is always author-written and shallow. That does
    // not hold here — `Expr::SymRule`'s operands are ordinary `Expr`s, so
    // a rule's RHS can be a LOCAL VARIABLE holding a term a compiled
    // `for`-loop built to unbounded depth at runtime, same as the
    // already-tested target-tree case above.
    //
    // This test proves the gap couldn't be caught by `sir_sym_walk_once`/
    // `replace_repeated`'s OWN target-tree depth tracking: the rule here
    // is `Blank() -> <a 600-deep term>` matched against a single bare
    // SHALLOW symbol target. The match itself is instant (`Blank()`
    // matches anything, no recursion), but `sir_sym_substitute` must then
    // rebuild the entire 600-deep RHS to produce the replacement —
    // independent of how deep (or shallow) the target being rewritten
    // was. Before the fix, this recursed uncapped and would eventually
    // raise Ruby's own `SystemStackError` (an uncontrolled crash, not a
    // clean, catchable error); after the fix, it raises the same clean
    // "sir-runtime-symbolic: depth-limit" error the target-tree guard
    // already produces.
    let stmts = vec![
        let_binding("deep_rhs", sym("leaf")),
        Stmt::ForRange {
            var: "i".into(),
            start: ilit(0),
            stop: ilit(600),
            step: ilit(1),
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "deep_rhs".into(),
                    scope: Scope::Local,
                    value: sym_apply(sym("Wrap"), vec![local("deep_rhs")]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "__sys_write__".into(),
                args: vec![
                    Expr::StrLit { value: "stdout".into(), span: s() },
                    Expr::StrLit { value: "once".into(), span: s() },
                    Expr::BoolLit { value: false, span: s() },
                    replace_all(
                        sym("shallow_target"),
                        vec![rule(blank(), local("deep_rhs"), false)],
                        false,
                    ),
                ],
                effects: EffectSet::PURE.with(Effect::MayPrint),
                span: s(),
            },
            span: s(),
        },
    ];
    let mut m = symbolic_module(stmts);
    let mut features: Vec<Feature> = SYMBOLIC_FEATURES.to_vec();
    features.extend_from_slice(&[
        Feature::ConsoleIO,
        Feature::Strings,
        Feature::Loops,
        Feature::MutableBindings,
    ]);
    m.manifest = FeatureManifest::from_features(&features);

    let source = semantic_ir_to_ruby::compile(&m).expect("ruby emit").source;
    if std::process::Command::new("ruby").arg("--version").output().is_err() {
        return; // skip: no ruby on PATH
    }
    let dir = std::env::temp_dir();
    let path = dir.join(format!("sir_ruby_symbolic_rule_depth_{}.rb", std::process::id()));
    std::fs::write(&path, &source).expect("write temp ruby");
    let out = std::process::Command::new("ruby").arg(&path).output().expect("spawn ruby");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a non-zero exit from the depth-limit raise, got success:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("sir-runtime-symbolic: depth-limit"),
        "expected a depth-limit error, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("SystemStackError") && !stderr.contains("stack level too deep"),
        "expected a clean raise, not a native stack overflow:\n{stderr}"
    );
}
