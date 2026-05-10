"""Unit tests for special_sums.py — classic convergent infinite series.

Each test verifies structural pattern recognition by checking what
try_special_infinite returns for a given (f, k, lo) triple.
"""

import math

import pytest
from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
    MUL,
    NEG,
    POW,
    IRApply,
    IRInteger,
    IRSymbol,
)

from cas_summation.special_sums import try_special_infinite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval_ir(node) -> float:
    """Naively evaluate an IR arithmetic tree to a Python float."""
    from symbolic_ir import (
        ADD,
        DIV,
        EXP,
        MUL,
        POW,
        SUB,
        IRApply,
        IRFloat,
        IRInteger,
        IRRational,
        IRSymbol,
    )

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
    if isinstance(node, IRApply):
        if node.head == ADD:
            return sum(_eval_ir(a) for a in node.args)
        if node.head == SUB:
            return _eval_ir(node.args[0]) - _eval_ir(node.args[1])
        if node.head == MUL:
            result = 1.0
            for a in node.args:
                result *= _eval_ir(a)
            return result
        if node.head == DIV:
            return _eval_ir(node.args[0]) / _eval_ir(node.args[1])
        if node.head == NEG:
            return -_eval_ir(node.args[0])
        if node.head == POW:
            return _eval_ir(node.args[0]) ** _eval_ir(node.args[1])
        if node.head == EXP:
            return math.exp(_eval_ir(node.args[0]))
    raise ValueError(f"Cannot eval: {node}")


_k = IRSymbol("k")
_x = IRSymbol("x")


# ---------------------------------------------------------------------------
# Basel problem: Σ_{k=1}^∞ 1/k^2 = π²/6
# ---------------------------------------------------------------------------


class TestBasel:
    def test_recognises_1_over_k_squared(self):
        """try_special_infinite returns π²/6 for 1/k^2, lo=1."""
        f = IRApply(DIV, (IRInteger(1), IRApply(POW, (_k, IRInteger(2)))))
        result = try_special_infinite(f, _k, IRInteger(1))
        assert result is not None
        assert _eval_ir(result) == pytest.approx(math.pi**2 / 6, rel=1e-9)

    def test_wrong_lo_returns_none(self):
        """lo=0 does not match the Basel pattern (sum starts at k=1)."""
        f = IRApply(DIV, (IRInteger(1), IRApply(POW, (_k, IRInteger(2)))))
        result = try_special_infinite(f, _k, IRInteger(0))
        assert result is None


# ---------------------------------------------------------------------------
# Basel-4: Σ_{k=1}^∞ 1/k^4 = π⁴/90
# ---------------------------------------------------------------------------


class TestBaselFour:
    def test_recognises_1_over_k_fourth(self):
        """try_special_infinite returns π⁴/90 for 1/k^4, lo=1."""
        f = IRApply(DIV, (IRInteger(1), IRApply(POW, (_k, IRInteger(4)))))
        result = try_special_infinite(f, _k, IRInteger(1))
        assert result is not None
        assert _eval_ir(result) == pytest.approx(math.pi**4 / 90, rel=1e-9)


# ---------------------------------------------------------------------------
# Leibniz: Σ_{k=0}^∞ (-1)^k/(2k+1) = π/4
# ---------------------------------------------------------------------------


class TestLeibniz:
    def test_recognises_leibniz(self):
        """try_special_infinite returns π/4 for the Leibniz series."""
        # (-1)^k / (2k + 1)
        neg_one_pow_k = IRApply(POW, (IRInteger(-1), _k))
        denom = IRApply(ADD, (IRApply(MUL, (IRInteger(2), _k)), IRInteger(1)))
        f = IRApply(DIV, (neg_one_pow_k, denom))
        result = try_special_infinite(f, _k, IRInteger(0))
        assert result is not None
        assert _eval_ir(result) == pytest.approx(math.pi / 4, rel=1e-9)

    def test_wrong_lo_returns_none(self):
        """lo=1 does not match the Leibniz pattern (starts at k=0)."""
        neg_one_pow_k = IRApply(POW, (IRInteger(-1), _k))
        denom = IRApply(ADD, (IRApply(MUL, (IRInteger(2), _k)), IRInteger(1)))
        f = IRApply(DIV, (neg_one_pow_k, denom))
        result = try_special_infinite(f, _k, IRInteger(1))
        assert result is None


# ---------------------------------------------------------------------------
# Taylor series for e: Σ_{k=0}^∞ 1/k! = e
# ---------------------------------------------------------------------------


class TestExpSeries:
    def test_recognises_inv_factorial(self):
        """try_special_infinite returns %e for 1/k!, lo=0."""
        # 1 / GammaFunc(k + 1)
        gamma_kp1 = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
        f = IRApply(DIV, (IRInteger(1), gamma_kp1))
        result = try_special_infinite(f, _k, IRInteger(0))
        assert result is not None
        # Result should be the %e symbol
        assert isinstance(result, IRSymbol) and result.name == "%e"

    def test_recognises_xk_over_factorial(self):
        """try_special_infinite returns exp(x) for x^k/k!, lo=0."""
        gamma_kp1 = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
        f = IRApply(DIV, (IRApply(POW, (_x, _k)), gamma_kp1))
        result = try_special_infinite(f, _k, IRInteger(0))
        assert result is not None
        # Should be EXP(x)
        from symbolic_ir import EXP
        assert isinstance(result, IRApply) and result.head == EXP
        assert result.args[0] == _x


# ---------------------------------------------------------------------------
# Unrecognised patterns → None
# ---------------------------------------------------------------------------


class TestUnrecognised:
    def test_harmonic_series_returns_none(self):
        """Σ 1/k diverges and is not in the table."""
        f = IRApply(DIV, (IRInteger(1), _k))
        result = try_special_infinite(f, _k, IRInteger(1))
        assert result is None

    def test_k_squared_returns_none(self):
        """Σ k^2 diverges — not a convergent series."""
        f = IRApply(POW, (_k, IRInteger(2)))
        result = try_special_infinite(f, _k, IRInteger(1))
        assert result is None
