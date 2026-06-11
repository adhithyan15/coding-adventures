"""Tests for the Track I1 canonical infinite-series recogniser.

Verifies that ``try_closed_form_series`` returns the correct IR for each
recognised series and ``None`` for shapes that fall outside the table.

Strategy
--------

For each closed form we:
1. Construct the summand IR by hand to match exactly what the parser
   would emit (the same shapes already used by ``test_special_sums``
   and ``test_gosper``).
2. Call ``try_closed_form_series(f, k, lo, hi)`` directly to verify
   the structural recogniser.
3. Numerically evaluate the returned IR (via a small recursive helper)
   and compare to the expected mathematical value.
4. End-to-end smoke-test through ``evaluate_sum`` to confirm the
   dispatcher wiring routes ``hi = %inf`` cases through the new path
   without disturbing finite Gosper / Faulhaber routes.

We also assert that representative fall-through cases — odd zeta,
wrong lower bound, finite upper bound, non-rational summand — return
``None`` so the dispatcher continues to its next handler.
"""

from __future__ import annotations

import math
from fractions import Fraction

import pytest
from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EXP,
    GAMMA_FUNC,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SINH,
    SUB,
    SUM,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_summation import evaluate_sum
from cas_summation.series_closed_forms import (
    _bernoulli,
    _eta_even,
    _zeta_even,
    try_closed_form_series,
)

# ---------------------------------------------------------------------------
# Stub VM (mirrors test_summation.py's; folds %pi/%e to floats so we can
# numerically validate the closed forms).
# ---------------------------------------------------------------------------


class _StubVM:
    """Minimal IR evaluator: folds Integer/Rational arithmetic, leaves
    symbols alone."""

    def eval(self, node: IRNode) -> IRNode:
        from symbolic_ir import SUB as _SUB

        if isinstance(node, (IRInteger, IRRational, IRFloat, IRSymbol)):
            return node
        if not isinstance(node, IRApply):
            return node
        args = tuple(self.eval(a) for a in node.args)

        def _to_frac(n: IRNode) -> Fraction | None:
            if isinstance(n, IRInteger):
                return Fraction(n.value)
            if isinstance(n, IRRational):
                return Fraction(n.numer, n.denom)
            return None

        if node.head == ADD:
            total = Fraction(0)
            symbolic = []
            for a in args:
                f = _to_frac(a)
                if f is None:
                    symbolic.append(a)
                else:
                    total += f
            if not symbolic:
                return _int_or_frac(total)
            if total == 0 and len(symbolic) == 1:
                return symbolic[0]
            return IRApply(ADD, tuple(symbolic + [_int_or_frac(total)]))
        if node.head == _SUB and len(args) == 2:
            fa, fb = _to_frac(args[0]), _to_frac(args[1])
            if fa is not None and fb is not None:
                return _int_or_frac(fa - fb)
            return IRApply(_SUB, args)
        if node.head == MUL:
            total = Fraction(1)
            symbolic = []
            for a in args:
                f = _to_frac(a)
                if f is None:
                    symbolic.append(a)
                else:
                    total *= f
            if not symbolic:
                return _int_or_frac(total)
            if total == 1 and len(symbolic) == 1:
                return symbolic[0]
            return IRApply(MUL, tuple(symbolic + [_int_or_frac(total)]))
        if node.head == DIV and len(args) == 2:
            fa, fb = _to_frac(args[0]), _to_frac(args[1])
            if fa is not None and fb is not None and fb != 0:
                return _int_or_frac(fa / fb)
            return IRApply(DIV, args)
        if node.head == POW and len(args) == 2:
            fa, fb = _to_frac(args[0]), _to_frac(args[1])
            if (
                fa is not None
                and fb is not None
                and fb.denominator == 1
            ):
                exp_int = int(fb.numerator)
                if exp_int >= 0:
                    return _int_or_frac(fa**exp_int)
                if fa != 0:
                    return _int_or_frac(Fraction(1) / (fa ** -exp_int))
            return IRApply(POW, args)
        return IRApply(node.head, args)


