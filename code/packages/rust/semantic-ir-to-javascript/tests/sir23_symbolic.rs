//! End-to-end integration test: hand-built SIR23 nodes → JavaScript →
//! `node`.
//!
//! `src/lib.rs`'s own test module proves the emitted *shape* (exact
//! substring assertions on generated source, mirroring the TypeScript
//! backend's SIR23 tests). This file proves the emitted *behaviour*: a
//! `SymReplaceAll { repeated: true }` node, run for real under Node.js,
//! must actually reduce `Add(Add(z, 0), 0)` to the bare symbol `z` via
//! the `x_ + 0 -> x_` rule — not just produce plausible-looking source.
//!
//! Node is optional at test time; when unavailable the test degrades to
//! a no-op rather than failing (mirroring `run_with_node.rs`).

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span, Stmt,
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

fn sym(name: &str) -> Expr {
    Expr::SymSymbol {
        name: name.into(),
        span: sp(),
    }
}

fn local(name: &str) -> Expr {
    Expr::VarRef {
        name: name.into(),
        scope: Scope::Local,
        span: sp(),
    }
}

fn sym_apply(head: Expr, args: Vec<Expr>) -> Expr {
    Expr::SymApply {
        head: Box::new(head),
        args,
        span: sp(),
    }
}

fn blank() -> Expr {
    Expr::SymPatternBlank {
        head: None,
        span: sp(),
    }
}

fn named(name: &str, pattern: Expr) -> Expr {
    Expr::SymPatternNamed {
        name: name.into(),
        pattern: Box::new(pattern),
        span: sp(),
    }
}

fn rule(lhs: Expr, rhs: Expr, delayed: bool) -> Expr {
    Expr::SymRule {
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        delayed,
        span: sp(),
    }
}

fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: sp(),
    }
}

fn print(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("print", vec![arg]),
        span: sp(),
    }
}

fn module_with_main(stmts: Vec<Stmt>, value: Expr, features: &[Feature]) -> Module {
    Module {
        name: "sir23".into(),
        manifest: FeatureManifest::from_features(features),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value,
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
    }
}

fn run_module(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

#[test]
fn replace_repeated_reduces_nested_add_zero_to_bare_symbol() {
    // Rule: x_ + 0 -> x_ (Wolfram `x_ + 0 :> x_`, held as `RuleDelayed`
    // here so the RHS is exactly the same pattern-bound `x_` node).
    let x_pat = named("x", blank());
    let zero = || Expr::IntLit {
        value: 0,
        span: sp(),
    };
    let r = rule(
        sym_apply(sym("Add"), vec![x_pat.clone(), zero()]),
        x_pat,
        true,
    );

    // expr: Add(Add(z, 0), 0)  —  both `+ 0`s should fire, to a fixed point.
    let inner = sym_apply(sym("Add"), vec![sym("z"), zero()]);
    let expr = sym_apply(sym("Add"), vec![inner, zero()]);

    let replace_repeated = Expr::SymReplaceAll {
        expr: Box::new(expr),
        rules: vec![r],
        repeated: true,
        span: sp(),
    };

    let module = module_with_main(
        vec![print(replace_repeated)],
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr, Feature::PatternMatching],
    );

    if let Some(stdout) = run_module(&module, "sym_replace_repeated") {
        assert_eq!(stdout, "z");
    }
}

#[test]
fn replace_all_single_pass_does_not_retry_at_same_position() {
    // `/.` (single pass): a -> b applied to Add(a, a) fires once at EACH
    // occurrence of `a` (bottom-up, one visit per node), not repeatedly —
    // there is nothing to retry here since `a`'s replacement `b` doesn't
    // itself match the rule, so replaceAll and replaceRepeated agree on
    // this particular input; the real single-pass-vs-fixed-point contrast
    // is `replace_repeated_reduces_nested_add_zero_to_bare_symbol` above
    // (repeated=true fires at TWO nested positions in one call).
    let r = rule(sym("a"), sym("b"), false);
    let expr = sym_apply(sym("Pair"), vec![sym("a"), sym("a")]);
    let replace_all = Expr::SymReplaceAll {
        expr: Box::new(expr),
        rules: vec![r],
        repeated: false,
        span: sp(),
    };
    let module = module_with_main(
        vec![print(replace_all)],
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr, Feature::PatternMatching],
    );
    if let Some(stdout) = run_module(&module, "sym_replace_all") {
        assert_eq!(stdout, "Pair(b, b)");
    }
}

