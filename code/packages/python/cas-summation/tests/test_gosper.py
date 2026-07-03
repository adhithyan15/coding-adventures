"""Tests for Gosper's algorithm — Track H1.

These tests cover the closed-form acceptance cases the spec calls out:

* ``∑_{k=1}^{N} k·2^k = (N-1)·2^(N+1) + 2``  — polynomial × c^k.
* ``∑_{k=0}^{N} k·k! = (N+1)! − 1``           — polynomial × factorial.
* ``∑_{k=0}^{N} 2^k = 2^(N+1) − 1``           — pure geometric (now
                                                 routed via the existing
                                                 geometric handler; we
                                                 verify a regression-free
                                                 closed form).
* Fall-through:  ``∑ sin(k)`` returns the unevaluated ``Sum`` IR.
* Regression: the dispatcher still solves the polynomial-only Faulhaber
              case via the original handler.

We verify each acceptance case by evaluating the symbolic closed form
at a concrete value of the free parameter ``N`` (substituting through
``cas_substitution.subst`` then reducing through the local stub VM) and
comparing to the direct numeric sum.
"""

from __future__ import annotations

from fractions import Fraction

from cas_substitution import subst
from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
    MUL,
    NEG,
    POW,
    SIN,
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
from cas_summation.gosper import (
    _decompose,
    _hyp_ratio,
    _poly_add,
    _poly_gcd,
    _poly_mul,
    _poly_shift,
    try_gosper_sum,
)

# ---------------------------------------------------------------------------
# Minimal stub VM — same as test_summation.py's, kept self-contained so we
# don't tangle the suites together.  Evaluates arithmetic on Integer /
# Rational leaves, recurses into Apply nodes, and also reduces
# ``GammaFunc(IRInteger)`` to a concrete factorial so we can verify
# closed-form numerics that contain Gamma terms.
# ---------------------------------------------------------------------------


class _StubVM:
    def eval(self, node: IRNode) -> IRNode:
        if isinstance(node, (IRInteger, IRRational, IRFloat, IRSymbol)):
            return node
        if not isinstance(node, IRApply):
            return node
        args = tuple(self.eval(a) for a in node.args)

        def _frac(n: IRNode) -> Fraction | None:
            if isinstance(n, IRInteger):
                return Fraction(n.value)
            if isinstance(n, IRRational):
                return Fraction(n.numer, n.denom)
            return None

        def _to_ir(f: Fraction) -> IRNode:
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
                r = Fraction(1)
                for v in vals:
                    r *= v
                return _to_ir(r)
        if head == DIV:
            v0, v1 = _frac(args[0]), _frac(args[1])
            if v0 is not None and v1 is not None and v1 != 0:
                return _to_ir(v0 / v1)
        if head == POW:
            v0, v1 = _frac(args[0]), _frac(args[1])
            if v0 is not None and v1 is not None and v1.denominator == 1:
                exp = v1.numerator
                if exp >= 0:
                    return _to_ir(v0 ** exp)
                # Negative exponent on a non-zero rational base.
                if v0 != 0:
                    return _to_ir(Fraction(1) / (v0 ** (-exp)))
        if head == NEG:
            v = _frac(args[0])
            if v is not None:
                return _to_ir(-v)
        # Special-case: GammaFunc(IRInteger n>0) = (n-1)!.
        if head == GAMMA_FUNC and len(args) == 1:
            v = _frac(args[0])
            if v is not None and v.denominator == 1 and v.numerator >= 1:
                n = v.numerator - 1
                f = 1
                for i in range(1, n + 1):
                    f *= i
                return IRInteger(f)
        return IRApply(node.head, args)


_VM = _StubVM()
_k = IRSymbol("k")
_N = IRSymbol("N")


def _eval_at(node: IRNode, sym: IRSymbol, value: int) -> Fraction | None:
    """Substitute ``sym → value`` in ``node``, evaluate via the stub VM,
    and return the rational value (or ``None`` if irreducible)."""
    sub = subst(IRInteger(value), sym, node)
    out = _VM.eval(sub)
    if isinstance(out, IRInteger):
        return Fraction(out.value)
    if isinstance(out, IRRational):
        return Fraction(out.numer, out.denom)
    return None


