"""End-to-end simplify — canonical + numeric-fold + identity rules."""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    POW,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRSymbol,
)

from cas_simplify import simplify

x, y, z = IRSymbol("x"), IRSymbol("y"), IRSymbol("z")
ZERO = IRInteger(0)
ONE = IRInteger(1)


# ---- single-rule tests -----------------------------------------------------


def test_add_zero() -> None:
    assert simplify(IRApply(ADD, (x, ZERO))) == x


def test_mul_one() -> None:
    assert simplify(IRApply(MUL, (x, ONE))) == x


def test_mul_zero() -> None:
    assert simplify(IRApply(MUL, (x, ZERO))) == ZERO


def test_pow_zero() -> None:
    assert simplify(IRApply(POW, (x, ZERO))) == ONE


def test_pow_one() -> None:
    assert simplify(IRApply(POW, (x, ONE))) == x


def test_one_to_anything() -> None:
    assert simplify(IRApply(POW, (ONE, x))) == ONE


def test_sub_self() -> None:
    assert simplify(IRApply(SUB, (x, x))) == ZERO


def test_div_self() -> None:
    assert simplify(IRApply(DIV, (x, x))) == ONE


def test_log_exp() -> None:
    assert simplify(IRApply(LOG, (IRApply(EXP, (x,)),))) == x


def test_exp_log() -> None:
    assert simplify(IRApply(EXP, (IRApply(LOG, (x,)),))) == x


def test_sin_zero() -> None:
    assert simplify(IRApply(SIN, (ZERO,))) == ZERO


def test_cos_zero() -> None:
    assert simplify(IRApply(COS, (ZERO,))) == ONE


# ---- compound interactions -------------------------------------------------


def test_canonical_then_identity() -> None:
    """Mul(Add(x, 0), 1) → x."""
    inner = IRApply(ADD, (x, ZERO))
    expr = IRApply(MUL, (inner, ONE))
    assert simplify(expr) == x


def test_numeric_fold_collapses_literals() -> None:
    """Add(1, 2, 3, x) → Add(6, x)."""
    expr = IRApply(ADD, (IRInteger(1), IRInteger(2), IRInteger(3), x))
    out = simplify(expr)
    assert out == IRApply(ADD, (IRInteger(6), x))


def test_double_zero_add_collapses() -> None:
    """((z+0)+0) → z."""
    inner = IRApply(ADD, (z, ZERO))
    expr = IRApply(ADD, (inner, ZERO))
    assert simplify(expr) == z


def test_already_simple_unchanged() -> None:
    """Simple expressions pass through unchanged (idempotent)."""
    expr = IRApply(ADD, (x, y))
    out = simplify(expr)
    # Canonical may sort args, so check equivalence both ways.
    assert out in {IRApply(ADD, (x, y)), IRApply(ADD, (y, x))}


def test_idempotent() -> None:
    """simplify(simplify(x)) == simplify(x)."""
    expr = IRApply(MUL, (IRApply(ADD, (x, ZERO)), ONE))
    once = simplify(expr)
    twice = simplify(once)
    assert once == twice


def test_simplify_descends() -> None:
    """Simplification fires inside subexpressions: Mul(2, Add(z, 0)) → Mul(2, z)."""
    inner = IRApply(ADD, (z, ZERO))
    expr = IRApply(MUL, (IRInteger(2), inner))
    out = simplify(expr)
    # Because canonical sorts Mul args (Integer < Symbol):
    assert out == IRApply(MUL, (IRInteger(2), z))