#[test]
fn typed_blank_matches_only_constrained_head() {
    // f(x_Integer) -> x_ matched against f(5) and f(z) (a bare Symbol):
    // only the Integer-headed argument matches; the Symbol one is left
    // unchanged by replaceAll's "no match, no rewrite" fallthrough.
    let x_pat = named(
        "x",
        Expr::SymPatternBlank {
            head: Some(Box::new(sym("Integer"))),
            span: sp(),
        },
    );
    let r = rule(sym_apply(sym("f"), vec![x_pat.clone()]), x_pat, false);
    let matching = sym_apply(
        sym("f"),
        vec![Expr::IntLit {
            value: 5,
            span: sp(),
        }],
    );
    let non_matching = sym_apply(sym("f"), vec![sym("z")]);
    let expr = sym_apply(sym("Pair"), vec![matching, non_matching]);
    let replace_all = Expr::SymReplaceAll {
        expr: Box::new(expr),
        rules: vec![r],
        repeated: false,
        span: sp(),
    };
    let module = module_with_main(
        vec![print(replace_all)],
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr, Feature::PatternMatching],
    );
    if let Some(stdout) = run_module(&module, "sym_typed_blank") {
        assert_eq!(stdout, "Pair(5, f(z))");
    }
}

#[test]
fn print_on_deeply_nested_term_truncates_instead_of_crashing_node() {
    // Regression test (/security-review finding): `Symbolic.toDisplayString`
    // — reached from `print`/`puts` via `formatSeen` — recursed over the
    // FULL term tree with no depth cap of its own (only `replaceAll`/
    // `replaceRepeated`'s walk enforced `MAX_TERM_DEPTH`). A term built via
    // 20,000 real *runtime* firings of `Symbolic.apply` (an ordinary
    // compiled `for`-loop, NOT a hand-built 20,000-node static AST — the
    // whole point being that a tiny, shallow compiled program can build an
    // arbitrarily deep runtime VALUE) bypassed that cap entirely, so
    // `toDisplayString` needed its own guard. Comfortably above the
    // empirically-measured ~5000-level pre-fix crash threshold for this
    // walk (a smaller count wouldn't actually exercise the crash this
    // test guards against), so reverting the fix makes this test fail via
    // a genuine `node` crash (`run_module`'s `output.status.success()`),
    // not just the truncation-string assertion below. `node` must exit
    // cleanly with a truncated `...` rather than crashing with "Maximum
    // call stack size exceeded".
    //
    // for i in range(0, 20000, 1) { acc = Symbolic-apply(f, [acc]) }
    // print(acc)
    let stmts = vec![
        Stmt::LetBinding {
            name: "acc".into(),
            sir_type: None,
            value: sym("leaf"),
            span: sp(),
        },
        Stmt::ForRange {
            var: "i".into(),
            start: Expr::IntLit {
                value: 0,
                span: sp(),
            },
            stop: Expr::IntLit {
                value: 20000,
                span: sp(),
            },
            step: Expr::IntLit {
                value: 1,
                span: sp(),
            },
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "acc".into(),
                    scope: Scope::Local,
                    value: sym_apply(sym("f"), vec![local("acc")]),
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            span: sp(),
        },
        print(local("acc")),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[
            Feature::SymbolicExpr,
            Feature::Loops,
            Feature::MutableBindings,
        ],
    );
    if let Some(stdout) = run_module(&module, "sym_deep_display") {
        assert!(stdout.contains("..."), "got: {stdout}");
    }
}

