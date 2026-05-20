"""Integration tests for summation.py dispatcher."""



from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
    MUL,
    POW,
    PRODUCT,
    SUM,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_summation import evaluate_product, evaluate_sum

# ---------------------------------------------------------------------------
# Minimal stub VM
# ---------------------------------------------------------------------------


class _StubVM:
    """Minimal VM that evaluates arithmetic over IRInteger/IRRational."""

    def eval(self, node):
        from fractions import Fraction

        from symbolic_ir import (
            MUL,
            NEG,
            POW,
            SUB,
            IRApply,
            IRFloat,
            IRInteger,
            IRRational,
        )

        if isinstance(node, (IRInteger, IRRational, IRFloat, IRSymbol)):
            return node
        if not isinstance(node, IRApply):
            return node
        args = [self.eval(a) for a in node.args]

        def _frac(n):
            if isinstance(n, IRInteger):
                return Fraction(n.value)
            if isinstance(n, IRRational):
                return Fraction(n.numer, n.denom)
            return None

        def _to_ir(f: Fraction):
            if f.denominator == 1:
                return IRInteger(f.numerator)
            return IRRational(f.numerator, f.denominator)

        head = node.head
        if head == ADD:
            vals = [_frac(a) for a in args]
            if all(v is not None for v in vals):
                return _to_ir(sum(vals, Fraction(0)))
        if head == SUB:
            v0, v1 = _frac(args[0]), _frac(args[1])
            if v0 is not None and v1 is not None:
                return _to_ir(v0 - v1)
        if head == MUL:
            vals = [_frac(a) for a in args]
            if all(v is not None for v in vals):
                result = Fraction(1)
                for v in vals:
                    result *= v
                return _to_ir(result)
        if head == DIV:
            v0, v1 = _frac(args[0]), _frac(args[1])
            if v0 is not None and v1 is not None and v1 != 0:
                return _to_ir(v0 / v1)
        if head == POW:
            v0, v1 = _frac(args[0]), _frac(args[1])
            if v0 is not None and v1 is not None and v1.denominator == 1:
                exp = v1.numerator
                if exp >= 0:
                    return _to_ir(v0**exp)
        if head == NEG:
            v = _frac(args[0])
            if v is not None:
                return _to_ir(-v)
        # Rebuild with evaluated args
        return IRApply(node.head, tuple(args))


_VM = _StubVM()
_k = IRSymbol("k")
_n = IRSymbol("n")


# ---------------------------------------------------------------------------
# Tests: evaluate_sum
# ---------------------------------------------------------------------------


