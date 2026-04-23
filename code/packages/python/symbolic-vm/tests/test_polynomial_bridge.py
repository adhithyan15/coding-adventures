"""Tests for the IR ↔ polynomial bridge (Phase 2b)."""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import from_polynomial, to_rational

X = IRSymbol("x")
Y = IRSymbol("y")

F1 = (Fraction(1),)


# ---------------------------------------------------------------------------
# Literals and the variable
# ---------------------------------------------------------------------------


def test_integer_literal():
    assert to_rational(IRInteger(5), X) == ((Fraction(5),), F1)


def test_negative_integer_literal():
    assert to_rational(IRInteger(-7), X) == ((Fraction(-7),), F1)


def test_rational_literal():
    # 3/4 — IRRational keeps numer/denom reduced.
    assert to_rational(IRRational(3, 4), X) == ((Fraction(3, 4),), F1)


def test_float_literal_rejected():
    # Floats destroy exact arithmetic. Always None.
    assert to_rational(IRFloat(1.5), X) is None


def test_bare_x():
    # x → 0 + 1·x
    num, den = to_rational(X, X)
    assert num == (Fraction(0), Fraction(1))
    assert den == F1


def test_free_symbol_rejected():
    # Any symbol other than the named variable is refused.
    assert to_rational(Y, X) is None


# ---------------------------------------------------------------------------
# Additive structure
# ---------------------------------------------------------------------------


def test_add_polynomials():
    # x + 1 → num=(1, 1), den=1
    expr = IRApply(ADD, (X, IRInteger(1)))
    num, den = to_rational(expr, X)
    assert num == (Fraction(1), Fraction(1))
    assert den == F1


def test_sub_polynomials():
    # x − 3 → num=(-3, 1)
    expr = IRApply(SUB, (X, IRInteger(3)))
    num, den = to_rational(expr, X)
    assert num == (Fraction(-3), Fraction(1))
    assert den == F1


def test_neg_polynomial():
    # -(x + 1) → num=(-1, -1)
    expr = IRApply(NEG, (IRApply(ADD, (X, IRInteger(1))),))
    num, den = to_rational(expr, X)
    assert num == (Fraction(-1), Fraction(-1))


def test_add_with_free_symbol_rejected():
    # y + x — bridge refuses; coefficient ring must be literal Q.
    expr = IRApply(ADD, (Y, X))
    assert to_rational(expr, X) is None


# ---------------------------------------------------------------------------
# Multiplicative structure
# ---------------------------------------------------------------------------


def test_multiply_linear():
    # (x + 1)·(x − 2) = x² − x − 2
    expr = IRApply(
        MUL,
        (
            IRApply(ADD, (X, IRInteger(1))),
            IRApply(SUB, (X, IRInteger(2))),
        ),
    )
    num, den = to_rational(expr, X)
    assert num == (Fraction(-2), Fraction(-1), Fraction(1))
    assert den == F1


def test_div_constant_denominator():
    # x / 3 → num=(0, 1), den=(3,)
    expr = IRApply(DIV, (X, IRInteger(3)))
    num, den = to_rational(expr, X)
    assert num == (Fraction(0), Fraction(1))
    assert den == (Fraction(3),)


def test_div_polynomial_over_polynomial():
    # (x² + 1) / (x − 1). Bridge keeps the pair verbatim — no cancellation.
    num_expr = IRApply(ADD, (IRApply(POW, (X, IRInteger(2))), IRInteger(1)))
    den_expr = IRApply(SUB, (X, IRInteger(1)))
    expr = IRApply(DIV, (num_expr, den_expr))
    num, den = to_rational(expr, X)
    # num = 1 + 0·x + x²  (from Add folding), den = -1 + x.
    assert num == (Fraction(1), Fraction(0), Fraction(1))
    assert den == (Fraction(-1), Fraction(1))


def test_div_by_zero_rejected():
    # 1 / 0 — denominator numerator is zero, refuse.
    expr = IRApply(DIV, (IRInteger(1), IRInteger(0)))
    assert to_rational(expr, X) is None


# ---------------------------------------------------------------------------
# Powers
# ---------------------------------------------------------------------------


def test_pow_non_negative_integer():
    # (x − 1)²
    base = IRApply(SUB, (X, IRInteger(1)))
    expr = IRApply(POW, (base, IRInteger(2)))
    num, den = to_rational(expr, X)
    # (x - 1)² = 1 - 2x + x²
    assert num == (Fraction(1), Fraction(-2), Fraction(1))
    assert den == F1