#[test]
fn derive_display_on_a_deeply_nested_list_of_lists_truncates_at_the_same_depth_as_any_other_shape()
{
    // Regression test (/security-review finding on SIR_DISPLAY_DERIVE):
    // `deriveRenderList`'s matrix branch (every element of a `List(...)`
    // itself a `List`) used to reach into a `row`'s own `.args` directly,
    // skipping the `depth + 1 > MAX_TERM_DEPTH` check every OTHER shape
    // in this function family pays for descending one tree level, and
    // handing the row's OWN children `depth + 1` instead of `depth + 2` —
    // so two real tree-nesting levels consumed only one unit of the
    // shared depth budget. A chain of `N` nested single-element
    // `List(...)` wrappers (built via a real, compiled `for`-loop — an
    // ordinary runtime VALUE, not a giant hand-built static AST, exactly
    // like `print_on_deeply_nested_term_truncates_instead_of_crashing_
    // node` above) is exactly the shape that bug doubled: pre-fix, this
    // walk's effective depth budget was roughly `2 * MAX_TERM_DEPTH`
    // (~1024) real nesting levels before the "..." sentinel fired, vs.
    // every other shape's `MAX_TERM_DEPTH` (512). `N = 700` sits strictly
    // between those two numbers, so this is a genuine discriminating
    // test: reverting the `deriveRenderList` fix makes this test FAIL
    // (the pre-fix build renders the full 700-level nest with no "..." at
    // all, since 700 real levels only charged ~350 units of the buggy
    // halved budget), not just a weaker "doesn't crash" check.
    //
    // for i in range(0, 700, 1) { acc = List(acc) }
    // print(acc)   -- source_language "derive", so SIR_DISPLAY_DERIVE
    //                 is on and this walk actually goes through
    //                 deriveRenderList, not the generic toDisplayString.
    let stmts = vec![
        Stmt::LetBinding {
            name: "acc".into(),
            sir_type: None,
            value: sym_apply(sym("List"), vec![sym("leaf")]),
            span: sp(),
        },
        Stmt::ForRange {
            var: "i".into(),
            start: Expr::IntLit {
                value: 0,
                span: sp(),
            },
            stop: Expr::IntLit {
                value: 700,
                span: sp(),
            },
            step: Expr::IntLit {
                value: 1,
                span: sp(),
            },
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "acc".into(),
                    scope: Scope::Local,
                    value: sym_apply(sym("List"), vec![local("acc")]),
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            span: sp(),
        },
        print(local("acc")),
    ];
    let module = Module {
        name: "sir23".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::SymbolicExpr,
            Feature::Loops,
            Feature::MutableBindings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::IntLit {
                    value: 0,
                    span: sp(),
                },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("derive")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };
    if let Some(stdout) = run_module(&module, "sym_deep_derive_display") {
        assert!(
            stdout.contains("..."),
            "expected truncation well before 700 real nesting levels under the intended \
             MAX_TERM_DEPTH=512 cap, got the full render instead (the deriveRenderList depth-\
             charging regression this test guards against): {stdout}"
        );
    }
}

// ── SIR23 addendum item 2: held-form execution (Assign/Define/If) +
// user-function dispatch — see `code/specs/SIR23-symbolic-pattern-
// semantic-ir.md`'s addendum, "Environment / held-form execution model" /
// "Function dispatch". These mirror the shape of `derive-to-semantic-ir/
// tests/oracle.rs`'s own newly-`known_bug: None` cases
// (`variable_assignment_and_later_reference`,
// `single_param_function_definition_and_call`,
// `multi_param_function_definition_and_call`, `if_true_branch`,
// `if_false_branch`) but hand-build the SIR23 nodes directly (this
// crate's own established convention, see this file's module doc),
// rather than routing through `derive-to-semantic-ir`'s lowering.

#[test]
fn assign_binds_and_reads_back_in_a_later_statement() {
    // x := 5; x + 1  -- the SAME environment (`symEnv`) is shared across
    // every top-level statement in one compiled program (one `Symbolic`
    // IIFE evaluation = one `node` process = one flat session).
    let stmts = vec![
        print(sym_apply(
            sym("Assign"),
            vec![
                sym("x"),
                Expr::IntLit {
                    value: 5,
                    span: sp(),
                },
            ],
        )),
        print(sym_apply(
            sym("Add"),
            vec![
                sym("x"),
                Expr::IntLit {
                    value: 1,
                    span: sp(),
                },
            ],
        )),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_assign_read_back") {
        assert_eq!(stdout, "5\n6");
    }
}

