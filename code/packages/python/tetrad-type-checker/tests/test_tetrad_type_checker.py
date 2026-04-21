"""Tests for tetrad_type_checker.check() and check_source().

Coverage target: ≥95%.
"""

from __future__ import annotations

import pytest
from tetrad_parser import parse
from tetrad_parser.ast import BinaryExpr, CallExpr

from tetrad_type_checker import (
    TypeCheckResult,
    TypeWarning,
    check,
    check_source,
)
from tetrad_type_checker.types import FunctionTypeStatus

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def tc(src: str) -> TypeCheckResult:
    return check(parse(src))


def has_error(result: TypeCheckResult, fragment: str) -> bool:
    return any(fragment in e.message for e in result.errors)


def has_warning(result: TypeCheckResult, fragment: str) -> bool:
    return any(fragment in w.message for w in result.warnings)


# ---------------------------------------------------------------------------
# 1. Empty and trivial programs
# ---------------------------------------------------------------------------


def test_empty_program() -> None:
    result = tc("")
    assert result.errors == []
    assert result.warnings == []


def test_result_has_program() -> None:
    result = tc("let x = 1;")
    assert result.program is not None


def test_type_map_populated() -> None:
    result = tc("let x = 42;")
    assert len(result.type_map) > 0


# ---------------------------------------------------------------------------
# 2. Literal type inference
# ---------------------------------------------------------------------------


def test_int_literal_is_u8() -> None:
    prog = parse("let x = 42;")
    result = check(prog)
    decl = prog.decls[0]
    lit = decl.value  # type: ignore[union-attr]
    info = result.type_map[id(lit)]
    assert info.ty == "u8"
    assert info.source == "inferred"


def test_hex_literal_is_u8() -> None:
    prog = parse("let x = 0xFF;")
    result = check(prog)
    decl = prog.decls[0]
    lit = decl.value  # type: ignore[union-attr]
    info = result.type_map[id(lit)]
    assert info.ty == "u8"


# ---------------------------------------------------------------------------
# 3. Variable type inference
# ---------------------------------------------------------------------------


def test_let_untyped_infers_u8_from_literal() -> None:
    prog = parse("fn f() { let x = 1; }")
    result = check(prog)
    assert result.errors == []
    env = result.env
    # The function was checked; x was bound in local_env, not global env
    # We verify through function_status
    assert "f" in env.function_status


def test_let_typed_annotation_stored() -> None:
    prog = parse("fn f() { let x: u8 = 1; }")
    result = check(prog)
    assert result.errors == []


def test_name_expr_known_type() -> None:
    prog = parse("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    result = check(prog)
    decl = prog.decls[0]
    body_stmt = decl.body.stmts[0]
    add_expr = body_stmt.value  # type: ignore[union-attr]  # ReturnStmt.value
    # a + b should be u8 since both a and b are u8
    info = result.type_map[id(add_expr)]
    assert info.ty == "u8"


# ---------------------------------------------------------------------------
# 4. Binary expression type inference
# ---------------------------------------------------------------------------


def test_u8_plus_u8_is_u8() -> None:
    prog = parse("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    result = check(prog)
    decl = prog.decls[0]
    add = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(add)].ty == "u8"


def test_unknown_plus_u8_is_unknown() -> None:
    prog = parse("fn f(a, b: u8) { let x = a + b; }")
    result = check(prog)
    decl = prog.decls[0]
    let_stmt = decl.body.stmts[0]
    add = let_stmt.value  # type: ignore[union-attr]
    assert isinstance(add, BinaryExpr)
    assert result.type_map[id(add)].ty == "Unknown"


def test_unknown_plus_unknown_is_unknown() -> None:
    prog = parse("fn f(a, b) { let x = a + b; }")
    result = check(prog)
    decl = prog.decls[0]
    add = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(add)].ty == "Unknown"


def test_comparison_always_u8() -> None:
    # Comparison result is always u8 (0 or 1) regardless of operands
    prog = parse("fn f(a, b) { let x = a == b; }")
    result = check(prog)
    decl = prog.decls[0]
    eq_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(eq_expr)].ty == "u8"


