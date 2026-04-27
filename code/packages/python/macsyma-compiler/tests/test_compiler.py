"""macsyma-compiler tests.

These verify that a parsed MACSYMA program compiles to the expected
``IRApply`` shape, including all the flattening and head-rewriting
rules documented in ``compiler.py``.
"""

from __future__ import annotations

import pytest
from macsyma_compiler import CompileError, compile_macsyma
from macsyma_parser import parse_macsyma
from symbolic_ir import (
    ADD,
    ASSIGN,
    D,
    DEFINE,
    DIV,
    EQUAL,
    GREATER,
    LESS,
    LIST,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRSymbol,
)
from symbolic_ir.nodes import AND, OR


def compile_one(source: str):
    """Compile a source string and return its single statement's IR."""
    statements = compile_macsyma(parse_macsyma(source))
    assert len(statements) == 1, f"expected 1 statement, got {len(statements)}"
    return statements[0]


# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


def test_integer_literal() -> None:
    assert compile_one("42;") == IRInteger(42)


def test_float_literal() -> None:
    assert compile_one("3.14;") == IRFloat(3.14)


def test_name_becomes_symbol() -> None:
    assert compile_one("x;") == IRSymbol("x")


def test_percent_constant() -> None:
    # %pi and friends are still just identifiers at the IR level;
    # the VM will give them special meaning.
    assert compile_one("%pi;") == IRSymbol("%pi")


# ---------------------------------------------------------------------------
# Binary arithmetic
# ---------------------------------------------------------------------------


def test_simple_addition() -> None:
    assert compile_one("x + 1;") == IRApply(ADD, (IRSymbol("x"), IRInteger(1)))


def test_subtraction_left_associative() -> None:
    # (a - b) - c  (not a - (b - c))
    result = compile_one("a - b - c;")
    expected = IRApply(
        SUB,
        (
            IRApply(SUB, (IRSymbol("a"), IRSymbol("b"))),
            IRSymbol("c"),
        ),
    )
    assert result == expected


def test_multiplication_division_left_associative() -> None:
    result = compile_one("a / b * c;")
    expected = IRApply(
        MUL,
        (
            IRApply(DIV, (IRSymbol("a"), IRSymbol("b"))),
            IRSymbol("c"),
        ),
    )
    assert result == expected


def test_precedence_add_lower_than_mul() -> None:
    # 1 + 2 * 3 → Add(1, Mul(2, 3))
    result = compile_one("1 + 2 * 3;")
    expected = IRApply(
        ADD,
        (IRInteger(1), IRApply(MUL, (IRInteger(2), IRInteger(3)))),
    )
    assert result == expected


def test_parens_override_precedence() -> None:
    # (1 + 2) * 3 → Mul(Add(1, 2), 3)
    result = compile_one("(1 + 2) * 3;")
    expected = IRApply(
        MUL,
        (IRApply(ADD, (IRInteger(1), IRInteger(2))), IRInteger(3)),
    )
    assert result == expected


# ---------------------------------------------------------------------------
# Power and unary
# ---------------------------------------------------------------------------


def test_power() -> None:
    assert compile_one("x^2;") == IRApply(
        POW, (IRSymbol("x"), IRInteger(2))
    )


def test_double_star_power() -> None:
    assert compile_one("x ** 2;") == IRApply(
        POW, (IRSymbol("x"), IRInteger(2))
    )


def test_power_right_associative() -> None:
    # a^b^c = a^(b^c)
    result = compile_one("a^b^c;")
    expected = IRApply(
        POW,
        (
            IRSymbol("a"),
            IRApply(POW, (IRSymbol("b"), IRSymbol("c"))),
        ),
    )
    assert result == expected


def test_unary_minus() -> None:
    assert compile_one("-x;") == IRApply(NEG, (IRSymbol("x"),))


def test_unary_plus_is_identity() -> None:
    assert compile_one("+x;") == IRSymbol("x")


def test_unary_minus_binds_tighter_than_mul() -> None:
    # a * -b → Mul(a, Neg(b))
    result = compile_one("a * -b;")
    expected = IRApply(
        MUL,
        (IRSymbol("a"), IRApply(NEG, (IRSymbol("b"),))),
    )
    assert result == expected


# ---------------------------------------------------------------------------
# Function calls and standard-function rewriting
# ---------------------------------------------------------------------------


def test_user_function_call() -> None:
    # `f(x, y)` with user-defined f → Apply(Symbol('f'), (x, y))
    result = compile_one("f(x, y);")
    expected = IRApply(IRSymbol("f"), (IRSymbol("x"), IRSymbol("y")))
    assert result == expected


