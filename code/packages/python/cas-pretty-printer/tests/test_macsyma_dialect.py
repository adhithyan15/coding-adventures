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


def test_negative_coeff_no_parens() -> None:
    """Mul(-2, x) prints as `-2*x`, not `(-2)*x`."""
    x = IRSymbol("x")
    expr = IRApply(MUL, (IRInteger(-2), x))
    assert fmt(expr) == "-2*x"


def test_negative_coeff_in_pow_keeps_parens() -> None:
    """Pow(x, -3) still wraps the exponent: `x^(-3)`."""
    x = IRSymbol("x")
    expr = IRApply(POW, (x, IRInteger(-3)))
    assert fmt(expr) == "x^(-3)"


def test_add_neg_literal_first_arg() -> None:
    """Add(-1, y) sugars to `y - 1`."""
    y = IRSymbol("y")
    expr = IRApply(ADD, (IRInteger(-1), y))
    assert fmt(expr) == "y - 1"


def test_add_neg_literal_first_arg_large() -> None:
    """Add(-5, x) sugars to `x - 5`."""
    x = IRSymbol("x")
    expr = IRApply(ADD, (IRInteger(-5), x))
    assert fmt(expr) == "x - 5"


def test_mul_with_neg_second_arg() -> None:
    """Mul(a, Neg(b)) sugars to `-(a*b)`."""
    a, b = IRSymbol("a"), IRSymbol("b")
    expr = IRApply(MUL, (a, IRApply(NEG, (b,))))
    assert fmt(expr) == "-(a*b)"


def test_add_of_mul_with_neg_second_arg() -> None:
    """Add(a, Mul(b, Neg(c))) sugars to `a - b*c`.

    This exercises the one-level recursive sugar peek in the Add rule:
    the walker spots that Mul(b, Neg(c)) is Neg under sugar, then emits
    a Sub instead of ``a + -(b*c)``.
    """
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    inner = IRApply(MUL, (b, IRApply(NEG, (c,))))
    expr = IRApply(ADD, (a, inner))
    # Should be a - b*c, not a + -(b*c)
    result = fmt(expr)
    assert result == "a - b*c"


def test_diff_product_rule_display() -> None:
    """cos(y)*cos(y) + sin(y)*Neg(sin(y)) → `cos(y)*cos(y) - sin(y)*sin(y)`.

    This models the output of diff(sin(y)*cos(y), y): the VM produces
    Add(Mul(Cos(y), Cos(y)), Mul(Sin(y), Neg(Sin(y)))).  The sugar
    pipeline should fold this to a Sub with clean operands.
    """
    from symbolic_ir import COS
    from symbolic_ir import SIN as SIN_SYM

    y = IRSymbol("y")
    inner_c = IRApply(COS, (y,))
    inner_s = IRApply(SIN_SYM, (y,))
    # Add(cos(y)*cos(y),  sin(y)*(-sin(y)))
    expr = IRApply(ADD, (
        IRApply(MUL, (inner_c, inner_c)),
        IRApply(MUL, (inner_s, IRApply(NEG, (inner_s,)))),
    ))
    result = fmt(expr)
    assert result == "cos(y)*cos(y) - sin(y)*sin(y)"


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


# ---- MACSYMA-specific function-name aliases --------------------------------
#
# These tests verify that IR heads which have MACSYMA-specific surface
# spellings (different from the generic lowercase defaults) are rendered
# correctly by MacsymaDialect.  Each test builds a minimal unevaluated
# IRApply and checks the printed surface form.


def test_alias_ratsimp() -> None:
    """RatSimplify head → ``ratsimp``."""
    x = IRSymbol("x")
    head = IRSymbol("RatSimplify")
    assert fmt(IRApply(head, (x,))) == "ratsimp(x)"


def test_alias_partfrac() -> None:
    """Apart head → ``partfrac`` in MACSYMA (not ``apart``)."""
    x = IRSymbol("x")
    head = IRSymbol("Apart")
    assert fmt(IRApply(head, (x, x))) == "partfrac(x, x)"


def test_alias_trigsimp() -> None:
    """TrigSimplify head → ``trigsimp``."""
    x = IRSymbol("x")
    head = IRSymbol("TrigSimplify")
    assert fmt(IRApply(head, (x,))) == "trigsimp(x)"


def test_alias_trigexpand() -> None:
    """TrigExpand head → ``trigexpand``."""
    x = IRSymbol("x")
    head = IRSymbol("TrigExpand")
    assert fmt(IRApply(head, (x,))) == "trigexpand(x)"


def test_alias_trigreduce() -> None:
    """TrigReduce head → ``trigreduce``."""
    x = IRSymbol("x")
    head = IRSymbol("TrigReduce")
    assert fmt(IRApply(head, (x,))) == "trigreduce(x)"