class TestEvaluateSum:
    def test_constant_summand(self):
        """sum(5, k, 1, 10) = 5 * 10 = 50."""
        result = evaluate_sum(IRInteger(5), _k, IRInteger(1), IRInteger(10), _VM)
        assert isinstance(result, IRInteger) and result.value == 50

    def test_geometric_half_inf(self):
        """sum((1/2)^k, k, 0, inf) = 2."""
        r = IRRational(1, 2)
        f = IRApply(POW, (r, _k))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, (IRInteger, IRRational))
        from fractions import Fraction
        val = (
            Fraction(result.value)
            if isinstance(result, IRInteger)
            else Fraction(result.numer, result.denom)
        )
        assert val == Fraction(2)

    def test_power_sum_k1_concrete(self):
        """sum(k, k, 1, 4) = 10."""
        result = evaluate_sum(_k, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 10

    def test_power_sum_k2_concrete(self):
        """sum(k^2, k, 1, 4) = 30."""
        f = IRApply(POW, (_k, IRInteger(2)))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 30

    def test_power_sum_k1_symbolic_n(self):
        """sum(k, k, 1, n) → a non-unevaluated IR tree (Faulhaber)."""
        result = evaluate_sum(_k, _k, IRInteger(1), _n, _VM)
        # Should NOT be a SUM node
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_unevaluated_fallback(self):
        """sum(sin(k), k, 1, n) → unevaluated SUM node."""
        from symbolic_ir import SIN
        f = IRApply(SIN, (_k,))
        result = evaluate_sum(f, _k, IRInteger(1), _n, _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_power_sum_k3_concrete(self):
        """sum(k^3, k, 1, 3) = 1+8+27 = 36."""
        f = IRApply(POW, (_k, IRInteger(3)))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(3), _VM)
        assert isinstance(result, IRInteger)
        assert result.value == 36

    def test_geometric_mul_coeff_base_pow(self):
        """sum(3 * (1/2)^k, k, 0, inf) = 3 * 2 = 6 — MUL(c, POW(r,k)) form."""
        r = IRRational(1, 2)
        f = IRApply(MUL, (IRInteger(3), IRApply(POW, (r, _k))))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # 3 * 1/(1 - 1/2) = 6
        from fractions import Fraction
        assert isinstance(result, (IRInteger, IRRational))
        val = (
            Fraction(result.value)
            if isinstance(result, IRInteger)
            else Fraction(result.numer, result.denom)
        )
        assert val == Fraction(6)

    def test_geometric_pow_base_mul_coeff(self):
        """sum((1/2)^k * 2, k, 0, inf) = 4 — MUL(POW(r,k), c) form."""
        r = IRRational(1, 2)
        f = IRApply(MUL, (IRApply(POW, (r, _k)), IRInteger(2)))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # 2 * 1/(1 - 1/2) = 4
        from fractions import Fraction
        assert isinstance(result, (IRInteger, IRRational))
        val = (
            Fraction(result.value)
            if isinstance(result, IRInteger)
            else Fraction(result.numer, result.denom)
        )
        assert val == Fraction(4)

    def test_power_sum_scaled_k2(self):
        """sum(2*k^2, k, 1, 4) = 2*30 = 60 — MUL(c, POW(k,m)) form."""
        f = IRApply(MUL, (IRInteger(2), IRApply(POW, (_k, IRInteger(2)))))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 60

    def test_power_sum_scaled_k(self):
        """sum(3*k, k, 1, 4) = 3*10 = 30 — MUL(c, k) form."""
        f = IRApply(MUL, (IRInteger(3), _k))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 30

    def test_geometric_finite_r3(self):
        """sum(3^k, k, 0, 3) = (3^4-1)/(3-1) = 80/2 = 40."""
        f = IRApply(POW, (IRInteger(3), _k))
        result = evaluate_sum(f, _k, IRInteger(0), IRInteger(3), _VM)
        assert isinstance(result, IRInteger) and result.value == 40

    def test_inf_upper_raw_symbol(self):
        """sum with hi=inf (no %) also returns unevaluated for unknown patterns."""
        from symbolic_ir import SIN
        f = IRApply(SIN, (_k,))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Tests: evaluate_product
# ---------------------------------------------------------------------------


class TestEvaluateProduct:
    def test_factorial_product(self):
        """product(k, k, 1, n) → GammaFunc(n+1)."""
        result = evaluate_product(_k, _k, IRInteger(1), _n, _VM)
        assert isinstance(result, IRApply) and result.head == GAMMA_FUNC

    def test_constant_product(self):
        """product(2, k, 0, 4) → 2^5."""
        result = evaluate_product(IRInteger(2), _k, IRInteger(0), IRInteger(4), _VM)
        # 2^5 = 32
        assert isinstance(result, IRInteger) and result.value == 32

    def test_unevaluated_product(self):
        """product(k^3, k, 1, n) → unevaluated PRODUCT node."""
        f = IRApply(POW, (_k, IRInteger(3)))
        result = evaluate_product(f, _k, IRInteger(1), _n, _VM)
        assert isinstance(result, IRApply) and result.head == PRODUCT


# ---------------------------------------------------------------------------
# Phase 39: Telescoping sums — ``∑_{k=lo}^{hi} [g(k+1) − g(k)] = g(hi+1) − g(lo)``
#
# The dispatcher detects the structural ``f = g(k+1) − g(k)`` shape (and
# its antisymmetric ``g(k) − g(k+1)`` form) by substituting ``k → k+1`` in
# one half of the SUB and comparing against the other half after VM
# normalisation.  Tests below cover concrete numeric bounds (where the
# stub VM can fully evaluate the closed form) plus the symbolic case
# (where the result is a SUB tree of substituted expressions).
# ---------------------------------------------------------------------------


class TestEvaluateSumTelescoping:
    def test_standard_telescope_concrete_bounds(self):
        """∑_{k=1}^{4} [(k+1)² − k²] = 5² − 1² = 24.

        The two halves ``(k+1)²`` and ``k²`` differ by the standard
        ``k → k+1`` shift, so the dispatcher must recognise this as
        telescoping (not just evaluate it by Faulhaber after expansion).
        """
        from symbolic_ir import SUB

        k_plus_one_sq = IRApply(
            POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))
        )
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        f = IRApply(SUB, (k_plus_one_sq, k_sq))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 24

    def test_antisymmetric_telescope_concrete_bounds(self):
        """∑_{k=1}^{3} [k² − (k+1)²] = 1² − 4² = −15.

        The flipped orientation: ``f = g(k) − g(k+1)`` yields
        ``g(lo) − g(hi+1)``.  ``g(k) = k²`` → result is ``1 − 16 = −15``.
        """
        from symbolic_ir import SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        k_plus_one_sq = IRApply(
            POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))
        )
        f = IRApply(SUB, (k_sq, k_plus_one_sq))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(3), _VM)
        assert isinstance(result, IRInteger) and result.value == -15

    def test_telescope_linear_g(self):
        """∑_{k=1}^{10} [(k+1) − k] = 10 (each term equals 1)."""
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRApply(ADD, (_k, IRInteger(1))), _k))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(10), _VM)
        # g(k) = k; closed form = g(11) − g(1) = 11 − 1 = 10.
        assert isinstance(result, IRInteger) and result.value == 10

    def test_telescope_with_constant_offset_in_g(self):
        """``g(k) = k + 5`` → telescope still recognises ``g(k+1) − g(k) = 1``.

        Result: ``∑_{k=1}^{5} [(k + 6) − (k + 5)] = 5``.
        """
        from symbolic_ir import SUB

        g_at_k_plus_1 = IRApply(
            ADD,
            (IRApply(ADD, (_k, IRInteger(1))), IRInteger(5)),
        )
        g_at_k = IRApply(ADD, (_k, IRInteger(5)))
        f = IRApply(SUB, (g_at_k_plus_1, g_at_k))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(5), _VM)
        assert isinstance(result, IRInteger) and result.value == 5

    def test_telescope_falls_through_when_shift_doesnt_match(self):
        """``∑ [k² − k]`` is NOT telescoping (``k² ≠ g(k+1)`` for any choice
        of ``g(k) = k``).  The stub VM falls back to Faulhaber/numeric.

        For ``k=1..3``: ``(1−1)+(4−2)+(9−3) = 0+2+6 = 8``.
        """
        from symbolic_ir import SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        f = IRApply(SUB, (k_sq, _k))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(3), _VM)
        # Numeric small-range path computes the answer:
        assert isinstance(result, IRInteger) and result.value == 8

    def test_telescope_does_not_fire_on_constant_difference(self):
        """``∑ [5 − 3] = ∑ 2`` is *constant*, so step 1 (constant summand)
        fires first and the telescope rule never runs.  Result: 2 · 10.
        """
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRInteger(5), IRInteger(3)))
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(10), _VM)
        assert isinstance(result, IRInteger) and result.value == 20

    def test_telescope_with_symbolic_upper_bound(self):
        """``∑_{k=1}^{n} [(k+1) − k]`` symbolic n → result is ``(n+1) − 1``
        (or an equivalent simplified IR shape).  Must not be unevaluated."""
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRApply(ADD, (_k, IRInteger(1))), _k))
        result = evaluate_sum(f, _k, IRInteger(1), _n, _VM)
        # The stub VM doesn't fully simplify symbolic SUB chains; we just
        # require it isn't the unevaluated SUM node.
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_telescope_does_not_fire_for_infinite_upper(self):
        """``∑_{k=0}^{∞} [g(k+1) − g(k)]`` requires a limit argument we
        don't yet implement; must fall through to the unevaluated form
        (the classic-infinite recogniser does not pattern-match this
        shape either).
        """
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRApply(ADD, (_k, IRInteger(1))), _k))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # Stays unevaluated (or numeric fallback fails on infinite range).
        assert isinstance(result, IRApply) and result.head == SUM
