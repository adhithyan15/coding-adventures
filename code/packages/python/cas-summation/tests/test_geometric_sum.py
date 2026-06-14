"""Unit tests for geometric_sum.py — geometric series evaluation.

We verify:
- Finite geometric sum matches direct computation.
- Infinite geometric sum matches the 1/(1−r) formula.
- Various lower bounds and coefficients.
"""

import pytest
from symbolic_ir import IRInteger, IRRational

from cas_summation.geometric_sum import geometric_sum_ir

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval_ir(node) -> float:
    """Naively evaluate an IR arithmetic tree to a Python float."""
    from symbolic_ir import (
        ADD,
        DIV,
        MUL,
        NEG,
        POW,
        SUB,
        IRApply,
        IRFloat,
        IRInteger,
        IRRational,
    )

    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
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
    raise ValueError(f"Cannot eval: {node}")


# ---------------------------------------------------------------------------
# Finite geometric sum tests
# ---------------------------------------------------------------------------


class TestFiniteGeometricSum:
    """Σ_{k=lo}^{hi} coeff·r^k matches direct computation."""

    def test_r_half_lo0_hi4(self):
        """Σ_{k=0}^4 (1/2)^k = 1 + 1/2 + 1/4 + 1/8 + 1/16 = 31/16."""
        r = IRRational(1, 2)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(0),
            hi=IRInteger(4),
            is_infinite=False,
        )
        assert _eval_ir(result) == pytest.approx(31 / 16)

    def test_r2_lo0_hi4(self):
        """Σ_{k=0}^4 2^k = 1+2+4+8+16 = 31."""
        r = IRInteger(2)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(0),
            hi=IRInteger(4),
            is_infinite=False,
        )
        assert _eval_ir(result) == pytest.approx(31.0)

    def test_r3_lo0_hi3(self):
        """Σ_{k=0}^3 3^k = 1+3+9+27 = 40."""
        r = IRInteger(3)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(0),
            hi=IRInteger(3),
            is_infinite=False,
        )
        assert _eval_ir(result) == pytest.approx(40.0)

    def test_lo1_hi5_r2(self):
        """Σ_{k=1}^5 2^k = 2+4+8+16+32 = 62."""
        r = IRInteger(2)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(1),
            hi=IRInteger(5),
            is_infinite=False,
        )
        assert _eval_ir(result) == pytest.approx(62.0)

    def test_coeff_3_r2_lo0_hi3(self):
        """Σ_{k=0}^3 3·2^k = 3·(1+2+4+8) = 45."""
        r = IRInteger(2)
        result = geometric_sum_ir(
            coeff=IRInteger(3),
            base=r,
            lo=IRInteger(0),
            hi=IRInteger(3),
            is_infinite=False,
        )
        assert _eval_ir(result) == pytest.approx(45.0)


# ---------------------------------------------------------------------------
# Infinite geometric sum tests
# ---------------------------------------------------------------------------


class TestInfiniteGeometricSum:
    """Σ_{k=lo}^∞ coeff·r^k = coeff·r^lo / (1 - r)."""

    def test_r_half_lo0_inf(self):
        """Σ_{k=0}^∞ (1/2)^k = 2."""
        r = IRRational(1, 2)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(0),
            hi=None,
            is_infinite=True,
        )
        assert _eval_ir(result) == pytest.approx(2.0)

    def test_r_third_lo0_inf(self):
        """Σ_{k=0}^∞ (1/3)^k = 3/2."""
        r = IRRational(1, 3)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(0),
            hi=None,
            is_infinite=True,
        )
        assert _eval_ir(result) == pytest.approx(1.5)

    def test_r_half_lo1_inf(self):
        """Σ_{k=1}^∞ (1/2)^k = (1/2)/(1 - 1/2) = 1."""
        r = IRRational(1, 2)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(1),
            hi=None,
            is_infinite=True,
        )
        assert _eval_ir(result) == pytest.approx(1.0)

    def test_coeff2_r_half_lo0_inf(self):
        """Σ_{k=0}^∞ 2·(1/2)^k = 2·2 = 4."""
        r = IRRational(1, 2)
        result = geometric_sum_ir(
            coeff=IRInteger(2),
            base=r,
            lo=IRInteger(0),
            hi=None,
            is_infinite=True,
        )
        assert _eval_ir(result) == pytest.approx(4.0)

    def test_r_quarter_lo2_inf(self):
        """Σ_{k=2}^∞ (1/4)^k = (1/4)^2 / (1 - 1/4) = (1/16)/(3/4) = 1/12."""
        r = IRRational(1, 4)
        result = geometric_sum_ir(
            coeff=IRInteger(1),
            base=r,
            lo=IRInteger(2),
            hi=None,
            is_infinite=True,
        )
        assert _eval_ir(result) == pytest.approx(1 / 12)