#[test]
fn single_param_define_then_call_dispatches_by_substitution() {
    // F(x) := x*x; F(5) -- Define echoes the bare name `F` (never the
    // stored record, `define_handler`'s own documented invariant); the
    // call zips `params` against the call's (already-evaluated) args by
    // position, substitutes into `body`, and re-evaluates: x*x -> 5*5 -> 25.
    let stmts = vec![
        print(sym_apply(
            sym("Define"),
            vec![
                sym("F"),
                sym_apply(sym("List"), vec![sym("x")]),
                sym_apply(sym("Mul"), vec![sym("x"), sym("x")]),
            ],
        )),
        print(sym_apply(
            sym("F"),
            vec![Expr::IntLit {
                value: 5,
                span: sp(),
            }],
        )),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_define_call_single_param") {
        assert_eq!(stdout, "F\n25");
    }
}

#[test]
fn multi_param_define_then_call_dispatches_by_position() {
    // G(a, b) := a + b; G(3, 4) -- params zipped by POSITION, not name.
    let stmts = vec![
        print(sym_apply(
            sym("Define"),
            vec![
                sym("G"),
                sym_apply(sym("List"), vec![sym("a"), sym("b")]),
                sym_apply(sym("Add"), vec![sym("a"), sym("b")]),
            ],
        )),
        print(sym_apply(
            sym("G"),
            vec![
                Expr::IntLit {
                    value: 3,
                    span: sp(),
                },
                Expr::IntLit {
                    value: 4,
                    span: sp(),
                },
            ],
        )),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_define_call_multi_param") {
        assert_eq!(stdout, "G\n7");
    }
}

#[test]
fn arity_mismatch_leaves_the_user_function_call_unevaluated() {
    // F(x) := x*x; F(1, 2) -- a 2-arg call against a 1-param definition:
    // `apply_user_function`'s `None` return ("arity mismatch") means the
    // call is left exactly as `evalApply`'s generic "no handler matched"
    // fallthrough would rebuild it: the evaluated head plus the
    // evaluated (but otherwise untouched) args, printed via the generic
    // `head(args, ...)` convention (this module doesn't set
    // `source_language("derive")`, so `SIR_DISPLAY_DERIVE` is off here).
    let stmts = vec![
        print(sym_apply(
            sym("Define"),
            vec![
                sym("F"),
                sym_apply(sym("List"), vec![sym("x")]),
                sym_apply(sym("Mul"), vec![sym("x"), sym("x")]),
            ],
        )),
        print(sym_apply(
            sym("F"),
            vec![
                Expr::IntLit {
                    value: 1,
                    span: sp(),
                },
                Expr::IntLit {
                    value: 2,
                    span: sp(),
                },
            ],
        )),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_define_call_arity_mismatch") {
        assert_eq!(stdout, "F\nF(1, 2)");
    }
}

#[test]
fn if_true_and_false_branches_select_the_right_arm() {
    // IF(1 > 0, 42, 0) -> 42; IF(1 > 2, 42, 99) -> 99 -- `If`'s condition
    // is evaluated (it's held, so NOT pre-evaluated by `evalApply`'s
    // argument loop; `ifHandler` evaluates it itself), then branches.
    let if_of = |cond_gt: (i64, i64), then_v: i64, else_v: i64| {
        sym_apply(
            sym("If"),
            vec![
                sym_apply(
                    sym("Greater"),
                    vec![
                        Expr::IntLit {
                            value: cond_gt.0,
                            span: sp(),
                        },
                        Expr::IntLit {
                            value: cond_gt.1,
                            span: sp(),
                        },
                    ],
                ),
                Expr::IntLit {
                    value: then_v,
                    span: sp(),
                },
                Expr::IntLit {
                    value: else_v,
                    span: sp(),
                },
            ],
        )
    };
    let stmts = vec![
        print(if_of((1, 0), 42, 0)),
        print(if_of((1, 2), 42, 99)),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_if_branches") {
        assert_eq!(stdout, "42\n99");
    }
}

