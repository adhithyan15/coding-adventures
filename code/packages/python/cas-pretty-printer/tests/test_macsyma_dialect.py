"""MACSYMA dialect — round-trip and shape tests."""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EQUAL,
    GREATER,
    INV,
    LESS,
    LIST,
    MUL,
    NEG,
    NOT_EQUAL,
    POW,
    SIN,
    SUB,
    D,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_pretty_printer import MacsymaDialect, pretty

D_MAC = MacsymaDialect()


def fmt(node: object) -> str:
    return pretty(node, D_MAC)  # type: ignore[arg-type]


# ---- leaves ----------------------------------------------------------------


def test_integer_positive() -> None:
    assert fmt(IRInteger(42)) == "42"


def test_integer_negative_at_top() -> None:
    # At the top level, no surrounding context — emit raw.
    assert fmt(IRInteger(-5)) == "-5"


def test_rational() -> None:
    assert fmt(IRRational(3, 4)) == "3/4"


def test_float() -> None:
    text = fmt(IRFloat(3.14))
    assert text.startswith("3.14")


def test_string() -> None:
    assert fmt(IRString("hello")) == '"hello"'


def test_symbol() -> None:
    assert fmt(IRSymbol("x")) == "x"


# ---- binary ops ------------------------------------------------------------


def test_add_two_args() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(ADD, (x, IRInteger(1)))) == "x + 1"


def test_add_three_args() -> None:
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    assert fmt(IRApply(ADD, (a, b, c))) == "a + b + c"


def test_mul_basic() -> None:
    x, y = IRSymbol("x"), IRSymbol("y")
    assert fmt(IRApply(MUL, (x, y))) == "x*y"


def test_pow_basic() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(POW, (x, IRInteger(2)))) == "x^2"


# ---- precedence and parens -------------------------------------------------


def test_add_inside_mul_no_parens() -> None:
    """Add(x, Mul(y, z)) prints as `x + y*z` — Mul binds tighter."""
    y, z = IRSymbol("y"), IRSymbol("z")
    expr = IRApply(ADD, (IRSymbol("x"), IRApply(MUL, (y, z))))
    assert fmt(expr) == "x + y*z"


def test_mul_of_add_needs_parens() -> None:
    """Mul(Add(x, y), z) prints as `(x + y)*z`."""
    x, y, z = IRSymbol("x"), IRSymbol("y"), IRSymbol("z")
    expr = IRApply(MUL, (IRApply(ADD, (x, y)), z))
    assert fmt(expr) == "(x + y)*z"


def test_pow_right_associative() -> None:
    """Pow(a, Pow(b, c)) prints as `a^b^c` — right child takes prec, no parens."""
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    expr = IRApply(POW, (a, IRApply(POW, (b, c))))
    assert fmt(expr) == "a^b^c"


def test_pow_left_side_needs_parens() -> None:
    """Pow(Pow(a, b), c) prints as `(a^b)^c`."""
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    expr = IRApply(POW, (IRApply(POW, (a, b)), c))
    assert fmt(expr) == "(a^b)^c"


def test_unary_neg() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(NEG, (x,))) == "-x"


def test_neg_of_add_needs_parens() -> None:
    x, y = IRSymbol("x"), IRSymbol("y")
    assert fmt(IRApply(NEG, (IRApply(ADD, (x, y)),))) == "-(x + y)"


# ---- sugar -----------------------------------------------------------------


def test_sub_sugar() -> None:
    """Add(x, Neg(y)) sugars to `x - y`."""
    x, y = IRSymbol("x"), IRSymbol("y")
    expr = IRApply(ADD, (x, IRApply(NEG, (y,))))
    assert fmt(expr) == "x - y"


def test_div_sugar() -> None:
    """Mul(x, Inv(y)) sugars to `x/y`."""
    x, y = IRSymbol("x"), IRSymbol("y")
    expr = IRApply(MUL, (x, IRApply(INV, (y,))))
    assert fmt(expr) == "x/y"


def test_neg_via_minus_one_sugar() -> None:
    """Mul(-1, x) sugars to `-x`."""
    x = IRSymbol("x")
    expr = IRApply(MUL, (IRInteger(-1), x))
    assert fmt(expr) == "-x"


# ---- containers ------------------------------------------------------------


def test_list_brackets() -> None:
    assert fmt(IRApply(LIST, (IRInteger(1), IRInteger(2), IRInteger(3)))) == "[1, 2, 3]"


def test_empty_list() -> None:
    assert fmt(IRApply(LIST, ())) == "[]"


# ---- function calls --------------------------------------------------------


def test_sin_call() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(SIN, (x,))) == "sin(x)"


def test_diff_call() -> None:
    x = IRSymbol("x")
    expr = IRApply(D, (IRApply(POW, (x, IRInteger(2))), x))
    assert fmt(expr) == "diff(x^2, x)"


def test_user_function() -> None:
    f = IRSymbol("foo")
    x = IRSymbol("x")
    assert fmt(IRApply(f, (x, IRInteger(1)))) == "foo(x, 1)"


def test_no_arg_call() -> None:
    f = IRSymbol("hello")
    assert fmt(IRApply(f, ())) == "hello()"


# ---- comparisons / divisions / explicit Sub & Div --------------------------


def test_explicit_sub() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(SUB, (a, b))) == "a - b"


def test_explicit_div() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(DIV, (a, b))) == "a/b"


def test_equal() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(EQUAL, (a, b))) == "a = b"


def test_not_equal() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(NOT_EQUAL, (a, b))) == "a # b"


def test_less() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(LESS, (a, b))) == "a < b"


def test_greater() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(GREATER, (a, b))) == "a > b"


# ---- nested expression -----------------------------------------------------


def test_polynomial() -> None:
    """`x^2 + 2*x + 1`."""
    x = IRSymbol("x")
    expr = IRApply(
        ADD,
        (
            IRApply(POW, (x, IRInteger(2))),
            IRApply(MUL, (IRInteger(2), x)),
            IRInteger(1),
        ),
    )
    assert fmt(expr) == "x^2 + 2*x + 1"


def test_diff_call_with_compound_arg() -> None:
    """`diff(sin(x) + cos(x), x)`."""
    x = IRSymbol("x")
    inner = IRApply(ADD, (IRApply(SIN, (x,)), IRApply(COS, (x,))))
    expr = IRApply(D, (inner, x))
    assert fmt(expr) == "diff(sin(x) + cos(x), x)"


# ---- error path ------------------------------------------------------------


def test_unknown_node_type_raises() -> None:
    class Bogus:
        pass

    with pytest.raises(TypeError):
        pretty(Bogus(), D_MAC)  # type: ignore[arg-type]


def test_unsupported_style_raises() -> None:
    with pytest.raises(ValueError):
        pretty(IRInteger(1), D_MAC, style="2d")
