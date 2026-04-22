"""Tests for tetrad_parser.parse().

Coverage target: ≥95%.

Structure
---------
1.  Empty and trivial programs
2.  Integer literal expressions (decimal + hex)
3.  Name expressions
4.  Binary expressions — precedence and associativity
5.  Unary expressions
6.  Grouped expressions
7.  Call expressions
8.  in() and out() expressions
9.  Let statements (with and without type annotations)
10. Assign statements
11. If / if-else statements
12. While statements
13. Return statements
14. Expression statements
15. Function declarations (typed and untyped)
16. Global declarations
17. Type annotations
18. Full programs (spec TET00 examples)
19. ParseError cases
"""

from __future__ import annotations

import pytest

from tetrad_parser import (
    ParseError,
    parse,
)
from tetrad_parser.ast import (
    AssignStmt,
    BinaryExpr,
    CallExpr,
    ExprStmt,
    FnDecl,
    GlobalDecl,
    GroupExpr,
    IfStmt,
    InExpr,
    IntLiteral,
    LetStmt,
    NameExpr,
    OutExpr,
    Program,
    ReturnStmt,
    UnaryExpr,
    WhileStmt,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def expr(src: str) -> object:
    """Parse ``src`` as a top-level global declaration's value expression."""
    prog = parse(f"let _x = {src};")
    return prog.decls[0].value  # type: ignore[union-attr]


def stmt(src: str) -> object:
    """Parse ``src`` as the first statement in a function body."""
    prog = parse(f"fn _f() {{ {src} }}")
    return prog.decls[0].body.stmts[0]  # type: ignore[union-attr]


def fn(src: str) -> FnDecl:
    """Parse ``src`` as a top-level function declaration."""
    prog = parse(src)
    return prog.decls[0]  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# 1. Empty and trivial programs
# ---------------------------------------------------------------------------


def test_empty_program() -> None:
    prog = parse("")
    assert isinstance(prog, Program)
    assert prog.decls == []


def test_program_has_line_col() -> None:
    prog = parse("")
    assert prog.line == 0
    assert prog.column == 0


def test_single_global() -> None:
    prog = parse("let x = 1;")
    assert len(prog.decls) == 1
    assert isinstance(prog.decls[0], GlobalDecl)


def test_multiple_decls() -> None:
    prog = parse("let x = 1;\nlet y = 2;\nfn f() { return x; }")
    assert len(prog.decls) == 3


# ---------------------------------------------------------------------------
# 2. Integer literals
# ---------------------------------------------------------------------------


def test_int_literal_decimal() -> None:
    node = expr("42")
    assert isinstance(node, IntLiteral)
    assert node.value == 42


def test_int_literal_zero() -> None:
    node = expr("0")
    assert isinstance(node, IntLiteral)
    assert node.value == 0


def test_int_literal_255() -> None:
    assert expr("255").value == 255


def test_hex_literal() -> None:
    node = expr("0xFF")
    assert isinstance(node, IntLiteral)
    assert node.value == 255


def test_hex_literal_small() -> None:
    assert expr("0x0A").value == 10


def test_literal_position() -> None:
    prog = parse("let x = 42;")
    lit = prog.decls[0].value  # type: ignore[union-attr]
    assert lit.line == 1
    assert lit.column == 9


# ---------------------------------------------------------------------------
# 3. Name expressions
# ---------------------------------------------------------------------------


def test_name_expr() -> None:
    node = expr("foo")
    assert isinstance(node, NameExpr)
    assert node.name == "foo"


def test_name_underscore() -> None:
    assert expr("_x").name == "_x"


def test_name_mixed_case() -> None:
    assert expr("MyVar").name == "MyVar"


# ---------------------------------------------------------------------------
# 4. Binary expressions — precedence and associativity
# ---------------------------------------------------------------------------


def test_add() -> None:
    node = expr("1 + 2")
    assert isinstance(node, BinaryExpr)
    assert node.op == "+"
    assert node.left.value == 1  # type: ignore[union-attr]
    assert node.right.value == 2  # type: ignore[union-attr]


def test_mul_binds_tighter_than_add() -> None:
    # 1 + 2 * 3  → BinaryExpr('+', 1, BinaryExpr('*', 2, 3))
    node = expr("1 + 2 * 3")
    assert isinstance(node, BinaryExpr)
    assert node.op == "+"
    assert isinstance(node.right, BinaryExpr)
    assert node.right.op == "*"
    assert node.right.left.value == 2  # type: ignore[union-attr]
    assert node.right.right.value == 3  # type: ignore[union-attr]


def test_add_left_associative() -> None:
    # 1 + 2 + 3  → BinaryExpr('+', BinaryExpr('+', 1, 2), 3)
    node = expr("1 + 2 + 3")
    assert isinstance(node, BinaryExpr)
    assert node.op == "+"
    assert isinstance(node.left, BinaryExpr)
    assert node.left.op == "+"
    assert node.right.value == 3  # type: ignore[union-attr]


def test_and_binds_tighter_than_or() -> None:
    # a || b && c  → BinaryExpr('||', a, BinaryExpr('&&', b, c))
    node = expr("a || b && c")
    assert isinstance(node, BinaryExpr)
    assert node.op == "||"
    assert isinstance(node.right, BinaryExpr)
    assert node.right.op == "&&"


def test_comparison_vs_add() -> None:
    # a + b > c  → BinaryExpr('>', BinaryExpr('+', a, b), c)
    node = expr("a + b > c")
    assert isinstance(node, BinaryExpr)
    assert node.op == ">"
    assert isinstance(node.left, BinaryExpr)
    assert node.left.op == "+"


def test_all_arithmetic_ops() -> None:
    for op in ["+", "-", "*", "/", "%"]:
        node = expr(f"a {op} b")
        assert isinstance(node, BinaryExpr)
        assert node.op == op


def test_all_bitwise_ops() -> None:
    for op in ["&", "|", "^"]:
        node = expr(f"a {op} b")
        assert isinstance(node, BinaryExpr)
        assert node.op == op


def test_shift_ops() -> None:
    assert expr("a << 2").op == "<<"
    assert expr("a >> 2").op == ">>"


def test_comparison_ops() -> None:
    for op in ["==", "!=", "<", "<=", ">", ">="]:
        node = expr(f"a {op} b")
        assert isinstance(node, BinaryExpr)
        assert node.op == op


def test_logical_ops() -> None:
    assert expr("a && b").op == "&&"
    assert expr("a || b").op == "||"


def test_precedence_bitwise_over_comparison() -> None:
    # a | b == c  → BinaryExpr('==', BinaryExpr('|', a, b), c)
    # pipe(50) < eq(30) … wait, higher number = tighter binding
    # Actually: == has bp 30, | has bp 50 — so | binds tighter than ==
    # a | b == c  → BinaryExpr('==', BinaryExpr('|', a, b), c)
    node = expr("a | b == c")
    assert node.op == "=="
    assert isinstance(node.left, BinaryExpr)
    assert node.left.op == "|"


def test_mul_before_subtraction() -> None:
    node = expr("1 * 2 - 3")
    assert node.op == "-"
    assert isinstance(node.left, BinaryExpr)
    assert node.left.op == "*"


# ---------------------------------------------------------------------------
# 5. Unary expressions
# ---------------------------------------------------------------------------


def test_unary_not() -> None:
    node = expr("!a")
    assert isinstance(node, UnaryExpr)
    assert node.op == "!"
    assert node.operand.name == "a"  # type: ignore[union-attr]


def test_unary_bitwise_not() -> None:
    node = expr("~a")
    assert isinstance(node, UnaryExpr)
    assert node.op == "~"


def test_unary_negate() -> None:
    node = expr("-a")
    assert isinstance(node, UnaryExpr)
    assert node.op == "-"


def test_unary_binds_tighter_than_mul() -> None:
    # -a * b  → BinaryExpr('*', UnaryExpr('-', a), b)
    node = expr("-a * b")
    assert isinstance(node, BinaryExpr)
    assert node.op == "*"
    assert isinstance(node.left, UnaryExpr)
    assert node.left.op == "-"


def test_double_unary() -> None:
    node = expr("!!a")
    assert isinstance(node, UnaryExpr)
    assert node.op == "!"
    assert isinstance(node.operand, UnaryExpr)
    assert node.operand.op == "!"


# ---------------------------------------------------------------------------
# 6. Grouped expressions
# ---------------------------------------------------------------------------


def test_group_expr() -> None:
    node = expr("(1 + 2)")
    assert isinstance(node, GroupExpr)
    assert isinstance(node.expr, BinaryExpr)
    assert node.expr.op == "+"


def test_group_overrides_precedence() -> None:
    # (1 + 2) * 3  → BinaryExpr('*', GroupExpr(BinaryExpr('+', 1, 2)), 3)
    node = expr("(1 + 2) * 3")
    assert isinstance(node, BinaryExpr)
    assert node.op == "*"
    assert isinstance(node.left, GroupExpr)
    assert isinstance(node.left.expr, BinaryExpr)
    assert node.left.expr.op == "+"


def test_nested_groups() -> None:
    node = expr("((a))")
    assert isinstance(node, GroupExpr)
    assert isinstance(node.expr, GroupExpr)
    assert isinstance(node.expr.expr, NameExpr)


# ---------------------------------------------------------------------------
# 7. Call expressions
# ---------------------------------------------------------------------------


def test_call_no_args() -> None:
    node = expr("f()")
    assert isinstance(node, CallExpr)
    assert node.name == "f"
    assert node.args == []


def test_call_one_arg() -> None:
    node = expr("f(1)")
    assert isinstance(node, CallExpr)
    assert len(node.args) == 1
    assert node.args[0].value == 1  # type: ignore[union-attr]


def test_call_two_args() -> None:
    node = expr("add(1, 2)")
    assert isinstance(node, CallExpr)
    assert node.name == "add"
    assert len(node.args) == 2


def test_call_expr_args() -> None:
    node = expr("f(a + b, c * 2)")
    assert isinstance(node, CallExpr)
    assert isinstance(node.args[0], BinaryExpr)
    assert isinstance(node.args[1], BinaryExpr)


def test_call_in_expression() -> None:
    # f(1) + 2
    node = expr("f(1) + 2")
    assert isinstance(node, BinaryExpr)
    assert node.op == "+"
    assert isinstance(node.left, CallExpr)


# ---------------------------------------------------------------------------
# 8. in() and out() expressions
# ---------------------------------------------------------------------------


def test_in_expr() -> None:
    node = expr("in()")
    assert isinstance(node, InExpr)


def test_out_expr() -> None:
    node = expr("out(42)")
    assert isinstance(node, OutExpr)
    assert isinstance(node.value, IntLiteral)
    assert node.value.value == 42


def test_out_name_arg() -> None:
    node = expr("out(x)")
    assert isinstance(node, OutExpr)
    assert isinstance(node.value, NameExpr)
    assert node.value.name == "x"


def test_out_as_stmt() -> None:
    s = stmt("out(x);")
    assert isinstance(s, ExprStmt)
    assert isinstance(s.expr, OutExpr)


# ---------------------------------------------------------------------------
# 9. Let statements
# ---------------------------------------------------------------------------


def test_let_stmt_basic() -> None:
    s = stmt("let x = 42;")
    assert isinstance(s, LetStmt)
    assert s.name == "x"
    assert s.declared_type is None
    assert isinstance(s.value, IntLiteral)
    assert s.value.value == 42


def test_let_stmt_with_type() -> None:
    s = stmt("let x: u8 = 0;")
    assert isinstance(s, LetStmt)
    assert s.declared_type == "u8"
    assert s.name == "x"


def test_let_stmt_expr_value() -> None:
    s = stmt("let z = a + b;")
    assert isinstance(s, LetStmt)
    assert isinstance(s.value, BinaryExpr)


# ---------------------------------------------------------------------------
# 10. Assign statements
# ---------------------------------------------------------------------------


def test_assign_stmt() -> None:
    s = stmt("x = 10;")
    assert isinstance(s, AssignStmt)
    assert s.name == "x"
    assert isinstance(s.value, IntLiteral)


def test_assign_stmt_expr() -> None:
    s = stmt("x = x + 1;")
    assert isinstance(s, AssignStmt)
    assert isinstance(s.value, BinaryExpr)


def test_assign_not_eq_eq() -> None:
    # "x == 10;" is NOT an assign — it's an expression statement
    s = stmt("x == 10;")
    assert isinstance(s, ExprStmt)
    assert isinstance(s.expr, BinaryExpr)
    assert s.expr.op == "=="


# ---------------------------------------------------------------------------
# 11. If / if-else statements
# ---------------------------------------------------------------------------


def test_if_no_else() -> None:
    s = stmt("if a > 0 { out(a); }")
    assert isinstance(s, IfStmt)
    assert isinstance(s.condition, BinaryExpr)
    assert s.condition.op == ">"
    assert s.else_block is None


def test_if_with_else() -> None:
    s = stmt("if a { let x = 1; } else { let x = 2; }")
    assert isinstance(s, IfStmt)
    assert s.else_block is not None
    assert len(s.else_block.stmts) == 1


def test_if_then_block_stmts() -> None:
    s = stmt("if a { let x = 1; let y = 2; }")
    assert isinstance(s, IfStmt)
    assert len(s.then_block.stmts) == 2


def test_nested_if() -> None:
    src = "if a { if b { out(1); } }"
    s = stmt(src)
    assert isinstance(s, IfStmt)
    inner = s.then_block.stmts[0]
    assert isinstance(inner, IfStmt)


# ---------------------------------------------------------------------------
# 12. While statements
# ---------------------------------------------------------------------------


def test_while_stmt() -> None:
    s = stmt("while n > 0 { n = n - 1; }")
    assert isinstance(s, WhileStmt)
    assert isinstance(s.condition, BinaryExpr)
    assert s.condition.op == ">"


def test_while_empty_body() -> None:
    s = stmt("while 0 { }")
    assert isinstance(s, WhileStmt)
    assert s.body.stmts == []


# ---------------------------------------------------------------------------
# 13. Return statements
# ---------------------------------------------------------------------------


def test_return_with_value() -> None:
    s = stmt("return x + 1;")
    assert isinstance(s, ReturnStmt)
    assert isinstance(s.value, BinaryExpr)


def test_return_no_value() -> None:
    s = stmt("return;")
    assert isinstance(s, ReturnStmt)
    assert s.value is None


# ---------------------------------------------------------------------------
# 14. Expression statements
# ---------------------------------------------------------------------------


def test_call_as_expr_stmt() -> None:
    s = stmt("f(1, 2);")
    assert isinstance(s, ExprStmt)
    assert isinstance(s.expr, CallExpr)


def test_in_as_expr_stmt() -> None:
    # reading from I/O and discarding is legal
    s = stmt("in();")
    assert isinstance(s, ExprStmt)
    assert isinstance(s.expr, InExpr)


# ---------------------------------------------------------------------------
# 15. Function declarations
# ---------------------------------------------------------------------------


def test_fn_no_params_no_return() -> None:
    decl = fn("fn f() { return; }")
    assert isinstance(decl, FnDecl)
    assert decl.name == "f"
    assert decl.params == []
    assert decl.param_types == []
    assert decl.return_type is None


def test_fn_two_params_no_types() -> None:
    decl = fn("fn add(a, b) { return a; }")
    assert decl.params == ["a", "b"]
    assert decl.param_types == [None, None]


def test_fn_typed_params_and_return() -> None:
    decl = fn("fn add(a: u8, b: u8) -> u8 { return a; }")
    assert decl.params == ["a", "b"]
    assert decl.param_types == ["u8", "u8"]
    assert decl.return_type == "u8"


def test_fn_partial_types() -> None:
    decl = fn("fn f(a: u8, b) { return a; }")
    assert decl.param_types == ["u8", None]
    assert decl.return_type is None


def test_fn_body_has_stmts() -> None:
    decl = fn("fn f() { let x = 1; return x; }")
    assert len(decl.body.stmts) == 2


def test_fn_position() -> None:
    decl = fn("fn f() { }")
    assert decl.line == 1
    assert decl.column == 1


# ---------------------------------------------------------------------------
# 16. Global declarations
# ---------------------------------------------------------------------------


def test_global_decl() -> None:
    prog = parse("let COUNT = 10;")
    g = prog.decls[0]
    assert isinstance(g, GlobalDecl)
    assert g.name == "COUNT"
    assert g.declared_type is None
    assert g.value.value == 10  # type: ignore[union-attr]


def test_global_decl_typed() -> None:
    prog = parse("let x: u8 = 0;")
    g = prog.decls[0]
    assert isinstance(g, GlobalDecl)
    assert g.declared_type == "u8"


# ---------------------------------------------------------------------------
# 17. Type annotations
# ---------------------------------------------------------------------------


def test_u8_is_only_valid_type_param() -> None:
    decl = fn("fn f(x: u8) { }")
    assert decl.param_types == ["u8"]


def test_u8_is_only_valid_type_return() -> None:
    decl = fn("fn f() -> u8 { return 0; }")
    assert decl.return_type == "u8"


def test_u8_is_only_valid_type_let() -> None:
    s = stmt("let x: u8 = 1;")
    assert s.declared_type == "u8"  # type: ignore[union-attr]


# ---------------------------------------------------------------------------
# 18. Full programs — TET00 examples
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

ADD = """
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

COUNTDOWN = """
fn count_down(n) {
    while n > 0 {
        out(n);
        n = n - 1;
    }
}
"""

BITWISE = """
fn mask_nibble(x: u8) -> u8 {
    return x & 0x0F;
}
"""


def test_multiply_structure() -> None:
    prog = parse(MULTIPLY)
    assert len(prog.decls) == 1
    f = prog.decls[0]
    assert isinstance(f, FnDecl)
    assert f.name == "multiply"
    assert f.params == ["a", "b"]
    assert f.param_types == [None, None]
    assert len(f.body.stmts) == 3  # let, while, return


def test_multiply_while_condition() -> None:
    prog = parse(MULTIPLY)
    f = prog.decls[0]
    while_stmt = f.body.stmts[1]
    assert isinstance(while_stmt, WhileStmt)
    assert isinstance(while_stmt.condition, BinaryExpr)
    assert while_stmt.condition.op == ">"


def test_typed_add() -> None:
    prog = parse(ADD)
    f = prog.decls[0]
    assert isinstance(f, FnDecl)
    assert f.param_types == ["u8", "u8"]
    assert f.return_type == "u8"
    body_stmt = f.body.stmts[0]
    assert isinstance(body_stmt, ReturnStmt)
    assert isinstance(body_stmt.value, BinaryExpr)
    assert body_stmt.value.op == "+"


def test_echo_io() -> None:
    prog = parse(ECHO)
    f = prog.decls[0]
    stmts = f.body.stmts
    assert isinstance(stmts[0], LetStmt)
    assert isinstance(stmts[0].value, InExpr)
    assert isinstance(stmts[1], ExprStmt)
    assert isinstance(stmts[1].expr, OutExpr)


def test_countdown() -> None:
    prog = parse(COUNTDOWN)
    f = prog.decls[0]
    assert f.name == "count_down"
    while_s = f.body.stmts[0]
    assert isinstance(while_s, WhileStmt)
    body = while_s.body.stmts
    assert isinstance(body[0], ExprStmt)
    assert isinstance(body[0].expr, OutExpr)
    assert isinstance(body[1], AssignStmt)


def test_bitwise_mask() -> None:
    prog = parse(BITWISE)
    f = prog.decls[0]
    assert f.param_types == ["u8"]
    assert f.return_type == "u8"
    ret = f.body.stmts[0]
    assert isinstance(ret, ReturnStmt)
    assert isinstance(ret.value, BinaryExpr)
    assert ret.value.op == "&"


# ---------------------------------------------------------------------------
# 19. ParseError cases
# ---------------------------------------------------------------------------


def test_error_unexpected_top_level() -> None:
    with pytest.raises(ParseError) as exc:
        parse("42;")
    assert "expected fn or let at top level" in str(exc.value)


def test_error_missing_brace() -> None:
    with pytest.raises(ParseError):
        parse("fn f() { let x = 1; ")


def test_error_missing_semi() -> None:
    with pytest.raises(ParseError):
        parse("fn f() { let x = 1 }")


def test_error_unexpected_token_in_expr() -> None:
    with pytest.raises(ParseError) as exc:
        parse("let x = ;")
    assert "unexpected token" in str(exc.value)


def test_error_in_without_parens() -> None:
    with pytest.raises(ParseError) as exc:
        parse("let x = in;")
    assert "in must be called as in()" in str(exc.value)


def test_error_unknown_type_param() -> None:
    with pytest.raises(ParseError) as exc:
        parse("fn f(x: foo) { }")
    assert "unknown type 'foo'" in str(exc.value)


def test_error_unknown_type_return() -> None:
    with pytest.raises(ParseError) as exc:
        parse("fn f() -> bar { }")
    assert "unknown type 'bar'" in str(exc.value)


def test_error_unknown_type_let() -> None:
    with pytest.raises(ParseError) as exc:
        parse("let x: int = 1;")
    assert "unknown type 'int'" in str(exc.value)


def test_error_arrow_no_type() -> None:
    with pytest.raises(ParseError) as exc:
        parse("fn f() -> { }")
    assert "expected type" in str(exc.value)


def test_error_missing_rparen_call() -> None:
    with pytest.raises(ParseError):
        parse("let x = f(1, 2;")


def test_error_missing_rparen_group() -> None:
    with pytest.raises(ParseError):
        parse("let x = (1 + 2;")


def test_error_has_line_col() -> None:
    with pytest.raises(ParseError) as exc:
        parse("let x = ;")
    err = exc.value
    assert hasattr(err, "line")
    assert hasattr(err, "column")
    assert err.line >= 1
    assert err.column >= 1


def test_error_extra_comma_in_call() -> None:
    with pytest.raises(ParseError):
        parse("let x = f(1,,2);")


def test_error_eof_in_expr() -> None:
    with pytest.raises(ParseError):
        parse("fn f() { let x = ")


def test_parse_error_inherits_exception() -> None:
    with pytest.raises(ParseError):
        parse("bad")


def test_binary_position_tracked() -> None:
    prog = parse("let x = 1 + 2;")
    node = prog.decls[0].value  # type: ignore[union-attr]
    assert isinstance(node, BinaryExpr)
    assert node.line == 1


def test_fn_return_type_only_u8() -> None:
    with pytest.raises(ParseError):
        parse("fn f() -> i32 { }")
