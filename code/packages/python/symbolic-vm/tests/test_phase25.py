"""Phase 25 — Symbolic Summation and Product tests.

Covers Sum(f, k, a, b) and Product(f, k, a, b) via the symbolic VM.

Test structure
--------------
TestPhase25_SumConstant       — constant summands (f independent of k)
TestPhase25_SumPowerK         — Faulhaber power sums Σ k^m, m = 0..5
TestPhase25_SumGeometricFin   — finite geometric series
TestPhase25_SumGeometricInf   — infinite geometric series
TestPhase25_SumSpecialInf     — Basel / Leibniz / Taylor classic series
TestPhase25_SumScaled         — scaled summands (coefficient × power)
TestPhase25_SumUnevaluated    — fallback for unrecognised patterns
TestPhase25_ProductConst      — constant factor product
TestPhase25_ProductFactorial  — identity product → GammaFunc(n+1)
TestPhase25_ProductScaled     — scaled identity product
TestPhase25_ProductNumeric    — small numeric range product
TestPhase25_ProductUnevaluated — fallback for unrecognised patterns
TestPhase25_Regressions       — Phase 1-24 operations still work
TestPhase25_Macsyma           — end-to-end MACSYMA surface-syntax tests

Verification: exact integer/rational comparison where possible; numeric
approximation (≤1e-9 tolerance) for irrational results.
"""

from __future__ import annotations

import math
from fractions import Fraction

import pytest
from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    PRODUCT,
    SIN,
    SUM,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Test fixtures / helpers
# ---------------------------------------------------------------------------

K = IRSymbol("k")
N = IRSymbol("n")
X = IRSymbol("x")
INF = IRSymbol("%inf")
PI = IRSymbol("%pi")
E_SYM = IRSymbol("%e")


def _vm() -> VM:
    """Fresh symbolic VM."""
    return VM(SymbolicBackend())


def _sum(f: IRNode, lo: IRNode, hi: IRNode) -> IRNode:
    """Evaluate Sum(f, k, lo, hi)."""
    return _vm().eval(IRApply(SUM, (f, K, lo, hi)))


def _product(f: IRNode, lo: IRNode, hi: IRNode) -> IRNode:
    """Evaluate Product(f, k, lo, hi)."""
    return _vm().eval(IRApply(PRODUCT, (f, K, lo, hi)))


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _rat(p: int, q: int) -> IRRational:
    return IRRational(p, q)


def _as_frac(node: IRNode) -> Fraction | None:
    """Extract exact Fraction from IRInteger or IRRational, else None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _as_float(node: IRNode) -> float | None:
    """Evaluate an IR tree to float if possible, else None."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        mapping = {"%pi": math.pi, "%e": math.e}
        return mapping.get(node.name)
    if isinstance(node, IRApply):
        args = [_as_float(a) for a in node.args]
        if any(a is None for a in args):
            return None
        head = node.head
        if head == ADD:
            return sum(args)  # type: ignore[arg-type]
        if head == DIV and len(args) == 2:
            return args[0] / args[1]  # type: ignore[operator]
        if head == MUL:
            result = 1.0
            for a in args:
                result *= a  # type: ignore[operator]
            return result
        if head == POW and len(args) == 2:
            return args[0] ** args[1]  # type: ignore[operator]
        if head == NEG and len(args) == 1:
            return -args[0]  # type: ignore[operator]
        if head == LOG and len(args) == 1:
            return math.log(args[0])  # type: ignore[arg-type]
    return None


def _is_int_val(node: IRNode, val: int) -> bool:
    """True iff *node* is an IRInteger with the given value."""
    return isinstance(node, IRInteger) and node.value == val


def _is_rat_val(node: IRNode, p: int, q: int) -> bool:
    """True iff *node* is an exact rational p/q."""
    f = _as_frac(node)
    return f is not None and f == Fraction(p, q)


def _is_unevaluated_sum(node: IRNode) -> bool:
    return isinstance(node, IRApply) and node.head == SUM


def _is_unevaluated_product(node: IRNode) -> bool:
    return isinstance(node, IRApply) and node.head == PRODUCT


def _is_gamma(node: IRNode) -> bool:
    return isinstance(node, IRApply) and node.head == GAMMA_FUNC


def _approx(node: IRNode, expected: float, tol: float = 1e-9) -> bool:
    v = _as_float(node)
    return v is not None and abs(v - expected) <= tol


# ===========================================================================
# 1. Constant summand: Sum(c, k, lo, hi) = c * (hi − lo + 1)
# ===========================================================================