#[test]
fn self_referential_assign_does_not_infinite_loop() {
    // x := x; x -- the self-loop guard (`eval_symbol`'s own comment:
    // "x := x would recurse forever without this"). Without the guard,
    // the SECOND statement's lookup would recurse until `MAX_EVAL_DEPTH`
    // fires and `Symbolic.unwrap` throws, crashing `node` with a non-zero
    // exit (caught by `run_module`'s own `output.status.success()`
    // assertion) -- WITH the guard, both statements return instantly.
    let stmts = vec![
        print(sym_apply(sym("Assign"), vec![sym("x"), sym("x")])),
        print(sym("x")),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_self_referential_assign") {
        assert_eq!(stdout, "x\nx");
    }
}

// ── SIR23 addendum item 3 of 4: calculus / elementary-function handlers ──

fn int_lit(value: i64) -> Expr {
    Expr::IntLit { value, span: sp() }
}

#[test]
fn elementary_function_identity_folds_are_exact_integers() {
    // Sin(0) -> 0, Cos(0) -> 1, Sqrt(4) -> 2 -- `sinHandler`/`cosHandler`/
    // `sqrtHandler`'s numeric-argument branch, each producing a plain
    // EXACT integer term (not a float), mirroring
    // `handlers.rs::{sin_handler, cos_handler, sqrt_handler}`'s own
    // `va == Numeric::Int(0)`/`Int(1)` special cases (for `Sqrt`, via the
    // perfect-square round-trip check).
    let stmts = vec![
        print(sym_apply(sym("Sin"), vec![int_lit(0)])),
        print(sym_apply(sym("Cos"), vec![int_lit(0)])),
        print(sym_apply(sym("Sqrt"), vec![int_lit(4)])),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_elementary_identity_folds") {
        assert_eq!(stdout, "0\n1\n2");
    }
}

#[test]
fn sin_of_a_free_symbol_stays_unevaluated() {
    // Sin(x) where `x` is an unbound free symbol -- `to_numeric` fails,
    // so (per `SymbolicBackend::new()`'s `simplify: true` -- confirmed by
    // reading `derive-runtime`/`reduce-runtime`/`maple-runtime`'s own
    // `src/lib.rs`, none of which ever construct a `simplify: false`
    // backend) this is NOT an error: the call passes through unevaluated,
    // exactly like any other unrecognised shape in this dispatcher,
    // printed via the generic `head(args, ...)` convention (this module
    // never sets `source_language("derive")`, so `SIR_DISPLAY_DERIVE` is
    // off here, same as `arity_mismatch_leaves_the_user_function_call_
    // unevaluated` above).
    let stmts = vec![print(sym_apply(sym("Sin"), vec![sym("x")]))];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_sin_of_free_symbol") {
        assert_eq!(stdout, "Sin(x)");
    }
}

#[test]
fn differentiate_a_power_via_the_power_rule() {
    // D(Pow(x, 2), x) -- `diffPowTerm`'s constant-exponent branch:
    // `n * base^(n-1) * d/dx[base]` = `Mul(Mul(2, Pow(x, Sub(2, 1))), 1)`,
    // then re-evaluated (`derivativeHandler`'s own `evalTerm(result, depth
    // + 1)` call) through the already-shipped arithmetic folding (item 1):
    // `Sub(2, 1) -> 1`, `Pow(x, 1) -> x` (identity), `Mul(2, x)` stays,
    // outer `Mul(_, 1) -> _` (identity) -- final shape `Mul(2, x)`.
    let stmts = vec![print(sym_apply(
        sym("D"),
        vec![sym_apply(sym("Pow"), vec![sym("x"), int_lit(2)]), sym("x")],
    ))];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_differentiate_power") {
        assert_eq!(stdout, "Mul(2, x)");
    }
}