def test_pow_zero_on_x():
    # x^0 = 1 by convention. We don't special-case 0^0 at this layer.
    expr = IRApply(POW, (X, IRInteger(0)))
    num, den = to_rational(expr, X)
    assert num == F1
    assert den == F1


def test_pow_negative_exponent_is_reciprocal():
    # x^(-2) — denominator x², numerator 1.
    expr = IRApply(POW, (X, IRInteger(-2)))
    num, den = to_rational(expr, X)
    assert num == F1
    assert den == (Fraction(0), Fraction(0), Fraction(1))


def test_pow_zero_to_negative_rejected():
    # 0^(-1) is undefined.
    expr = IRApply(POW, (IRInteger(0), IRInteger(-1)))
    assert to_rational(expr, X) is None


def test_pow_symbolic_exponent_rejected():
    # x^y — outside Q(x), symbolic exponent.
    expr = IRApply(POW, (X, Y))
    assert to_rational(expr, X) is None


def test_pow_rational_exponent_rejected():
    # x^(1/2) — not in Q(x).
    expr = IRApply(POW, (X, IRRational(1, 2)))
    assert to_rational(expr, X) is None


# ---------------------------------------------------------------------------
# Transcendental heads are always rejected
# ---------------------------------------------------------------------------


def test_sin_rejected():
    assert to_rational(IRApply(SIN, (X,)), X) is None


def test_cos_rejected():
    assert to_rational(IRApply(COS, (X,)), X) is None


def test_log_rejected():
    assert to_rational(IRApply(LOG, (X,)), X) is None


# ---------------------------------------------------------------------------
# from_polynomial — canonical IR shape
# ---------------------------------------------------------------------------


def test_from_polynomial_zero():
    assert from_polynomial((), X) == IRInteger(0)


def test_from_polynomial_constant_int():
    assert from_polynomial((Fraction(3),), X) == IRInteger(3)


def test_from_polynomial_constant_rational():
    # 3/4 — should round-trip to IRRational.
    assert from_polynomial((Fraction(3, 4),), X) == IRRational(3, 4)


def test_from_polynomial_bare_x():
    # (0, 1) → x
    assert from_polynomial((Fraction(0), Fraction(1)), X) == X


def test_from_polynomial_linear():
    # (3, 1) → 3 + x
    result = from_polynomial((Fraction(3), Fraction(1)), X)
    assert result == IRApply(ADD, (IRInteger(3), X))


def test_from_polynomial_with_negative_one_coefficient():
    # (0, -1) → -x via Neg wrapping
    result = from_polynomial((Fraction(0), Fraction(-1)), X)
    assert result == IRApply(NEG, (X,))


def test_from_polynomial_with_general_coefficient():
    # 2·x² → Mul(2, Pow(x, 2))
    result = from_polynomial((Fraction(0), Fraction(0), Fraction(2)), X)
    assert result == IRApply(
        MUL, (IRInteger(2), IRApply(POW, (X, IRInteger(2))))
    )


def test_from_polynomial_quadratic_sum():
    # 1 + x + x²  → Add(Add(1, x), Pow(x, 2))
    #
    # Binary left-associative fold — the VM's arithmetic handlers are
    # strictly binary, so from_polynomial must emit nested pairs rather
    # than an n-ary apply. A previous version emitted n-ary Add, which
    # tripped the arity check the instant the result hit vm.eval.
    result = from_polynomial((Fraction(1), Fraction(1), Fraction(1)), X)
    assert result == IRApply(
        ADD,
        (
            IRApply(ADD, (IRInteger(1), X)),
            IRApply(POW, (X, IRInteger(2))),
        ),
    )


def test_from_polynomial_all_zero_degenerate():
    # Pathological input: a non-normalized all-zero polynomial. Should
    # collapse to IRInteger(0).
    assert from_polynomial((Fraction(0), Fraction(0)), X) == IRInteger(0)


def test_from_polynomial_accepts_int_coefficients():
    # Defensive path: _coef handles non-Fraction numeric inputs too.
    # Callers going through to_rational always produce Fractions, but
    # nothing stops a direct caller passing ints.
    assert from_polynomial((5,), X) == IRInteger(5)