# ---------------------------------------------------------------------------
# Internal helper tests — these exercise the polynomial arithmetic
# building blocks directly so a regression doesn't have to climb the
# whole dispatcher.
# ---------------------------------------------------------------------------


class TestPolyHelpers:
    def test_add_basic(self):
        # (1 + 2k) + (3 + k^2) = 4 + 2k + k^2
        result = _poly_add(
            [Fraction(1), Fraction(2)],
            [Fraction(3), Fraction(0), Fraction(1)],
        )
        assert result == [Fraction(4), Fraction(2), Fraction(1)]

    def test_mul_basic(self):
        # (1 + k) · (1 + k) = 1 + 2k + k^2
        result = _poly_mul(
            [Fraction(1), Fraction(1)],
            [Fraction(1), Fraction(1)],
        )
        assert result == [Fraction(1), Fraction(2), Fraction(1)]

    def test_shift_basic(self):
        # Shift (k^2) by +1: (k+1)^2 = 1 + 2k + k^2
        p = [Fraction(0), Fraction(0), Fraction(1)]
        assert _poly_shift(p, 1) == [Fraction(1), Fraction(2), Fraction(1)]

    def test_gcd_basic(self):
        # gcd(k^2 - 1, k - 1) = k - 1 (monic)
        a = [Fraction(-1), Fraction(0), Fraction(1)]
        b = [Fraction(-1), Fraction(1)]
        g = _poly_gcd(a, b)
        # Monic (k - 1).
        assert g == [Fraction(-1), Fraction(1)]


# ---------------------------------------------------------------------------
# Acceptance cases — the four canonical Gosper-summable shapes the spec
# asks us to support.
# ---------------------------------------------------------------------------


def _make_k_times_two_k() -> IRNode:
    """Build the IR for the summand ``k · 2^k``."""
    return IRApply(MUL, (_k, IRApply(POW, (IRInteger(2), _k))))


def _make_k_times_kfact() -> IRNode:
    """Build the IR for the summand ``k · k!`` = ``k · GammaFunc(k+1)``."""
    gamma = IRApply(GAMMA_FUNC, (IRApply(ADD, (_k, IRInteger(1))),))
    return IRApply(MUL, (_k, gamma))


class TestGosperAcceptance:
    def test_k_times_2_to_k_concrete(self):
        """``∑_{k=1}^{5} k·2^k = 1·2 + 2·4 + 3·8 + 4·16 + 5·32 = 258``.

        With the dispatcher's small-range numeric path also able to
        handle this, we go through ``evaluate_sum`` and confirm the
        answer is 258.  Verifies the dispatch is sane, even though we
        can't distinguish which path resolved it.
        """
        f = _make_k_times_two_k()
        result = evaluate_sum(f, _k, IRInteger(1), IRInteger(5), _VM)
        # Allow either IRInteger or IRRational since arithmetic may yield
        # either depending on the path.
        if isinstance(result, IRInteger):
            assert result.value == 258
        elif isinstance(result, IRRational):
            assert Fraction(result.numer, result.denom) == Fraction(258)
        else:
            raise AssertionError(f"unexpected result type: {result!r}")

    def test_k_times_2_to_k_symbolic_closed_form(self):
        """Bypass the numeric path: feed Gosper a symbolic upper bound.

        With ``hi = N`` symbolic, the numeric small-range path can't
        fire — so a non-``Sum`` result means Gosper produced the
        closed form.  We verify the closed form at N = 1, 2, 3, 5 by
        substitution.
        """
        f = _make_k_times_two_k()
        result = try_gosper_sum(f, _k, IRInteger(1), _N)
        assert result is not None, "Gosper should accept k·2^k"
        # result should be IR; not a Sum node.
        assert not (isinstance(result, IRApply) and result.head == SUM)
        # Verify against direct sums at small N.
        for N in (1, 2, 3, 5, 7):
            expected = sum(j * (2 ** j) for j in range(1, N + 1))
            val = _eval_at(result, _N, N)
            assert val == Fraction(expected), (
                f"Gosper mismatch at N={N}: got {val}, want {expected}"
            )

    def test_k_times_k_factorial_symbolic_closed_form(self):
        """``∑_{k=0}^{N} k·k! = (N+1)! − 1`` — verify against direct
        sums at small N values."""
        f = _make_k_times_kfact()
        result = try_gosper_sum(f, _k, IRInteger(0), _N)
        assert result is not None, "Gosper should accept k·k!"
        assert not (isinstance(result, IRApply) and result.head == SUM)
        for N in (0, 1, 2, 3, 4, 5):
            # k · k! for k = 0, 1, …, N
            import math
            expected = sum(j * math.factorial(j) for j in range(0, N + 1))
            val = _eval_at(result, _N, N)
            assert val == Fraction(expected), (
                f"Gosper mismatch at N={N}: got {val}, want {expected}"
            )

    def test_geometric_2_to_k(self):
        """``∑_{k=0}^{N} 2^k = 2^(N+1) − 1`` — geometric handler still
        works; we only need to confirm no regression.  Use a concrete
        N so the geometric path produces a numeric answer."""
        f = IRApply(POW, (IRInteger(2), _k))
        result = evaluate_sum(f, _k, IRInteger(0), IRInteger(5), _VM)
        # 2^6 - 1 = 63
        if isinstance(result, IRInteger):
            assert result.value == 63
        else:
            raise AssertionError(f"expected integer 63, got {result!r}")