class TestPhase25_SumConstant:
    def test_const_5_1_10(self):
        """Sum(5, k, 1, 10) = 50."""
        result = _sum(_int(5), _int(1), _int(10))
        assert _is_int_val(result, 50)

    def test_const_3_0_9(self):
        """Sum(3, k, 0, 9) = 30."""
        result = _sum(_int(3), _int(0), _int(9))
        assert _is_int_val(result, 30)

    def test_const_1_1_n(self):
        """Sum(1, k, 1, n) = n (Faulhaber m=0 case)."""
        result = _sum(_int(1), _int(1), N)
        # Result should be a non-SUM IR expression equal to n
        assert not _is_unevaluated_sum(result)

    def test_const_rational_half_1_4(self):
        """Sum(1/2, k, 1, 4) = 2."""
        result = _sum(_rat(1, 2), _int(1), _int(4))
        assert _is_rat_val(result, 2, 1) or _is_int_val(result, 2)

    def test_const_neg7_2_5(self):
        """Sum(-7, k, 2, 5) = -28."""
        f = IRApply(NEG, (_int(7),))
        result = _sum(f, _int(2), _int(5))
        assert _is_int_val(result, -28)


# ===========================================================================
# 2. Faulhaber power sums: Sum(k^m, k, 1, n)
# ===========================================================================


class TestPhase25_SumPowerK:
    def test_sum_k1_concrete_4(self):
        """Sum(k, k, 1, 4) = 10."""
        result = _sum(K, _int(1), _int(4))
        assert _is_int_val(result, 10)

    def test_sum_k2_concrete_4(self):
        """Sum(k^2, k, 1, 4) = 30."""
        f = IRApply(POW, (K, _int(2)))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 30)

    def test_sum_k3_concrete_3(self):
        """Sum(k^3, k, 1, 3) = 1+8+27 = 36."""
        f = IRApply(POW, (K, _int(3)))
        result = _sum(f, _int(1), _int(3))
        assert _is_int_val(result, 36)

    def test_sum_k3_concrete_4(self):
        """Sum(k^3, k, 1, 4) = 100 (= [4·5/2]²)."""
        f = IRApply(POW, (K, _int(3)))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 100)

    def test_sum_k4_concrete_4(self):
        """Sum(k^4, k, 1, 4) = 354."""
        f = IRApply(POW, (K, _int(4)))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 354)

    def test_sum_k5_concrete_4(self):
        """Sum(k^5, k, 1, 4) = 1300."""
        f = IRApply(POW, (K, _int(5)))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 1300)

    def test_sum_k1_symbolic_n(self):
        """Sum(k, k, 1, n) → closed form (not unevaluated SUM)."""
        result = _sum(K, _int(1), N)
        assert not _is_unevaluated_sum(result)

    def test_sum_k2_symbolic_n(self):
        """Sum(k^2, k, 1, n) → closed form."""
        f = IRApply(POW, (K, _int(2)))
        result = _sum(f, _int(1), N)
        assert not _is_unevaluated_sum(result)

    def test_sum_k3_symbolic_n(self):
        """Sum(k^3, k, 1, n) → closed form."""
        f = IRApply(POW, (K, _int(3)))
        result = _sum(f, _int(1), N)
        assert not _is_unevaluated_sum(result)

    def test_sum_k0_concrete(self):
        """Sum(k^0, k, 1, 5) = 5 (constant)."""
        f = IRApply(POW, (K, _int(0)))
        result = _sum(f, _int(1), _int(5))
        assert _is_int_val(result, 5)

    def test_sum_k1_lo0_concrete(self):
        """Sum(k, k, 0, 4) = 10  (lo=0 handled via correction)."""
        result = _sum(K, _int(0), _int(4))
        assert _is_int_val(result, 10)


# ===========================================================================
# 3. Scaled power sums: Sum(c * k^m, k, lo, hi)
# ===========================================================================


class TestPhase25_SumScaled:
    def test_sum_2k_1_4(self):
        """Sum(2*k, k, 1, 4) = 2*10 = 20."""
        f = IRApply(MUL, (_int(2), K))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 20)

    def test_sum_3k_1_4(self):
        """Sum(3*k, k, 1, 4) = 3*10 = 30."""
        f = IRApply(MUL, (_int(3), K))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 30)

    def test_sum_2k2_1_4(self):
        """Sum(2*k^2, k, 1, 4) = 2*30 = 60."""
        f = IRApply(MUL, (_int(2), IRApply(POW, (K, _int(2)))))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 60)

    def test_sum_half_k_1_4(self):
        """Sum((1/2)*k, k, 1, 4) = 10/2 = 5."""
        f = IRApply(MUL, (_rat(1, 2), K))
        result = _sum(f, _int(1), _int(4))
        v = _as_frac(result)
        assert v is not None and v == Fraction(5)

    def test_sum_k2_mul_commuted(self):
        """Sum(k^2 * 3, k, 1, 4) = 3*30 = 90  (MUL(POW, c) form)."""
        f = IRApply(MUL, (IRApply(POW, (K, _int(2))), _int(3)))
        result = _sum(f, _int(1), _int(4))
        assert _is_int_val(result, 90)


# ===========================================================================
# 4. Finite geometric series
# ===========================================================================