def test_all_comparisons_are_u8() -> None:
    for op in ["==", "!=", "<", "<=", ">", ">="]:
        prog = parse(f"fn f(a, b) {{ let x = a {op} b; }}")
        result = check(prog)
        decl = prog.decls[0]
        cmp_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
        assert result.type_map[id(cmp_expr)].ty == "u8", f"comparison {op} should be u8"


def test_logical_and_is_u8() -> None:
    prog = parse("fn f(a, b) { let x = a && b; }")
    result = check(prog)
    decl = prog.decls[0]
    expr_node = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(expr_node)].ty == "u8"


def test_logical_or_is_u8() -> None:
    prog = parse("fn f(a, b) { let x = a || b; }")
    result = check(prog)
    decl = prog.decls[0]
    expr_node = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(expr_node)].ty == "u8"


# ---------------------------------------------------------------------------
# 5. Unary expression type inference
# ---------------------------------------------------------------------------


def test_unary_not_is_u8() -> None:
    prog = parse("fn f(a) { let x = !a; }")
    result = check(prog)
    decl = prog.decls[0]
    unary = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(unary)].ty == "u8"


def test_unary_bitwise_not_is_u8() -> None:
    prog = parse("fn f(a) { let x = ~a; }")
    result = check(prog)
    decl = prog.decls[0]
    unary = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(unary)].ty == "u8"


def test_unary_negate_preserves_type() -> None:
    prog = parse("fn f(a: u8) { let x = -a; }")
    result = check(prog)
    decl = prog.decls[0]
    unary = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(unary)].ty == "u8"


def test_unary_negate_unknown_stays_unknown() -> None:
    prog = parse("fn f(a) { let x = -a; }")
    result = check(prog)
    decl = prog.decls[0]
    unary = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(unary)].ty == "Unknown"


# ---------------------------------------------------------------------------
# 6. Call expression type inference
# ---------------------------------------------------------------------------


def test_call_typed_return() -> None:
    prog = parse(
        "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
        "fn f() { let x = add(1, 2); }"
    )
    result = check(prog)
    f_decl = prog.decls[1]
    call_expr = f_decl.body.stmts[0].value  # type: ignore[union-attr]
    assert isinstance(call_expr, CallExpr)
    assert result.type_map[id(call_expr)].ty == "u8"


def test_call_untyped_return_is_unknown() -> None:
    prog = parse("fn add(a, b) { return a + b; }\nfn f() { let x = add(1, 2); }")
    result = check(prog)
    f_decl = prog.decls[1]
    call_expr = f_decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(call_expr)].ty == "Unknown"


def test_call_unknown_function_is_unknown() -> None:
    # Calling an undeclared function — type checker returns Unknown
    # (compiler will reject this, not the type checker)
    prog = parse("fn f() { let x = unknown_fn(1); }")
    result = check(prog)
    decl = prog.decls[0]
    call_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(call_expr)].ty == "Unknown"


# ---------------------------------------------------------------------------
# 7. in() and out() types
# ---------------------------------------------------------------------------


def test_in_expr_is_unknown() -> None:
    prog = parse("fn f() { let x = in(); }")
    result = check(prog)
    decl = prog.decls[0]
    in_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(in_expr)].ty == "Unknown"


def test_out_expr_is_void() -> None:
    prog = parse("fn f() { out(42); }")
    result = check(prog)
    decl = prog.decls[0]
    out_expr = decl.body.stmts[0].expr  # type: ignore[union-attr]
    assert result.type_map[id(out_expr)].ty == "Void"


# ---------------------------------------------------------------------------
# 8. Group expression
# ---------------------------------------------------------------------------


def test_group_preserves_type() -> None:
    prog = parse("fn f(a: u8) { let x = (a); }")
    result = check(prog)
    decl = prog.decls[0]
    group = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(group)].ty == "u8"


# ---------------------------------------------------------------------------
# 9. Function status classification
# ---------------------------------------------------------------------------


def test_fully_typed_function() -> None:
    result = tc("fn add(a: u8, b: u8) -> u8 { return a + b; }")
    assert result.env.function_status["add"] is FunctionTypeStatus.FULLY_TYPED


