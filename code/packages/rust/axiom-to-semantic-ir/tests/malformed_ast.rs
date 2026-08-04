//! Defensive-path coverage: `compile()` accepts an arbitrary, already-parsed
//! `GrammarASTNode` — not only trees `axiom-parser` itself produces. These
//! tests hand-build malformed or pathologically-deep trees directly
//! (bypassing `axiom-parser`'s own `MAX_RULE_DEPTH` guard entirely) to
//! exercise this crate's defensive "malformed node" / depth-guard branches
//! that no `compile_source` call through a real parse can ever reach —
//! adversarial-input testing in the same spirit as this repo's own
//! `lessons.md` guidance ("DoS guards need adversarial repro").
//!
//! None of these shapes can arise from `axiom-parser`'s own grammar (every
//! branch here defends against a hypothetical malformed or hand-built tree,
//! e.g. from a future alternate frontend reusing this crate's `compile`
//! entry point over some other CST) — but a public `compile(&GrammarASTNode,
//! &str)` API that panics or misbehaves on malformed input is a real
//! robustness gap, so this crate proactively returns a clean `Err` on all of
//! them instead.

use coding_adventures_axiom_to_semantic_ir::compile;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn leaf(rule_name: &str) -> GrammarASTNode {
    GrammarASTNode {
        rule_name: rule_name.to_string(),
        children: vec![],
        start_line: Some(1),
        start_column: Some(1),
        end_line: Some(1),
        end_column: Some(1),
    }
}

fn node(rule_name: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
    GrammarASTNode {
        rule_name: rule_name.to_string(),
        children,
        start_line: Some(1),
        start_column: Some(1),
        end_line: Some(1),
        end_column: Some(1),
    }
}

fn wrap_program(child: GrammarASTNode) -> GrammarASTNode {
    node("program", vec![ASTNodeOrToken::Node(child)])
}

fn tok(type_name: &str, value: &str) -> Token {
    Token {
        type_: TokenType::Name,
        value: value.to_string(),
        line: 1,
        column: 1,
        type_name: Some(type_name.to_string()),
        flags: None,
        cv: None,
    }
}

fn t(type_name: &str, value: &str) -> ASTNodeOrToken {
    ASTNodeOrToken::Token(tok(type_name, value))
}

// --- lower_program: wrong root / nested `program` --------------------------

#[test]
fn a_non_program_root_is_rejected() {
    let tree = leaf("not_program");
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("expected `program` root"), "{}", err.message);
}

#[test]
fn a_program_node_with_more_than_one_child_is_rejected_as_nested() {
    // `program` itself is meant to have exactly one child (the chosen `expr`
    // alternative); a >1-child `program` node stops `unwrap_single` from
    // peeling past it, landing squarely on `lower_node`'s own
    // `"program" => Err(...)` dispatch arm.
    let tree = node("program", vec![ASTNodeOrToken::Node(leaf("a")), ASTNodeOrToken::Node(leaf("b"))]);
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("nested program node"), "{}", err.message);
}

// --- lower_node: unknown rule name / non-expression rule names -------------

#[test]
fn an_unrecognised_rule_name_is_rejected() {
    let tree = wrap_program(leaf("totally_unknown_rule"));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("no lowering for rule"), "{}", err.message);
}

#[test]
fn call_args_cannot_be_lowered_as_a_standalone_expression() {
    let tree = wrap_program(leaf("call_args"));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("call_args"), "{}", err.message);
}

#[test]
fn arglist_cannot_be_lowered_as_a_standalone_expression() {
    let tree = wrap_program(leaf("arglist"));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("arglist"), "{}", err.message);
}

#[test]
fn elem_list_cannot_be_lowered_as_a_standalone_expression() {
    let tree = wrap_program(leaf("elem_list"));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("elem_list"), "{}", err.message);
}

#[test]
fn a_bare_type_expr_cannot_be_lowered_as_a_standalone_expression() {
    let tree = wrap_program(leaf("type_expr"));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("type expression"), "{}", err.message);
}

// --- lower_token: unexpected token type --------------------------------

#[test]
fn an_unexpected_token_type_is_rejected() {
    let tree = node("program", vec![t("SEMI", ";")]);
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("unexpected token"), "{}", err.message);
}

// --- lower_if: malformed shape --------------------------------------------

