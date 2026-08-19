//! SIR23 execution proof, Tier A pattern matcher (Phase A Slice 4):
//! `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` on the Python backend --
//! hand-builds a module calling each node directly (bypassing the
//! frontend, since no frontend targets this backend for SIR23 yet), emits
//! Python, runs it with a real `python3`/`python` interpreter (with
//! `coding-adventures-sir-runtime-core` and the new
//! `coding-adventures-sir-runtime-symbolic` package -- plus that package's
//! own two dependencies, `coding-adventures-symbolic-ir` and
//! `coding-adventures-cas-pattern-matching` -- on `PYTHONPATH`), and
//! asserts stdout. Mirrors `tests/sir22_array.rs`'s pattern (this backend's
//! own precedent for an *imported-package* SIR domain); skips (does not
//! fail) when no usable Python interpreter is on `PATH`.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir23_symbolic.rs` -- Tier A cases only (`replace_repeated`,
//! `replace_all`, typed-blank matching, the depth-limit DoS guard). The JS
//! reference's remaining tests (`assign`/`define`/`if`/elementary-function/
//! differentiation cases) all exercise `evalTerm` -- Tier B, explicitly out
//! of scope for this slice -- so they have no analogue here. Test
//! construction (hand-built `Module`s, `ForRange`-built deep term) mirrors
//! `semantic-ir-to-ruby`'s own just-merged `tests/sir23_symbolic.rs`
//! (`sir23-ruby-matcher` branch).
//!
//! Unlike JS/Ruby (which inline their symbolic runtime), this backend
//! follows the TypeScript backend's *imported-package* model -- see
//! `semantic-ir-to-python/src/runtime.rs`'s `RUNTIME_SYMBOLIC` doc comment
//! -- so every test here must add `sir-runtime-symbolic/src` (and its own
//! two dependencies' `src` dirs) to `PYTHONPATH` alongside
//! `sir-runtime-core/src`, exactly as `tests/sir22_array.rs` does for
//! `sir-runtime-array`.

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

