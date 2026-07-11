//! Unit tests asserting exact `Expr` shapes produced by
//! `wolfram_to_semantic_ir::compile_source` — one per grammar production,
//! mirroring `matlab-to-semantic-ir`'s own `tests/test_lower.rs` structure.

use semantic_ir::{Expr, Feature, Module, Stmt};
use wolfram_to_semantic_ir::compile_source;

/// Lower a single-statement source to its one top-level `Expr`.
fn lower_one(src: &str) -> Expr {
    let module = compile_source(src, "test").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let main = module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("no main function");
    assert_eq!(
        main.body.stmts.len(),
        1,
        "expected exactly one statement for {src:?}, got {}",
        main.body.stmts.len()
    );
    match &main.body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => expr.clone(),
        other => panic!("expected an ExprStmt, got {other:?}"),
    }
}

fn manifest_has(module: &Module, feature: Feature) -> bool {
    module.manifest.iter().any(|f| f == feature)
}

fn sym(name: &str) -> Expr {
    Expr::SymSymbol {
        name: name.to_string(),
        span: semantic_ir::Span::synthetic(),
    }
}

fn int(v: i64) -> Expr {
    Expr::IntLit {
        value: v,
        span: semantic_ir::Span::synthetic(),
    }
}

fn apply(head: Expr, args: Vec<Expr>) -> Expr {
    Expr::SymApply {
        head: Box::new(head),
        args,
        span: semantic_ir::Span::synthetic(),
    }
}

/// Equality that ignores spans (every helper above uses a synthetic span;
/// real lowered spans point at real source positions) — recursively zero
/// out spans on both sides before comparing.
fn assert_shape_eq(actual: Expr, expected: Expr) {
    assert_eq!(strip_spans(actual), strip_spans(expected));
}

fn strip_spans(expr: Expr) -> Expr {
    let z = semantic_ir::Span::synthetic();
    match expr {
        Expr::IntLit { value, .. } => Expr::IntLit { value, span: z },
        Expr::FloatLit { value, .. } => Expr::FloatLit { value, span: z },
        Expr::StrLit { value, .. } => Expr::StrLit { value, span: z },
        Expr::SymSymbol { name, .. } => Expr::SymSymbol { name, span: z },
        Expr::SymRational { numer, denom, .. } => Expr::SymRational { numer, denom, span: z },
        Expr::SymApply { head, args, .. } => Expr::SymApply {
            head: Box::new(strip_spans(*head)),
            args: args.into_iter().map(strip_spans).collect(),
            span: z,
        },
        Expr::SymPatternBlank { head, .. } => Expr::SymPatternBlank {
            head: head.map(|h| Box::new(strip_spans(*h))),
            span: z,
        },
        Expr::SymPatternNamed { name, pattern, .. } => Expr::SymPatternNamed {
            name,
            pattern: Box::new(strip_spans(*pattern)),
            span: z,
        },
        Expr::SymRule { lhs, rhs, delayed, .. } => Expr::SymRule {
            lhs: Box::new(strip_spans(*lhs)),
            rhs: Box::new(strip_spans(*rhs)),
            delayed,
            span: z,
        },
        Expr::SymReplaceAll { expr, rules, repeated, .. } => Expr::SymReplaceAll {
            expr: Box::new(strip_spans(*expr)),
            rules: rules.into_iter().map(strip_spans).collect(),
            repeated,
            span: z,
        },
        other => other,
    }
}

// --- literals ---------------------------------------------------------