def _int_or_frac(f: Fraction) -> IRNode:
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _numeric(node: IRNode) -> float:
    """Recursively evaluate IR to a Python float (substitutes %pi, %e, log).

    Used only for test assertions — production code never needs floats.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "%pi":
            return math.pi
        if node.name == "%e":
            return math.e
        raise AssertionError(f"Free symbol {node.name} in numeric eval")
    if isinstance(node, IRApply):
        if node.head == ADD:
            return sum(_numeric(a) for a in node.args)
        if node.head == SUB:
            return _numeric(node.args[0]) - _numeric(node.args[1])
        if node.head == MUL:
            result = 1.0
            for a in node.args:
                result *= _numeric(a)
            return result
        if node.head == DIV:
            return _numeric(node.args[0]) / _numeric(node.args[1])
        if node.head == NEG:
            return -_numeric(node.args[0])
        if node.head == POW:
            return _numeric(node.args[0]) ** _numeric(node.args[1])
        if node.head == LOG:
            return math.log(_numeric(node.args[0]))
        if node.head == EXP:
            return math.exp(_numeric(node.args[0]))
    raise AssertionError(f"Unsupported numeric eval for {node!r}")


_k = IRSymbol("k")
_x = IRSymbol("x")
_INF = IRSymbol("%inf")


# ---------------------------------------------------------------------------
# IR-shape helpers — match the parser's emitted forms for the summands.
# ---------------------------------------------------------------------------


def _inv_k_pow(m: int) -> IRNode:
    """Build ``1/k^m`` IR.  m=1 → ``1/k`` (no Pow)."""
    if m == 1:
        return IRApply(DIV, (IRInteger(1), _k))
    return IRApply(DIV, (IRInteger(1), IRApply(POW, (_k, IRInteger(m)))))


def _alt_inv_k_pow(m: int) -> IRNode:
    """Build ``(-1)^(k-1) / k^m`` IR."""
    neg_one_pow = IRApply(POW, (IRInteger(-1), IRApply(SUB, (_k, IRInteger(1)))))
    if m == 1:
        return IRApply(DIV, (neg_one_pow, _k))
    return IRApply(
        DIV, (neg_one_pow, IRApply(POW, (_k, IRInteger(m))))
    )


def _inv_factorial() -> IRNode:
    """Build ``1/k!`` (= ``1/Gamma(k+1)``)."""
    gamma = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
    return IRApply(DIV, (IRInteger(1), gamma))


def _xk_over_factorial() -> IRNode:
    """Build ``x^k / k!``."""
    gamma = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
    return IRApply(DIV, (IRApply(POW, (_x, _k)), gamma))


def _gamma_lin(slope: int, intercept: int) -> IRNode:
    """Build ``GammaFunc(slope·k + intercept + 1)``."""
    return IRApply(
        GAMMA_FUNC,
        (
            IRApply(
                ADD,
                (
                    IRApply(MUL, (IRInteger(slope), _k)),
                    IRInteger(intercept + 1),
                ),
            ),
        ),
    )


def _pow_x_lin(slope: int, intercept: int) -> IRNode:
    """Build ``x^(slope·k + intercept)`` (or ``x^(slope·k)`` if intercept=0)."""
    if intercept == 0:
        exp = IRApply(MUL, (IRInteger(slope), _k))
    else:
        exp = IRApply(
            ADD,
            (IRApply(MUL, (IRInteger(slope), _k)), IRInteger(intercept)),
        )
    return IRApply(POW, (_x, exp))


def _cos_summand() -> IRNode:
    """``(-1)^k · x^(2k) / (2k)!``."""
    sign = IRApply(POW, (IRInteger(-1), _k))
    body = IRApply(DIV, (_pow_x_lin(2, 0), _gamma_lin(2, 0)))
    return IRApply(MUL, (sign, body))


def _sin_summand() -> IRNode:
    """``(-1)^k · x^(2k+1) / (2k+1)!``."""
    sign = IRApply(POW, (IRInteger(-1), _k))
    body = IRApply(DIV, (_pow_x_lin(2, 1), _gamma_lin(2, 1)))
    return IRApply(MUL, (sign, body))


def _cosh_summand() -> IRNode:
    """``x^(2k) / (2k)!``."""
    return IRApply(DIV, (_pow_x_lin(2, 0), _gamma_lin(2, 0)))


def _sinh_summand() -> IRNode:
    """``x^(2k+1) / (2k+1)!``."""
    return IRApply(DIV, (_pow_x_lin(2, 1), _gamma_lin(2, 1)))


# ---------------------------------------------------------------------------
# Bernoulli helper — independent verification against known values.
# ---------------------------------------------------------------------------


class TestBernoulli:
    """Smoke-test the generic Bernoulli recurrence against a known table.

    Reference (Knuth convention, ``B_1 = -1/2``):

    +----+------------+
    | n  | B_n        |
    +====+============+
    | 0  | 1          |
    | 1  | −1/2       |
    | 2  | 1/6        |
    | 4  | −1/30      |
    | 6  | 1/42       |
    | 8  | −1/30      |
    | 10 | 5/66       |
    | 12 | −691/2730  |
    +----+------------+
    """

    def test_known_values(self):
        assert _bernoulli(0) == Fraction(1)
        assert _bernoulli(1) == Fraction(-1, 2)
        assert _bernoulli(2) == Fraction(1, 6)
        assert _bernoulli(3) == Fraction(0)
        assert _bernoulli(4) == Fraction(-1, 30)
        assert _bernoulli(6) == Fraction(1, 42)
        assert _bernoulli(8) == Fraction(-1, 30)
        assert _bernoulli(10) == Fraction(5, 66)
        assert _bernoulli(12) == Fraction(-691, 2730)

    def test_odd_indices_zero(self):
        """B_{2m+1} = 0 for m ≥ 1 (Bernoulli's identity)."""
        for n in (3, 5, 7, 9, 11):
            assert _bernoulli(n) == Fraction(0)

    def test_zeta_coefficient_matches_table(self):
        """Check the derived ζ(2m)/π^(2m) coefficient against the spec."""
        assert _zeta_even(1) == Fraction(1, 6)
        assert _zeta_even(2) == Fraction(1, 90)
        assert _zeta_even(3) == Fraction(1, 945)
        assert _zeta_even(4) == Fraction(1, 9450)
        assert _zeta_even(5) == Fraction(1, 93555)
        assert _zeta_even(6) == Fraction(691, 638512875)

    def test_eta_coefficient_matches_table(self):
        """Check the derived η(2m)/π^(2m) coefficient against the spec."""
        assert _eta_even(1) == Fraction(1, 12)
        assert _eta_even(2) == Fraction(7, 720)
        assert _eta_even(3) == Fraction(31, 30240)


# ---------------------------------------------------------------------------
# Zeta(2m) family
# ---------------------------------------------------------------------------


class TestZetaFamily:
    @pytest.mark.parametrize(
        "m, expected_value",
        [
            (1, math.pi**2 / 6),
            (2, math.pi**4 / 90),
            (3, math.pi**6 / 945),
            (4, math.pi**8 / 9450),
            (5, math.pi**10 / 93555),
            (6, 691 * math.pi**12 / 638512875),
        ],
    )
    def test_zeta_2m(self, m: int, expected_value: float):
        f = _inv_k_pow(2 * m)
        result = try_closed_form_series(f, _k, IRInteger(1), _INF)
        assert result is not None
        assert _numeric(result) == pytest.approx(expected_value, rel=1e-12)

    def test_odd_zeta_falls_through(self):
        """``Σ 1/k³`` is not closed-form — should return None."""
        f = _inv_k_pow(3)
        assert try_closed_form_series(f, _k, IRInteger(1), _INF) is None

    def test_zeta_14_falls_through(self):
        """``Σ 1/k¹⁴`` is past the supported m ≤ 6 range."""
        f = _inv_k_pow(14)
        assert try_closed_form_series(f, _k, IRInteger(1), _INF) is None

    def test_wrong_lo_falls_through(self):
        """``Σ_{k=2}^∞ 1/k²`` — spec requires lo=1."""
        f = _inv_k_pow(2)
        assert try_closed_form_series(f, _k, IRInteger(2), _INF) is None


# ---------------------------------------------------------------------------
# Eta family — alternating zetas.
# ---------------------------------------------------------------------------


class TestEtaFamily:
    def test_eta_1_mercator(self):
        """``Σ (-1)^(k-1)/k = log(2)``."""
        f = _alt_inv_k_pow(1)
        result = try_closed_form_series(f, _k, IRInteger(1), _INF)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == LOG
        assert result.args[0] == IRInteger(2)
        assert _numeric(result) == pytest.approx(math.log(2), rel=1e-12)

    @pytest.mark.parametrize(
        "m, expected_value",
        [
            (1, math.pi**2 / 12),
            (2, 7 * math.pi**4 / 720),
            (3, 31 * math.pi**6 / 30240),
        ],
    )
    def test_eta_2m(self, m: int, expected_value: float):
        f = _alt_inv_k_pow(2 * m)
        result = try_closed_form_series(f, _k, IRInteger(1), _INF)
        assert result is not None
        assert _numeric(result) == pytest.approx(expected_value, rel=1e-12)


# ---------------------------------------------------------------------------
# Factorial-based series.
# ---------------------------------------------------------------------------


class TestFactorialSeries:
    def test_e_series(self):
        """``Σ_{k=0}^∞ 1/k! = %e``."""
        f = _inv_factorial()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRSymbol) and result.name == "%e"

    def test_exp_series(self):
        """``Σ_{k=0}^∞ x^k/k! = exp(x)``."""
        f = _xk_over_factorial()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == EXP
        assert result.args[0] == _x

    def test_cos_series(self):
        """``Σ (-1)^k · x^(2k)/(2k)! = cos(x)``."""
        f = _cos_summand()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == COS
        assert result.args[0] == _x

    def test_sin_series(self):
        """``Σ (-1)^k · x^(2k+1)/(2k+1)! = sin(x)``."""
        f = _sin_summand()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == SIN
        assert result.args[0] == _x

    def test_cosh_series(self):
        """``Σ x^(2k)/(2k)! = cosh(x)``."""
        f = _cosh_summand()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == COSH
        assert result.args[0] == _x

    def test_sinh_series(self):
        """``Σ x^(2k+1)/(2k+1)! = sinh(x)``."""
        f = _sinh_summand()
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == SINH
        assert result.args[0] == _x

    def test_wrong_lo_factorial(self):
        """``Σ_{k=1}^∞ 1/k!`` falls through (spec requires lo=0)."""
        f = _inv_factorial()
        assert try_closed_form_series(f, _k, IRInteger(1), _INF) is None