def test_untyped_function() -> None:
    result = tc("fn add(a, b) { return a + b; }")
    assert result.env.function_status["add"] is FunctionTypeStatus.UNTYPED


def test_partial_params_only() -> None:
    result = tc("fn f(a: u8, b) { return a; }")
    assert result.env.function_status["f"] is FunctionTypeStatus.PARTIALLY_TYPED


def test_partial_return_only() -> None:
    result = tc("fn f(a) -> u8 { return 0; }")
    assert result.env.function_status["f"] is FunctionTypeStatus.PARTIALLY_TYPED


def test_fully_typed_with_unknown_body_is_partial() -> None:
    # fn calls untyped callee → result of call is Unknown → PARTIALLY_TYPED
    result = tc(
        "fn helper(a, b) { return a; }\n"
        "fn f(a: u8, b: u8) -> u8 { return helper(a, b); }"
    )
    assert result.env.function_status["f"] is FunctionTypeStatus.PARTIALLY_TYPED


def test_multiple_functions_classified() -> None:
    result = tc(
        "fn typed(a: u8) -> u8 { return a; }\n"
        "fn untyped(a) { return a; }"
    )
    assert result.env.function_status["typed"] is FunctionTypeStatus.FULLY_TYPED
    assert result.env.function_status["untyped"] is FunctionTypeStatus.UNTYPED


# ---------------------------------------------------------------------------
# 10. Warnings
# ---------------------------------------------------------------------------


def test_warning_for_untyped_function() -> None:
    result = tc("fn f(a, b) { return a; }")
    assert has_warning(result, "JIT warmup required")


def test_no_warning_for_fully_typed() -> None:
    result = tc("fn f(a: u8) -> u8 { return a; }")
    assert not has_warning(result, "JIT warmup required")


def test_warning_calls_untyped_from_typed() -> None:
    result = tc(
        "fn helper(a) { return a; }\n"
        "fn f(a: u8, b: u8) -> u8 { return helper(a); }"
    )
    assert has_warning(result, "downgraded to PARTIALLY_TYPED")


def test_warning_has_hint() -> None:
    result = tc("fn f(a) { return a; }")
    w = result.warnings[0]
    assert isinstance(w, TypeWarning)
    assert w.hint != ""


# ---------------------------------------------------------------------------
# 11. Hard errors
# ---------------------------------------------------------------------------


def test_error_let_u8_assigned_unknown() -> None:
    result = tc("fn f() { let x: u8 = in(); }")
    assert has_error(result, "Unknown")


def test_error_return_unknown_from_typed_fn() -> None:
    result = tc("fn f() -> u8 { return in(); }")
    assert has_error(result, "unknown type")


def test_error_global_u8_assigned_unknown() -> None:
    result = tc("let x: u8 = in();")
    # in() is not legal in global context (no function), but the type checker
    # just flags the Unknown assignment
    assert len(result.errors) > 0


def test_no_error_for_well_typed_program() -> None:
    result = tc("fn add(a: u8, b: u8) -> u8 { return a + b; }")
    assert result.errors == []


def test_no_error_for_untyped_program() -> None:
    result = tc("fn add(a, b) { return a + b; }")
    assert result.errors == []


# ---------------------------------------------------------------------------
# 12. Global declarations
# ---------------------------------------------------------------------------


def test_global_untyped_literal() -> None:
    result = tc("let COUNT = 10;")
    assert result.errors == []
    info = result.env.lookup_var("COUNT")
    assert info is not None
    assert info.ty == "u8"


def test_global_typed_annotation() -> None:
    result = tc("let x: u8 = 5;")
    assert result.errors == []
    info = result.env.lookup_var("x")
    assert info is not None
    assert info.ty == "u8"


def test_global_visible_to_functions() -> None:
    result = tc("let COUNT = 10;\nfn f() { out(COUNT); }")
    assert result.errors == []


# ---------------------------------------------------------------------------
# 13. Check through if/while statements
# ---------------------------------------------------------------------------


def test_if_condition_checked() -> None:
    prog = parse("fn f(a: u8) -> u8 { if a > 0 { return a; } return 0; }")
    result = check(prog)
    decl = prog.decls[0]
    if_stmt = decl.body.stmts[0]
    cond = if_stmt.condition  # type: ignore[union-attr]
    assert result.type_map[id(cond)].ty == "u8"


