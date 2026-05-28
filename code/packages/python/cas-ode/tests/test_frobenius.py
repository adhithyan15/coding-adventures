"""Tests for the Track C1 Frobenius / power-series ODE solver.

These exercise both the standalone helper :func:`try_frobenius_series`
and the integration with the dispatcher :func:`cas_ode.solve_ode`.

Scope (per the PR spec)
-----------------------
The helper handles second-order linear ODEs with a regular singular
point at ``x = 0`` whose indicial roots are rational and differ by a
non-integer.  Out-of-scope cases (integer-difference roots, irregular
singular points, regular points) must return ``None`` from the helper
so the dispatcher falls through to other handlers (or to the
unevaluated form).
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    EQUAL,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)
from symbolic_ir.nodes import BESSEL_J, BESSEL_Y, D

from cas_ode import build_ode_handler_table, solve_ode
from cas_ode.frobenius import (
    _is_regular_singular,
    _poly_coeffs,
    _roots_differ_by_integer,
    _solve_indicial,
    try_frobenius_series,
)

# ---------------------------------------------------------------------------
# Tiny IR builders to keep ODE construction readable.
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Y_PRIME = IRApply(D, (Y, X))
Y_DOUBLE = IRApply(D, (Y_PRIME, X))


def add(a, b):
    return IRApply(ADD, (a, b))


def sub(a, b):
    return IRApply(SUB, (a, b))


def mul(a, b):
    return IRApply(MUL, (a, b))


def neg(a):
    return IRApply(NEG, (a,))


def pow_(a, b):
    return IRApply(POW, (a, b))


def x_pow(k):
    if k == 0:
        return IRInteger(1)
    if k == 1:
        return X
    return pow_(X, IRInteger(k))


# ---------------------------------------------------------------------------
# Unit-test helpers in isolation
# ---------------------------------------------------------------------------


def test_poly_coeffs_constant():
    coeffs = _poly_coeffs(IRInteger(3), X, 3)
    assert coeffs == [Fraction(3), 0, 0, 0]


def test_poly_coeffs_x_squared_minus_quarter():
    # x² - 1/4
    expr = sub(pow_(X, IRInteger(2)), IRRational(1, 4))
    coeffs = _poly_coeffs(expr, X, 3)
    assert coeffs == [Fraction(-1, 4), Fraction(0), Fraction(1), Fraction(0)]


def test_poly_coeffs_rejects_non_polynomial():
    # cos(x) — head we don't recognise as a monomial.
    cos_x = IRApply(IRSymbol("Cos"), (X,))
    assert _poly_coeffs(cos_x, X, 3) is None


def test_solve_indicial_rational_roots():
    # F(r) = r² - 1/4 → r₁ = 1/2, r₂ = -1/2
    p0 = Fraction(1)
    q0 = Fraction(-1, 4)
    roots = _solve_indicial(p0, q0)
    assert roots == (Fraction(1, 2), Fraction(-1, 2))


def test_solve_indicial_complex_returns_none():
    # F(r) = r² + 1 → complex roots
    p0 = Fraction(1)
    q0 = Fraction(1)
    assert _solve_indicial(p0, q0) is None


def test_solve_indicial_irrational_returns_none():
    # F(r) = r² - 2 → irrational
    p0 = Fraction(1)
    q0 = Fraction(-2)
    assert _solve_indicial(p0, q0) is None


def test_roots_differ_by_integer():
    assert _roots_differ_by_integer(Fraction(1, 2), Fraction(-1, 2)) is True
    assert _roots_differ_by_integer(Fraction(3), Fraction(1)) is True
    assert _roots_differ_by_integer(Fraction(1, 2), Fraction(-1)) is False
    assert _roots_differ_by_integer(Fraction(2, 3), Fraction(1, 3)) is False


def test_is_regular_singular_bessel_form():
    # P(x) = x², Q(x) = x, R(x) = x² - ν²  (Bessel ν=1)
    P = [Fraction(0), Fraction(0), Fraction(1), Fraction(0)]
    Q = [Fraction(0), Fraction(1), Fraction(0), Fraction(0)]
    R = [Fraction(-1), Fraction(0), Fraction(1), Fraction(0)]
    pq = _is_regular_singular(P, Q, R)
    assert pq is not None
    tildeP, tildeQ = pq
    # tildeP = x·p = x·(x/x²) = 1, so [1, 0, ...]
    assert tildeP[0] == Fraction(1)
    # tildeQ = x²·q = x²·(x²-1)/x² = x² - 1, so [-1, 0, 1, ...]
    assert tildeQ[0] == Fraction(-1)
    assert tildeQ[2] == Fraction(1)


def test_is_regular_singular_regular_point_returns_none():
    # P(0) ≠ 0 — x=0 is a regular (non-singular) point.
    P = [Fraction(1), Fraction(0), Fraction(0), Fraction(0)]
    Q = [Fraction(0), Fraction(0), Fraction(0), Fraction(0)]
    R = [Fraction(0), Fraction(0), Fraction(0), Fraction(0)]
    assert _is_regular_singular(P, Q, R) is None


# ---------------------------------------------------------------------------
# Acceptance test #1: Bessel ν=1/2 ODE flows through the dispatcher and
# is recognised by Phase 21 as the BesselJ(1/2)/BesselY(1/2) family.
#
# The Frobenius helper itself BAILS on this ODE (indicial roots ±1/2 differ
# by 1, which is an integer — out of scope).  This is intentional: the
# dispatcher tries the named-ODE recognisers BEFORE Frobenius, so for the
# acceptance case the Bessel handler answers first.  The test verifies the
# end-to-end behaviour required by the spec.
# ---------------------------------------------------------------------------


def _bessel_half_ode():
    """Build ``x²y'' + xy' + (x² - 1/4)y`` as an IR zero-form expression."""
    return add(
        add(
            mul(x_pow(2), Y_DOUBLE),
            mul(X, Y_PRIME),
        ),
        mul(sub(x_pow(2), IRRational(1, 4)), Y),
    )