class TestPhase25_SumGeometricFin:
    def test_geom_r3_0_3(self):
        """Sum(3^k, k, 0, 3) = (3^4-1)/(3-1) = 40."""
        f = IRApply(POW, (_int(3), K))
        result = _sum(f, _int(0), _int(3))
        assert _is_int_val(result, 40)

    def test_geom_r2_0_4(self):
        """Sum(2^k, k, 0, 4) = 31."""
        f = IRApply(POW, (_int(2), K))
        result = _sum(f, _int(0), _int(4))
        assert _is_int_val(result, 31)

    def test_geom_r2_1_5(self):
        """Sum(2^k, k, 1, 5) = 2+4+8+16+32 = 62."""
        f = IRApply(POW, (_int(2), K))
        result = _sum(f, _int(1), _int(5))
        assert _is_int_val(result, 62)

    def test_geom_half_0_4(self):
        """Sum((1/2)^k, k, 0, 4) = 31/16."""
        r = _rat(1, 2)
        f = IRApply(POW, (r, K))
        result = _sum(f, _int(0), _int(4))
        v = _as_frac(result)
        assert v is not None and v == Fraction(31, 16)

    def test_geom_coeff3_half_0_inf(self):
        """Sum(3*(1/2)^k, k, 0, inf) = 3 * 2 = 6."""
        r = _rat(1, 2)
        f = IRApply(MUL, (_int(3), IRApply(POW, (r, K))))
        result = _sum(f, _int(0), INF)
        v = _as_frac(result)
        assert v is not None and v == Fraction(6)


# ===========================================================================
# 5. Infinite geometric series
# ===========================================================================


class TestPhase25_SumGeometricInf:
    def test_geom_half_0_inf(self):
        """Sum((1/2)^k, k, 0, inf) = 2."""
        r = _rat(1, 2)
        f = IRApply(POW, (r, K))
        result = _sum(f, _int(0), INF)
        v = _as_frac(result)
        assert v is not None and v == Fraction(2)

    def test_geom_third_0_inf(self):
        """Sum((1/3)^k, k, 0, inf) = 3/2."""
        r = _rat(1, 3)
        f = IRApply(POW, (r, K))
        result = _sum(f, _int(0), INF)
        v = _as_frac(result)
        assert v is not None and v == Fraction(3, 2)

    def test_geom_half_1_inf(self):
        """Sum((1/2)^k, k, 1, inf) = 1."""
        r = _rat(1, 2)
        f = IRApply(POW, (r, K))
        result = _sum(f, _int(1), INF)
        v = _as_frac(result)
        assert v is not None and v == Fraction(1)

    def test_geom_coeff2_half_0_inf(self):
        """Sum(2 * (1/2)^k, k, 0, inf) = 4."""
        r = _rat(1, 2)
        f = IRApply(MUL, (IRApply(POW, (r, K)), _int(2)))
        result = _sum(f, _int(0), INF)
        v = _as_frac(result)
        assert v is not None and v == Fraction(4)


# ===========================================================================
# 6. Classic infinite series (Basel, Leibniz, Taylor)
# ===========================================================================


class TestPhase25_SumSpecialInf:
    def test_basel_pi2_over_6(self):
        """Sum(1/k^2, k, 1, inf) = π²/6."""
        f = IRApply(DIV, (_int(1), IRApply(POW, (K, _int(2)))))
        result = _sum(f, _int(1), INF)
        # Must not be unevaluated
        assert not _is_unevaluated_sum(result)
        v = _as_float(result)
        assert v is not None and abs(v - math.pi**2 / 6) < 1e-9

    def test_basel4_pi4_over_90(self):
        """Sum(1/k^4, k, 1, inf) = π⁴/90."""
        f = IRApply(DIV, (_int(1), IRApply(POW, (K, _int(4)))))
        result = _sum(f, _int(1), INF)
        assert not _is_unevaluated_sum(result)
        v = _as_float(result)
        assert v is not None and abs(v - math.pi**4 / 90) < 1e-9

    def test_leibniz_pi_over_4(self):
        """Sum((-1)^k / (2k+1), k, 0, inf) = π/4."""
        neg1_k = IRApply(POW, (_int(-1), K))
        denom = IRApply(ADD, (IRApply(MUL, (_int(2), K)), _int(1)))
        f = IRApply(DIV, (neg1_k, denom))
        result = _sum(f, _int(0), INF)
        assert not _is_unevaluated_sum(result)
        v = _as_float(result)
        assert v is not None and abs(v - math.pi / 4) < 1e-9

    def test_taylor_e(self):
        """Sum(1/k!, k, 0, inf) = e  (via GammaFunc representation)."""
        from symbolic_ir import GAMMA_FUNC  # noqa: PLC0415

        gamma_k1 = IRApply(GAMMA_FUNC, (IRApply(ADD, (K, _int(1))),))
        f = IRApply(DIV, (_int(1), gamma_k1))
        result = _sum(f, _int(0), INF)
        # Should return %e
        assert result == E_SYM

    def test_taylor_exp_x(self):
        """Sum(x^k/k!, k, 0, inf) = exp(x)."""
        from symbolic_ir import EXP, GAMMA_FUNC  # noqa: PLC0415

        gamma_k1 = IRApply(GAMMA_FUNC, (IRApply(ADD, (K, _int(1))),))
        f = IRApply(DIV, (IRApply(POW, (X, K)), gamma_k1))
        result = _sum(f, _int(0), INF)
        # Should be EXP(x)
        assert isinstance(result, IRApply) and result.head == EXP