#[test]
fn an_if_expr_with_the_wrong_number_of_branches_is_rejected() {
    // A second, dummy child (a KEYWORD token) keeps `if_expr` from being a
    // single-child node -- `unwrap_single` would otherwise peel straight
    // through it into the lone `expr` child before `lower_node`'s dispatch
    // ever saw rule_name `"if_expr"` at all.
    let tree = wrap_program(node(
        "if_expr",
        vec![t("KEYWORD", "if"), ASTNodeOrToken::Node(node("expr", vec![t("NUMBER", "1")]))],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed `if`"), "{}", err.message);
}

// --- lower_declared_define / lower_undeclared_define: missing pieces -------

#[test]
fn a_declared_define_with_no_name_token_is_rejected() {
    let body = node("expr", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node(
        "declared_define",
        vec![t("LPAREN", "("), ASTNodeOrToken::Node(body)],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing name"), "{}", err.message);
}

#[test]
fn a_declared_define_with_no_body_is_rejected() {
    let tree = wrap_program(node("declared_define", vec![t("NAME", "f"), t("LPAREN", "(")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing body"), "{}", err.message);
}

#[test]
fn an_undeclared_define_with_the_wrong_name_token_count_is_rejected() {
    let tree = wrap_program(node("undeclared_define", vec![t("NAME", "f"), t("LPAREN", "(")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed undeclared function definition"), "{}", err.message);
}

#[test]
fn an_undeclared_define_with_no_body_is_rejected() {
    let tree = wrap_program(node(
        "undeclared_define",
        vec![t("NAME", "f"), t("NAME", "x")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing body"), "{}", err.message);
}

// --- lower_assignment: missing pieces --------------------------------------

#[test]
fn an_assignment_with_no_name_is_rejected() {
    let tree = wrap_program(node(
        "assignment",
        vec![t("LPAREN", "("), ASTNodeOrToken::Node(node("expr", vec![t("NUMBER", "1")]))],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing name"), "{}", err.message);
}

#[test]
fn an_assignment_with_no_right_hand_side_is_rejected() {
    let tree = wrap_program(node("assignment", vec![t("NAME", "x"), t("ASSIGN", ":=")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing right-hand side"), "{}", err.message);
}

// --- lower_declaration / lower_has_query: missing pieces -------------------

#[test]
fn a_declaration_with_no_target_is_rejected() {
    let type_expr = node("type_expr", vec![t("NAME", "Integer")]);
    let tree = wrap_program(node(
        "declaration",
        vec![t("COLON", ":"), ASTNodeOrToken::Node(type_expr)],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing target"), "{}", err.message);
}

#[test]
fn a_declaration_with_no_type_is_rejected() {
    let decl_target = node("decl_target", vec![t("NAME", "a")]);
    let tree = wrap_program(node(
        "declaration",
        vec![ASTNodeOrToken::Node(decl_target), t("COLON", ":")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing type"), "{}", err.message);
}

#[test]
fn a_has_query_with_the_wrong_number_of_type_exprs_is_rejected() {
    let type_expr = node("type_expr", vec![t("NAME", "Integer")]);
    let tree = wrap_program(node(
        "has_query",
        vec![ASTNodeOrToken::Node(type_expr), t("KEYWORD", "has")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed `has` query"), "{}", err.message);
}

// --- lower_coercion: missing target type ------------------------------------

#[test]
fn a_coercion_with_no_target_type_is_rejected() {
    let additive = node("additive", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node(
        "coercion",
        vec![ASTNodeOrToken::Node(additive), t("COERCE", "::")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("missing target type"), "{}", err.message);
}

// --- lower_comparison / lower_coercion: the defensive "no operator found"
// fallback to `lower_first_node` -- unreachable via any real parse (a
// single-child `comparison`/`coercion` node is already peeled away by
// `unwrap_single` before `lower_node` ever dispatches on its rule name), but
// a real robustness property of a public `compile` entry point that accepts
// an arbitrary hand-built tree.

#[test]
fn a_comparison_node_with_no_matching_operator_token_falls_back_to_its_first_child() {
    let coercion = node("coercion", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node(
        "comparison",
        vec![ASTNodeOrToken::Node(coercion), t("SEMI", ";")],
    ));
    let module = compile(&tree, "test").expect("should fall back to lowering the first child");
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 1);
}

#[test]
fn a_comparison_node_with_no_node_children_at_all_is_rejected() {
    let tree = wrap_program(node("comparison", vec![t("SEMI", ";"), t("COMMA", ",")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("has no expression child"), "{}", err.message);
}

#[test]
fn a_coercion_node_with_no_additive_child_falls_back_to_its_first_child() {
    // The fallback target must have >1 child of its own (else `unwrap_single`
    // would peel straight through it to its own bare NAME token, lowering
    // successfully) AND an unrecognised rule name, so `lower_first_node`'s
    // recursive dispatch on the fallback path (line ~706) genuinely fails.
    let undispatchable = node("some_undispatchable_rule", vec![t("NAME", "a"), t("SEMI", ";")]);
    let tree = wrap_program(node(
        "coercion",
        vec![ASTNodeOrToken::Node(undispatchable), t("SEMI", ";")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("no lowering for rule"), "{}", err.message);
}

// --- lower_comparison: operator at position 0 (malformed shape) ------------

#[test]
fn a_comparison_node_with_the_operator_in_the_first_position_is_rejected() {
    let coercion = node("coercion", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node(
        "comparison",
        vec![t("EQ", "="), ASTNodeOrToken::Node(coercion)],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed comparison node"), "{}", err.message);
}

// --- lower_postfix: a `call_args`-less second child (defensive fallback) ---

#[test]
fn a_postfix_node_with_no_call_args_child_returns_the_bare_atom() {
    let atom = node("atom", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node(
        "postfix",
        vec![ASTNodeOrToken::Node(atom), t("SEMI", ";")],
    ));
    let module = compile(&tree, "test").expect("should lower to the bare atom");
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 1);
}

// --- lower_atom: the list_literal/group short-circuit + single-token arm ---

#[test]
fn an_atom_node_wrapping_a_list_literal_child_directly_lowers_through_it() {
    let list_literal = node("list_literal", vec![t("LBRACKET", "["), t("RBRACKET", "]")]);
    let tree = wrap_program(node(
        "atom",
        vec![ASTNodeOrToken::Node(list_literal), t("SEMI", ";")],
    ));
    let module = compile(&tree, "test").expect("should lower through the list_literal child");
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 1);
}

#[test]
fn an_atom_node_with_one_non_list_group_node_child_and_one_token_lowers_the_token() {
    let tree = wrap_program(node(
        "atom",
        vec![ASTNodeOrToken::Node(leaf("some_other_rule")), t("NUMBER", "1")],
    ));
    let module = compile(&tree, "test").expect("should fall through to the lone token");
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 1);
}

// --- lower_unary: a bare-token operand (defensive robustness) --------------

#[test]
fn a_unary_operand_that_is_a_bare_token_still_lowers() {
    let tree = wrap_program(node("unary", vec![t("MINUS", "-"), t("NUMBER", "5")]));
    let module = compile(&tree, "test").expect("a bare-token unary operand should still lower");
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 1);
}

// --- lower_type_expr: MAX_EXPR_DEPTH guard via a hand-built deep chain -----
//
// `axiom-parser`'s own `MAX_RULE_DEPTH` (140) never lets a real parse build a
// `type_expr` chain deep enough to trip this crate's own `MAX_EXPR_DEPTH`
// (256) either -- mirrors the plain-expression depth-guard tests above.

fn deep_type_expr_chain(levels: usize) -> GrammarASTNode {
    let mut current = node("type_expr", vec![t("NAME", "Integer")]);
    for _ in 0..levels {
        let type_expr_list = node("type_expr_list", vec![ASTNodeOrToken::Node(current)]);
        let ctor_args = node(
            "type_ctor_args",
            vec![t("LPAREN", "("), ASTNodeOrToken::Node(type_expr_list), t("RPAREN", ")")],
        );
        current = node(
            "type_expr",
            vec![t("NAME", "List"), ASTNodeOrToken::Node(ctor_args)],
        );
    }
    current
}

#[test]
fn a_pathologically_deep_hand_built_type_expr_trips_its_own_depth_guard() {
    let decl_target = node("decl_target", vec![t("NAME", "a")]);
    let tree = wrap_program(node(
        "declaration",
        vec![ASTNodeOrToken::Node(decl_target), ASTNodeOrToken::Node(deep_type_expr_chain(300))],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("type expression nesting too deep"), "{}", err.message);
}

#[test]
fn a_moderately_deep_hand_built_type_expr_stays_under_its_own_depth_guard() {
    let decl_target = node("decl_target", vec![t("NAME", "a")]);
    let tree = wrap_program(node(
        "declaration",
        vec![ASTNodeOrToken::Node(decl_target), ASTNodeOrToken::Node(deep_type_expr_chain(5))],
    ));
    assert!(compile(&tree, "test").is_ok());
}

// --- lower_binary_chain: malformed shapes ----------------------------------

#[test]
fn a_binary_chain_with_a_trailing_operator_and_no_right_operand_is_rejected() {
    let operand = node("multiplicative", vec![t("NUMBER", "1")]);
    let tree = wrap_program(node("additive", vec![ASTNodeOrToken::Node(operand), t("PLUS", "+")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("no right operand"), "{}", err.message);
}

#[test]
fn a_binary_chain_with_an_unrecognised_operator_token_is_rejected() {
    let a = node("multiplicative", vec![t("NUMBER", "1")]);
    let b = node("multiplicative", vec![t("NUMBER", "2")]);
    let tree = wrap_program(node(
        "additive",
        vec![ASTNodeOrToken::Node(a), t("SEMI", ";"), ASTNodeOrToken::Node(b)],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("expected a binary operator"), "{}", err.message);
}

// --- lower_unary / lower_power: malformed shapes ---------------------------

#[test]
fn a_unary_node_with_the_wrong_child_count_is_rejected() {
    let tree = wrap_program(node(
        "unary",
        vec![t("MINUS", "-"), t("NUMBER", "1"), t("NUMBER", "2")],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed unary"), "{}", err.message);
}

#[test]
fn a_power_node_with_a_non_pow_operator_token_is_rejected() {
    let base = node("postfix", vec![t("NUMBER", "1")]);
    let exp = node("unary", vec![t("NUMBER", "2")]);
    let tree = wrap_program(node(
        "power",
        vec![ASTNodeOrToken::Node(base), t("SEMI", ";"), ASTNodeOrToken::Node(exp)],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("expected CARET or POW"), "{}", err.message);
}

#[test]
fn a_power_node_with_the_wrong_child_count_is_rejected() {
    let base = node("postfix", vec![t("NUMBER", "1")]);
    let mid = node("unary", vec![t("NUMBER", "2")]);
    let extra = node("unary", vec![t("NUMBER", "3")]);
    let tree = wrap_program(node(
        "power",
        vec![
            ASTNodeOrToken::Node(base),
            t("CARET", "^"),
            ASTNodeOrToken::Node(mid),
            ASTNodeOrToken::Node(extra),
        ],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed power node"), "{}", err.message);
}

// --- lower_atom: unrecognised token shape -----------------------------------

#[test]
fn an_atom_with_an_unrecognised_token_shape_is_rejected() {
    let tree = wrap_program(node("atom", vec![t("SEMI", ";"), t("SEMI", ";")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("unrecognised atom token shape"), "{}", err.message);
}

// --- lower_group: empty group ------------------------------------------------

#[test]
fn an_empty_group_is_rejected() {
    let tree = wrap_program(node("group", vec![t("LPAREN", "("), t("RPAREN", ")")]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("empty group"), "{}", err.message);
}

// --- lower_type_expr / lower_type_ctor_args: malformed shapes ---------------

#[test]
fn a_type_expr_with_no_name_token_is_rejected() {
    let tree = wrap_program(node("declaration", vec![
        ASTNodeOrToken::Node(node("decl_target", vec![t("NAME", "a")])),
        ASTNodeOrToken::Node(node("type_expr", vec![t("LPAREN", "(")])),
    ]));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed type expression"), "{}", err.message);
}

#[test]
fn a_paren_optional_type_ctor_arg_with_no_name_token_is_rejected() {
    let ctor_args = node("type_ctor_args", vec![t("SEMI", ";")]);
    let type_expr = node("type_expr", vec![t("NAME", "Fraction"), ASTNodeOrToken::Node(ctor_args)]);
    let tree = wrap_program(node(
        "declaration",
        vec![
            ASTNodeOrToken::Node(node("decl_target", vec![t("NAME", "a")])),
            ASTNodeOrToken::Node(type_expr),
        ],
    ));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("malformed paren-optional type argument"), "{}", err.message);
}

// --- MAX_EXPR_DEPTH guard: a pathologically deep hand-built tree ------------
//
// `axiom-parser`'s own `MAX_RULE_DEPTH` (140) never lets a real parse build a
// CST deep enough to trip this crate's own `MAX_EXPR_DEPTH` (256) -- so this
// guard's only adversarial-input test is a directly hand-built tree that
// bypasses the parser entirely, mirroring how a hostile or buggy alternate
// CST producer might reuse this crate's public `compile` entry point.

fn deep_unary_chain(levels: usize) -> GrammarASTNode {
    let mut current = node("power", vec![t("NUMBER", "1")]);
    for _ in 0..levels {
        current = node("unary", vec![t("MINUS", "-"), ASTNodeOrToken::Node(current)]);
    }
    current
}

#[test]
fn a_pathologically_deep_hand_built_tree_trips_the_depth_guard() {
    let tree = wrap_program(deep_unary_chain(300));
    let err = compile(&tree, "test").unwrap_err();
    assert!(err.message.contains("too deep"), "{}", err.message);
}

#[test]
fn a_moderately_deep_hand_built_tree_stays_under_the_depth_guard() {
    let tree = wrap_program(deep_unary_chain(10));
    assert!(compile(&tree, "test").is_ok());
}

// --- AxiomLowerError::Display -------------------------------------------

#[test]
fn the_error_display_impl_includes_position_and_message() {
    let tree = leaf("not_program");
    let err = compile(&tree, "test").unwrap_err();
    let rendered = format!("{err}");
    assert!(rendered.contains("AxiomLowerError"));
    assert!(rendered.contains("expected `program` root"));
}