def test_alias_realpart() -> None:
    """Re head → ``realpart`` in MACSYMA (not ``re``)."""
    x = IRSymbol("x")
    head = IRSymbol("Re")
    assert fmt(IRApply(head, (x,))) == "realpart(x)"


def test_alias_imagpart() -> None:
    """Im head → ``imagpart`` in MACSYMA (not ``im``)."""
    x = IRSymbol("x")
    head = IRSymbol("Im")
    assert fmt(IRApply(head, (x,))) == "imagpart(x)"


def test_alias_carg() -> None:
    """Arg head → ``carg`` in MACSYMA (not ``arg``)."""
    x = IRSymbol("x")
    head = IRSymbol("Arg")
    assert fmt(IRApply(head, (x,))) == "carg(x)"


def test_alias_primep() -> None:
    """IsPrime head → ``primep`` in MACSYMA."""
    head = IRSymbol("IsPrime")
    assert fmt(IRApply(head, (IRInteger(7),))) == "primep(7)"


def test_alias_next_prime() -> None:
    """NextPrime head → ``next_prime`` in MACSYMA."""
    head = IRSymbol("NextPrime")
    assert fmt(IRApply(head, (IRInteger(7),))) == "next_prime(7)"


def test_alias_prev_prime() -> None:
    """PrevPrime head → ``prev_prime`` in MACSYMA."""
    head = IRSymbol("PrevPrime")
    assert fmt(IRApply(head, (IRInteger(11),))) == "prev_prime(11)"


def test_alias_ifactor() -> None:
    """FactorInteger head → ``ifactor`` in MACSYMA."""
    head = IRSymbol("FactorInteger")
    assert fmt(IRApply(head, (IRInteger(12),))) == "ifactor(12)"


def test_alias_moebius() -> None:
    """MoebiusMu head → ``moebius`` in MACSYMA."""
    head = IRSymbol("MoebiusMu")
    assert fmt(IRApply(head, (IRInteger(6),))) == "moebius(6)"


def test_alias_chinese() -> None:
    """ChineseRemainder head → ``chinese`` in MACSYMA."""
    head = IRSymbol("ChineseRemainder")
    a, b = IRInteger(2), IRInteger(3)
    assert fmt(IRApply(head, (a, b))) == "chinese(2, 3)"


def test_alias_numdigits() -> None:
    """IntegerLength head → ``numdigits`` in MACSYMA."""
    head = IRSymbol("IntegerLength")
    assert fmt(IRApply(head, (IRInteger(1000),))) == "numdigits(1000)"


def test_alias_sublist() -> None:
    """Select head → ``sublist`` in MACSYMA (not ``select``)."""
    head = IRSymbol("Select")
    f = IRSymbol("evenp")
    lst = IRApply(IRSymbol("List"), (IRInteger(1), IRInteger(2)))
    assert fmt(IRApply(head, (lst, f))) == "sublist([1, 2], evenp)"


def test_alias_invert() -> None:
    """Inverse head → ``invert`` in MACSYMA (not ``inverse``)."""
    head = IRSymbol("Inverse")
    m = IRSymbol("M")
    assert fmt(IRApply(head, (m,))) == "invert(M)"


def test_alias_cbrt() -> None:
    """Cbrt head → ``cbrt`` (same in all dialects; present in default table)."""
    head = IRSymbol("Cbrt")
    assert fmt(IRApply(head, (IRInteger(2),))) == "cbrt(2)"


def test_alias_conjugate() -> None:
    """Conjugate head → ``conjugate``."""
    x = IRSymbol("x")
    head = IRSymbol("Conjugate")
    assert fmt(IRApply(head, (x,))) == "conjugate(x)"


def test_alias_rectform() -> None:
    """RectForm head → ``rectform``."""
    x = IRSymbol("x")
    head = IRSymbol("RectForm")
    assert fmt(IRApply(head, (x,))) == "rectform(x)"


def test_alias_polarform() -> None:
    """PolarForm head → ``polarform``."""
    x = IRSymbol("x")
    head = IRSymbol("PolarForm")
    assert fmt(IRApply(head, (x,))) == "polarform(x)"


def test_alias_determinant() -> None:
    """Determinant head → ``determinant``."""
    m = IRSymbol("M")
    head = IRSymbol("Determinant")
    assert fmt(IRApply(head, (m,))) == "determinant(M)"


def test_alias_transpose() -> None:
    """Transpose head → ``transpose``."""
    m = IRSymbol("M")
    head = IRSymbol("Transpose")
    assert fmt(IRApply(head, (m,))) == "transpose(M)"
