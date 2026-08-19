//! SIR23 execution proof, Tier A pattern matcher (Phase A Slice 4):
//! `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` on the C backend —
//! hand-builds a module calling each node directly (bypassing the
//! frontend, since no frontend targets this backend for SIR23 yet), emits
//! C, compiles with a real gcc/clang-style compiler, runs, asserts stdout.
//! Skips gracefully when no `cc` is present, mirroring `sir22_array.rs`'s
//! identical `find_cc`/`compile_and_link` pattern (copied verbatim below).
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir23_symbolic.rs` — Tier A cases only (`replace_repeated`,
//! `replace_all`, typed-blank matching, rational reduction, and the
//! depth-limit DoS guard), matching the already-merged
//! `semantic-ir-to-ruby` SIR23 PR's own `tests/sir23_symbolic.rs`
//! (`sir23-ruby-matcher` branch) test-construction pattern, adapted to
//! actually compile-and-run under a real C toolchain. The JS reference's
//! remaining tests (`assign`/`define`/`if`/elementary-function/
//! differentiation cases) all exercise `evalTerm` — Tier B, explicitly
//! out of scope for this slice — so they have no analogue here.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt, CURRENT_SIR_VERSION,
};

// ── compile/run harness — copied from `sir22_array.rs` ──────────────────

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_link(module: &Module) -> Option<(PathBuf, String)> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_symbolic_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    Some((exe, artifact.source))
}

fn run(module: &Module) -> Option<(String, bool, String)> {
    let (exe, _src) = compile_and_link(module)?;
    let r = Command::new(&exe).output().expect("run");
    Some((
        String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"),
        r.status.success(),
        String::from_utf8_lossy(&r.stderr).replace("\r\n", "\n"),
    ))
}

// ── module-building helpers ──────────────────────────────────────────────

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
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

// ── Tier A: real compile-and-run proof ───────────────────────────────────

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
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(ok, "expected success, got stderr:\n{stderr}");
            assert_eq!(stdout.trim_end(), "z");
        }
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn replace_all_single_pass_does_not_retry_at_same_position() {
    // `/.` (single pass): a -> b applied to Pair(a, a) fires once at EACH
    // occurrence of `a` (bottom-up, one visit per node), not repeatedly.
    let r = rule(sym("a"), sym("b"), false);
    let expr = sym_apply(sym("Pair"), vec![sym("a"), sym("a")]);
    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], false))]);
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(ok, "expected success, got stderr:\n{stderr}");
            assert_eq!(stdout.trim_end(), "Pair(b, b)");
        }
        None => eprintln!("skip: no cc"),
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
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(ok, "expected success, got stderr:\n{stderr}");
            let lines: Vec<&str> = stdout.trim_end().lines().collect();
            assert_eq!(lines, vec!["5", "f(z)"]);
        }
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_rational_term_prints_reduced() {
    // `_sir_symterm_rational` reduces numer/denom by their gcd at
    // construction time, mirroring the JS/Ruby references — 6/8 must
    // print as "3/4".
    let r = Expr::SymRational { numer: 6, denom: 8, span: s() };
    let m = symbolic_module(vec![print_stmt(r)]);
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(ok, "expected success, got stderr:\n{stderr}");
            assert_eq!(stdout.trim_end(), "3/4");
        }
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_rational_with_int64_min_numerator_prints_exactly_not_saturated() {
    // Regression test (/security-review finding): `_sir_symterm_rational`'s
    // gcd-reduction avoids `-INT64_MIN` signed-overflow UB by tracking sign
    // separately and computing magnitudes in `uint64_t`, then narrowing
    // back to `int64_t` on the way out. `INT64_MIN`'s magnitude (2^63) is
    // NOT representable as a positive `int64_t`, so the denominator's own
    // narrowing correctly saturates it to `INT64_MAX` when it can't fit —
    // but the numerator, being SIGNED, CAN represent that exact magnitude
    // as `INT64_MIN` itself. Applying denominator-style saturation to the
    // numerator too would silently round the exact value
    // `-9223372036854775808` to `-9223372036854775807` with no error. This
    // must print the exact boundary value, not an off-by-one-corrupted one.
    let r = Expr::SymRational { numer: i64::MIN, denom: 1, span: s() };
    let m = symbolic_module(vec![print_stmt(r)]);
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(ok, "expected success, got stderr:\n{stderr}");
            assert_eq!(stdout.trim_end(), "-9223372036854775808/1");
        }
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn depth_limit_guard_aborts_instead_of_crashing() {
    // A runtime-built term nested past SIR_SYMTERM_MAX_TERM_DEPTH (512)
    // must abort cleanly (fprintf + exit(1)) from `_sir_symterm_replace_
    // all`'s tree walk, not overflow the native C stack. Built via a REAL
    // compiled `for`-loop in the emitted program (600 runtime firings of
    // `Wrap(acc)`, not a hand-built 600-node static AST — mirrors the JS
    // reference's own depth-limit test and the already-merged Ruby PR's
    // identical construction), then run through `replaceAll` with an
    // empty rule set (no rule ever fires, so every level of the walk is
    // exercised).
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
        print_stmt(replace_all(local("acc"), vec![], false)),
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

    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(
                !ok,
                "expected a non-zero exit from the depth-limit abort, got success with stdout:\n{stdout}"
            );
            assert!(
                stderr.contains("sir-runtime-symbolic: depth-limit"),
                "expected a depth-limit error, got:\n{stderr}"
            );
        }
        None => eprintln!("skip: no cc"),
    }
}

// ── malformed hand-built shapes: rejected cleanly, not a runtime panic ───

#[test]
fn apply_rule_on_a_non_rule_is_rejected_cleanly_at_runtime() {
    // `_sir_symterm_apply_rule` requires a `Rule`/`RuleDelayed`-headed
    // term; feeding `replaceAll` a rules list containing a bare Symbol
    // (not built via `sir_rule`/`sir_rule_delayed`) is a malformed
    // hand-built shape no frontend would ever emit, but the runtime must
    // still fail loudly rather than dereference `args[0]`/`args[1]` on a
    // 0-arg Apply/non-Apply term.
    let bogus_rule = sym("NotARule");
    let expr = sym("x");
    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![bogus_rule], false))]);
    match run(&m) {
        Some((stdout, ok, stderr)) => {
            assert!(
                !ok,
                "expected a non-zero exit for a malformed rule, got success with stdout:\n{stdout}"
            );
            assert!(
                stderr.contains("expected Rule/RuleDelayed"),
                "expected a Rule/RuleDelayed error, got:\n{stderr}"
            );
        }
        None => eprintln!("skip: no cc"),
    }
}