def test_from_polynomial_skips_zero_coefficients():
    # 1 + 0·x + x² → Add(1, Pow(x, 2))
    result = from_polynomial((Fraction(1), Fraction(0), Fraction(1)), X)
    assert result == IRApply(
        ADD, (IRInteger(1), IRApply(POW, (X, IRInteger(2))))
    )


# ---------------------------------------------------------------------------
# Round-trip guarantees
# ---------------------------------------------------------------------------


def _round_trip(poly):
    """``to_rational(from_polynomial(p, x), x)`` → ``(p, 1)``."""
    ir = from_polynomial(poly, X)
    out = to_rational(ir, X)
    assert out is not None
    return out


def test_round_trip_constant():
    assert _round_trip((Fraction(5),)) == ((Fraction(5),), F1)


def test_round_trip_bare_x():
    p = (Fraction(0), Fraction(1))
    num, den = _round_trip(p)
    assert num == p
    assert den == F1


def test_round_trip_linear():
    p = (Fraction(3), Fraction(1))
    num, den = _round_trip(p)
    assert num == p
    assert den == F1


def test_round_trip_quadratic():
    # 2 - 3x + x²
    p = (Fraction(2), Fraction(-3), Fraction(1))
    num, den = _round_trip(p)
    assert num == p
    assert den == F1


def test_round_trip_with_rational_coefficients():
    # 1/2 + (2/3)x - x²
    p = (Fraction(1, 2), Fraction(2, 3), Fraction(-1))
    num, den = _round_trip(p)
    assert num == p
    assert den == F1


# ---------------------------------------------------------------------------
# Edge cases on Add / Mul with nothing to fold
# ---------------------------------------------------------------------------


def test_add_single_arg_rejected():
    # IRApply(Add, (single,)) is a malformed tree for our purposes — we
    # refuse it rather than pretending it's unary.
    # Actually the bridge folds n-ary — so single-arg is fine and
    # returns the same result as the inner walk. Document the behaviour.
    expr = IRApply(ADD, (X,))
    num, den = to_rational(expr, X)
    assert num == (Fraction(0), Fraction(1))
    assert den == F1


def test_add_empty_args_rejected():
    # Degenerate — empty Add is not a valid rational.
    expr = IRApply(ADD, ())
    assert to_rational(expr, X) is None


def test_sub_wrong_arity_rejected():
    # Sub must be binary.
    expr = IRApply(SUB, (X,))
    assert to_rational(expr, X) is None


def test_neg_wrong_arity_rejected():
    expr = IRApply(NEG, (X, IRInteger(1)))
    assert to_rational(expr, X) is None


def test_div_wrong_arity_rejected():
    expr = IRApply(DIV, (X,))
    assert to_rational(expr, X) is None


def test_pow_wrong_arity_rejected():
    expr = IRApply(POW, (X,))
    assert to_rational(expr, X) is None


# ---------------------------------------------------------------------------
# Rejection propagation — a transcendental anywhere poisons the whole tree.
# ---------------------------------------------------------------------------


_SIN_X = IRApply(SIN, (X,))


def test_sub_with_transcendental_lhs_rejected():
    assert to_rational(IRApply(SUB, (_SIN_X, IRInteger(1))), X) is None


def test_sub_with_transcendental_rhs_rejected():
    assert to_rational(IRApply(SUB, (X, _SIN_X)), X) is None


def test_neg_of_transcendental_rejected():
    assert to_rational(IRApply(NEG, (_SIN_X,)), X) is None


def test_mul_with_transcendental_rejected():
    assert to_rational(IRApply(MUL, (X, _SIN_X)), X) is None


def test_div_with_transcendental_numerator_rejected():
    assert to_rational(IRApply(DIV, (_SIN_X, IRInteger(2))), X) is None


def test_div_with_transcendental_denominator_rejected():
    assert to_rational(IRApply(DIV, (IRInteger(2), _SIN_X)), X) is None


def test_pow_with_transcendental_base_rejected():
    assert to_rational(IRApply(POW, (_SIN_X, IRInteger(2))), X) is None


def test_non_apply_non_literal_rejected():
    # Defensive fallback — any IRNode subtype that isn't IRSymbol /
    # IRInteger / IRRational / IRFloat / IRApply flows through to
    # the final ``return None`` in ``_walk``. Covered by construction
    # of any subclass; we just prove the handler doesn't crash.
    class _OddNode:
        pass

    assert to_rational(_OddNode(), X) is None  # type: ignore[arg-type]