def test_acceptance_bessel_half_via_solve_ode():
    """``x²y'' + xy' + (x² - 1/4)y = 0`` resolves through the dispatcher
    to ``%c1·BesselJ(1/2, x) + %c2·BesselY(1/2, x)``."""
    from symbolic_vm import VM, SymbolicBackend

    backend = SymbolicBackend()
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    vm = VM(backend)

    expr = _bessel_half_ode()
    result = solve_ode(expr, Y, X, vm)
    assert result is not None
    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    # Walk the solution looking for BESSEL_J / BESSEL_Y heads.
    found_j = [False]
    found_y = [False]

    def walk(node):
        if isinstance(node, IRApply):
            if node.head == BESSEL_J:
                found_j[0] = True
            if node.head == BESSEL_Y:
                found_y[0] = True
            for a in node.args:
                walk(a)

    walk(result)
    assert found_j[0], "Solution did not contain BesselJ"
    assert found_y[0], "Solution did not contain BesselY"


def test_frobenius_bails_on_integer_difference_roots():
    """Direct call to the Frobenius helper on Bessel ν=1/2 must return None
    because the indicial roots (½, -½) differ by 1 — an integer.
    """
    expr = _bessel_half_ode()
    assert try_frobenius_series(expr, Y, X) is None


# ---------------------------------------------------------------------------
# Test #2: Indicial roots differ by a positive integer → BAIL.
# Construct an Euler-Cauchy-style ODE with roots r₁=1, r₂=0.
# x²y'' + x·y' = 0  →  r(r-1)+r = r² = 0 ... no that's repeated.
# Use x²y'' - x·y' = 0  →  r(r-1) - r = r² - 2r = r(r-2). Roots 2, 0.
# Differ by 2 — integer.
# ---------------------------------------------------------------------------


def test_frobenius_integer_difference_returns_none():
    # x²y'' - x·y' = 0
    expr = sub(
        mul(x_pow(2), Y_DOUBLE),
        mul(X, Y_PRIME),
    )
    assert try_frobenius_series(expr, Y, X) is None


# ---------------------------------------------------------------------------
# Test #3: Equal indicial roots → BAIL.
# x²y'' + x·y' = 0 has F(r) = r(r-1) + r = r²; both roots = 0.
# ---------------------------------------------------------------------------


def test_frobenius_equal_roots_returns_none():
    # x²y'' + x·y' = 0
    expr = add(
        mul(x_pow(2), Y_DOUBLE),
        mul(X, Y_PRIME),
    )
    assert try_frobenius_series(expr, Y, X) is None


# ---------------------------------------------------------------------------
# Test #4: Irregular singular point at x=0 → BAIL.
# x³y'' + y = 0 has the y''-coefficient vanishing to order 3 at x=0,
# which is beyond the Frobenius helper's m ≤ 2 scope.
# ---------------------------------------------------------------------------


def test_frobenius_irregular_singular_returns_none():
    expr = add(
        mul(x_pow(3), Y_DOUBLE),
        mul(IRInteger(1), Y),
    )
    assert try_frobenius_series(expr, Y, X) is None


