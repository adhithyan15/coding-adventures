//! SIR23 execution proof, Tier A pattern matcher (Phase A Slice 4):
//! `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` on the Rust backend — hand-
//! builds a module calling each node directly (bypassing the frontend,
//! since no frontend targets this backend for SIR23 yet), emits Rust,
//! compiles it with a real `rustc`, runs the binary, and asserts stdout/
//! exit status. Mirrors `sir22_array.rs`'s pattern; skips (does not fail)
//! when no `rustc`/usable linker is on the host, exactly like every other
//! `compile_and_run_*`/`sir22_array.rs` test in this crate.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir23_symbolic.rs` (Tier A cases only — the JS reference's
//! remaining tests exercise `evalTerm`, Tier B, explicitly out of scope
//! for this slice) and cross-checked against `semantic-ir-to-ruby`'s own
//! (also Tier-A-only) port for algorithm parity.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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

fn symbolic_module(stmts: Vec<Stmt>, extra_features: &[Feature]) -> Module {
    let mut features = vec![Feature::ConsoleIO, Feature::Strings];
    features.extend_from_slice(SYMBOLIC_FEATURES);
    features.extend_from_slice(extra_features);
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

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile `m` to Rust, run it with a real `rustc`, and return
/// `(stdout, success)` — or `None` to skip when no `rustc`/usable linker is
/// on the host, matching `sir22_array.rs::run_array_program` exactly.
/// Unique temp-file names per call (PID + a monotonic counter) — a
/// constant name would let concurrently-running `cargo test` threads
/// collide on the same path.
fn run_symbolic_program(m: &Module) -> Option<(String, bool, String)> {
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    if !rustc_available() {
        return None;
    }

    let artifact =
        semantic_ir_to_rust::compile(m).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed));
    let src_path = dir.join(format!("sir_rust_symbolic_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_rust_symbolic_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out =
        cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    let stdout = String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&run_out.stderr).replace("\r\n", "\n");
    Some((stdout, run_out.status.success(), stderr))
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

    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], true))], &[]);
    if let Some((stdout, success, stderr)) = run_symbolic_program(&m) {
        assert!(success, "expected success, got stderr:\n{stderr}");
        assert_eq!(stdout.trim_end(), "z");
    }
}

#[test]
fn replace_all_single_pass_does_not_retry_at_same_position() {
    // `/.` (single pass): a -> b applied to Pair(a, a) fires once at EACH
    // occurrence of `a` (bottom-up, one visit per node), not repeatedly —
    // there is nothing to retry here since `a`'s replacement `b` doesn't
    // itself match the rule, so replaceAll and replaceRepeated agree on
    // this particular input; the real single-pass-vs-fixed-point contrast
    // is `replace_repeated_reduces_nested_add_zero_to_bare_symbol` above
    // (repeated=true fires at TWO nested positions in one call).
    let r = rule(sym("a"), sym("b"), false);
    let expr = sym_apply(sym("Pair"), vec![sym("a"), sym("a")]);
    let m = symbolic_module(vec![print_stmt(replace_all(expr, vec![r], false))], &[]);
    if let Some((stdout, success, stderr)) = run_symbolic_program(&m) {
        assert!(success, "expected success, got stderr:\n{stderr}");
        assert_eq!(stdout.trim_end(), "Pair(b, b)");
    }
}

#[test]
fn typed_blank_matches_only_constrained_head() {
    // f(x_Integer) -> x_ matched against f(5) and f(z) (a bare Symbol):
    // only the Integer-headed argument matches; the Symbol one is left
    // unchanged by replaceAll's "no match, no rewrite" fallthrough.
    let x_pat = named("x", blank_typed("Integer"));
    let r = rule(sym_apply(sym("f"), vec![x_pat.clone()]), x_pat, false);
    let e_int_term = sym_apply(sym("f"), vec![ilit(5)]);
    let e_sym_term = sym_apply(sym("f"), vec![sym("z")]);
    let m = symbolic_module(
        vec![
            print_stmt(replace_all(e_int_term, vec![r.clone()], false)),
            print_stmt(replace_all(e_sym_term, vec![r], false)),
        ],
        &[],
    );
    if let Some((stdout, success, stderr)) = run_symbolic_program(&m) {
        assert!(success, "expected success, got stderr:\n{stderr}");
        let lines: Vec<&str> = stdout.trim_end().lines().collect();
        assert_eq!(lines, vec!["5", "f(z)"]);
    }
}

#[test]
fn a_rational_term_prints_reduced() {
    // `sir_sym_rational` reduces numer/denom by their gcd at construction
    // time, mirroring the JS/Ruby references — 6/8 must print as "3/4".
    let r = Expr::SymRational { numer: 6, denom: 8, span: s() };
    let m = symbolic_module(vec![print_stmt(r)], &[]);
    if let Some((stdout, success, stderr)) = run_symbolic_program(&m) {
        assert!(success, "expected success, got stderr:\n{stderr}");
        assert_eq!(stdout.trim_end(), "3/4");
    }
}

#[test]
fn depth_limit_guard_raises_a_catchable_error_instead_of_crashing() {
    // A runtime-built term nested past SIR_SYM_MAX_TERM_DEPTH (512) must
    // `raise` (a catchable `SirError` panic, per this backend's existing
    // SIR17 exception convention — see runtime.rs's `sir_sym_raise`), not
    // overflow the native Rust stack. Built via a REAL compiled `for`-loop
    // in the emitted program (600 runtime firings of `Wrap(acc)`, not a
    // hand-built 600-node static AST — mirrors the JS reference's own
    // `print_on_deeply_nested_term_truncates_instead_of_crashing_node` and
    // the Ruby reference's `depth_limit_guard_raises_a_ruby_error_instead_
    // of_crashing`), then run through `replaceAll` with an empty rule set
    // (no rule ever fires, so every level of the walk is exercised).
    //
    // `Feature::Exceptions` is declared so this backend's `emit_main` wraps
    // the program in the `catch_unwind`/`install_panic_hook`/
    // `report_uncaught` machinery — WITHOUT it an uncaught `raise` would
    // still abort with a non-zero exit (Rust's default panic behaviour),
    // but stderr would carry Rust's generic "Box<dyn Any>" panic banner
    // instead of the readable `RuntimeError: sir-runtime-symbolic:
    // depth-limit` message `report_uncaught` prints — declaring the
    // feature is what this test needs to assert on that readable message.
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
    let m = symbolic_module(
        stmts,
        &[Feature::Exceptions, Feature::Loops, Feature::MutableBindings],
    );

    if let Some((stdout, success, stderr)) = run_symbolic_program(&m) {
        assert!(
            !success,
            "expected a non-zero exit from the depth-limit raise, got success; stdout:\n{stdout}"
        );
        assert!(
            stderr.contains("sir-runtime-symbolic: depth-limit"),
            "expected a depth-limit error, got:\n{stderr}"
        );
    }
}