# ---------------------------------------------------------------------------
# Fall-through cases.
# ---------------------------------------------------------------------------


class TestFallthrough:
    def test_sin_k_returns_none(self):
        """``Σ sin(k)`` — not in the table, returns None."""
        f = IRApply(SIN, (_k,))
        assert try_closed_form_series(f, _k, IRInteger(1), _INF) is None

    def test_finite_upper_bound_returns_none(self):
        """Finite hi → caller should route through Faulhaber/Gosper."""
        f = _inv_k_pow(2)
        assert try_closed_form_series(f, _k, IRInteger(1), IRInteger(100)) is None

    def test_negative_x_in_exp_series_still_matches(self):
        """``x = -y`` is still a symbolic base; recogniser should fire."""
        y = IRSymbol("y")
        gamma = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
        neg_y = IRApply(NEG, (y,))
        f = IRApply(DIV, (IRApply(POW, (neg_y, _k)), gamma))
        result = try_closed_form_series(f, _k, IRInteger(0), _INF)
        assert result is not None
        assert isinstance(result, IRApply) and result.head == EXP


# ---------------------------------------------------------------------------
# End-to-end dispatcher integration.
# ---------------------------------------------------------------------------


class TestDispatcherIntegration:
    """Confirm ``evaluate_sum`` routes the new patterns through the I1
    handler and leaves earlier handlers undisturbed."""

    def test_evaluate_sum_zeta_6(self):
        """``evaluate_sum(1/k^6, k, 1, %inf)`` → ``π^6/945``."""
        f = _inv_k_pow(6)
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(1), _INF, vm)
        assert _numeric(result) == pytest.approx(math.pi**6 / 945, rel=1e-12)

    def test_evaluate_sum_eta_1(self):
        """``evaluate_sum((-1)^(k-1)/k, k, 1, %inf)`` → ``log(2)``."""
        f = _alt_inv_k_pow(1)
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(1), _INF, vm)
        # VM doesn't fold log; should be a LOG node.
        assert isinstance(result, IRApply) and result.head == LOG

    def test_evaluate_sum_cos_series(self):
        """``evaluate_sum((-1)^k · x^(2k)/(2k)!, k, 0, %inf)`` → ``cos(x)``."""
        f = _cos_summand()
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(0), _INF, vm)
        assert isinstance(result, IRApply) and result.head == COS
        assert result.args[0] == _x

    def test_evaluate_sum_falls_through_finite(self):
        """``evaluate_sum(1/k^2, k, 1, 100)`` should NOT use the I1
        path; numeric small-range handler computes it directly."""
        f = _inv_k_pow(2)
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(100), vm)
        # The numeric handler returns a Rational; ζ(2) ≈ 1.6449.
        assert _numeric(result) == pytest.approx(
            sum(1 / k**2 for k in range(1, 101)), rel=1e-9
        )

    def test_gosper_regression_finite_k_times_2k(self):
        """Track H1 regression: ``Σ_{k=1}^N k·2^k`` still routes through
        Gosper, not through this new module."""
        N = IRSymbol("N")
        f = IRApply(MUL, (_k, IRApply(POW, (IRInteger(2), _k))))
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(1), N, vm)
        # Gosper handler returns a non-trivial closed form (an IRApply).
        # Verify it's not the unevaluated SUM.
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_unrecognised_infinite_returns_unevaluated(self):
        """``Σ sin(k)`` should fall all the way through to the unevaluated
        SUM IR — no spurious closed form."""
        f = IRApply(SIN, (_k,))
        vm = _StubVM()
        result = evaluate_sum(f, _k, IRInteger(1), _INF, vm)
        assert isinstance(result, IRApply) and result.head == SUM