def test_while_condition_checked() -> None:
    prog = parse("fn f(a: u8) { while a > 0 { a = a - 1; } }")
    result = check(prog)
    decl = prog.decls[0]
    while_stmt = decl.body.stmts[0]
    cond = while_stmt.condition  # type: ignore[union-attr]
    assert result.type_map[id(cond)].ty == "u8"


# ---------------------------------------------------------------------------
# 14. check_source convenience function
# ---------------------------------------------------------------------------


def test_check_source_valid() -> None:
    result = check_source("fn f(a: u8) -> u8 { return a; }")
    assert isinstance(result, TypeCheckResult)
    assert result.errors == []


def test_check_source_lex_error() -> None:
    from tetrad_lexer import LexError

    with pytest.raises(LexError):
        check_source("fn f() { let x = @; }")


def test_check_source_parse_error() -> None:
    from tetrad_parser import ParseError

    with pytest.raises(ParseError):
        check_source("fn f( { }")


# ---------------------------------------------------------------------------
# 15. TypeInfo attributes
# ---------------------------------------------------------------------------


def test_type_info_has_line_col() -> None:
    prog = parse("let x = 42;")
    result = check(prog)
    decl = prog.decls[0]
    lit = decl.value  # type: ignore[union-attr]
    info = result.type_map[id(lit)]
    assert info.line >= 0
    assert info.column >= 0


def test_type_info_source_inferred() -> None:
    prog = parse("fn f(a: u8, b: u8) -> u8 { return a + b; }")
    result = check(prog)
    decl = prog.decls[0]
    add_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(add_expr)].source == "inferred"


# ---------------------------------------------------------------------------
# 16. End-to-end: immediate_jit_eligible inference
# ---------------------------------------------------------------------------


def test_fully_typed_is_jit_eligible() -> None:
    result = tc("fn add(a: u8, b: u8) -> u8 { return a + b; }")
    assert result.env.function_status["add"] is FunctionTypeStatus.FULLY_TYPED


def test_untyped_is_not_jit_eligible() -> None:
    result = tc("fn add(a, b) { return a + b; }")
    assert result.env.function_status["add"] is not FunctionTypeStatus.FULLY_TYPED


# ---------------------------------------------------------------------------
# 17. Full programs from TET00
# ---------------------------------------------------------------------------


MULTIPLY = """
fn multiply(a, b) {
    let result = 0;
    while b > 0 {
        result = result + a;
        b = b - 1;
    }
    return result;
}
"""

TYPED_ADD = """
fn add(a: u8, b: u8) -> u8 {
    return a + b;
}
"""

ECHO = """
fn main() {
    let x = in();
    out(x);
}
"""


def test_multiply_is_untyped() -> None:
    result = tc(MULTIPLY)
    assert result.env.function_status["multiply"] is FunctionTypeStatus.UNTYPED
    assert has_warning(result, "JIT warmup required")


def test_typed_add_is_fully_typed() -> None:
    result = tc(TYPED_ADD)
    assert result.env.function_status["add"] is FunctionTypeStatus.FULLY_TYPED
    assert result.errors == []
    assert not has_warning(result, "JIT warmup required")


def test_echo_is_untyped() -> None:
    result = tc(ECHO)
    assert result.env.function_status["main"] is FunctionTypeStatus.UNTYPED


def test_mixed_program() -> None:
    result = tc(MULTIPLY + TYPED_ADD)
    assert result.env.function_status["multiply"] is FunctionTypeStatus.UNTYPED
    assert result.env.function_status["add"] is FunctionTypeStatus.FULLY_TYPED


# ---------------------------------------------------------------------------
# 18. Coverage gap: uncovered branches
# ---------------------------------------------------------------------------

import tetrad_type_checker as _tc_mod  # noqa: E402


def test_name_expr_undefined_var_is_unknown() -> None:
    # NameExpr where the variable is not in scope → Unknown (line 151)
    prog = parse("fn f() { let x = undefined_var; }")
    result = check(prog)
    decl = prog.decls[0]
    name_expr = decl.body.stmts[0].value  # type: ignore[union-attr]
    assert result.type_map[id(name_expr)].ty == "Unknown"