#[test]
fn integer_and_real_literals() {
    assert_shape_eq(lower_one("42\n"), int(42));
    assert_shape_eq(
        lower_one("1.5\n"),
        Expr::FloatLit {
            value: 1.5,
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn string_literal_loses_its_quotes() {
    assert_shape_eq(
        lower_one("\"hello\"\n"),
        Expr::StrLit {
            value: "hello".to_string(),
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn bare_symbol_is_symbolic_data() {
    let module = compile_source("foo\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert_shape_eq(lower_one("foo\n"), sym("foo"));
}

// --- arithmetic ---------------------------------------------------------

#[test]
fn additive_lowers_to_add_apply() {
    assert_shape_eq(lower_one("1 + 2\n"), apply(sym("Add"), vec![int(1), int(2)]));
}

#[test]
fn subtraction_is_left_associative() {
    assert_shape_eq(
        lower_one("a - b - c\n"),
        apply(sym("Sub"), vec![apply(sym("Sub"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn multiplication_and_division() {
    assert_shape_eq(lower_one("a * b\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a / b\n"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_is_right_associative() {
    assert_shape_eq(
        lower_one("a ^ b ^ c\n"),
        apply(sym("Pow"), vec![sym("a"), apply(sym("Pow"), vec![sym("b"), sym("c")])]),
    );
}

#[test]
fn unary_minus_is_neg_and_plus_is_noop() {
    assert_shape_eq(lower_one("-x\n"), apply(sym("Neg"), vec![sym("x")]));
    assert_shape_eq(lower_one("+x\n"), sym("x"));
}

#[test]
fn explicit_head_application_is_bridged_and_left_folded() {
    // Plus[1, 2, 3] -> Add(Add(1, 2), 3), byte-identical to 1 + 2 + 3.
    assert_shape_eq(
        lower_one("Plus[1, 2, 3]\n"),
        apply(sym("Add"), vec![apply(sym("Add"), vec![int(1), int(2)]), int(3)]),
    );
    assert_shape_eq(lower_one("Times[a, b]\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("Power[2, 10]\n"), apply(sym("Pow"), vec![int(2), int(10)]));
}

#[test]
fn sin_is_already_canonical_and_unknown_heads_pass_through() {
    assert_shape_eq(lower_one("Sin[x]\n"), apply(sym("Sin"), vec![sym("x")]));
    assert_shape_eq(
        lower_one("f[x, y]\n"),
        apply(sym("f"), vec![sym("x"), sym("y")]),
    );
    assert_shape_eq(lower_one("f[]\n"), apply(sym("f"), vec![]));
}

#[test]
fn nested_application_is_left_associative() {
    assert_shape_eq(
        lower_one("f[x][y]\n"),
        apply(apply(sym("f"), vec![sym("x")]), vec![sym("y")]),
    );
}

// --- comparisons / logic -------------------------------------------------

#[test]
fn comparisons_lower_to_their_canonical_heads() {
    assert_shape_eq(lower_one("a == b\n"), apply(sym("Equal"), vec![sym("a"), sym("b")]));
    assert_shape_eq(
        lower_one("a <= b\n"),
        apply(sym("LessEqual"), vec![sym("a"), sym("b")]),
    );
}

#[test]
fn logic_chains_and_not() {
    assert_shape_eq(
        lower_one("a && b || c\n"),
        apply(sym("Or"), vec![apply(sym("And"), vec![sym("a"), sym("b")]), sym("c")]),
    );
    assert_shape_eq(lower_one("!x\n"), apply(sym("Not"), vec![sym("x")]));
}

// --- lists / grouping -----------------------------------------------------

#[test]
fn list_literal_lowers_to_list_head() {
    assert_shape_eq(lower_one("{1, 2, 3}\n"), apply(sym("List"), vec![int(1), int(2), int(3)]));
    assert_shape_eq(lower_one("{}\n"), apply(sym("List"), vec![]));
}

#[test]
fn grouping_parens_are_transparent() {
    assert_shape_eq(
        lower_one("(a + b) * c\n"),
        apply(sym("Mul"), vec![apply(sym("Add"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

// --- assignment: pure data, no host binding -----------------------------

#[test]
fn set_lowers_to_assign_apply() {
    assert_shape_eq(lower_one("x = 5\n"), apply(sym("Assign"), vec![sym("x"), int(5)]));
    let module = compile_source("x = 5\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
}

#[test]
fn setdelayed_lowers_to_define_apply() {
    assert_shape_eq(
        lower_one("f[x] := x\n"),
        apply(sym("Define"), vec![apply(sym("f"), vec![sym("x")]), sym("x")]),
    );
}

// --- patterns -------------------------------------------------------------

#[test]
fn bare_blank_lowers_to_pattern_blank() {
    let module = compile_source("_\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::PatternMatching));
    let expr = lower_one("_\n");
    assert!(matches!(expr, Expr::SymPatternBlank { head: None, .. }));
}

#[test]
fn head_constrained_blank_lowers_with_head() {
    let expr = lower_one("_Integer\n");
    match expr {
        Expr::SymPatternBlank { head: Some(h), .. } => {
            assert_eq!(strip_spans(*h), sym("Integer"));
        }
        other => panic!("expected SymPatternBlank with a head, got {other:?}"),
    }
}

#[test]
fn named_pattern_lowers_to_pattern_named() {
    let expr = lower_one("x_\n");
    match expr {
        Expr::SymPatternNamed { name, pattern, .. } => {
            assert_eq!(name, "x");
            assert!(matches!(*pattern, Expr::SymPatternBlank { head: None, .. }));
        }
        other => panic!("expected SymPatternNamed, got {other:?}"),
    }
}

#[test]
fn named_head_constrained_pattern() {
    let expr = lower_one("x_Integer\n");
    match expr {
        Expr::SymPatternNamed { name, pattern, .. } => {
            assert_eq!(name, "x");
            match *pattern {
                Expr::SymPatternBlank { head: Some(h), .. } => {
                    assert_eq!(strip_spans(*h), sym("Integer"));
                }
                other => panic!("expected a head-constrained blank, got {other:?}"),
            }
        }
        other => panic!("expected SymPatternNamed, got {other:?}"),
    }
}

// --- rules and replacement ------------------------------------------------

#[test]
fn rule_lowers_to_sym_rule() {
    let expr = lower_one("a -> b\n");
    match expr {
        Expr::SymRule { lhs, rhs, delayed, .. } => {
            assert_eq!(strip_spans(*lhs), sym("a"));
            assert_eq!(strip_spans(*rhs), sym("b"));
            assert!(!delayed);
        }
        other => panic!("expected SymRule, got {other:?}"),
    }
    let module = compile_source("a -> b\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::PatternMatching));
}

#[test]
fn ruledelayed_sets_the_delayed_flag() {
    let expr = lower_one("a :> b\n");
    match expr {
        Expr::SymRule { delayed, .. } => assert!(delayed),
        other => panic!("expected SymRule, got {other:?}"),
    }
}

#[test]
fn rule_rewrites_bound_pattern_names_on_the_rhs() {
    // x_ -> x + 1: the bare `x` reference on the RHS must be rewritten into
    // the same SymPatternNamed reference shape a fresh `x_` produces, so a
    // later matcher's substitution step fills it in.
    let expr = lower_one("x_ -> x + 1\n");
    let Expr::SymRule { rhs, .. } = expr else {
        panic!("expected SymRule");
    };
    let Expr::SymApply { args, .. } = *rhs else {
        panic!("expected the rhs to be Add(x, 1)");
    };
    assert!(matches!(&args[0], Expr::SymPatternNamed { name, .. } if name == "x"));
}

#[test]
fn replaceall_lowers_to_sym_replaceall_one_pass() {
    let expr = lower_one("x /. a -> b\n");
    match expr {
        Expr::SymReplaceAll { repeated, rules, .. } => {
            assert!(!repeated);
            assert_eq!(rules.len(), 1);
        }
        other => panic!("expected SymReplaceAll, got {other:?}"),
    }
}

#[test]
fn replacerepeated_sets_the_repeated_flag() {
    let expr = lower_one("x //. a -> b\n");
    match expr {
        Expr::SymReplaceAll { repeated, .. } => assert!(repeated),
        other => panic!("expected SymReplaceAll, got {other:?}"),
    }
}

#[test]
fn replaceall_flattens_a_list_of_rules() {
    let expr = lower_one("x /. {a -> b, c -> d}\n");
    match expr {
        Expr::SymReplaceAll { rules, .. } => assert_eq!(rules.len(), 2),
        other => panic!("expected SymReplaceAll, got {other:?}"),
    }
}

#[test]
fn condition_test_keeps_bare_symbol_references() {
    // Unlike a Rule's rhs, Condition's test must NOT have its bare `x`
    // rewritten into pattern-reference form.
    let expr = lower_one("x_ /; x > 2\n");
    let Expr::SymApply { head, args, .. } = expr else {
        panic!("expected a Condition SymApply");
    };
    assert_eq!(strip_spans(*head), sym("Condition"));
    let Expr::SymApply { args: test_args, .. } = &args[1] else {
        panic!("expected the test to be Greater(x, 2)");
    };
    assert_eq!(strip_spans(test_args[0].clone()), sym("x"));
}

// --- W-6 sugar: /@ @@ [[ ]] ------------------------------------------------

#[test]
fn map_and_apply_sugar_lower_to_their_long_form_heads() {
    assert_shape_eq(lower_one("f /@ x\n"), apply(sym("Map"), vec![sym("f"), sym("x")]));
    assert_shape_eq(lower_one("f @@ x\n"), apply(sym("Apply"), vec![sym("f"), sym("x")]));
}

#[test]
fn double_bracket_lowers_to_part() {
    assert_shape_eq(lower_one("x[[2]]\n"), apply(sym("Part"), vec![sym("x"), int(2)]));
}

#[test]
fn multi_index_double_bracket_folds_into_nested_part() {
    assert_shape_eq(
        lower_one("m[[1, 2]]\n"),
        apply(sym("Part"), vec![apply(sym("Part"), vec![sym("m"), int(1)]), int(2)]),
    );
}

// --- W-11 pure functions ----------------------------------------------------

#[test]
fn slot_forms_lower_to_slot_apply() {
    assert_shape_eq(lower_one("#\n"), apply(sym("Slot"), vec![int(1)]));
    assert_shape_eq(lower_one("#2\n"), apply(sym("Slot"), vec![int(2)]));
    assert_shape_eq(lower_one("##\n"), apply(sym("SlotSequence"), vec![int(1)]));
}

#[test]
fn ampersand_wraps_the_body_in_function() {
    assert_shape_eq(
        lower_one("#^2 &\n"),
        apply(
            sym("Function"),
            vec![apply(sym("Pow"), vec![apply(sym("Slot"), vec![int(1)]), int(2)])],
        ),
    );
}

#[test]
fn pure_function_applied_immediately() {
    assert_shape_eq(
        lower_one("#&[9]\n"),
        apply(apply(sym("Function"), vec![apply(sym("Slot"), vec![int(1)])]), vec![int(9)]),
    );
}

#[test]
fn named_function_long_form_normalises_a_single_param_to_a_list() {
    assert_shape_eq(
        lower_one("Function[x, x^2]\n"),
        apply(
            sym("Function"),
            vec![
                apply(sym("List"), vec![sym("x")]),
                apply(sym("Pow"), vec![sym("x"), int(2)]),
            ],
        ),
    );
}

// --- W-21 sugar: | /; ? --------------------------------------------------

#[test]
fn alternatives_folds_into_one_n_ary_apply() {
    assert_shape_eq(
        lower_one("a | b | c\n"),
        apply(sym("Alternatives"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn patterntest_lowers_and_chains_left_associatively() {
    assert_shape_eq(
        lower_one("_?IntegerQ?Positive\n"),
        apply(
            sym("PatternTest"),
            vec![
                apply(sym("PatternTest"), vec![Expr::SymPatternBlank { head: None, span: semantic_ir::Span::synthetic() }, sym("IntegerQ")]),
                sym("Positive"),
            ],
        ),
    );
}

// --- error cases ------------------------------------------------------------

#[test]
fn a_syntax_error_is_reported_as_a_lower_error() {
    assert!(compile_source("1 +\n", "test").is_err());
}

#[test]
fn a_small_wolfram_program_compiles() {
    let module = compile_source(
        "f[x_] := x^2\nf[3] + Sin[0]\n{1, 2, 3} /. a_ -> a + 1\n",
        "test",
    )
    .unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
}

#[test]
fn multiple_top_level_statements_each_become_an_expr_stmt() {
    let module = compile_source("1\n2\n3\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
    assert!(matches!(main.body.value, Expr::NilLit { .. }));
}

// --- recursion-depth / chain-length guards (DoS hardening) -----------------
//
// These mirror the exact regression scale `matlab-to-semantic-ir`'s own
// security review established (60,000 flat operands) rather than a smaller,
// less convincing number.

#[test]
fn a_deeply_parenthesised_expression_is_rejected_cleanly() {
    let nesting = 5_000;
    let src = format!("{}0{}\n", "(".repeat(nesting), ")".repeat(nesting));
    // The parser's own MAX_RULE_DEPTH trips first on input this deep; either
    // way this must be a clean Err, never a crash (the surrounding test
    // process itself is the proof: if this overflowed the stack, the whole
    // test binary would abort, not fail one assertion).
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_huge_flat_additive_chain_is_rejected_cleanly_not_crashed() {
    let terms = 60_000;
    let src = format!("{}\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" + "));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_huge_flat_alternatives_chain_is_rejected_cleanly() {
    let terms = 60_000;
    let src = format!("{}\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" | "));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_chain_at_exactly_the_cap_still_compiles() {
    // MAX_EXPR_DEPTH is 256; 256 operands is exactly at the cap (255
    // operators), one more trips it.
    let ok_terms = 256;
    let ok_src = format!("{}\n", (0..ok_terms).map(|_| "1").collect::<Vec<_>>().join(" + "));
    assert!(compile_source(&ok_src, "test").is_ok());

    let bad_terms = 257;
    let bad_src = format!("{}\n", (0..bad_terms).map(|_| "1").collect::<Vec<_>>().join(" + "));
    assert!(compile_source(&bad_src, "test").is_err());
}

// A chained application/part/pure-function-apply run is iterative in
// `lower_postfix`/`lower_amp`, not recursive through `lower_node` -- it
// never engages `MAX_EXPR_DEPTH`'s own recursion check no matter how many
// groups there are, which is a different grammar shape from the flat
// operator chains above (a run of TOKENS with an optional trailing
// `arglist` node each, not a run of `Node` operands). Found during
// security review, after the initial per-production chain-length audit
// above missed it — see `add_chain_depth` in `src/lower.rs`.

#[test]
fn a_huge_chained_bracket_application_is_rejected_cleanly_not_crashed() {
    let chains = 60_000;
    let src = format!("x{}\n", "[0]".repeat(chains));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_huge_chained_double_bracket_part_is_rejected_cleanly_not_crashed() {
    let chains = 60_000;
    let src = format!("x{}\n", "[[0]]".repeat(chains));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_huge_chained_pure_function_amp_apply_is_rejected_cleanly_not_crashed() {
    let chains = 60_000;
    let src = format!("(#&){}\n", "[0]".repeat(chains));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_huge_chained_ampersand_run_is_rejected_cleanly_not_crashed() {
    let count = 60_000;
    let src = format!("x{}\n", " &".repeat(count));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_bracket_chain_at_exactly_the_cap_still_compiles() {
    let ok_chains = 256;
    let ok_src = format!("x{}\n", "[0]".repeat(ok_chains));
    assert!(compile_source(&ok_src, "test").is_ok());

    let bad_chains = 257;
    let bad_src = format!("x{}\n", "[0]".repeat(bad_chains));
    assert!(compile_source(&bad_src, "test").is_err());
}

// A prior version of the chained-application guard capped "how many
// bracket groups" and "how many indices per group" as two INDEPENDENT
// counts, each bounded to MAX_EXPR_DEPTH. That looks safe per-axis but the
// two axes multiply: an LDBRACKET group folds one Part per index, so N
// chained groups each carrying M indices builds N*M levels of nesting, not
// N. Found during round 2 of security review — see `add_chain_depth`'s
// doc comment in src/lower.rs.

#[test]
fn a_multiplicative_bracket_times_index_combination_is_rejected_cleanly() {
    // 256 chained [[..]] groups, each with 256 indices: the old per-axis
    // caps (group_count <= 256 AND indices_per_group <= 256) both
    // individually passed, but the real nesting depth was 256*256 = 65536.
    let indices = (0..256).map(|_| "1").collect::<Vec<_>>().join(",");
    let group = format!("[[{indices}]]");
    let src = format!("x{}\n", group.repeat(256));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_cumulative_chain_depth_at_exactly_the_cap_still_compiles() {
    // A single group with exactly MAX_EXPR_DEPTH (256) indices should still
    // compile (256 <= 256); one more group of any size should not.
    let indices = (0..256).map(|_| "1").collect::<Vec<_>>().join(",");
    let ok_src = format!("x[[{indices}]]\n");
    assert!(compile_source(&ok_src, "test").is_ok());

    let bad_src = format!("x[[{indices}]][0]\n");
    assert!(compile_source(&bad_src, "test").is_err());
}