def test_diff_rewrites_to_D() -> None:
    # diff(x^2, x) → Apply(D, (Pow(x, 2), x))
    result = compile_one("diff(x^2, x);")
    expected = IRApply(
        D,
        (
            IRApply(POW, (IRSymbol("x"), IRInteger(2))),
            IRSymbol("x"),
        ),
    )
    assert result == expected


def test_sin_rewrites_to_Sin() -> None:
    result = compile_one("sin(x);")
    assert result == IRApply(SIN, (IRSymbol("x"),))


def test_empty_function_call() -> None:
    # No-arg function call.
    result = compile_one("f();")
    assert result == IRApply(IRSymbol("f"), ())


# ---------------------------------------------------------------------------
# Comparisons and logic
# ---------------------------------------------------------------------------


def test_equality() -> None:
    assert compile_one("x = 4;") == IRApply(
        EQUAL, (IRSymbol("x"), IRInteger(4))
    )


def test_less_than() -> None:
    assert compile_one("a < b;") == IRApply(
        LESS, (IRSymbol("a"), IRSymbol("b"))
    )


def test_greater_than() -> None:
    assert compile_one("a > b;") == IRApply(
        GREATER, (IRSymbol("a"), IRSymbol("b"))
    )


def test_logical_and_variadic() -> None:
    # a and b and c → And(a, b, c) (not nested).
    result = compile_one("a and b and c;")
    expected = IRApply(AND, (IRSymbol("a"), IRSymbol("b"), IRSymbol("c")))
    assert result == expected


def test_logical_or_variadic() -> None:
    result = compile_one("a or b or c;")
    expected = IRApply(OR, (IRSymbol("a"), IRSymbol("b"), IRSymbol("c")))
    assert result == expected


# ---------------------------------------------------------------------------
# Assignment and definition
# ---------------------------------------------------------------------------


def test_simple_assignment() -> None:
    # a : 5  → Assign(a, 5)
    result = compile_one("a : 5;")
    assert result == IRApply(ASSIGN, (IRSymbol("a"), IRInteger(5)))


def test_function_definition() -> None:
    # f(x) := x^2 → Define(f, List(x), Pow(x, 2))
    result = compile_one("f(x) := x^2;")
    expected = IRApply(
        DEFINE,
        (
            IRSymbol("f"),
            IRApply(LIST, (IRSymbol("x"),)),
            IRApply(POW, (IRSymbol("x"), IRInteger(2))),
        ),
    )
    assert result == expected


def test_multi_arg_function_definition() -> None:
    result = compile_one("add(x, y) := x + y;")
    expected = IRApply(
        DEFINE,
        (
            IRSymbol("add"),
            IRApply(LIST, (IRSymbol("x"), IRSymbol("y"))),
            IRApply(ADD, (IRSymbol("x"), IRSymbol("y"))),
        ),
    )
    assert result == expected


def test_variable_definition_delayed() -> None:
    # `x := 5` is still DEFINE with empty params.
    result = compile_one("x := 5;")
    expected = IRApply(
        DEFINE,
        (IRSymbol("x"), IRApply(LIST, ()), IRInteger(5)),
    )
    assert result == expected


# ---------------------------------------------------------------------------
# Lists
# ---------------------------------------------------------------------------


def test_list_literal() -> None:
    result = compile_one("[1, 2, 3];")
    expected = IRApply(
        LIST, (IRInteger(1), IRInteger(2), IRInteger(3))
    )
    assert result == expected


def test_empty_list() -> None:
    assert compile_one("[];") == IRApply(LIST, ())


# ---------------------------------------------------------------------------
# Programs with multiple statements
# ---------------------------------------------------------------------------


def test_multiple_statements() -> None:
    stmts = compile_macsyma(parse_macsyma("a : 1; b : 2; a + b;"))
    assert len(stmts) == 3
    assert stmts[0] == IRApply(ASSIGN, (IRSymbol("a"), IRInteger(1)))
    assert stmts[1] == IRApply(ASSIGN, (IRSymbol("b"), IRInteger(2)))
    assert stmts[2] == IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))


def test_empty_program() -> None:
    assert compile_macsyma(parse_macsyma("")) == []


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


def test_non_program_root_raises() -> None:
    from macsyma_parser import parse_macsyma as _parse

    # Build a parsed tree and try to compile an inner node.
    ast = _parse("x;")
    inner = ast.children[0]  # statement
    with pytest.raises(CompileError):
        compile_macsyma(inner)  # type: ignore[arg-type]
