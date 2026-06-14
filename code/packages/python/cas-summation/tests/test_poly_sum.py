"""Unit tests for poly_sum.py — Faulhaber polynomial sum formulas.

We verify two things for each formula:
1. faulhaber_ir(m, concrete_n) evaluates to the known integer value.
2. The formula is structurally correct (produces an IRNode, not None).

Concrete spot-checks:
  S(4, 0) = 4        (1+1+1+1)
  S(4, 1) = 10       (1+2+3+4)
  S(4, 2) = 30       (1+4+9+16)
  S(4, 3) = 100      (1+8+27+64)
  S(4, 4) = 354      (1+16+81+256)
  S(4, 5) = 1300     (1+32+243+1024)
"""


from symbolic_ir import IRInteger, IRSymbol

from cas_summation.poly_sum import faulhaber_ir, poly_sum_ir

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


# Known Faulhaber values for n=4.
_EXPECTED_S4: dict[int, int] = {
    0: 4,
    1: 10,
    2: 30,
    3: 100,
    4: 354,
    5: 1300,
}

# Known Faulhaber values for n=10.
_EXPECTED_S10: dict[int, int] = {
    0: 10,
    1: 55,
    2: 385,
    3: 3025,
    4: 25333,
    5: 220825,
}


# ---------------------------------------------------------------------------
# Tests: faulhaber_ir
# ---------------------------------------------------------------------------


class TestFaulhaberIR:
    """faulhaber_ir(m, concrete_n) matches the known sum Σ_{k=1}^n k^m."""

    def test_m0_n4(self):
        """Σ_{k=1}^4 k^0 = 4."""
        node = faulhaber_ir(0, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(4.0)

    def test_m1_n4(self):
        """Σ_{k=1}^4 k^1 = 10."""
        node = faulhaber_ir(1, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(10.0)

    def test_m2_n4(self):
        """Σ_{k=1}^4 k^2 = 30."""
        node = faulhaber_ir(2, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(30.0)

    def test_m3_n4(self):
        """Σ_{k=1}^4 k^3 = 100."""
        node = faulhaber_ir(3, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(100.0)

    def test_m4_n4(self):
        """Σ_{k=1}^4 k^4 = 354."""
        node = faulhaber_ir(4, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(354.0)

    def test_m5_n4(self):
        """Σ_{k=1}^4 k^5 = 1300."""
        node = faulhaber_ir(5, IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(1300.0)

    def test_m1_n10(self):
        """Σ_{k=1}^10 k = 55."""
        node = faulhaber_ir(1, IRInteger(10))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(55.0)

    def test_m2_n10(self):
        """Σ_{k=1}^10 k^2 = 385."""
        node = faulhaber_ir(2, IRInteger(10))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(385.0)

    def test_m3_n10(self):
        """Σ_{k=1}^10 k^3 = 3025."""
        node = faulhaber_ir(3, IRInteger(10))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(3025.0)

    def test_m6_returns_none(self):
        """m=6 is unsupported: should return None."""
        node = faulhaber_ir(6, IRInteger(10))
        assert node is None

    def test_symbolic_n_produces_irnode(self):
        """faulhaber_ir with symbolic n produces a valid IRNode."""
        n = IRSymbol("n")
        for m in range(6):
            node = faulhaber_ir(m, n)
            assert node is not None, f"m={m} returned None"


# ---------------------------------------------------------------------------
# Tests: poly_sum_ir
# ---------------------------------------------------------------------------


class TestPolySumIR:
    """poly_sum_ir evaluates Σ_{k=lo}^n k^m with concrete n."""

    def test_lo1_m2_n4(self):
        """Σ_{k=1}^4 k^2 = 30."""
        from fractions import Fraction

        node = poly_sum_ir(m=2, coeff=Fraction(1), lo_val=1, hi=IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(30.0)

    def test_lo0_m1_n4(self):
        """Σ_{k=0}^4 k = 0+1+2+3+4 = 10."""
        from fractions import Fraction

        node = poly_sum_ir(m=1, coeff=Fraction(1), lo_val=0, hi=IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(10.0)

    def test_lo0_m0_n4(self):
        """Σ_{k=0}^4 k^0 = 5 (five ones: k=0,1,2,3,4)."""
        from fractions import Fraction

        node = poly_sum_ir(m=0, coeff=Fraction(1), lo_val=0, hi=IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(5.0)

    def test_lo2_m1_n5(self):
        """Σ_{k=2}^5 k = 2+3+4+5 = 14."""
        from fractions import Fraction

        node = poly_sum_ir(m=1, coeff=Fraction(1), lo_val=2, hi=IRInteger(5))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(14.0)

    def test_coeff_3_m1_n4(self):
        """Σ_{k=1}^4 3k = 3·10 = 30."""
        from fractions import Fraction

        node = poly_sum_ir(m=1, coeff=Fraction(3), lo_val=1, hi=IRInteger(4))
        assert node is not None
        assert _eval_ir(node) == pytest.approx(30.0)

    def test_m6_returns_none(self):
        """m=6 → None (unsupported)."""
        from fractions import Fraction

        node = poly_sum_ir(m=6, coeff=Fraction(1), lo_val=1, hi=IRInteger(4))
        assert node is None


import pytest  # noqa: E402 — placed here to avoid import-before-use in type stubs
