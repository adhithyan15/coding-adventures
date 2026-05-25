"""Integration tests for summation.py dispatcher."""



from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
    LOG,
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

    def test_transcendental_numerator_closes_via_phase49(self):
        """``g(k) = sin(k)/k²``: previously refused by the
        degree-aware path because ``sin(k)`` isn't a polynomial.
        Phase 49 (bounded-numerator widening) recognises that
        ``|sin(k)| ≤ 1`` and the denominator ``k²`` diverges, so the
        quotient vanishes at ∞.

        The antisymmetric telescope ``g(k) − g(k+1)`` from k=1 to ∞
        closes to ``g(1) = sin(1)/1 = sin(1)``.
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
        # Phase 49 closure: g(1) = sin(1)/1² = sin(1).  Either:
        #   - the result is exactly the Sin apply (``Sin(1)``)
        #   - or it's the Div shape that vm-eval folded to ``Sin(1)/1``
        # Pin the new behaviour: result is no longer the unevaluated Sum.
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 49 should close this telescope; got {result!r}"
        )


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


# ---------------------------------------------------------------------------
# Phase 49 — Bounded × vanishing recogniser.
#
# Extends `_g_vanishes_at_infinity` to accept ``Div(bounded, diverging)``
# shapes where the numerator is uniformly bounded (``Sin``, ``Cos``, or
# constants — closed under Mul/Add/Neg) and the denominator diverges.
# Closes telescopes like ``∑ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1)``.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase49BoundedNumerator:
    def test_sin_over_k_squared_closes(self):
        """``∑_{k=1}^∞ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1)``.

        Phase 49: ``sin(k)`` is bounded, ``k²`` diverges, so the
        quotient vanishes.  Antisymmetric telescope closes to
        ``g(1) = sin(1)/1²``.
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
        # Must close — not the unevaluated Sum form.
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 49 should close; got {result!r}"
        )

    def test_cos_over_k_cube_closes(self):
        """``∑_{k=1}^∞ [cos(k)/k³ − cos(k+1)/(k+1)³] = cos(1)``.

        Phase 49: ``cos(k)`` is bounded; ``k³`` diverges.
        """
        from symbolic_ir import COS, POW, SUB

        cos_k = IRApply(COS, (_k,))
        cos_kp1 = IRApply(COS, (IRApply(ADD, (_k, IRInteger(1))),))
        g_k = IRApply(DIV, (cos_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(
            DIV,
            (
                cos_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(3))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_cos_product_over_diverging(self):
        """``sin(k)·cos(k)`` is bounded (product of two bounded functions),
        so ``sin(k)·cos(k)/k²`` vanishes at ∞.
        """
        from symbolic_ir import COS, MUL, POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k))
        sin_kp1 = IRApply(SIN, (IRApply(ADD, (_k, IRInteger(1))),))
        cos_kp1 = IRApply(COS, (IRApply(ADD, (_k, IRInteger(1))),))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(
            DIV,
            (
                num_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_unbounded_numerator_still_refused(self):
        """Regression: ``g(k) = k/k³`` looks like ``unbounded/diverging``
        on the surface (Phase 49 doesn't apply), but Phase 42 catches
        it via deg-difference (deg 1 < deg 3).  This pins the right
        recogniser fires — not Phase 49 — and the sum still closes.
        """
        from symbolic_ir import POW, SUB

        g_k = IRApply(DIV, (_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(
            DIV,
            (
                IRApply(ADD, (_k, IRInteger(1))),
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(3))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    # Phase 49's test_log_numerator_still_refused was removed by Phase 50:
    # log(k)/k² now closes via the new Phase 50 Log/polynomial recogniser
    # (see TestEvaluateSumPhase50LogOverPolynomial below).


# ---------------------------------------------------------------------------
# Phase 50 — Log/polynomial growth-rate recogniser.
#
# Extends `_g_vanishes_at_infinity` to accept `Log(diverging)/diverging`
# shapes via the growth-rate argument: log grows slower than any
# positive-degree polynomial / exponential, so `log/poly → 0`.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase50LogOverPolynomial:
    def test_log_over_k_squared_closes(self):
        """``∑_{k=1}^∞ [log(k)/k² − log(k+1)/(k+1)²]`` closes via Phase 50.

        Both halves vanish at ∞ (log/k² → 0 by squeeze).  Antisymmetric
        telescope reduces to ``g(1) = log(1)/1² = 0`` mathematically,
        but the symbolic shape may stay as a Log expression.  Test pins
        that the result is no longer the unevaluated Sum.
        """
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        log_kp1 = IRApply(LOG, (IRApply(ADD, (_k, IRInteger(1))),))
        g_k = IRApply(DIV, (log_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(
            DIV,
            (
                log_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Math: limit of log(k)/k² IS 0, but Phase 49 isn't smart enough
        # to recognise that.  Stay unevaluated until a transcendental
        # growth-rate recogniser lands.
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 50 should close; got {result!r}"
        )

    def test_log_over_k_cube_closes(self):
        """``∑_{k=1}^∞ [log(k)/k³ − log(k+1)/(k+1)³]`` closes via Phase 50.

        Higher denominator degree, same Log numerator.
        """
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        log_kp1 = IRApply(LOG, (IRApply(ADD, (_k, IRInteger(1))),))
        g_k = IRApply(DIV, (log_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(
            DIV,
            (
                log_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(3))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_of_polynomial_argument(self):
        """``∑ [log(k²+1)/k³ − log((k+1)²+1)/(k+1)³]`` closes.

        Phase 50 with a non-trivial polynomial inside the Log.
        """
        from symbolic_ir import LOG, POW, SUB

        # k² + 1
        k_sq_plus_1 = IRApply(ADD, (IRApply(POW, (_k, IRInteger(2))), IRInteger(1)))
        log_term = IRApply(LOG, (k_sq_plus_1,))
        # (k+1)² + 1
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq_plus_1 = IRApply(ADD, (IRApply(POW, (kp1, IRInteger(2))), IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1_sq_plus_1,))
        g_k = IRApply(DIV, (log_term, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (log_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_of_constant_numerator_still_refused(self):
        """Regression: ``g(k) = log(5)/k²`` — the Log argument is
        constant, not diverging.  Phase 41's constant-numerator path
        catches this case (``log(5)`` is constant in k).  Verify that
        Phase 50 doesn't accidentally trigger on it AND that the sum
        still closes via Phase 41.
        """
        from symbolic_ir import LOG, POW, SUB

        log_5 = IRApply(LOG, (IRInteger(5),))
        g_k = IRApply(DIV, (log_5, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(
            DIV,
            (log_5, IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2)))),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Phase 41 still closes this (log(5) is constant-in-k).
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_of_negative_argument_refused(self):
        """Regression: ``log(Mul(-1, k))`` is complex / undefined for
        odd k.  ``_h_diverges_at_infinity`` correctly refuses the inner
        argument (negative leading coefficient), so Phase 50 must NOT
        close this telescope.
        """
        from symbolic_ir import LOG, MUL, POW, SUB

        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        log_neg_k = IRApply(LOG, (neg_k,))
        log_neg_kp1 = IRApply(
            LOG, (IRApply(MUL, (IRInteger(-1), IRApply(ADD, (_k, IRInteger(1))))),)
        )
        g_k = IRApply(DIV, (log_neg_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(
            DIV,
            (
                log_neg_kp1,
                IRApply(POW, (IRApply(ADD, (_k, IRInteger(1))), IRInteger(2))),
            ),
        )
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Must stay unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM


# ---------------------------------------------------------------------------
# Phase 52 — Bounded × polynomial numerator pattern.
#
# Extends Phase 49 to recognise that Mul(bounded, polynomial)
# numerators have effective growth = polynomial part's degree.
# Closes shapes like sin(k)·k/k³, k·cos(k)/k², (sin(k)+1)·k/k².
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase52BoundedTimesPolynomial:
    def test_sin_times_k_over_k_cubed_closes(self):
        """``sin(k)·k/k³``: bounded × deg 1, denominator deg 3 → vanishes."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        num_k = IRApply(MUL, (sin_k, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 52 should close; got {result!r}"
        )

    def test_k_times_cos_over_k_squared_closes(self):
        """``k·cos(k)/k²``: deg 1 × bounded over deg 2 → vanishes."""
        from symbolic_ir import COS, POW, SUB

        cos_k = IRApply(COS, (_k,))
        num_k = IRApply(MUL, (_k, cos_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        num_kp1 = IRApply(MUL, (kp1, cos_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_times_k_squared_over_k_cubed_closes(self):
        """``sin(k)·k²/k³``: bounded × deg 2 / deg 3 → vanishes (deg 2 < 3)."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, kp1_sq))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_times_k_equal_degrees_refused(self):
        """``sin(k)·k²/k²``: bounded × deg 2 / deg 2 — degrees tie.
        Could be anywhere in [-1, 1]; doesn't vanish.  Phase 52 must
        refuse.
        """
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, kp1_sq))
        g_k = IRApply(DIV, (num_k, k_sq))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_sq))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # deg(num) = 2, deg(den) = 2 → can't decide, stay unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_pure_polynomial_still_phase_42(self):
        """Regression: ``k/k²`` should still close via Phase 42
        (no bounded factor — Phase 52 shouldn't interfere).
        """
        from symbolic_ir import POW, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_k = IRApply(DIV, (_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)


class TestEvaluateSumPhase51SqrtNumerator:
    """Phase 51 — ``Sqrt(P(k))`` as numerator: effective degree = deg(P)/2.

    ``Sqrt(P(k)) / Q(k)`` vanishes at infinity when ``deg(Q) > deg(P)/2``,
    i.e. when ``2*deg(Q) > deg(P)``.  Integer arithmetic keeps comparisons
    exact.

    Examples:
    - ``Sqrt(k) / k``      — ½ < 1 → vanishes (2*1 = 2 > 1)
    - ``Sqrt(k²) / k²``    — 1 = 1 → stays (2*2 = 4 > 2, wait: 2 = 2 exactly...
      actually Sqrt(k²) = k, effective deg 1, den deg 2: 2*2=4>2 → vanishes)
    """

    def test_sqrt_k_over_k_squared_closes(self):
        """``Sqrt(k)/k²``: effective degree ½ < 2 → vanishes.

        ``2 * 2 = 4 > 1 = deg(k)`` → closes.
        """
        from symbolic_ir import POW, SQRT, SUB

        g_k = IRApply(DIV, (IRApply(SQRT, (_k,)), IRApply(POW, (_k, IRInteger(2)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(DIV, (IRApply(SQRT, (kp1,)), IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 51: Sqrt(k)/k² should close; got {result!r}"
        )

    def test_sqrt_k_squared_over_k_cubed_closes(self):
        """``Sqrt(k²)/k³``: effective degree 1 < 3 → vanishes.

        ``Sqrt(k²)`` has inner deg 2; effective half-degree = 1.
        ``2 * 3 = 6 > 2 = deg(k²)`` → closes.
        """
        from symbolic_ir import POW, SQRT, SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        g_k = IRApply(DIV, (IRApply(SQRT, (k_sq,)), IRApply(POW, (_k, IRInteger(3)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        g_kp1 = IRApply(DIV, (IRApply(SQRT, (kp1_sq,)), IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_over_k_equal_degrees_refused(self):
        """``Sqrt(k)/Sqrt(k)`` stays unevaluated — same effective degree
        (but expressed as ``Sqrt(k)/k^{1/2}``, which we test via the
        case ``Sqrt(k)/k``: effective deg ½ vs deg 1 → vanishes since ½ < 1).

        Test the boundary: ``Sqrt(k) / 1`` is NOT a Div form → skip.
        Test the tight case: ``Sqrt(k²) / k`` → effective deg 1 = den deg 1
        → equal, should NOT close.
        """
        from symbolic_ir import SQRT, SUB

        # Numerator effective degree 1 (= deg(k²)/2), denominator degree 1.
        # 2*1 = 2 is NOT > 2 = deg(k²), so Phase 51 refuses.
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        g_k = IRApply(DIV, (IRApply(SQRT, (k_sq,)), _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        g_kp1 = IRApply(DIV, (IRApply(SQRT, (kp1_sq,)), kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # deg(Sqrt(k²)) effective = 1, deg(den) = 1 → tie → unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM, (
            f"Phase 51: Sqrt(k²)/k has equal effective degree; should stay unevaluated; got {result!r}"
        )

    def test_sqrt_of_negative_polynomial_refused(self):
        """``Sqrt(-k)/k²``: inner polynomial has negative leading coeff.

        Phase 51 must refuse this shape (Sqrt of a negative-leading-coeff
        polynomial is not real-valued for large k).
        """
        from symbolic_ir import POW, SQRT, SUB

        neg_k = IRApply(MUL, (IRInteger(-1), _k))
        g_k = IRApply(DIV, (IRApply(SQRT, (neg_k,)), IRApply(POW, (_k, IRInteger(2)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        neg_kp1 = IRApply(MUL, (IRInteger(-1), kp1))
        g_kp1 = IRApply(DIV, (IRApply(SQRT, (neg_kp1,)), IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Phase 51 refuses Sqrt(-k); falls through to Phase 42 which also
        # refuses (numerator is not polynomial) → stays unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase53SqrtTimesPolynomialNumerator:
    """Phase 53 — ``Mul(Sqrt(P), polynomial_factors)`` as numerator.

    Effective growth = ``deg(P)/2 + deg(Q)``.  The quotient vanishes when
    ``deg(den) > deg(P)/2 + deg(Q)``, equivalently
    ``2*deg(den) > deg(P) + 2*deg(Q)``.

    Examples:
    - ``Sqrt(k) · k / k³``:  eff = ½ + 1 = 3/2 < 3 → closes
    - ``Sqrt(k²) · k / k³``: eff = 1 + 1 = 2 < 3 → closes
    - ``Sqrt(k) · k² / k²``: eff = ½ + 2 = 5/2 > 2 → stays
    """

    def test_sqrt_k_times_k_over_k_cubed_closes(self):
        """``Sqrt(k)·k/k³``: effective degree = ½+1 = 3/2, den deg 3 → vanishes.

        ``2*3 = 6 > 1 + 2*1 = 3`` → closes.
        """
        from symbolic_ir import POW, SQRT, SUB

        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 53: Sqrt(k)·k/k³ should close; got {result!r}"
        )

    def test_sqrt_k_squared_times_k_over_k_cubed_closes(self):
        """``Sqrt(k²)·k/k³``: effective degree = 1+1 = 2, den deg 3 → vanishes.

        ``2*3 = 6 > 2 + 2*1 = 4`` → closes.
        """
        from symbolic_ir import POW, SQRT, SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (IRApply(SQRT, (k_sq,)), _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1_sq,)), kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_times_k_squared_over_k_cubed_closes(self):
        """``Sqrt(k)·k²/k³``: effective degree = ½+2 = 5/2, den deg 3 → vanishes.

        ``2*3 = 6 > 1 + 2*2 = 5`` → closes.
        """
        from symbolic_ir import POW, SQRT, SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), kp1_sq))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_times_k_squared_over_k_squared_stays(self):
        """``Sqrt(k)·k²/k²``: effective degree = ½+2 = 5/2 > 2 → stays.

        ``2*2 = 4 NOT > 1 + 2*2 = 5`` → unevaluated.
        """
        from symbolic_ir import POW, SQRT, SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), kp1_sq))
        g_k = IRApply(DIV, (num_k, k_sq))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_sq))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Effective degree 5/2 > 2 → can't prove vanishing → unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_regression_sqrt_k_over_k_squared_still_via_phase51(self):
        """Regression: plain ``Sqrt(k)/k²`` still closes via Phase 51.

        Phase 53 requires a Mul node; plain Sqrt is handled by Phase 51.
        """
        from symbolic_ir import POW, SQRT, SUB

        g_k = IRApply(DIV, (IRApply(SQRT, (_k,)), IRApply(POW, (_k, IRInteger(2)))))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        g_kp1 = IRApply(DIV, (IRApply(SQRT, (kp1,)), IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Regression: Sqrt(k)/k² should close via Phase 51; got {result!r}"
        )


# ---------------------------------------------------------------------------
# Phase 54 — Log × polynomial numerator pattern.
# ---------------------------------------------------------------------------
# ``log(h(k)) · P(k) / Q(k)`` vanishes at infinity when ``deg(Q) > deg(P)``
# (strictly).  The log factor grows sub-polynomially — slower than any
# positive power of k — so the effective growth degree is just ``deg(P)``.
#
# The helper ``_split_log_polynomial_factor`` requires exactly one
# Log(diverging) factor in a Mul node; all other factors must be polynomial.
# It returns ``(log_factor, poly_deg_sum)``.  The branch in
# ``_g_vanishes_at_infinity`` closes when ``den_deg > poly_deg_sum``.
#
# Equal degrees are refused because ``log(k) * constant`` diverges to ±∞.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase54LogTimesPolynomialNumerator:
    """Phase 54: Mul(Log(diverging), polynomial_factors) numerator."""

    def test_log_k_times_k_over_k_cubed_closes(self):
        """``log(k)·k/k³``: log×poly_deg_1 over deg_3.  Phase 54 closes.

        The summand comes from a telescoping difference
        ``g(k) − g(k+1)`` where ``g(k) = log(k)·k/k³ = log(k)/k²``.
        poly_deg=1, den_deg=3, so 3 > 1 → vanishes.
        """
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 54 should close log(k)·k/k³; got {result!r}"
        )

    def test_log_k_times_k_squared_over_k_cubed_closes(self):
        """``log(k)·k²/k³``: log×poly_deg_2 over deg_3.  3 > 2 → closes."""
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (log_k, k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (log_kp1, kp1_sq))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 54 should close log(k)·k²/k³; got {result!r}"
        )

    def test_log_k_times_k_over_k_squared_closes(self):
        """``log(k)·k/k²``: log×poly_deg_1 over deg_2.  2 > 1 → closes."""
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 54 should close log(k)·k/k²; got {result!r}"
        )

    def test_log_k_times_k_squared_over_k_squared_refused(self):
        """``log(k)·k²/k²`` reduces to ``log(k)`` — diverges.

        poly_deg=2, den_deg=2.  Equality means the expression is
        ``log(k) * constant``, which grows without bound.  Phase 54
        must refuse (equal degrees are not strictly greater).
        """
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        k_sq = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (log_k, k_sq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (log_kp1, kp1_sq))
        g_k = IRApply(DIV, (num_k, k_sq))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_sq))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # log(k)*k²/k² = log(k) → diverges; must stay unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM, (
            f"Phase 54: equal degrees should stay unevaluated; got {result!r}"
        )

    def test_regression_log_k_over_k_cubed_still_phase50(self):
        """Regression: plain ``log(k)/k³`` still closes via Phase 50.

        Phase 54 requires a Mul node; a bare Log(k) numerator goes via
        Phase 50's ``_is_log_of_diverging_in_k`` fast path.
        """
        from symbolic_ir import LOG, POW, SUB

        log_k = IRApply(LOG, (_k,))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        g_k = IRApply(DIV, (log_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (log_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Regression: log(k)/k³ should close via Phase 50; got {result!r}"
        )


# ---------------------------------------------------------------------------
# Phase 55 — Bounded × Log(diverging) numerator pattern.
# ---------------------------------------------------------------------------
# ``sin(k)·log(k)/Q(k)`` (and similar shapes) vanish at infinity when
# ``Q(k)`` is any diverging function (polynomial degree ≥ 1, exponential,
# etc.).  The numerator grows sub-polynomially — bounded × log(h) is
# dominated by any polynomial denominator.
#
# The helper ``_is_bounded_times_log_in_k`` requires exactly one
# Log(diverging) factor in a Mul node; all other factors must be bounded.
# The branch in ``_g_vanishes_at_infinity`` closes when the denominator
# passes ``_h_diverges_at_infinity``.
#
# This is the bounded-times-log complement of Phase 52 (bounded×polynomial)
# and Phase 54 (log×polynomial).
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase55BoundedTimesLogNumerator:
    """Phase 55: Mul(bounded, Log(diverging)) numerator + diverging denominator."""

    def test_sin_k_times_log_k_over_k_squared_closes(self):
        """``sin(k)·log(k)/k²``: bounded×log over poly-deg-2.  Phase 55 closes.

        ``|sin(k)| ≤ 1`` and ``log(k)`` grows sub-polynomially, so the
        numerator is dominated by the degree-2 polynomial denominator.
        poly_deg of numerator = 0 (no polynomial factor); denominator diverges
        polynomially at rate k² → quotient vanishes.
        """
        from symbolic_ir import LOG, POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 55 should close sin(k)·log(k)/k²; got {result!r}"
        )

    def test_cos_k_times_log_k_over_k_closes(self):
        """``cos(k)·log(k)/k``: bounded×log over poly-deg-1.  Phase 55 closes.

        Even with a degree-1 denominator (``k``), the sub-polynomial
        growth of the numerator is dominated → quotient vanishes.
        """
        from symbolic_ir import COS, LOG, SUB

        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (cos_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (cos_kp1, log_kp1))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 55 should close cos(k)·log(k)/k; got {result!r}"
        )

    def test_two_bounded_factors_times_log_over_k_cubed_closes(self):
        """``sin(k)·cos(k)·log(k)/k³``: two bounded factors × log / poly-deg-3.

        Multiple bounded factors in the Mul are all accepted — each
        individually bounded, and the product of bounded functions is bounded.
        Phase 55 closes since the denominator diverges.
        """
        from symbolic_ir import COS, LOG, POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1, log_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 55 should close sin(k)·cos(k)·log(k)/k³; got {result!r}"
        )

    def test_bounded_times_log_of_k_squared_over_k_cubed_closes(self):
        """``sin(k)·log(k²)/k³``: log argument is ``k²`` (diverges).

        ``_is_log_of_diverging_in_k(Log(k²), k)`` returns True since
        ``k²`` is a positive-degree polynomial.  Using ``k²`` as the log
        argument avoids nesting issues in the structural telescoping check
        (``subst(k+1, k, Log(Pow(k,2)))`` produces ``Log(Pow(k+1,2))``
        which compares structurally equal to the manually-built ``g(k+1)``).
        Phase 55 closes.
        """
        from symbolic_ir import LOG, POW, SIN, SUB

        k_sq = IRApply(POW, (_k, IRInteger(2)))
        sin_k = IRApply(SIN, (_k,))
        log_ksq = IRApply(LOG, (k_sq,))  # log(k²) as numerator factor
        num_k = IRApply(MUL, (sin_k, log_ksq))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_sq = IRApply(POW, (kp1, IRInteger(2)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1_sq = IRApply(LOG, (kp1_sq,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1_sq))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 55 should close sin(k)·log(k²)/k³; got {result!r}"
        )

    def test_bounded_times_log_constant_denominator_refused(self):
        """``sin(k)·log(k)/1``: denominator is constant — does not diverge.

        The numerator shape passes ``_is_bounded_times_log_in_k``, but
        the denominator (``1``) is not recognised as diverging by
        ``_h_diverges_at_infinity``.  Phase 55 correctly refuses, and
        no other phase closes the sum.  Result must stay unevaluated.
        """
        from symbolic_ir import LOG, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1))
        # Denominator = 1 (constant): _h_diverges_at_infinity returns False.
        g_k = IRApply(DIV, (num_k, IRInteger(1)))
        g_kp1 = IRApply(DIV, (num_kp1, IRInteger(1)))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # sin(k)·log(k)/1 does not vanish; must stay unevaluated.
        assert isinstance(result, IRApply) and result.head == SUM, (
            f"Phase 55: constant denominator should stay unevaluated; got {result!r}"
        )


# ---------------------------------------------------------------------------
# Phase 56 — Bounded × Sqrt(diverging) numerator pattern.
#
# Mirror of Phase 55 (bounded × Log) with Sqrt instead.  Numerator
# Mul(bounded, Sqrt(P(k))) has effective polynomial degree deg(P)/2;
# vanishes against denominators of degree > deg(P)/2 (polynomial) or
# against any non-polynomial diverging denominator (Exp / Pow / Log×poly).
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase56BoundedTimesSqrtNumerator:
    def test_sin_times_sqrt_k_over_k_squared_closes(self):
        """``sin(k)·sqrt(k)/k²``: half-deg 1/2 < deg 2 → vanishes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM), (
            f"Phase 56 should close; got {result!r}"
        )

    def test_cos_times_sqrt_k_cubed_over_k_squared_closes(self):
        """``cos(k)·sqrt(k³)/k²``: half-deg 3/2 < 2 → vanishes (tight margin)."""
        from symbolic_ir import COS, POW, SQRT, SUB

        cos_k = IRApply(COS, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (cos_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (cos_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_two_bounded_factors_times_sqrt_closes(self):
        """``sin(k)·cos(k)·sqrt(k)/k²``: two bounded × sqrt(k) → vanishes."""
        from symbolic_ir import COS, POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        cos_kp1 = IRApply(COS, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_bounded_times_sqrt_over_exponential_closes(self):
        """``sin(k)·sqrt(k³)/2^k``: Sqrt sub-polynomial / exponential dominates."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (sin_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_3))
        # Denominator is 2^k — exponential, dominates polynomial of any degree.
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_times_sqrt_k_cubed_over_k_refused(self):
        """``sin(k)·sqrt(k³)/k``: half-deg 3/2 > deg 1 → does NOT vanish.
        Phase 56 must refuse.
        """
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (sin_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # half-deg(num) = 3/2 > 1 = deg(den) → does not vanish.
        assert isinstance(result, IRApply) and result.head == SUM

    def test_two_sqrt_factors_now_closed_by_phase61(self):
        """``Mul(sin(k), sqrt(k), sqrt(k))/k³`` — two sqrt factors.
        Phase 56 used to refuse this conservatively.  Phase 61 now
        handles it: effective_x2 = 1 + 1 = 2; 2·3 = 6 > 2 → closes.
        """
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Phase 61 handles two-sqrt: x2=2, 2·3=6 > 2 → closes.
        assert not (isinstance(result, IRApply) and result.head == SUM)


# ---------------------------------------------------------------------------
# Phase 57 — Bounded × Log(diverging) × Sqrt(positive-poly) numerator.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase57BoundedLogSqrtNumerator:
    def test_sin_log_sqrt_over_k_squared_closes(self):
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_sqrt_only_over_k_squared_closes(self):
        from symbolic_ir import POW, SQRT, SUB

        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (log_k, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_cos_log_sqrt_k_cubed_over_k_squared_closes(self):
        from symbolic_ir import COS, POW, SQRT, SUB

        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (cos_k, log_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (cos_kp1, log_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_bounded_log_sqrt_over_exponential_closes(self):
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_sqrt_k_cubed_over_k_refused(self):
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k3 = IRApply(SQRT, (IRApply(POW, (_k, IRInteger(3))),))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (IRApply(POW, (kp1, IRInteger(3))),))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_two_log_factors_now_closed_by_phase64(self):
        """sin(k) · log(k) · log(k+1) · √k / k²: Phase 57 conservatively refused two Logs,
        but Phase 64 now correctly closes this — effective_x2=1; 2·2=4 > 1 → closes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        log_kp1_inner = IRApply(LOG, (IRApply(ADD, (_k, IRInteger(1))),))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k, log_kp1_inner, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp2 = IRApply(LOG, (IRApply(ADD, (kp1, IRInteger(1))),))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, log_kp2, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_no_sqrt_falls_through_to_phase55(self):
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(2)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(2)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        # Phase 55 catches this (bounded × Log).
        assert not (isinstance(result, IRApply) and result.head == SUM)


# Phase 58 — Bounded × Log(diverging) × polynomial numerator.
# ---------------------------------------------------------------------------


class TestEvaluateSumPhase58BoundedLogPolyNumerator:
    def test_sin_log_k_times_k_over_k_cubed_closes(self):
        """sin(k)·log(k)·k / k³: poly_deg=1, den_deg=3, 3>1 → vanishes."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_cos_log_k_times_k_sq_over_k_fourth_closes(self):
        """cos(k)·log(k)·k² / k⁴: poly_deg=2, den_deg=4, 4>2 → vanishes."""
        from symbolic_ir import COS, POW, SUB

        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (cos_k, log_k, k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (cos_kp1, log_kp1, kp1_2))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(4)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(4)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_two_bounded_log_k_times_k_over_k_cubed_closes(self):
        """sin(k)·cos(k)·log(k)·k / k³: two bounded + log + poly → vanishes."""
        from symbolic_ir import COS, POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k, log_k, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1, log_kp1, kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (_k, IRInteger(3)))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (kp1, IRInteger(3)))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_k_times_k_sq_over_exponential_closes(self):
        """sin(k)·log(k)·k² / 2ᵏ: exponential denominator → vanishes."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, log_k, k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, kp1_2))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_k_times_k_sq_over_k_sq_refused(self):
        """sin(k)·log(k)·k² / k²: equal degrees → log(k)·C → ∞, refused."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, log_k, k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, kp1_2))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_sin_log_k_times_k_cubed_over_k_sq_refused(self):
        """sin(k)·log(k)·k³ / k²: poly_deg>den_deg → numerator wins, refused."""
        from symbolic_ir import POW, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        num_k = IRApply(MUL, (sin_k, log_k, k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, kp1_3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase59BoundedSqrtPolyNumerator:
    """Phase 59 — ``Mul(bounded, Sqrt(positive-poly), polynomial)`` numerator.

    Fills the gap between:
      - Phase 53: ``Mul(Sqrt, polynomial_only)`` — refuses bounded factors.
      - Phase 56: ``Mul(bounded, Sqrt)`` — refuses polynomial factors.

    Effective growth: ``C · k^{deg(P)/2 + poly_deg}``.
    ×2 trick: ``effective_x2 = deg(P) + 2·poly_deg``.
    Vanishes when ``2·den_deg > effective_x2`` or non-polynomial diverging denom.
    """

    def test_sin_sqrt_k_times_k_over_k_cubed_closes(self):
        """sin(k)·√k·k / k³: sqrt_inner_deg=1, poly_deg=1, x2=3; 2·3=6>3 → closes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_cos_sqrt_k_sq_times_k_sq_over_k_fourth_closes(self):
        """cos(k)·√(k²)·k² / k⁴: sqrt_inner_deg=2, poly_deg=2, x2=6; 2·4=8>6 → closes."""
        from symbolic_ir import COS, POW, SQRT, SUB

        cos_k = IRApply(COS, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (cos_k, sqrt_k2, k2))
        k4 = IRApply(POW, (_k, IRInteger(4)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (cos_kp1, sqrt_kp1_2, kp1_2))
        kp1_4 = IRApply(POW, (kp1, IRInteger(4)))
        g_k = IRApply(DIV, (num_k, k4))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_4))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_two_bounded_sqrt_k_times_k_over_k_cubed_closes(self):
        """sin(k)·cos(k)·√k·k / k³: two bounded factors + sqrt + poly → closes."""
        from symbolic_ir import COS, POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k, sqrt_k, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        cos_kp1 = IRApply(COS, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1, sqrt_kp1, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_times_k_sq_over_exponential_closes(self):
        """sin(k)·√k·k² / 2^k: non-polynomial diverging denom → closes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, sqrt_k, k2))
        exp_k = IRApply(POW, (IRInteger(2), _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1, kp1_2))
        exp_kp1 = IRApply(POW, (IRInteger(2), kp1))
        g_k = IRApply(DIV, (num_k, exp_k))
        g_kp1 = IRApply(DIV, (num_kp1, exp_kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_sq_times_k_over_k_sq_refused(self):
        """sin(k)·√(k²)·k / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (sin_k, sqrt_k2, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_2, kp1))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_sin_sqrt_k_times_k_cubed_over_k_sq_refused(self):
        """sin(k)·√k·k³ / k²: x2=1+6=7; 2·2=4 < 7 → numerator wins, refused."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        num_k = IRApply(MUL, (sin_k, sqrt_k, k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1, kp1_3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase60BoundedLogSqrtPolyNumerator:
    """Phase 60 — ``Mul(bounded, Log(diverging), Sqrt(positive-poly), polynomial)`` numerator.

    Extends Phase 57 (bounded × Log × Sqrt, refuses poly) by allowing
    polynomial factors alongside Log and Sqrt.

    Effective growth: ``log(k) · k^{deg(P)/2 + poly_deg}``.
    ×2 trick: ``effective_x2 = sqrt_inner_deg + 2·poly_deg``.
    Vanishes when ``2·den_deg > effective_x2`` or non-polynomial diverging denom.
    """

    def test_sin_log_k_sqrt_k_times_k_over_k_cubed_closes(self):
        """sin(k)·log(k)·√k·k / k³: sqrt_x2=1, poly_deg=1, x2=3; 2·3=6>3 → closes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_cos_log_k_sqrt_k_sq_times_k_sq_over_k_fourth_closes(self):
        """cos(k)·log(k)·√(k²)·k² / k⁴: sqrt_x2=2, poly_deg=2, x2=6; 2·4=8>6 → closes."""
        from symbolic_ir import COS, POW, SQRT, SUB

        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (cos_k, log_k, sqrt_k2, k2))
        k4 = IRApply(POW, (_k, IRInteger(4)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (cos_kp1, log_kp1, sqrt_kp1_2, kp1_2))
        kp1_4 = IRApply(POW, (kp1, IRInteger(4)))
        g_k = IRApply(DIV, (num_k, k4))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_4))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_two_bounded_log_sqrt_k_times_k_over_k_cubed_closes(self):
        """sin(k)·cos(k)·log(k)·√k·k / k³: two bounded + log + sqrt + poly → closes."""
        from symbolic_ir import COS, POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        cos_k = IRApply(COS, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, cos_k, log_k, sqrt_k, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        cos_kp1 = IRApply(COS, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, cos_kp1, log_kp1, sqrt_kp1, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_sqrt_k_times_k_sq_over_exponential_closes(self):
        """sin(k)·log(k)·√k·k² / 2^k: non-polynomial diverging denom → closes."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k, k2))
        exp_k = IRApply(POW, (IRInteger(2), _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1, kp1_2))
        exp_kp1 = IRApply(POW, (IRInteger(2), kp1))
        g_k = IRApply(DIV, (num_k, exp_k))
        g_kp1 = IRApply(DIV, (num_kp1, exp_kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_sqrt_k_sq_times_k_over_k_sq_refused(self):
        """sin(k)·log(k)·√(k²)·k / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k2, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1_2, kp1))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_sin_log_sqrt_k_times_k_cubed_over_k_sq_refused(self):
        """sin(k)·log(k)·√k·k³ / k²: x2=1+6=7; 2·2=4 < 7 → numerator wins, refused."""
        from symbolic_ir import POW, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        num_k = IRApply(MUL, (sin_k, log_k, sqrt_k, k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, sqrt_kp1, kp1_3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase61TwoSqrtPolyNumerator:
    """Phase 61: Mul(Sqrt(P1), Sqrt(P2), polynomial..., bounded...) numerator.

    effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_sqrt_k_times_sqrt_k_cubed_over_k_cubed_closes(self):
        """√k · √(k³) / k³: x2=1+3=4; 2·3=6 > 4 → closes.

        g(k) = √k · √(k³) / k³ — all factors plain functions of k.
        g(k+1) = √(k+1) · √((k+1)³) / (k+1)³  [clean substitution]
        """
        from symbolic_ir import SQRT, SUB

        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k = IRApply(SQRT, (_k,))
        sqrt_k3 = IRApply(SQRT, (k3,))
        num_k = IRApply(MUL, (sqrt_k, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        num_kp1 = IRApply(MUL, (sqrt_kp1, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_sq_times_sqrt_k_times_k_over_k_cubed_closes(self):
        """√(k²) · √k · k / k³: x2=2+1+2=5; 2·3=6 > 5 → closes."""
        from symbolic_ir import SQRT, SUB

        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sqrt_k2, sqrt_k, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_2, sqrt_kp1, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_times_sqrt_k_over_k_sq_closes(self):
        """sin(k) · √k · √k / k²: x2=1+1=2; 2·2=4 > 2 → closes (bounded factor)."""
        from symbolic_ir import SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k1, sqrt_k2))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_1, sqrt_kp1_2))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_times_sqrt_k_over_exponential_closes(self):
        """√k · √k / 2^k: non-polynomial diverging denominator."""
        from symbolic_ir import SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_sq_times_sqrt_k_sq_over_k_sq_refused(self):
        """√(k²) · √(k²) / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused."""
        from symbolic_ir import SQRT, SUB

        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2_1 = IRApply(SQRT, (k2,))
        sqrt_k2_2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (sqrt_k2_1, sqrt_k2_2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2_1 = IRApply(SQRT, (kp1_2,))
        sqrt_kp1_2_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_2_1, sqrt_kp1_2_2))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_sqrt_k_times_sqrt_k_cubed_over_k_sq_refused(self):
        """√k · √(k³) / k²: x2=1+3=4; 2·2=4 not > 4 → equal, refused."""
        from symbolic_ir import SQRT, SUB

        sqrt_k = IRApply(SQRT, (_k,))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k3 = IRApply(SQRT, (k3,))
        num_k = IRApply(MUL, (sqrt_k, sqrt_k3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        num_kp1 = IRApply(MUL, (sqrt_kp1, sqrt_kp1_3))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase62TwoLogPolyNumerator:
    """Phase 62: Mul(Log(h1), Log(h2), polynomial..., bounded...) numerator.

    effective_x2 = 2·poly_deg.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_log_k_sq_over_k_sq_closes(self):
        """log(k)·log(k) / k²: poly_deg=0, effective_x2=0; 2·2=4 > 0 → closes."""
        from symbolic_ir import LOG, SUB

        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, log_k2))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, log_kp1_2))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_times_k_over_k_cubed_closes(self):
        """log(k)² · k / k³: poly_deg=1, effective_x2=2; 2·3=6 > 2 → closes."""
        from symbolic_ir import LOG, SUB

        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, log_k2, _k))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, log_kp1_2, kp1))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_k_sq_over_k_sq_closes(self):
        """sin(k) · log(k)² / k²: bounded + two logs, poly_deg=0; 2·2=4 > 0 → closes."""
        from symbolic_ir import LOG, SIN, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k, log_k2))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1, log_kp1_2))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_over_exponential_closes(self):
        """log(k)² / 2^k: non-polynomial diverging denominator."""
        from symbolic_ir import LOG, SUB

        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, log_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, log_kp1_2))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_times_k_sq_over_k_sq_refused(self):
        """log(k)² · k² / k²: poly_deg=2, effective_x2=4; 2·2=4 not > 4 → refused."""
        from symbolic_ir import LOG, SUB

        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        num_k = IRApply(MUL, (log_k, log_k2, k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        num_kp1 = IRApply(MUL, (log_kp1, log_kp1_2, kp1_2))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_log_k_sq_times_k_over_k_refused(self):
        """log(k)² · k / k: poly_deg=1, effective_x2=2; 2·1=2 not > 2 → refused."""
        from symbolic_ir import LOG, SUB

        log_k = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k, log_k2, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1, log_kp1_2, kp1))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase63TwoSqrtLogPolyNumerator:
    """Phase 63: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), polynomial..., bounded...) numerator.

    effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    Log is sub-polynomial; same effective degree as Phase 61.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_sqrt_k_sq_log_k_over_k_sq_closes(self):
        """√k · √k · log(k) / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, log_k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, log_kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k3_sqrt_k_log_k_over_k3_closes(self):
        """√(k³) · √k · log(k) / k³: effective_x2=3+1=4; 2·3=6 > 4 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k3 = IRApply(SQRT, (k3,))
        sqrt_k = IRApply(SQRT, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k3, sqrt_k, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_3, sqrt_kp1, log_kp1))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_sq_log_k_over_k_sq_closes(self):
        """sin(k) · √k · √k · log(k) / k²: bounded + two Sqrts + one Log; effective_x2=2; 2·2=4 > 2 → closes."""
        from symbolic_ir import LOG, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k1, sqrt_k2, log_k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_1, sqrt_kp1_2, log_kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_sq_log_k_over_exponential_closes(self):
        """√k · √k · log(k) / 2^k: non-polynomial diverging denominator."""
        from symbolic_ir import LOG, SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, log_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k2_sq_log_k_over_k2_refused(self):
        """√(k²) · √(k²) · log(k) / k²: effective_x2=2+2=4; 2·2=4 not > 4 → refused."""
        from symbolic_ir import LOG, SQRT, SUB

        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2_1 = IRApply(SQRT, (k2,))
        sqrt_k2_2 = IRApply(SQRT, (k2,))
        log_k = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k2_1, sqrt_k2_2, log_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2_1 = IRApply(SQRT, (kp1_2,))
        sqrt_kp1_2_2 = IRApply(SQRT, (kp1_2,))
        log_kp1 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1))
        kp1_2b = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2b))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase64TwoLogSqrtPolyNumerator:
    """Phase 64: Mul(Log(h1), Log(h2), Sqrt(P), polynomial..., bounded...) numerator.

    effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg.
    log² is sub-polynomial; same effective degree formula as Phase 53.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_log_k_sq_sqrt_k_over_k_sq_closes(self):
        """log(k)² · √k / k²: effective_x2=1; 2·2=4 > 1 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, sqrt_k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, sqrt_kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_sqrt_k3_over_k3_closes(self):
        """log(k)² · √(k³) / k³: effective_x2=3; 2·3=6 > 3 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k3 = IRApply(SQRT, (k3,))
        num_k = IRApply(MUL, (log_k1, log_k2, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_log_k_sq_sqrt_k_over_k_sq_closes(self):
        """sin(k) · log(k)² · √k / k²: bounded + two Logs + Sqrt; effective_x2=1; 2·2=4 > 1 → closes."""
        from symbolic_ir import LOG, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, log_k1, log_k2, sqrt_k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, log_kp1_1, log_kp1_2, sqrt_kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_sqrt_k_over_exponential_closes(self):
        """log(k)² · √k / 2^k: non-polynomial diverging denominator."""
        from symbolic_ir import LOG, SQRT, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, sqrt_k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, sqrt_kp1))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_sq_sqrt_k2_over_k_refused(self):
        """log(k)² · √(k²) / k: effective_x2=2; 2·1=2 not > 2 → refused (equal)."""
        from symbolic_ir import LOG, SQRT, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (log_k1, log_k2, sqrt_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2 = IRApply(SQRT, (kp1_2,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, sqrt_kp1_2))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase65TwoSqrtTwoLogPolyNumerator:
    """Phase 65: Mul(Sqrt(P1), Sqrt(P2), Log(h1), Log(h2), polynomial..., bounded...) numerator.

    effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    log² sub-polynomial; same effective degree as Phase 61.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_sqrt_k_sq_log_k_sq_over_k_sq_closes(self):
        """√k · √k · log(k)² / k²: effective_x2=2; 2·2=4 > 2 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, log_k1, log_k2))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k3_sqrt_k_log_k_sq_over_k3_closes(self):
        """√(k³) · √k · log(k)² / k³: effective_x2=3+1=4; 2·3=6 > 4 → closes."""
        from symbolic_ir import LOG, SQRT, SUB

        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k3 = IRApply(SQRT, (k3,))
        sqrt_k = IRApply(SQRT, (_k,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k3, sqrt_k, log_k1, log_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_3, sqrt_kp1, log_kp1_1, log_kp1_2))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_sq_log_k_sq_over_k_sq_closes(self):
        """sin(k) · √k · √k · log(k)² / k²: bounded + two Sqrts + two Logs; effective_x2=2; closes."""
        from symbolic_ir import LOG, SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k1, sqrt_k2, log_k1, log_k2))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_sq_log_k_sq_over_exponential_closes(self):
        """√k · √k · log(k)² / 2^k: non-polynomial diverging denominator."""
        from symbolic_ir import LOG, SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, log_k1, log_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k2_sq_log_k_sq_over_k2_refused(self):
        """√(k²) · √(k²) · log(k)² / k²: effective_x2=2+2=4; 2·2=4 not > 4 → refused."""
        from symbolic_ir import LOG, SQRT, SUB

        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2_1 = IRApply(SQRT, (k2,))
        sqrt_k2_2 = IRApply(SQRT, (k2,))
        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (sqrt_k2_1, sqrt_k2_2, log_k1, log_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2_1 = IRApply(SQRT, (kp1_2,))
        sqrt_kp1_2_2 = IRApply(SQRT, (kp1_2,))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1_1, log_kp1_2))
        kp1_2b = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2b))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase66ThreeSqrtPolyNumerator:
    """Phase 66: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...) numerator.

    effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    Log factors refused; same ×2 arithmetic as Phase 61.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_sqrt_k_cubed_over_k_sq_closes(self):
        """√k · √k · √k / k²: effective_x2=3; 2·2=4 > 3 → closes."""
        from symbolic_ir import SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        sqrt_k3 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, sqrt_k3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k3_sqrt_k_sqrt_k_over_k3_closes(self):
        """√(k³) · √k · √k / k³: effective_x2=3+1+1=5; 2·3=6 > 5 → closes."""
        from symbolic_ir import SQRT, SUB

        k3 = IRApply(POW, (_k, IRInteger(3)))
        sqrt_k3 = IRApply(SQRT, (k3,))
        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sqrt_k3, sqrt_k1, sqrt_k2))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        sqrt_kp1_3 = IRApply(SQRT, (kp1_3,))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_3, sqrt_kp1_1, sqrt_kp1_2))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sin_sqrt_k_cubed_over_k_sq_closes(self):
        """sin(k) · √k · √k · √k / k²: bounded + three Sqrts; effective_x2=3; closes."""
        from symbolic_ir import SIN, SQRT, SUB

        sin_k = IRApply(SIN, (_k,))
        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        sqrt_k3 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sin_k, sqrt_k1, sqrt_k2, sqrt_k3))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sin_kp1 = IRApply(SIN, (kp1,))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sin_kp1, sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k_cubed_over_exponential_closes(self):
        """√k · √k · √k / 2^k: non-polynomial diverging denominator → closes."""
        from symbolic_ir import SQRT, SUB

        sqrt_k1 = IRApply(SQRT, (_k,))
        sqrt_k2 = IRApply(SQRT, (_k,))
        sqrt_k3 = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (sqrt_k1, sqrt_k2, sqrt_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        sqrt_kp1_1 = IRApply(SQRT, (kp1,))
        sqrt_kp1_2 = IRApply(SQRT, (kp1,))
        sqrt_kp1_3 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_k2_cubed_over_k_sq_refused(self):
        """√(k²) · √(k²) · √(k²) / k²: effective_x2=2+2+2=6; 2·2=4 not > 6 → refused."""
        from symbolic_ir import SQRT, SUB

        k2 = IRApply(POW, (_k, IRInteger(2)))
        sqrt_k2_1 = IRApply(SQRT, (k2,))
        sqrt_k2_2 = IRApply(SQRT, (k2,))
        sqrt_k2_3 = IRApply(SQRT, (k2,))
        num_k = IRApply(MUL, (sqrt_k2_1, sqrt_k2_2, sqrt_k2_3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        kp1_2_a = IRApply(POW, (kp1, IRInteger(2)))
        sqrt_kp1_2_1 = IRApply(SQRT, (kp1_2_a,))
        sqrt_kp1_2_2 = IRApply(SQRT, (kp1_2_a,))
        sqrt_kp1_2_3 = IRApply(SQRT, (kp1_2_a,))
        num_kp1 = IRApply(MUL, (sqrt_kp1_2_1, sqrt_kp1_2_2, sqrt_kp1_2_3))
        k2_b = IRApply(POW, (_k, IRInteger(2)))
        kp1_2_b = IRApply(POW, (kp1, IRInteger(2)))
        g_k66 = IRApply(DIV, (num_k, k2_b))
        g_kp1_66 = IRApply(DIV, (num_kp1, kp1_2_b))
        f66 = IRApply(SUB, (g_k66, g_kp1_66))
        result = evaluate_sum(f66, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestEvaluateSumPhase67ThreeLogPolyNumerator:
    """Phase 67: Mul(Log(h1), Log(h2), Log(h3), polynomial..., bounded...) numerator.

    effective_x2 = 2·poly_deg.
    log³ sub-polynomial; same ×2 arithmetic as Phase 62.
    Sqrt factors refused — use Phase 63/64/65 for sqrt+log combos.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_log_k_cubed_over_k_closes(self):
        """log(k)³ / k: effective_x2=0; 2·1=2 > 0 → closes."""
        from symbolic_ir import LOG, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        log_k3 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, log_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        log_kp1_3 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, log_kp1_3))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_cubed_poly_over_k_sq_closes(self):
        """log(k)³ · k / k²: effective_x2=2; 2·2=4 > 2 → closes."""
        from symbolic_ir import LOG, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        log_k3 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, log_k3, _k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        log_kp1_3 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, log_kp1_3, kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_log_k_cubed_over_exponential_closes(self):
        """log(k)³ / 2^k: non-polynomial diverging denominator → closes."""
        from symbolic_ir import LOG, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        log_k3 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, log_k3))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        log_kp1_3 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, log_kp1_3))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_sqrt_present_refused(self):
        """log(k)³ · √k / k²: sqrt factor present → refused (returns SUM node)."""
        from symbolic_ir import LOG, SQRT, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        log_k3 = IRApply(LOG, (_k,))
        sqrt_k = IRApply(SQRT, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, log_k3, sqrt_k))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        log_kp1_3 = IRApply(LOG, (kp1,))
        sqrt_kp1 = IRApply(SQRT, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, log_kp1_3, sqrt_kp1))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_log_k_cubed_over_k_refused_when_equal(self):
        """log(k)³ · k / k: effective_x2=2; 2·1=2 not > 2 → refused."""
        from symbolic_ir import LOG, SUB

        log_k1 = IRApply(LOG, (_k,))
        log_k2 = IRApply(LOG, (_k,))
        log_k3 = IRApply(LOG, (_k,))
        num_k = IRApply(MUL, (log_k1, log_k2, log_k3, _k))
        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        log_kp1_1 = IRApply(LOG, (kp1,))
        log_kp1_2 = IRApply(LOG, (kp1,))
        log_kp1_3 = IRApply(LOG, (kp1,))
        num_kp1 = IRApply(MUL, (log_kp1_1, log_kp1_2, log_kp1_3, kp1))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestPhase68ThreeSqrtLogPoly:
    """Phase 68: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging),
    polynomial..., bounded...) numerator.

    effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    Log is sub-polynomial — contributes 0 to effective degree.
    Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
    """

    def test_sqrt_k_cubed_log_k_over_k2_closes(self):
        """√k·√k·√k·log(k) / k²: effective_x2=3; 2·2=4 > 3 → closes."""
        from symbolic_ir import SQRT, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), IRApply(SQRT, (_k,)),
                              IRApply(SQRT, (_k,)), IRApply(LOG, (_k,))))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), IRApply(SQRT, (kp1,)),
                                IRApply(SQRT, (kp1,)), IRApply(LOG, (kp1,))))
        k2 = IRApply(POW, (_k, IRInteger(2)))
        kp1_2 = IRApply(POW, (kp1, IRInteger(2)))
        g_k = IRApply(DIV, (num_k, k2))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_2))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_three_sqrt_higher_log_k_over_k3_closes(self):
        """√(k³)·√k·√k·log(k) / k³: effective_x2=5; 2·3=6 > 5 → closes."""
        from symbolic_ir import SQRT, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        num_k = IRApply(MUL, (IRApply(SQRT, (k3,)), IRApply(SQRT, (_k,)),
                              IRApply(SQRT, (_k,)), IRApply(LOG, (_k,))))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1_3,)), IRApply(SQRT, (kp1,)),
                                IRApply(SQRT, (kp1,)), IRApply(LOG, (kp1,))))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_three_sqrt_log_poly_factor_over_k3_closes(self):
        """√k·√k·√k·log(k)·k / k³: effective_x2=5; 2·3=6 > 5 → closes."""
        from symbolic_ir import SQRT, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), IRApply(SQRT, (_k,)),
                              IRApply(SQRT, (_k,)), IRApply(LOG, (_k,)), _k))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), IRApply(SQRT, (kp1,)),
                                IRApply(SQRT, (kp1,)), IRApply(LOG, (kp1,)), kp1))
        k3 = IRApply(POW, (_k, IRInteger(3)))
        kp1_3 = IRApply(POW, (kp1, IRInteger(3)))
        g_k = IRApply(DIV, (num_k, k3))
        g_kp1 = IRApply(DIV, (num_kp1, kp1_3))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_three_sqrt_log_over_exponential_closes(self):
        """√k·√k·√k·log(k) / 2^k: non-polynomial diverging denom → closes."""
        from symbolic_ir import SQRT, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), IRApply(SQRT, (_k,)),
                              IRApply(SQRT, (_k,)), IRApply(LOG, (_k,))))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), IRApply(SQRT, (kp1,)),
                                IRApply(SQRT, (kp1,)), IRApply(LOG, (kp1,))))
        g_k = IRApply(DIV, (num_k, IRApply(POW, (IRInteger(2), _k))))
        g_kp1 = IRApply(DIV, (num_kp1, IRApply(POW, (IRInteger(2), kp1))))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert not (isinstance(result, IRApply) and result.head == SUM)

    def test_three_sqrt_log_over_k_refused_not_sufficient(self):
        """√k·√k·√k·log(k) / k: effective_x2=3; 2·1=2 not > 3 → refused."""
        from symbolic_ir import SQRT, SUB

        kp1 = IRApply(ADD, (_k, IRInteger(1)))
        num_k = IRApply(MUL, (IRApply(SQRT, (_k,)), IRApply(SQRT, (_k,)),
                              IRApply(SQRT, (_k,)), IRApply(LOG, (_k,))))
        num_kp1 = IRApply(MUL, (IRApply(SQRT, (kp1,)), IRApply(SQRT, (kp1,)),
                                IRApply(SQRT, (kp1,)), IRApply(LOG, (kp1,))))
        g_k = IRApply(DIV, (num_k, _k))
        g_kp1 = IRApply(DIV, (num_kp1, kp1))
        f = IRApply(SUB, (g_k, g_kp1))
        result = evaluate_sum(f, _k, IRInteger(1), IRSymbol("%inf"), _VM)
        assert isinstance(result, IRApply) and result.head == SUM