# ---------------------------------------------------------------------------
# Test #5: Regular (non-singular) point → BAIL (helper returns None);
# the dispatcher then routes the ODE to its normal handler.
# ---------------------------------------------------------------------------


def test_frobenius_regular_point_returns_none():
    # y'' + y = 0  — constant-coefficient ODE; x=0 is a regular point.
    expr = add(Y_DOUBLE, Y)
    assert try_frobenius_series(expr, Y, X) is None


# ---------------------------------------------------------------------------
# Test #6: A bona fide Frobenius ODE that does NOT belong to any named
# family.  2x²y'' + 3xy' - (1 + x)y = 0.
#
# tildeP(x) = 3/2  →  p₀ = 3/2
# tildeQ(x) = -(1 + x)/2  →  q₀ = -1/2, q₁ = -1/2
# F(r) = r(r-1) + 3r/2 - 1/2 = (2r-1)(r+1)/2.
# Roots: r₁ = 1/2, r₂ = -1.  Difference 3/2 — non-integer.
#
# Recurrence for r=1/2:
#   F(1/2 + 1) = F(3/2) = 5/2,
#   a₁ = -(a₀ · (0·(1/2) + (-1/2))) / (5/2) = -((-1/2))/(5/2) = 1/5.
# We verify the first two coefficients exactly.
# ---------------------------------------------------------------------------


def test_frobenius_produces_expected_series_non_named_ode():
    # 2x²y'' + 3xy' - (1+x)y = 0
    expr = add(
        add(
            mul(mul(IRInteger(2), x_pow(2)), Y_DOUBLE),
            mul(mul(IRInteger(3), X), Y_PRIME),
        ),
        neg(mul(add(IRInteger(1), X), Y)),
    )
    result = try_frobenius_series(expr, Y, X, N=4)
    assert result is not None
    # Result shape: Equal(y, Mul(Pow(x, 1/2), Add(...polynomial...)))
    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    series = result.args[1]
    # Top level is Mul(x^(1/2), poly).
    assert isinstance(series, IRApply)
    assert series.head == MUL
    x_pow_r, poly = series.args
    # x^(1/2)
    assert isinstance(x_pow_r, IRApply)
    assert x_pow_r.head == POW
    assert x_pow_r.args[0] == X
    assert x_pow_r.args[1] == IRRational(1, 2)
    # Polynomial: a₀ = 1, a₁ = 1/5, a₂ = 1/70, a₃ = 1/1890, a₄ = 1/83160.
    # We extract all the leading constants from the flattened Add.
    leaves = []

    def walk_add(node):
        if isinstance(node, IRApply) and node.head == ADD:
            walk_add(node.args[0])
            walk_add(node.args[1])
        else:
            leaves.append(node)

    walk_add(poly)
    # a₀ = 1 (leaf is IRInteger(1))
    assert leaves[0] == IRInteger(1)
    # a₁ · x = Mul(Rational(1,5), x)
    assert leaves[1] == IRApply(MUL, (IRRational(1, 5), X))
    # a₂ · x² = Mul(Rational(1, 70), Pow(x, 2))
    assert leaves[2] == IRApply(
        MUL, (IRRational(1, 70), IRApply(POW, (X, IRInteger(2))))
    )


# ---------------------------------------------------------------------------
# Additional regression: ensure the dispatcher hand-off works for an ODE
# that the named recognisers reject but Frobenius accepts.  Without VM
# integration the test only verifies ``solve_ode`` end-to-end.
# ---------------------------------------------------------------------------


def test_dispatcher_routes_to_frobenius_for_non_named_ode():
    from symbolic_vm import VM, SymbolicBackend

    backend = SymbolicBackend()
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    vm = VM(backend)

    # The 2x²y'' + 3xy' - (1+x)y = 0 ODE again.
    expr = add(
        add(
            mul(mul(IRInteger(2), x_pow(2)), Y_DOUBLE),
            mul(mul(IRInteger(3), X), Y_PRIME),
        ),
        neg(mul(add(IRInteger(1), X), Y)),
    )
    result = solve_ode(expr, Y, X, vm)
    assert result is not None
    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    # Must contain x^(1/2) somewhere — the leading exponent.
    found_half_pow = [False]

    def walk(node):
        if isinstance(node, IRApply):
            if (
                node.head == POW
                and len(node.args) == 2
                and node.args[0] == X
                and node.args[1] == IRRational(1, 2)
            ):
                found_half_pow[0] = True
            for a in node.args:
                walk(a)

    walk(result)
    assert found_half_pow[0], "Result did not contain x^(1/2)"