#[test]
fn differentiate_sin_via_the_chain_rule() {
    // D(Sin(x), x) -- `chainRuleTerm("Cos", x, "x")`:
    // `Mul(Cos(x), d/dx[x])` = `Mul(Cos(x), 1)`, re-evaluated to the bare
    // `Cos(x)` term via item 1's `a * 1 -> a` identity law (`Cos(x)`
    // itself passes through `cosHandler` unevaluated, since `x` is free).
    let stmts = vec![print(sym_apply(
        sym("D"),
        vec![sym_apply(sym("Sin"), vec![sym("x")]), sym("x")],
    ))];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_differentiate_sin_chain_rule") {
        assert_eq!(stdout, "Cos(x)");
    }
}

#[test]
fn integrate_a_bare_symbol() {
    // Integrate(x, x) -- `integrateTerm`'s bare-symbol case: `(1/2) * x^2`
    // = `Mul(Rational(1, 2), Pow(x, 2))`, an EXACT rational term (never a
    // float), matching this crate's own exact-rational discipline already
    // established by item 1's arithmetic folding
    // (`inexact_division_folds_to_a_rational`).
    let stmts = vec![print(sym_apply(sym("Integrate"), vec![sym("x"), sym("x")]))];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[Feature::SymbolicExpr],
    );
    if let Some(stdout) = run_module(&module, "sym_integrate_bare_symbol") {
        assert_eq!(stdout, "Mul(1/2, Pow(x, 2))");
    }
}

#[test]
fn differentiate_of_a_runtime_built_deep_term_stays_bounded_instead_of_crashing_node() {
    // SECURITY (CWE-674) investigation, prompted by `/security-review`
    // during this item's own development: `diffTerm`'s first argument to
    // `D` is an already-EVALUATED value (unlike `substituteSymbols`'s
    // always-source-literal `Define` body), so in principle it could be a
    // `Symbol` resolving to an arbitrarily deep RUNTIME-constructed term
    // -- the same "shallow compiled program, deep runtime value" concern
    // `print_on_deeply_nested_term_truncates_instead_of_crashing_node`
    // above documents for `toDisplayString`. See `diffTerm`'s own section
    // doc comment in `runtime.rs` for why this is NOT actually reachable
    // (verified, not assumed): every value `evalApply` hands to a
    // `HANDLERS` entry has already survived `evalTerm`'s own applicative-
    // order argument evaluation, which costs one recursive frame per
    // `Apply` level regardless of head, so it already hits the existing
    // `MAX_EVAL_DEPTH` cap (a HEAVIER per-frame cost than `diffTerm`'s own
    // walk) before `f` can ever reach `diffTerm` at all -- no additional
    // cap of `diffTerm`'s own is needed, and this test proves it directly
    // rather than leaving the claim unverified: a term 1,000
    // `Symbolic.apply` levels deep (built by an ordinary `for`-loop of
    // real *runtime* firings, NOT a hand-built giant static AST) that
    // never mentions `x` anywhere still differentiates correctly to the
    // constant `0`, with `node` exiting cleanly (`run_module`'s own
    // `output.status.success()` assertion) rather than crashing.
    //
    // acc = leaf; for i in range(0, 1000, 1) { acc = Symbolic-apply(f, [acc]) }
    // print(D(acc, x))
    let stmts = vec![
        Stmt::LetBinding {
            name: "acc".into(),
            sir_type: None,
            value: sym("leaf"),
            span: sp(),
        },
        Stmt::ForRange {
            var: "i".into(),
            start: int_lit(0),
            stop: int_lit(1000),
            step: int_lit(1),
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "acc".into(),
                    scope: Scope::Local,
                    value: sym_apply(sym("f"), vec![local("acc")]),
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            span: sp(),
        },
        print(sym_apply(sym("D"), vec![local("acc"), sym("x")])),
    ];
    let module = module_with_main(
        stmts,
        Expr::IntLit {
            value: 0,
            span: sp(),
        },
        &[
            Feature::SymbolicExpr,
            Feature::Loops,
            Feature::MutableBindings,
        ],
    );
    if let Some(stdout) = run_module(&module, "sym_differentiate_deep_runtime_term") {
        assert_eq!(stdout, "0");
    }
}