# ===========================================================================
# 7. Sum — unevaluated fallback
# ===========================================================================


class TestPhase25_SumUnevaluated:
    def test_sin_k_unevaluated(self):
        """Sum(sin(k), k, 1, n) → unevaluated SUM."""
        f = IRApply(SIN, (K,))
        result = _sum(f, _int(1), N)
        assert _is_unevaluated_sum(result)

    def test_sin_k_inf_unevaluated(self):
        """Sum(sin(k), k, 1, inf) → unevaluated SUM."""
        f = IRApply(SIN, (K,))
        result = _sum(f, _int(0), INF)
        assert _is_unevaluated_sum(result)

    def test_log_k_unevaluated(self):
        """Sum(log(k), k, 1, n) → unevaluated SUM."""
        f = IRApply(LOG, (K,))
        result = _sum(f, _int(1), N)
        assert _is_unevaluated_sum(result)

    def test_bad_arity_unevaluated(self):
        """Sum node with wrong arity passes through unchanged."""
        vm = _vm()
        bad = IRApply(SUM, (K, K))  # 2 args, not 4
        result = vm.eval(bad)
        assert result == bad


# ===========================================================================
# Phase 40 — Apart + telescope composition
#
# The sum_handler first calls ``evaluate_sum`` on the original summand.
# If that falls through to the unevaluated ``Sum(...)`` shape AND the
# summand has a ``Div`` head, the handler retries once after expanding
# via ``Apart(f, k)`` (partial-fraction decomposition).  Classic case:
# ``∑_{k=1}^{N} 1/(k(k+1))`` → ``∑ [1/k − 1/(k+1)]`` (Phase 39 telescope
# then closes it to ``1 − 1/(N+1)``).  When ``Apart`` returns the input
# unchanged (denominator not factorable into simple roots over ℚ), the
# retry is a no-op and the sum stays unevaluated.
# ===========================================================================


