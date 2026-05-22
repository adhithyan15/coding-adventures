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

    def test_telescope_does_not_fire_for_infinite_upper_when_g_grows(self):
        """``∑_{k=0}^{∞} [(k+1) − k]`` — here ``g(k) = k`` does NOT vanish
        at infinity (it grows), so Phase 41 refuses and the sum stays
        unevaluated.  This pins the Phase 41 guard against accidentally
        emitting ``−g(lo)`` for divergent telescopes.
        """
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRApply(ADD, (_k, IRInteger(1))), _k))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # Stays unevaluated — g doesn't vanish at infinity.
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Phase 41: limit-aware infinite telescope.
#
# When ``hi`` is ``%inf`` AND ``g(k)`` provably vanishes at infinity (per
# the narrow ``_g_vanishes_at_infinity`` recogniser — currently
# ``Div(const, positive-degree-polynomial-in-k)`` shapes), the dispatcher
# emits ``∑_{k=lo}^∞ [g(k+1) − g(k)] = −g(lo)`` (standard orientation) or
# ``∑_{k=lo}^∞ [g(k) − g(k+1)] = g(lo)`` (antisymmetric).
#
# The classic motivating example is ``∑_{k=1}^∞ 1/k − 1/(k+1) = 1``, the
# "1/(k·(k+1))" series after Apart decomposition.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase41InfiniteTelescope:
    def test_antisymmetric_1_over_k_minus_1_over_kp1(self):
        """``∑_{k=1}^∞ [1/k − 1/(k+1)] = 1 − 0 = 1``.

        g(k) = 1/k vanishes at infinity → Phase 41 emits g(lo) = 1/1 = 1.
        """
        from symbolic_ir import SUB

        f = IRApply(
            SUB,
            (
                IRApply(DIV, (IRInteger(1), _k)),
                IRApply(DIV, (IRInteger(1), IRApply(ADD, (_k, IRInteger(1))))),
            ),
        )
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_standard_orientation_1_over_kp1_minus_1_over_k(self):
        """``∑_{k=1}^∞ [1/(k+1) − 1/k] = 0 − 1 = −1``.

        Standard orientation g(k+1) − g(k) with g(k) = 1/k.  Phase 41
        emits −g(lo) = −1.
        """
        from symbolic_ir import SUB

        f = IRApply(
            SUB,
            (
                IRApply(DIV, (IRInteger(1), IRApply(ADD, (_k, IRInteger(1))))),
                IRApply(DIV, (IRInteger(1), _k)),
            ),
        )
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == -1

    def test_higher_starting_index(self):
        """``∑_{k=2}^∞ [1/k − 1/(k+1)] = g(2) = 1/2``."""
        from fractions import Fraction

        from symbolic_ir import SUB

        f = IRApply(
            SUB,
            (
                IRApply(DIV, (IRInteger(1), _k)),
                IRApply(DIV, (IRInteger(1), IRApply(ADD, (_k, IRInteger(1))))),
            ),
        )
        result = evaluate_sum(f, _k, IRInteger(2), IRSymbol("%inf"), _VM)
        # 1/2
        from symbolic_ir import IRRational

        assert isinstance(result, IRRational)
        assert Fraction(result.numer, result.denom) == Fraction(1, 2)

    def test_quadratic_denominator_vanishes(self):
        """``∑_{k=1}^∞ [1/k² − 1/(k+1)²]`` (telescope of 1/k²).

        g(k) = 1/k² vanishes at infinity → Phase 41 emits g(1) = 1.
        """
        from symbolic_ir import POW, SUB

        f = IRApply(
            SUB,
            (
                IRApply(DIV, (IRInteger(1), IRApply(POW, (_k, IRInteger(2))))),
                IRApply(
                    DIV,
                    (
                        IRInteger(1),
                        IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))),
                    ),
                ),
            ),
        )
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_constant_g_falls_through(self):
        """``∑_{k=1}^∞ [c − c] = ∑ 0`` — the SUB folds to 0 first (step 1
        constant rule), so Phase 41 never runs.  Result: 0."""
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRInteger(7), IRInteger(7)))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Constant zero summand: sum is also 0 (or stays as 0·∞ via the
        # constant-summand rule; either way must not be the unevaluated SUM
        # of the original SUB).
        if isinstance(result, IRInteger):
            assert result.value == 0
        else:
            # Some stub VMs return a `Mul(0, hi-lo+1)` style shape;
            # confirm it's not a Sum.
            assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_g_not_a_div_falls_through(self):
        """``∑_{k=1}^∞ [(k+1) − k]`` — g(k) = k is not a Div, doesn't
        vanish.  Phase 41 refuses; result is the unevaluated Sum."""
        from symbolic_ir import SUB

        f = IRApply(SUB, (IRApply(ADD, (_k, IRInteger(1))), _k))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Phase 42: degree-aware vanishing-at-infinity.
#
# Extends Phase 41's narrow constant-numerator recogniser to handle any
# proper rational ``P(k)/Q(k)`` where ``deg(P) < deg(Q)``.  This widens
# the set of telescopes that close at infinity to cover Apart outputs
# from any partial-fraction decomposition with non-constant numerators.
#
# The Phase 41 fast path (constant numerator) is preserved as a special
# case — these tests pin the new degree-aware behaviour.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase42DegreeAware:
    def test_proper_rational_k_over_k_squared_plus_1_minus_shift(self):
        """``∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)]``.

        g(k) = k/(k²+1) has deg(num)=1 < deg(den)=2, so vanishes at ∞.
        Closed form (antisymmetric): g(1) = 1/2.
        """
        from fractions import Fraction
        from symbolic_ir import POW, SUB

        # g(k) = k / (k² + 1)
        g_k = IRApply(
            DIV,
            (
                _k,
                IRApply(ADD, (IRApply(POW, (_k, IRInteger(2))), IRInteger(1))),
            ),
        )
        # g(k+1) = (k+1) / ((k+1)² + 1)
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(
            DIV,
            (
                kp1,
                IRApply(ADD, (IRApply(POW, (kp1, IRInteger(2))), IRInteger(1))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # g(1) = 1 / (1 + 1) = 1/2
        from symbolic_ir import IRRational

        assert isinstance(result, IRRational)
        assert Fraction(result.numer, result.denom) == Fraction(1, 2)

    def test_polynomial_degree_constant_numerator_still_works(self):
        """Regression: Phase 41 fast path (constant/Q) still closes.

        ``∑_{k=1}^∞ [1/k − 1/(k+1)] = 1`` must continue to work after the
        Phase 42 degree-aware widening replaces some of the recogniser.
        """
        from symbolic_ir import SUB

        f = IRApply(
            SUB,
            (
                IRApply(DIV, (IRInteger(1), _k)),
                IRApply(DIV, (IRInteger(1), IRApply(ADD, (_k, IRInteger(1))))),
            ),
        )
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_improper_rational_falls_through(self):
        """``∑_{k=1}^∞ [k/(k+1) − (k+1)/(k+2)]``.

        g(k) = k/(k+1) has deg(num)=1 = deg(den)=1; the limit is 1
        (not 0), so Phase 42 must refuse and the sum stays unevaluated.
        """
        from symbolic_ir import SUB

        g_k = IRApply(DIV, (_k, IRApply(ADD, (_k, IRInteger(1)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(
            DIV, (kp1, IRApply(ADD, (_k, IRInteger(2))))
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Limit isn't 0; must stay unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_super_improper_rational_falls_through(self):
        """``g(k) = k²/(k+1)``: limit is ∞, not 0.  Phase 42 refuses.

        We construct the SUB shape ``[k²/(k+1) − (k+1)²/(k+2)]`` and
        confirm Phase 41/42 both refuse to close it (the limit is +∞,
        not 0).
        """
        from symbolic_ir import POW, SUB

        g_k = IRApply(
            DIV, (IRApply(POW, (_k, IRInteger(2))), IRApply(ADD, (_k, IRInteger(1))))
        )
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(
            DIV,
            (IRApply(POW, (kp1, IRInteger(2))), IRApply(ADD, (_k, IRInteger(2)))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_transcendental_numerator_falls_through(self):
        """``g(k) = sin(k)/k²``: numerator is not a polynomial, so the
        degree-aware path refuses.  (The limit IS 0 by squeeze, but
        decidably proving so needs a transcendental limit-finder we
        don't have yet.)
        """
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        sin_kp1 = IRApply(SIN, (IRApply(ADD, (_k, IRInteger(1))),))
        g_k = IRApply(DIV, (sin_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(
            DIV,
            (
                sin_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Conservative: refuse since num is not a polynomial.
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Phase 43: transcendental vanishing-at-infinity.
#
# Extends Phase 41's denominator recogniser to also accept exponentially
# diverging shapes:
#   - Exp(h(k)) with h(k) a positive-degree polynomial in k
#   - Pow(b, h(k)) with |b| > 1 rational and h(k) positive-degree
#   - Mul(...) with at least one diverging factor and others constant
#     in k or also diverging
#
# Closes telescopes like ∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = g(0) = 1.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase43Transcendental:
    def test_antisymmetric_one_over_2_pow_k(self):
        """``∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = g(0) = 1/2^0 = 1``.

        Phase 43 recognises ``Pow(2, k)`` as a diverging denominator;
        the antisymmetric telescope emits ``g(0) = 1``.
        """
        from symbolic_ir import SUB

        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (IRInteger(2), IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_pow_3_with_higher_starting_index(self):
        """``∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = g(1) = 1/3``.

        Phase 43: base = 3 (>1), exponent = k (positive degree).
        Antisymmetric closed form is ``g(1) = 1/3``.
        """
        from fractions import Fraction
        from symbolic_ir import SUB

        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (IRInteger(3), _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (IRInteger(3), IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        from symbolic_ir import IRRational

        assert isinstance(result, IRRational)
        assert Fraction(result.numer, result.denom) == Fraction(1, 3)

    def test_pow_negative_base_with_magnitude_above_one(self):
        """``∑_{k=0}^∞ [1/(-2)^k − 1/(-2)^(k+1)] = 1/(-2)^0 = 1``.

        Phase 43 accepts ``|b| > 1`` for the base, so negative bases
        with magnitude > 1 still count (the magnitude diverges; the
        sign oscillates but doesn't affect the limit being 0).
        """
        from symbolic_ir import SUB

        neg2 = IRInteger(-2)
        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (neg2, _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (neg2, IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_base_one_falls_through(self):
        """``Pow(1, k) = 1`` doesn't diverge; Phase 43 refuses.

        ``∑_{k=0}^∞ [1/1^k − 1/1^(k+1)] = ∑ [1 − 1] = ∑ 0`` — but the
        constant-summand rule fires first (Step 1) so it never reaches
        Phase 43.  This test pins the Phase 43 ``|b| > 1`` guard against
        accidentally claiming ``b = 1`` diverges.
        """
        from symbolic_ir import SUB

        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (IRInteger(1), _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (IRInteger(1), IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # Constant-summand path folds [1 − 1] = 0 first; check it's
        # either the integer 0 OR fell through to unevaluated (the
        # stub VM may not fold the SUB).  Definitely must not emit
        # ``−g(lo) = −1`` via a wrongly-fired Phase 43.
        if isinstance(result, IRInteger):
            assert result.value == 0
        else:
            assert isinstance(result, IRApply) and result.head == SUM

    def test_pow_fractional_base_above_one_diverges(self):
        """``Pow(3/2, k)`` has rational base 3/2 with magnitude > 1 →
        diverges.  ``∑_{k=0}^∞ [1/(3/2)^k − 1/(3/2)^(k+1)] = 1``.
        """
        from symbolic_ir import SUB

        three_halves = IRRational(3, 2)
        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (three_halves, _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (three_halves, IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRInteger) and result.value == 1

    def test_pow_half_falls_through(self):
        """``Pow(1/2, k) = (1/2)^k → 0``, not diverging.  Phase 43
        refuses (we need denominator to diverge).
        """
        from symbolic_ir import SUB

        half = IRRational(1, 2)
        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (half, _k))))
        g_kp1 = IRApply(
            DIV,
            (IRInteger(1), IRApply(POW, (half, IRApply(ADD, (_k, IRInteger(1)))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # Phase 43 must refuse — the denominator (1/2)^k → 0, so g(k)
        # actually diverges; emitting ``−g(lo)`` would be wrong.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_mul_of_polynomial_and_exponential_diverges(self):
        """``∑_{k=1}^∞ [1/(k·2^k) − 1/((k+1)·2^(k+1))] = 1/2``.

        Denominator is ``k · 2^k`` — Phase 43 ``Mul`` case combines a
        positive-degree polynomial factor (``k``) with an exponential
        factor (``2^k``), both diverging.  Closed form: ``g(1) = 1/2``.
        """
        from fractions import Fraction
        from symbolic_ir import SUB

        g_k = IRApply(
            DIV,
            (IRInteger(1), IRApply(MUL, (_k, IRApply(POW, (IRInteger(2), _k))))),
        )
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(
            DIV, (IRInteger(1), IRApply(MUL, (kp1, IRApply(POW, (IRInteger(2), kp1))))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        from symbolic_ir import IRRational

        assert isinstance(result, IRRational)
        assert Fraction(result.numer, result.denom) == Fraction(1, 2)

    # ----- Phase 43 sign-aware regressions (from security review) -----

    def test_exp_negative_polynomial_exponent_does_not_diverge(self):
        """``Exp(Mul(-1, k))`` represents ``exp(-k) → 0``, NOT ∞.

        The Phase 43 sign-aware guard must refuse to claim divergence
        when the polynomial exponent has a negative leading coefficient.
        Otherwise ``Div(c, exp(-k)) = c·exp(k) → ∞`` would be wrongly
        claimed to vanish.
        """
        from symbolic_ir import EXP, SUB

        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        neg_kp1 = IRApply(MUL, (IRInteger(-1), IRApply(ADD, (_k, IRInteger(1)))))
        g_k = IRApply(DIV, (IRInteger(1), IRApply(EXP, (neg_k,))))
        g_kp1 = IRApply(DIV, (IRInteger(1), IRApply(EXP, (neg_kp1,))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # MUST stay unevaluated — g(k) = 1/exp(-k) = exp(k) actually
        # diverges, so the sum doesn't have a finite closed form via
        # this rule.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_pow_negative_polynomial_exponent_does_not_diverge(self):
        """``Pow(2, Mul(-1, k)) = 2^(-k) → 0``, NOT ∞.

        Same regression as the Exp case: Phase 43 must refuse the
        ``Div(c, 2^(-k))`` shape because the denominator vanishes
        rather than diverging.
        """
        from symbolic_ir import SUB

        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        neg_kp1 = IRApply(MUL, (IRInteger(-1), IRApply(ADD, (_k, IRInteger(1)))))
        g_k = IRApply(DIV, (IRInteger(1), IRApply(POW, (IRInteger(2), neg_k))))
        g_kp1 = IRApply(DIV, (IRInteger(1), IRApply(POW, (IRInteger(2), neg_kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_mul_with_negative_polynomial_exponent_does_not_diverge(self):
        """``k · 2^(-k) → 0``, NOT ∞ (the exponential decay wins).

        Phase 43's ``Mul`` recursion must propagate the sign-aware
        refusal from its child: ``k`` is positive-degree, but ``2^(-k)``
        is correctly rejected by the Pow branch, so the overall ``Mul``
        recogniser refuses too.
        """
        from symbolic_ir import SUB

        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        neg_kp1 = IRApply(MUL, (IRInteger(-1), IRApply(ADD, (_k, IRInteger(1)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_k = IRApply(
            DIV,
            (IRInteger(1), IRApply(MUL, (_k, IRApply(POW, (IRInteger(2), neg_k))))),
        )
        g_kp1 = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(MUL, (kp1, IRApply(POW, (IRInteger(2), neg_kp1)))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_exp_with_neg_wrapper_does_not_diverge(self):
        """``Exp(Neg(k))`` — the canonical ``-k`` written as an
        explicit ``NEG`` wrapper rather than ``Mul(-1, k)``.  Same
        ``exp(-k) → 0`` behaviour; Phase 43 must refuse.
        """
        from symbolic_ir import EXP, NEG, SUB

        g_k = IRApply(DIV, (IRInteger(1), IRApply(EXP, (IRApply(NEG, (_k,)),))))
        g_kp1 = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(
                    EXP, (IRApply(NEG, (IRApply(ADD, (_k, IRInteger(1))),)),),
                ),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Phase 44: Log divergence in vanishing-at-infinity recogniser.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase44LogDivergence:
    def test_log_of_polynomial_recognised(self):
        """``Log(k+1)`` diverges → ``1/log(k+1) → 0`` → Phase 44 closes.

        The telescope detector compares ``g(k+1)`` (via substitution
        ``k → k+1`` in g) against the supplied ``g_kp1``.  Build
        ``g_kp1`` via the same substitution so the structural ``==``
        comparison succeeds under the stub VM (which doesn't
        canonicalise ``Add(Add(k,1), 1) ↔ Add(k, 2)``).
        """
        from cas_substitution import subst
        from symbolic_ir import LOG, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_k = IRApply(DIV, (IRInteger(1), IRApply(LOG, (kp1,))))
        g_kp1 = subst(kp1, _k, g_k)
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_of_exponential_recursion(self):
        """``Log(2^k)`` diverges → Phase 44 Log case delegates to the
        Phase 43 Pow check, which accepts.
        """
        from cas_substitution import subst
        from symbolic_ir import LOG, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_k = IRApply(
            DIV,
            (IRInteger(1), IRApply(LOG, (IRApply(POW, (IRInteger(2), _k)),))),
        )
        g_kp1 = subst(kp1, _k, g_k)
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_of_constant_falls_through(self):
        """``Log(5)`` is a finite constant; Phase 44 must refuse.

        The summand ``Sub(g, g)`` for constant g routes through the
        constant-summand rule (Step 1) before telescope detection
        even runs, producing ``0 * count``.  We just verify the
        result is NOT the wrong closed form ``−1/log(5)`` (which is
        what a buggy Phase 44 would emit by claiming Log(5) diverges).
        """
        from symbolic_ir import LOG, NEG, SUB

        g_const = IRApply(DIV, (IRInteger(1), IRApply(LOG, (IRInteger(5),))))
        f = IRApply(SUB, (g_const, g_const))
        result = evaluate_sum(f, _k, IRInteger(0), IRSymbol("%inf"), _VM)
        # Result must not be a Div or Neg(Div) shape (which would
        # indicate Phase 44 wrongly fired on a finite-limit g).  In
        # practice the constant-summand rule emits a Mul shape or
        # folds to 0.
        is_wrong = (
            isinstance(result, IRApply)
            and result.head in (DIV, NEG)
        )
        assert not is_wrong, f"Phase 44 wrongly fired on constant g: {result!r}"

    def test_log_of_negative_polynomial_falls_through(self):
        """``Log(Mul(-1, k))`` — the inner argument has negative
        leading coefficient.  Phase 43's sign-aware chain refuses it,
        and Phase 44 inherits that refusal via the recursion.
        """
        from cas_substitution import subst
        from symbolic_ir import LOG, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        g_k = IRApply(DIV, (IRInteger(1), IRApply(LOG, (neg_k,))))
        g_kp1 = subst(kp1, _k, g_k)
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_log_of_pow_negative_base_falls_through(self):
        """``Log(Pow(-2, k))`` — base ``-2`` makes ``(-2)^k`` oscillate
        in sign, so ``log((-2)^k)`` is not real-valued.

        Regression for the security review: Phase 43's Pow case accepts
        ``|b| > 1`` (so ``c/(-2)^k → 0`` is correct for the original
        purpose), but Phase 44's Log delegation must additionally require
        ``b > 1`` *strictly positive* to avoid claiming ``log(negative)``
        diverges.
        """
        from cas_substitution import subst
        from symbolic_ir import LOG, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        pow_neg2 = IRApply(POW, (IRInteger(-2), _k))
        g_k = IRApply(DIV, (IRInteger(1), IRApply(LOG, (pow_neg2,))))
        g_kp1 = subst(kp1, _k, g_k)
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Phase 44 must refuse — `log((-2)^k)` is complex / undefined
        # for odd k, so the sum doesn't have a real closed form.
        assert isinstance(result, IRApply) and result.head == SUM