/// Probe whether `exe` is a usable Python interpreter -- see
/// `tests/sir22_array.rs`'s identically-named helper for the exact
/// rationale (distinguishing a genuinely-absent interpreter from the
/// Windows Store `python3` stub).
fn python_is_runnable(exe: &str) -> bool {
    std::process::Command::new(exe)
        .args(["-c", "pass"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `PYTHONPATH` covering every sibling runtime package `sir-runtime-core`
/// itself unconditionally imports at module-load time (see
/// `tests/sir22_array.rs::run_array_program`'s own comment for why that
/// full set is needed even though this test file never touches
/// pairs/oop/range/regex/exceptions directly), plus the new
/// `sir-runtime-symbolic` package and ITS OWN two dependencies
/// (`symbolic-ir`, `cas-pattern-matching`) -- raw `PYTHONPATH` entries, not
/// an installed package, so nothing resolves `sir-runtime-symbolic`'s
/// `pyproject.toml` dependency list automatically.
fn symbolic_pythonpath() -> std::ffi::OsString {
    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
        py_root.join("sir-runtime-symbolic/src"),
        py_root.join("symbolic-ir/src"),
        py_root.join("cas-pattern-matching/src"),
    ])
    .expect("join PYTHONPATH")
}

/// Run emitted Python, returning stdout, or `None` to skip when no usable
/// interpreter is on `PATH`. Unique temp-file names per call (PID + a
/// monotonic counter) -- matches `tests/sir22_array.rs::run_array_program`'s
/// identical rationale (concurrently-running `cargo test` threads must not
/// collide on the same path).
fn run_symbolic_program(m: &Module) -> Option<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let exe = ["python3", "python"].into_iter().find(|e| python_is_runnable(e))?;

    let source = semantic_ir_to_python::compile(m).expect("python emit").source;
    let pythonpath = symbolic_pythonpath();

    let nonce = SEQ.fetch_add(1, Ordering::Relaxed);
    let file = std::env::temp_dir()
        .join(format!("sir_py_symbolic_{}_{}.py", std::process::id(), nonce));
    std::fs::write(&file, &source).expect("write temp python");
    let out = std::process::Command::new(exe)
        .arg(&file)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python");
    let _ = std::fs::remove_file(&file);

    assert!(
        out.status.success(),
        "emitted python failed under {exe}:\n{}\n--- source ---\n{source}",
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

    // expr: Add(Add(z, 0), 0) -- both `+ 0`s should fire, to a fixed point.
    let inner = sym_apply(sym("Add"), vec![sym("z"), ilit(0)]);
    let expr = sym_apply(sym("Add"), vec![inner, ilit(0)]);

    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], true))]);
    match run_symbolic_program(&m) {
        Some(stdout) => assert_eq!(stdout.trim_end(), "z"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn replace_all_single_pass_does_not_retry_at_same_position() {
    // `/.` (single pass): a -> b applied to Pair(a, a) fires once at EACH
    // occurrence of `a` (bottom-up, one visit per node), not repeatedly.
    let r = rule(sym("a"), sym("b"), false);
    let expr = sym_apply(sym("Pair"), vec![sym("a"), sym("a")]);
    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], false))]);
    match run_symbolic_program(&m) {
        Some(stdout) => assert_eq!(stdout.trim_end(), "Pair(b, b)"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn typed_blank_matches_only_constrained_head() {
    // f(x_Integer) -> x_ matched against f(5) and f(z): only the
    // Integer-headed argument matches; the Symbol one is left unchanged by
    // replaceAll's "no match, no rewrite" fallthrough.
    let x_pat = named("x", blank_typed("Integer"));
    let r = rule(sym_apply(sym("f"), vec![x_pat.clone()]), x_pat, false);
    let e_int_term = sym_apply(sym("f"), vec![ilit(5)]);
    let e_sym_term = sym_apply(sym("f"), vec![sym("z")]);
    let m = symbolic_module(vec![
        print_stmt(replace_all(e_int_term, vec![r.clone()], false)),
        print_stmt(replace_all(e_sym_term, vec![r], false)),
    ]);
    match run_symbolic_program(&m) {
        Some(stdout) => {
            let lines: Vec<&str> = stdout.trim_end().lines().collect();
            assert_eq!(lines, vec!["5", "f(z)"]);
        }
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn a_rational_term_prints_reduced() {
    // `_sir_sym_rational` reduces numer/denom by their gcd at construction
    // time (via `IRRational.__post_init__`) -- 6/8 must print as "3/4".
    let r = Expr::SymRational { numer: 6, denom: 8, span: s() };
    let m = symbolic_module(vec![print_stmt(r)]);
    match run_symbolic_program(&m) {
        Some(stdout) => assert_eq!(stdout.trim_end(), "3/4"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn depth_limit_guard_raises_a_clean_error_instead_of_crashing() {
    // A runtime-built term nested past MAX_TERM_DEPTH (512) must raise a
    // clean, catchable `ValueError` from `_sir_sym_unwrap`, not overflow the
    // native Python stack. Built via a REAL compiled `for`-loop in the
    // emitted program (600 runtime firings of `Wrap(acc)`, not a hand-built
    // 600-node static AST) -- mirrors the JS/Ruby references' own
    // equivalent regression test -- then run through `replace_all` with an
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

    let source = semantic_ir_to_python::compile(&m).expect("python emit").source;
    let exe = match ["python3", "python"].into_iter().find(|e| python_is_runnable(e)) {
        Some(exe) => exe,
        None => {
            eprintln!("skip: no python on PATH");
            return;
        }
    };
    let pythonpath = symbolic_pythonpath();
    let file = std::env::temp_dir()
        .join(format!("sir_py_symbolic_depth_{}.py", std::process::id()));
    std::fs::write(&file, &source).expect("write temp python");
    let out = std::process::Command::new(exe)
        .arg(&file)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python");
    let _ = std::fs::remove_file(&file);
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
fn rule_delayed_and_rule_currently_produce_identical_rewrites() {
    // Tier A has no evaluator yet, so `Rule`/`RuleDelayed` are matched and
    // substituted identically -- both sides of this call must produce the
    // exact same rewritten output.
    let x_pat = named("x", blank());
    let eager = rule(sym_apply(sym("Add"), vec![x_pat.clone(), ilit(0)]), x_pat.clone(), false);
    let delayed = rule(sym_apply(sym("Add"), vec![x_pat.clone(), ilit(0)]), x_pat, true);
    let target = sym_apply(sym("Add"), vec![sym("z"), ilit(0)]);
    let m = symbolic_module(vec![
        print_stmt(replace_all(target.clone(), vec![eager], false)),
        print_stmt(replace_all(target, vec![delayed], false)),
    ]);
    match run_symbolic_program(&m) {
        Some(stdout) => {
            let lines: Vec<&str> = stdout.trim_end().lines().collect();
            assert_eq!(lines, vec!["z", "z"]);
        }
        None => eprintln!("skip: no python on PATH"),
    }
}