def test_check_expr_unknown_node_type() -> None:
    # Fallback else branch in _check_expr for unrecognised AST nodes (line 211)
    from tetrad_type_checker.types import TypeEnvironment

    env = TypeEnvironment(functions={}, variables={}, function_status={})
    info = _tc_mod._check_expr("not_an_ast_node", env, {}, [])
    assert info.ty == "Unknown"
    assert info.source == "unknown"


def test_let_void_assigned_to_typed_var_errors() -> None:
    # LetStmt: annotated=u8, inferred=Void (not Unknown) → mismatch error (line 252)
    result = tc("fn f() { let x: u8 = out(1); }")
    assert has_error(result, "declared u8 but got Void")


def test_assign_void_to_u8_var_errors() -> None:
    # AssignStmt: existing=u8, inferred=Void → type-mismatch error (line 293)
    result = tc("fn f() { let x = 1; x = out(1); }")
    assert has_error(result, "has type u8 but assigned Void")


def test_if_else_block_both_checked() -> None:
    # IfStmt else block checked (lines 334-335)
    result = tc(
        "fn f(a: u8) { if a > 0 { let x = 1; } else { let y = 2; } }"
    )
    assert result.errors == []


def test_check_stmt_bare_block() -> None:
    # Block as a direct statement in _check_stmt (lines 347-349)
    from tetrad_parser.ast import Block, IntLiteral, LetStmt

    from tetrad_type_checker.types import TypeEnvironment

    inner = Block(
        stmts=[
            LetStmt(
                name="z",
                declared_type=None,
                value=IntLiteral(value=7, line=1, column=1),
                line=1,
                column=1,
            )
        ],
        line=1,
        column=1,
    )
    env = TypeEnvironment(functions={}, variables={}, function_status={})
    type_map: dict = {}
    errors: list = []
    warnings: list = []
    _tc_mod._check_stmt(inner, env, None, type_map, errors, warnings)
    assert errors == []


def test_exprs_in_stmt_expr_stmt() -> None:
    # ExprStmt branch in _exprs_in_stmt (line 400) — reached only when a
    # FULLY_TYPED function has an ExprStmt so _classify_function walks the body
    result = tc("fn f(a: u8) -> u8 { out(a); return a; }")
    assert result.env.function_status["f"] is FunctionTypeStatus.FULLY_TYPED


def test_exprs_in_stmt_if_with_else() -> None:
    # IfStmt else block in _exprs_in_stmt (line 405) — reached when
    # _classify_function walks a FULLY_TYPED function that has if/else
    result = tc(
        "fn f(a: u8) -> u8 { if a > 0 { return a; } else { return 0; } }"
    )
    assert result.errors == []


def test_exprs_in_stmt_bare_block() -> None:
    # Block branch in _exprs_in_stmt (lines 409-411)
    from tetrad_parser.ast import Block

    block = Block(stmts=[], line=1, column=1)
    exprs = _tc_mod._exprs_in_stmt(block)
    assert exprs == []


def test_exprs_in_expr_none_returns_empty() -> None:
    # _exprs_in_expr(None) defensive guard (line 417)
    assert _tc_mod._exprs_in_expr(None) == []


def test_exprs_in_expr_out_expr() -> None:
    # OutExpr branch in _exprs_in_expr (line 428) — reached when classifying
    # a FULLY_TYPED function that has out() in its body
    result = tc("fn f(a: u8) -> u8 { out(a); return a; }")
    assert result.env.function_status["f"] is FunctionTypeStatus.FULLY_TYPED


def test_global_type_mismatch_non_unknown() -> None:
    # GlobalDecl: annotated=u8, inferred=Void (not Unknown) → error (line 521)
    result = tc("let x: u8 = out(5);")
    assert has_error(result, "got Void")


def test_exprs_in_stmt_fallback_unknown_type() -> None:
    # _exprs_in_stmt fallback return [] for unrecognised statement types (line 411)
    assert _tc_mod._exprs_in_stmt("not_a_stmt") == []