class TestGosperFallThrough:
    def test_sin_summand_falls_through(self):
        """``∑ sin(k)`` is not hypergeometric — dispatcher falls through
        to the unevaluated ``Sum`` IR.  This is the key contract: when
        Gosper can't handle a shape, we must NOT lie about it.
        """
        f = IRApply(SIN, (_k,))
        result = evaluate_sum(f, _k, IRInteger(1), _N, _VM)
        assert isinstance(result, IRApply) and result.head == SUM

    def test_log_summand_falls_through(self):
        """``∑ log(k)`` shouldn't reach a Gosper closed form either."""
        from symbolic_ir import LOG
        f = IRApply(LOG, (_k,))
        result = evaluate_sum(f, _k, IRInteger(1), _N, _VM)
        assert isinstance(result, IRApply) and result.head == SUM


class TestGosperRegression:
    def test_faulhaber_still_works(self):
        """The Phase 25 Faulhaber handler (handles ``∑ k = N(N+1)/2``)
        must still take this case — Gosper sits as a *later* fallback,
        so adding Gosper should not affect the outcome here.
        """
        result = evaluate_sum(_k, _k, IRInteger(1), IRInteger(4), _VM)
        assert isinstance(result, IRInteger) and result.value == 10

    def test_constant_summand_unchanged(self):
        """``∑ 5 = 5·n`` — the constant handler still fires first."""
        result = evaluate_sum(IRInteger(5), _k, IRInteger(1), IRInteger(10), _VM)
        assert isinstance(result, IRInteger) and result.value == 50


# ---------------------------------------------------------------------------
# Lower-level sanity checks on the algorithmic pieces.
# ---------------------------------------------------------------------------


class TestGosperPieces:
    def test_decompose_k_times_2_to_k(self):
        f = _make_k_times_two_k()
        h = _decompose(f, _k)
        assert h is not None
        # Polynomial part should be just k → [0, 1] (coefficient list
        # encoding 0 + 1·k).
        assert h.poly == [Fraction(0), Fraction(1)]
        # One exponential factor, base 2, exponent = k.
        assert len(h.exp_factors) == 1
        base, exp_poly = h.exp_factors[0]
        assert base == Fraction(2)
        # Exponent k → [0, 1].
        assert exp_poly == [Fraction(0), Fraction(1)]

    def test_ratio_k_times_2_to_k(self):
        """a(k) = k · 2^k.  Then a(k+1)/a(k) = ((k+1)/k) · 2.
        So numer = 2·(k+1) = 2k + 2, denom = k.
        """
        f = _make_k_times_two_k()
        h = _decompose(f, _k)
        assert h is not None
        ratio = _hyp_ratio(h)
        assert ratio is not None
        numer, denom = ratio
        # numer should be 2 + 2k.
        assert numer == [Fraction(2), Fraction(2)]
        # denom should be k = 0 + k.
        assert denom == [Fraction(0), Fraction(1)]