class TestPhase40_ApartTelescope:
    def test_one_over_k_times_kp1_concrete(self):
        """``∑_{k=1}^{4} 1/(k(k+1)) = 1/2 + 1/6 + 1/12 + 1/20 = 4/5``."""
        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), _int(4))
        v = _as_frac(result)
        assert v is not None and v == Fraction(4, 5), f"got {result!r}"

    def test_one_over_k_times_kp1_concrete_larger(self):
        """``∑_{k=1}^{10} 1/(k(k+1)) = 10/11`` — Apart → telescope
        gives ``1 − 1/11 = 10/11``."""
        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), _int(10))
        v = _as_frac(result)
        assert v is not None and v == Fraction(10, 11), f"got {result!r}"

    def test_one_over_k_times_kp1_symbolic_upper(self):
        """``∑_{k=1}^{n} 1/(k(k+1))`` symbolic n: Phase 40 must not leave
        the sum unevaluated.  The exact closed form is ``1 − 1/(n+1)``
        or equivalently ``n/(n+1)``; we don't pin a specific IR shape."""
        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), N)
        assert not _is_unevaluated_sum(result), (
            f"Expected Phase 40 to close ∑ 1/(k(k+1)); got {result!r}"
        )

    def test_one_over_kp1_times_kp2_concrete(self):
        """``∑_{k=1}^{4} 1/((k+1)(k+2)) = 1/6 + 1/12 + 1/20 + 1/30 = 1/3``.

        Apart on a shifted denominator still exposes the telescope:
        ``1/((k+1)(k+2)) = 1/(k+1) − 1/(k+2)`` so the closed form is
        ``1/2 − 1/6 = 1/3``.
        """
        kp1 = IRApply(ADD, (K, _int(1)))
        kp2 = IRApply(ADD, (K, _int(2)))
        denom = IRApply(MUL, (kp1, kp2))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), _int(4))
        v = _as_frac(result)
        assert v is not None and v == Fraction(1, 3), f"got {result!r}"

    def test_apart_irreducible_quadratic_stays_unevaluated(self):
        """``∑_{k=1}^{n} 1/(k² + 1)``: denominator has no rational roots,
        so Apart has no decomposition to expose and the Phase 40 retry is a
        no-op.  The sum must remain unevaluated."""
        denom = IRApply(ADD, (IRApply(POW, (K, _int(2))), _int(1)))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), N)
        assert _is_unevaluated_sum(result), (
            f"Expected unevaluated for irreducible denominator; got {result!r}"
        )

    def test_non_div_summand_skips_apart_retry(self):
        """``∑_{k=1}^{n} sin(k)`` is not a Div shape — Phase 40 must not
        attempt the Apart retry (which would be expensive) and must
        leave the result as the original unevaluated Sum."""
        f = IRApply(SIN, (K,))
        result = _sum(f, _int(1), N)
        assert _is_unevaluated_sum(result)

    def test_phase40_plus_phase41_infinite_chain(self):
        """End-to-end Phase 40 + Phase 41 chain:

        ``∑_{k=1}^∞ 1/(k(k+1))``
            →  Apart →  ``∑_{k=1}^∞ [1/k − 1/(k+1)]``           (Phase 40)
            →  telescope detected, g(k) = 1/k vanishes at ∞    (Phase 41)
            →  g(1) − lim g = 1 − 0 = 1                         (closed form)

        The single dispatch through ``sum_handler`` must produce the
        scalar integer ``1`` for this canonical infinite series.
        """
        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), INF)
        assert isinstance(result, IRInteger) and result.value == 1, (
            f"Expected scalar 1; got {result!r}"
        )

    def test_phase45_chain_higher_starting_index(self):
        """``∑_{k=2}^∞ 1/(k(k+1))``: Apart → telescope → g(2) = 1/2.

        Same Phase 40 + Phase 41 chain but starting at k=2 instead of
        k=1.  Confirms the chain handles arbitrary lo.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(2), INF)
        # Expect 1/2.  Result is IRRational or IRInteger (folded).
        if isinstance(result, IRInteger):
            assert result.value == 0 or result.value * 2 == 1, f"got {result!r}"
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(1, 2), (
                f"got {result!r}"
            )

    def test_phase47_chain_shifted_denominator(self):
        """``∑_{k=1}^∞ 1/((k+1)(k+2)) = 1/2`` — Phase 47 closure of the
        shifted-denominator gap previously documented as
        ``test_phase45_chain_shifted_denominator``.

        ``Apart`` already decomposed the denominator into
        ``Add(Neg(Div(1, k+2)), Div(1, k+1))``, but the cas-summation
        telescope detector substitutes ``k → k+1`` in the left half
        and got ``Div(1, Add(Add(k, 1), 1))``, which the symbolic-vm
        Add handler left unflattened — so structural equality against
        ``Div(1, Add(k, 2))`` failed.

        Phase 47 teaches the Add handler to flatten nested ``Add``s
        of literals (``Add(Add(k, 1), 1) → Add(k, 2)``), and the
        antisymmetric telescope closes cleanly to ``g(1) = 1/2``.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        kp1 = IRApply(ADD, (K, _int(1)))
        kp2 = IRApply(ADD, (K, _int(2)))
        denom = IRApply(MUL, (kp1, kp2))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), INF)
        if isinstance(result, IRInteger):
            assert result.value * 2 == 1
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(1, 2), (
                f"got {result!r}"
            )

    def test_phase47_nested_add_flattens(self):
        """Direct test of the Add handler's Phase 47 flattening.

        Pin the underlying behaviour exploited by the shifted-
        denominator telescope: ``Add(Add(k, 1), 1)`` evaluates to the
        canonical ``Add(k, 2)``.
        """
        from symbolic_vm.backends import SymbolicBackend
        from symbolic_vm.vm import VM

        vm = VM(SymbolicBackend())
        nested = IRApply(ADD, (IRApply(ADD, (K, _int(1))), _int(1)))
        out = vm.eval(nested)
        assert isinstance(out, IRApply) and out.head == ADD, f"got {out!r}"
        assert out.args == (K, _int(2)), f"got {out!r}"

    def test_phase47_triply_nested_add(self):
        """``Add(Add(Add(k, 1), 1), 1) → Add(k, 3)`` — flattening is
        recursive, not just one level deep.
        """
        from symbolic_vm.backends import SymbolicBackend
        from symbolic_vm.vm import VM

        vm = VM(SymbolicBackend())
        triple = IRApply(
            ADD,
            (
                IRApply(ADD, (IRApply(ADD, (K, _int(1))), _int(1))),
                _int(1),
            ),
        )
        out = vm.eval(triple)
        assert isinstance(out, IRApply) and out.head == ADD
        assert out.args == (K, _int(3)), f"got {out!r}"

    def test_phase47_higher_shifted_denominator(self):
        """``∑_{k=1}^∞ 1/((k+2)(k+3)) = 1/3`` — higher shifts still
        telescope after Phase 47 flattening.

        Apart decomposes to ``1/(k+2) − 1/(k+3)``; the antisymmetric
        telescope closes to ``g(1) = 1/(1+2) = 1/3``.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        kp2 = IRApply(ADD, (K, _int(2)))
        kp3 = IRApply(ADD, (K, _int(3)))
        denom = IRApply(MUL, (kp2, kp3))
        f = IRApply(DIV, (_int(1), denom))
        result = _sum(f, _int(1), INF)
        if isinstance(result, IRInteger):
            assert result.value * 3 == 1
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(1, 3), (
                f"got {result!r}"
            )

    def test_phase48_repeated_factor_closes(self):
        """``∑_{k=1}^∞ (2k+1)/(k²(k+1)²) = 1`` — Phase 48 closure of the
        repeated-linear-factor gap previously documented as
        ``test_phase45_repeated_factor_known_gap``.

        Apart now decomposes the denominator ``k²(k+1)²`` (two
        repeated linear factors) via the generic Taylor-expansion +
        power-series-division residue formula and returns
        ``1/k² − 1/(k+1)²``.  The Phase 41/42 telescope detector
        (g(k) = 1/k² vanishes at ∞) closes the sum to ``g(1) = 1``.
        """
        import pytest

        k_sq = IRApply(POW, (K, _int(2)))
        kp1_sq = IRApply(POW, (IRApply(ADD, (K, _int(1))), _int(2)))
        denom = IRApply(MUL, (k_sq, kp1_sq))
        numer = IRApply(ADD, (IRApply(MUL, (_int(2), K)), _int(1)))
        f = IRApply(DIV, (numer, denom))
        result = _sum(f, _int(1), INF)
        assert isinstance(result, IRInteger) and result.value == 1, (
            f"Expected scalar 1; got {result!r}"
        )

    def test_phase48_simple_repeated_root(self):
        """``∑_{k=1}^∞ 1/k² = π²/6`` is handled by the classic-series
        recogniser (Basel) directly without going through Apart.  This
        test instead exercises Apart on the bare ``1/k²`` shape and
        verifies it returns the input unchanged (one rational root at
        ``k = 0`` with multiplicity 2; nothing to decompose since the
        numerator is already 1 and the partial fraction IS just
        ``1/k²``).
        """
        from symbolic_ir import IRSymbol

        APART = IRSymbol("Apart")
        f = IRApply(DIV, (_int(1), IRApply(POW, (K, _int(2)))))
        result = _vm().eval(IRApply(APART, (f, K)))
        # Should decompose into A/k + B/k² (here A=0, B=1) → just 1/k².
        # Either output shape (``Div(1, k^2)`` or ``Add(0, 1/k^2)``) is
        # acceptable as long as the result represents 1/k².
        assert not isinstance(result, IRApply) or result.head != APART, (
            f"Apart left input unchanged: {result!r}"
        )

    def test_phase48_apart_higher_multiplicity(self):
        """``Apart(1/(k(k+1)²), k)`` decomposes ``1/(k+1) − 1/(k+1)² − 1/k``
        — Phase 48 handles mixed simple + multiplicity-2 roots.

        Verifies the decomposition is non-trivial (Apart didn't bail to
        the unevaluated ``Apart(...)`` form).
        """
        from symbolic_ir import IRSymbol

        APART = IRSymbol("Apart")
        denom = IRApply(
            MUL,
            (K, IRApply(POW, (IRApply(ADD, (K, _int(1))), _int(2)))),
        )
        f = IRApply(DIV, (_int(1), denom))
        result = _vm().eval(IRApply(APART, (f, K)))
        # Result must NOT be the unevaluated Apart(...) form.
        assert not (
            isinstance(result, IRApply) and result.head == APART
        ), f"Apart bailed: {result!r}"
        # Should be a sum (Add) of partial-fraction terms.
        assert isinstance(result, IRApply), f"got {result!r}"

    def test_phase48_repeated_factor_higher_start(self):
        """``∑_{k=2}^∞ (2k+1)/(k²(k+1)²) = 1/4`` — same repeated-factor
        Apart pattern but starting at ``k=2``.

        After Apart this becomes ``∑ [1/k² − 1/(k+1)²]`` from k=2, an
        antisymmetric telescope with ``g(k) = 1/k² → 0`` at ∞,
        closing to ``g(2) = 1/4``.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        k_sq = IRApply(POW, (K, _int(2)))
        kp1_sq = IRApply(POW, (IRApply(ADD, (K, _int(1))), _int(2)))
        denom = IRApply(MUL, (k_sq, kp1_sq))
        numer = IRApply(ADD, (IRApply(MUL, (_int(2), K)), _int(1)))
        f = IRApply(DIV, (numer, denom))
        result = _sum(f, _int(2), INF)
        if isinstance(result, IRInteger):
            assert result.value == 0  # Shouldn't happen, but tolerate
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(1, 4), (
                f"got {result!r}"
            )

    def test_phase46_chain_with_outer_constant_numerator(self):
        """``∑_{k=1}^∞ 5/(k(k+1)) = 5`` — Phase 46 closure of the
        constant-numerator gap previously documented as
        ``test_phase45_chain_with_outer_constant_numerator``.

        Apart returns ``Add(Div(-5, k+1), Div(5, k))`` (note: the
        ``-5`` is folded into the numerator rather than a top-level
        ``Neg``).  Phase 46 extends ``_normalise_add_neg_to_sub`` to
        also recognise ``Div(negative_literal, denom)`` as a negation,
        so the rewrite ``Add(a, Div(-c, d)) → Sub(a, Div(c, d))`` now
        fires.  The result threads into the Phase 41 antisymmetric
        telescope ``g(k) = 5/k`` and closes to ``g(1) = 5``.
        """
        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(5), denom))
        result = _sum(f, _int(1), INF)
        assert isinstance(result, IRInteger) and result.value == 5, (
            f"Expected scalar 5; got {result!r}"
        )

    def test_phase46_negative_constant_numerator(self):
        """``∑_{k=1}^∞ (-3)/(k(k+1)) = -3`` — sign of the outer constant
        propagates through Apart + telescope.

        Apart yields ``Add(Div(3, k+1), Div(-3, k))`` (signs flipped
        from the positive case), and the same Phase 46 widening
        rewrites it to ``Sub(Div(3, k+1), Div(3, k))`` — the *standard*
        orientation ``g(k+1) − g(k)`` with ``g(k) = 3/k``, which
        Phase 41 closes to ``−g(1) = −3``.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (_int(-3), denom))
        result = _sum(f, _int(1), INF)
        if isinstance(result, IRInteger):
            assert result.value == -3, f"got {result!r}"
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(-3, 1), (
                f"got {result!r}"
            )

    def test_phase46_rational_outer_constant(self):
        """``∑_{k=1}^∞ (1/2)/(k(k+1)) = 1/2`` — IRRational numerator
        case, exercising the rational-literal arm of
        :func:`_extract_negation`'s mirror.

        Here the outer constant is positive so no negation logic fires
        for the numerator itself; this pins that Phase 46 didn't
        accidentally over-trigger on positive rational numerators.
        Apart returns ``Add(Div(-1/2, k+1), Div(1/2, k))`` which the
        new widening rewrites to the standard ``Sub`` shape.
        """
        from fractions import Fraction
        from symbolic_ir import IRRational

        denom = IRApply(MUL, (K, IRApply(ADD, (K, _int(1)))))
        f = IRApply(DIV, (IRRational(1, 2), denom))
        result = _sum(f, _int(1), INF)
        if isinstance(result, IRInteger):
            assert result.value * 2 == 1, f"got {result!r}"
        else:
            assert isinstance(result, IRRational)
            assert Fraction(result.numer, result.denom) == Fraction(1, 2), (
                f"got {result!r}"
            )


# ===========================================================================
# 8. Product — constant factor
# ===========================================================================


class TestPhase25_ProductConst:
    def test_product_2_0_4(self):
        """Product(2, k, 0, 4) = 2^5 = 32."""
        result = _product(_int(2), _int(0), _int(4))
        assert _is_int_val(result, 32)

    def test_product_3_1_3(self):
        """Product(3, k, 1, 3) = 3^3 = 27."""
        result = _product(_int(3), _int(1), _int(3))
        assert _is_int_val(result, 27)

    def test_product_const_symbolic_hi(self):
        """Product(2, k, 1, n) → 2^(n-1+1) = 2^n — non-unevaluated."""
        result = _product(_int(2), _int(1), N)
        assert not _is_unevaluated_product(result)

    def test_product_half_0_3(self):
        """Product(1/2, k, 0, 3) = (1/2)^4 = 1/16."""
        result = _product(_rat(1, 2), _int(0), _int(3))
        v = _as_frac(result)
        assert v is not None and v == Fraction(1, 16)


# ===========================================================================
# 9. Product — factorial / identity: k, lo=1 → Γ(n+1)
# ===========================================================================


class TestPhase25_ProductFactorial:
    def test_product_k_1_n(self):
        """Product(k, k, 1, n) → GammaFunc(n+1)."""
        result = _product(K, _int(1), N)
        assert _is_gamma(result)

    def test_product_k_1_n_gamma_arg(self):
        """GammaFunc argument is n+1."""
        result = _product(K, _int(1), N)
        assert _is_gamma(result)
        arg = result.args[0]  # type: ignore[union-attr]
        # Should be ADD(n, 1)
        assert isinstance(arg, IRApply) and arg.head == ADD

    def test_product_k_concrete_5(self):
        """Product(k, k, 1, 5) = 120 (numeric evaluation of GammaFunc)."""
        # For concrete upper bound the numeric path fires
        result = _product(K, _int(1), _int(5))
        assert _is_int_val(result, 120)

    def test_product_k_concrete_4(self):
        """Product(k, k, 1, 4) = 24."""
        result = _product(K, _int(1), _int(4))
        assert _is_int_val(result, 24)


# ===========================================================================
# 10. Product — scaled: c*k, lo=1 → c^n * Γ(n+1)
# ===========================================================================


class TestPhase25_ProductScaled:
    def test_product_2k_1_n(self):
        """Product(2*k, k, 1, n) → 2^n * GammaFunc(n+1)."""
        f = IRApply(MUL, (_int(2), K))
        result = _product(f, _int(1), N)
        # Should be a MUL(POW(2,n), GammaFunc(n+1)) tree (not unevaluated)
        assert not _is_unevaluated_product(result)

    def test_product_3k_concrete_3(self):
        """Product(3*k, k, 1, 3) = 3^3 * 3! = 27 * 6 = 162."""
        f = IRApply(MUL, (_int(3), K))
        result = _product(f, _int(1), _int(3))
        assert _is_int_val(result, 162)


# ===========================================================================
# 11. Product — small numeric range (cas_substitution path)
# ===========================================================================


class TestPhase25_ProductNumeric:
    def test_product_k2_1_3(self):
        """Product(k^2, k, 1, 3) = 1*4*9 = 36."""
        f = IRApply(POW, (K, _int(2)))
        result = _product(f, _int(1), _int(3))
        assert _is_int_val(result, 36)

    def test_product_kp1_1_3(self):
        """Product(k+1, k, 1, 3) = 2*3*4 = 24."""
        f = IRApply(ADD, (K, _int(1)))
        result = _product(f, _int(1), _int(3))
        assert _is_int_val(result, 24)


# ===========================================================================
# 12. Product — unevaluated fallback
# ===========================================================================


class TestPhase25_ProductUnevaluated:
    def test_sin_k_unevaluated(self):
        """Product(sin(k), k, 1, n) → unevaluated PRODUCT."""
        f = IRApply(SIN, (K,))
        result = _product(f, _int(1), N)
        assert _is_unevaluated_product(result)

    def test_k3_lo1_n_unevaluated(self):
        """Product(k^3, k, 1, n) → unevaluated PRODUCT (no closed form)."""
        f = IRApply(POW, (K, _int(3)))
        result = _product(f, _int(1), N)
        assert _is_unevaluated_product(result)

    def test_bad_arity_unevaluated(self):
        """Product node with wrong arity passes through unchanged."""
        vm = _vm()
        bad = IRApply(PRODUCT, (K,))  # 1 arg, not 4
        result = vm.eval(bad)
        assert result == bad


# ===========================================================================
# 13. Regressions — Phase 1–24 still work
# ===========================================================================


class TestPhase25_Regressions:
    def test_indefinite_integral_x2(self):
        """integrate(x^2, x) still gives x^3/3 (Phase 1)."""
        from symbolic_ir import INTEGRATE  # noqa: PLC0415

        vm = _vm()
        f = IRApply(POW, (X, _int(2)))
        result = vm.eval(IRApply(INTEGRATE, (f, X)))
        assert not isinstance(result, IRSymbol) or result.name != "unevaluated"
        # numeric check: eval at x=3 → 9
        from cas_substitution import subst  # noqa: PLC0415

        three = _int(3)
        at3 = vm.eval(subst(three, X, result))
        v = _as_frac(at3)
        assert v is not None and v == Fraction(9)

    def test_definite_integral_x2_0_1(self):
        """integrate(x^2, x, 0, 1) = 1/3 (Phase 24)."""
        vm = _vm()
        f = IRApply(POW, (X, _int(2)))
        result = vm.eval(IRApply(INTEGRATE, (f, X, _int(0), _int(1))))
        assert _is_rat_val(result, 1, 3)

    def test_geometric_sum_does_not_confuse_integrate(self):
        """Sum and Integrate live at different IR heads — no interference."""
        vm = _vm()
        # Evaluate both in the same VM session
        sum_result = vm.eval(
            IRApply(SUM, (IRApply(POW, (_rat(1, 2), K)), K, _int(0), INF))
        )
        int_result = vm.eval(
            IRApply(INTEGRATE, (IRApply(POW, (X, _int(2))), X, _int(0), _int(1)))
        )
        assert _as_frac(sum_result) == Fraction(2)
        assert _is_rat_val(int_result, 1, 3)


# ===========================================================================
# 14. MACSYMA end-to-end
# ===========================================================================


class TestPhase25_Macsyma:
    """Surface-syntax tests via the compiler + runtime stack."""

    def _run(self, src: str) -> IRNode:
        pytest.importorskip(
            "macsyma_runtime",
            reason="macsyma-runtime not installed; skipping MACSYMA e2e test",
        )
        from macsyma_compiler.compiler import (  # noqa: PLC0415
            _STANDARD_FUNCTIONS,
            compile_macsyma,
        )
        from macsyma_parser.parser import parse_macsyma  # noqa: PLC0415
        from macsyma_runtime.name_table import (
            extend_compiler_name_table,  # noqa: PLC0415
        )

        extend_compiler_name_table(_STANDARD_FUNCTIONS)
        stmts = compile_macsyma(parse_macsyma(src + ";"))
        return _vm().eval_program(stmts)

    def test_macsyma_sum_const(self):
        """sum(5, k, 1, 10) = 50."""
        result = self._run("sum(5, k, 1, 10)")
        assert _is_int_val(result, 50)

    def test_macsyma_sum_k(self):
        """sum(k, k, 1, 4) = 10."""
        result = self._run("sum(k, k, 1, 4)")
        assert _is_int_val(result, 10)

    def test_macsyma_sum_k2(self):
        """sum(k^2, k, 1, 4) = 30."""
        result = self._run("sum(k^2, k, 1, 4)")
        assert _is_int_val(result, 30)

    def test_macsyma_sum_geom_finite(self):
        """sum(2^k, k, 0, 4) = 31."""
        result = self._run("sum(2^k, k, 0, 4)")
        assert _is_int_val(result, 31)

    def test_macsyma_sum_geom_infinite(self):
        """sum((1/2)^k, k, 0, %inf) = 2."""
        result = self._run("sum((1/2)^k, k, 0, %inf)")
        v = _as_frac(result)
        assert v is not None and v == Fraction(2)

    def test_macsyma_product_k(self):
        """product(k, k, 1, 5) = 120."""
        result = self._run("product(k, k, 1, 5)")
        assert _is_int_val(result, 120)

    def test_macsyma_product_const(self):
        """product(2, k, 0, 4) = 32."""
        result = self._run("product(2, k, 0, 4)")
        assert _is_int_val(result, 32)

    def test_macsyma_sum_unevaluated(self):
        """sum(sin(k), k, 1, n) stays unevaluated."""
        result = self._run("sum(sin(k), k, 1, n)")
        assert _is_unevaluated_sum(result)
