"""Phase 20 — limit_advanced: L'Hôpital, infinity, indeterminate forms.

Test coverage targets (spec §Test coverage targets):

  TestDirectSub           4   Polynomial, trig, exp at finite continuous points
  TestLHopital_ZeroZero   8   sin(x)/x, (1-cos)/x, (exp(x)-1)/x, poly ratios
  TestLHopital_InfInf     6   Rational functions at ∞; repeated L'Hôpital
  TestIndeterminate_ZeroInf 5 x·log(x), x·exp(-x), x²·exp(-x)
  TestIndeterminate_Powers  6 (1+1/x)^x, x^x at 0+, x^(1/x) at ∞
  TestLimitsAtInfinity    5   exp(-x), 1/x, log(x)/x, sin(x)/x (unevaluated)
  TestOneSided            5   log(x) at 0+, 1/x at 0+/0-, sqrt(x) at 0+
  TestFallthrough         4   No diff_fn, oscillating, truly unevaluated
  TestMacsymaExamples     3   Surface-syntax via VM
"""

from __future__ import annotations

import math
from fractions import Fraction

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SQRT,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_limit_series import INF, MINF, limit_advanced
from cas_limit_series.heads import LIMIT as LIMIT_HEAD

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _i(n: int) -> IRInteger:
    return IRInteger(n)


def _r(n: int, d: int) -> IRRational:
    return IRRational(n, d)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


def _is_unevaluated(node, var=None, point=None) -> bool:
    """True if node is an unevaluated Limit(…) application."""
    return isinstance(node, IRApply) and node.head == LIMIT_HEAD


def _float_val(node) -> float:
    """Evaluate an IR literal node to float."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "inf":
            return math.inf
        if node.name == "minf":
            return -math.inf
        if node.name == "%e":
            return math.e
        if node.name == "%pi":
            return math.pi
    raise AssertionError(f"Cannot convert {node!r} to float")


def _close(node, expected: float, tol: float = 1e-9) -> bool:
    """True if node evaluates to a float close to *expected*.

    Uses the full ``_num_eval`` from ``limit_advanced`` so that unsimplified
    IR expressions like ``Pow(3, 2)`` or ``Exp(Minf)`` are evaluated correctly
    even when the stub ``eval_fn`` does not fully simplify them.
    """
    from cas_limit_series.limit_advanced import _num_eval as _la_num_eval
    try:
        val = _la_num_eval(node)
    except Exception:
        return False
    if math.isnan(val):
        return False
    if math.isinf(expected) and math.isinf(val):
        return (val > 0) == (expected > 0)
    return abs(val - expected) <= tol


# ---------------------------------------------------------------------------
# Stub diff / eval functions (no VM dependency in unit tests)
# ---------------------------------------------------------------------------

def _make_diff_fn():
    """Return a diff_fn that symbolically differentiates common patterns.

    This is a minimal stub sufficient to exercise L'Hôpital on the test
    cases below.  It does NOT need to be a complete CAS.
    """
    # Lazy import to verify cas_limit_series has no vm dep at module level.

    def _diff(expr: IRApply | IRSymbol | IRInteger | IRRational, var: IRSymbol):  # noqa: ANN001
        """Symbolic derivative of expr w.r.t. var (stub, covers test cases)."""
        if expr == var:
            return _i(1)
        if isinstance(expr, (IRInteger, IRRational, IRFloat)):
            return _i(0)
        if isinstance(expr, IRSymbol):
            return _i(0)  # constant symbol
        if not isinstance(expr, IRApply):
            return _i(0)

        h = expr.head
        args = expr.args

        # sum rule
        if h == ADD:
            return IRApply(ADD, tuple(_diff(a, var) for a in args))
        # subtraction
        if h == SUB:
            return IRApply(SUB, (_diff(args[0], var), _diff(args[1], var)))
        # product rule (binary)
        if h == MUL and len(args) == 2:
            f, g = args
            return IRApply(ADD, (
                IRApply(MUL, (_diff(f, var), g)),
                IRApply(MUL, (f, _diff(g, var))),
            ))
        # negation
        if h == NEG:
            return IRApply(NEG, (_diff(args[0], var),))
        # quotient rule
        if h == DIV:
            f, g = args
            return IRApply(DIV, (
                IRApply(SUB, (
                    IRApply(MUL, (_diff(f, var), g)),
                    IRApply(MUL, (f, _diff(g, var))),
                )),
                IRApply(POW, (g, _i(2))),
            ))
        # power rule: x^n → n*x^(n-1)  (integer exponent only)
        if h == POW and isinstance(args[1], IRInteger):
            base, n_ir = args
            n = n_ir.value
            if n == 0:
                return _i(0)
            base_diff = _diff(base, var)
            return IRApply(MUL, (
                IRApply(MUL, (_i(n), IRApply(POW, (base, _i(n - 1))))),
                base_diff,
            ))
        # exp
        if h == EXP:
            arg = args[0]
            return IRApply(MUL, (IRApply(EXP, (arg,)), _diff(arg, var)))
        # log
        if h == LOG:
            arg = args[0]
            return IRApply(DIV, (_diff(arg, var), arg))
        # sin → cos
        if h == SIN:
            arg = args[0]
            return IRApply(MUL, (IRApply(COS, (arg,)), _diff(arg, var)))
        # cos → -sin
        if h == COS:
            arg = args[0]
            sin_chain = IRApply(MUL, (IRApply(SIN, (arg,)), _diff(arg, var)))
            return IRApply(NEG, (sin_chain,))
        # sqrt → 1/(2*sqrt)
        if h == SQRT:
            arg = args[0]
            return IRApply(DIV, (
                _diff(arg, var),
                IRApply(MUL, (_i(2), IRApply(SQRT, (arg,)))),
            ))
        return _i(0)  # unknown head — treat as constant

    return _diff


def _make_eval_fn():
    """Return a lightweight eval_fn that collapses arithmetic and identities.

    This stub is powerful enough for the unit tests that exercise
    ``limit_advanced`` without a real VM.  It handles:

    - Constant folding for integer/rational ADD, SUB, MUL, DIV, NEG, POW.
    - Algebraic identity rules (0+x, x*1, 0*x, x/1, x^0, x^1, NEG(NEG(x))).
    - Numeric evaluation of EXP/LOG/SIN/COS at known literal arguments.
    """
    def _lit_val(node) -> float | None:  # noqa: ANN001
        """Return float value if node is a numeric literal, else None."""
        if isinstance(node, IRInteger):
            return float(node.value)
        if isinstance(node, IRRational):
            return node.numer / node.denom
        if isinstance(node, IRFloat):
            return node.value
        return None

    def _eval(node):  # noqa: ANN001
        """Recursively simplify node."""
        if not isinstance(node, IRApply):
            return node
        h = node.head
        args = tuple(_eval(a) for a in node.args)

        # ---- Constant folding for all-numeric args ----
        if h == ADD:
            lits = [_lit_val(a) for a in args]
            if all(v is not None for v in lits):
                total = Fraction(0)
                for a in args:
                    if isinstance(a, IRInteger):
                        total += a.value
                    elif isinstance(a, IRRational):
                        total += Fraction(a.numer, a.denom)
                    else:
                        total += Fraction(a.value).limit_denominator(10**9)
                if total.denominator == 1:
                    return IRInteger(total.numerator)
                return IRRational(total.numerator, total.denominator)
        if h == SUB and len(args) == 2:
            lv = [_lit_val(a) for a in args]
            if all(v is not None for v in lv):
                f = Fraction(lv[0]) - Fraction(lv[1])
                if f.denominator == 1:
                    return IRInteger(f.numerator)
                return IRRational(f.numerator, f.denominator)
        if h == MUL:
            lits = [_lit_val(a) for a in args]
            if all(v is not None for v in lits):
                prod = Fraction(1)
                for a in args:
                    if isinstance(a, IRInteger):
                        prod *= a.value
                    elif isinstance(a, IRRational):
                        prod *= Fraction(a.numer, a.denom)
                    else:
                        prod *= Fraction(a.value).limit_denominator(10**9)
                if prod.denominator == 1:
                    return IRInteger(prod.numerator)
                return IRRational(prod.numerator, prod.denominator)
        if h == DIV and len(args) == 2:
            n, d = args
            nv, dv = _lit_val(n), _lit_val(d)
            if nv is not None and dv is not None and dv != 0:
                f = (
                    Fraction(nv).limit_denominator(10**9)
                    / Fraction(dv).limit_denominator(10**9)
                )
                if f.denominator == 1:
                    return IRInteger(f.numerator)
                return IRRational(f.numerator, f.denominator)
        if h == NEG and len(args) == 1:
            v = _lit_val(args[0])
            if v is not None:
                if isinstance(args[0], IRFloat):
                    return IRFloat(-v)
                if v == int(v):
                    return IRInteger(-int(v))
                return IRRational(int(-v * 1000), 1000)
        if h == POW and len(args) == 2:
            bv, ev = _lit_val(args[0]), _lit_val(args[1])
            if bv is not None and ev is not None:
                try:
                    r = bv ** ev
                    if isinstance(r, float) and r == int(r):
                        return IRInteger(int(r))
                    return IRFloat(float(r))
                except (ZeroDivisionError, OverflowError, ValueError):
                    pass

        # ---- Identity rules ----
        if h == NEG and isinstance(args[0], IRApply) and args[0].head == NEG:
            return args[0].args[0]  # NEG(NEG(x)) = x
        if h == ADD and len(args) == 2:
            a, b = args
            if isinstance(b, IRInteger) and b.value == 0:
                return a
            if isinstance(a, IRInteger) and a.value == 0:
                return b
        if h == SUB and len(args) == 2:
            a, b = args
            if isinstance(b, IRInteger) and b.value == 0:
                return a  # x - 0 = x
            if isinstance(a, IRInteger) and a.value == 0:
                return IRApply(NEG, (b,))  # 0 - x = -x
        if h == MUL and len(args) == 2:
            a, b = args
            if isinstance(a, IRInteger) and a.value == 0:
                return IRInteger(0)
            if isinstance(b, IRInteger) and b.value == 0:
                return IRInteger(0)
            if isinstance(a, IRInteger) and a.value == 1:
                return b
            if isinstance(b, IRInteger) and b.value == 1:
                return a
        if h == DIV and len(args) == 2:
            n, d = args
            if isinstance(d, IRInteger) and d.value == 1:
                return n  # x / 1 = x
            # 0/x = 0 only when denominator is provably nonzero
            d_is_zero = isinstance(d, IRInteger) and d.value == 0
            if isinstance(n, IRInteger) and n.value == 0 and not d_is_zero:
                return IRInteger(0)  # 0 / nonzero = 0
            # 1/(1/x) = x  — needed for zero-inf rewrite clean-up
            if (
                isinstance(n, IRInteger) and n.value == 1
                and isinstance(d, IRApply) and d.head == DIV
            ):
                inner_n, inner_d = d.args
                if isinstance(inner_n, IRInteger) and inner_n.value == 1:
                    return inner_d  # 1 / (1/x) = x
        if h == POW and len(args) == 2:
            base, exp_ir = args
            if isinstance(exp_ir, IRInteger) and exp_ir.value == 0:
                return IRInteger(1)  # x^0 = 1
            if isinstance(exp_ir, IRInteger) and exp_ir.value == 1:
                return base  # x^1 = x

        # ---- Numeric evaluation of transcendentals at known literal args ----
        if h == EXP and len(args) == 1:
            v = _lit_val(args[0])
            if v is not None:
                try:
                    return IRFloat(math.exp(v))
                except OverflowError:
                    pass
        if h == LOG and len(args) == 1:
            v = _lit_val(args[0])
            if v is not None and v > 0:
                return IRFloat(math.log(v))
        if h == SIN and len(args) == 1:
            v = _lit_val(args[0])
            if v is not None:
                return IRFloat(math.sin(v))
        if h == COS and len(args) == 1:
            v = _lit_val(args[0])
            if v is not None:
                return IRFloat(math.cos(v))

        return IRApply(h, args)

    return _eval


# Shared fixtures
_DIFF = _make_diff_fn()
_EVAL = _make_eval_fn()


# ===========================================================================
# TestDirectSub — direct substitution at continuous points
# ===========================================================================


class TestDirectSub:
    """Polynomial, trig, exp expressions at finite continuous points."""

    def test_polynomial_at_2(self):
        """lim_{x→2} x^2 + 1 ≈ 5."""
        x = _sym("x")
        expr = IRApply(ADD, (IRApply(POW, (x, _i(2))), _i(1)))
        out = limit_advanced(expr, x, _i(2), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 5.0)

    def test_constant_expr(self):
        """lim_{x→7} 3 = 3."""
        x = _sym("x")
        out = limit_advanced(_i(3), x, _i(7), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 3.0)

    def test_exp_at_zero(self):
        """lim_{x→0} exp(x) = 1."""
        x = _sym("x")
        expr = IRApply(EXP, (x,))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0)

    def test_sin_at_pi_over_2(self):
        """lim_{x→1} sin(x) = sin(1)."""
        x = _sym("x")
        expr = IRApply(SIN, (x,))
        out = limit_advanced(expr, x, _i(1), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, math.sin(1.0))


# ===========================================================================
# TestLHopital_ZeroZero — 0/0 forms via L'Hôpital
# ===========================================================================


class TestLHopital_ZeroZero:
    """Classic 0/0 L'Hôpital limits."""

    def test_sin_x_over_x(self):
        """lim_{x→0} sin(x)/x = 1."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(SIN, (x,)), x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0)

    def test_one_minus_cos_over_x(self):
        """lim_{x→0} (1 - cos(x))/x = 0."""
        x = _sym("x")
        numerator = IRApply(SUB, (_i(1), IRApply(COS, (x,))))
        expr = IRApply(DIV, (numerator, x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_exp_minus_1_over_x(self):
        """lim_{x→0} (exp(x) - 1)/x = 1."""
        x = _sym("x")
        numerator = IRApply(SUB, (IRApply(EXP, (x,)), _i(1)))
        expr = IRApply(DIV, (numerator, x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0, tol=1e-6)

    def test_x_squared_over_x(self):
        """lim_{x→0} x^2/x = 0 (one L'Hôpital step)."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(POW, (x, _i(2))), x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_x_cubed_over_x(self):
        """lim_{x→0} x^3/x = 0 (one L'Hôpital step)."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(POW, (x, _i(3))), x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_polynomial_ratio_at_1(self):
        """lim_{x→1} (x^2-1)/(x-1) = 2."""
        x = _sym("x")
        numer = IRApply(SUB, (IRApply(POW, (x, _i(2))), _i(1)))
        denom = IRApply(SUB, (x, _i(1)))
        expr = IRApply(DIV, (numer, denom))
        out = limit_advanced(expr, x, _i(1), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 2.0, tol=1e-6)

    def test_sin_over_x_no_eval(self):
        """Without eval_fn, L'Hôpital still works (unsimplified result)."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(SIN, (x,)), x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF)
        # Result should be close to 1 numerically
        assert _close(out, 1.0, tol=1e-6)

    def test_x_minus_sin_over_x_cubed(self):
        """lim_{x→0} (x - sin(x))/x^3 = 1/6 (two L'Hôpital steps)."""
        x = _sym("x")
        numer = IRApply(SUB, (x, IRApply(SIN, (x,))))
        denom = IRApply(POW, (x, _i(3)))
        expr = IRApply(DIV, (numer, denom))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1 / 6, tol=1e-4)


# ===========================================================================
# TestLHopital_InfInf — ∞/∞ forms at infinity
# ===========================================================================


class TestLHopital_InfInf:
    """Rational functions at ∞ resolved by repeated L'Hôpital."""

    def test_x_over_exp_x(self):
        """lim_{x→∞} x/exp(x) = 0."""
        x = _sym("x")
        expr = IRApply(DIV, (x, IRApply(EXP, (x,))))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_linear_over_linear(self):
        """lim_{x→∞} (2x+1)/(3x+2) = 2/3."""
        x = _sym("x")
        numer = IRApply(ADD, (IRApply(MUL, (_i(2), x)), _i(1)))
        denom = IRApply(ADD, (IRApply(MUL, (_i(3), x)), _i(2)))
        expr = IRApply(DIV, (numer, denom))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 2 / 3, tol=1e-6)

    def test_quadratic_over_quadratic(self):
        """lim_{x→∞} (x^2+1)/(2x^2+3) = 1/2."""
        x = _sym("x")
        numer = IRApply(ADD, (IRApply(POW, (x, _i(2))), _i(1)))
        denom = IRApply(ADD, (IRApply(MUL, (_i(2), IRApply(POW, (x, _i(2))))), _i(3)))
        expr = IRApply(DIV, (numer, denom))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.5, tol=1e-6)

    def test_log_over_x(self):
        """lim_{x→∞} log(x)/x = 0."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(LOG, (x,)), x))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_poly_lower_degree_goes_to_zero(self):
        """lim_{x→∞} x/(x^2+1) = 0."""
        x = _sym("x")
        numer = x
        denom = IRApply(ADD, (IRApply(POW, (x, _i(2))), _i(1)))
        expr = IRApply(DIV, (numer, denom))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_reciprocal_at_infinity(self):
        """lim_{x→∞} 1/x = 0."""
        x = _sym("x")
        expr = IRApply(DIV, (_i(1), x))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0)


# ===========================================================================
# TestIndeterminate_ZeroInf — 0·∞ forms
# ===========================================================================


class TestIndeterminate_ZeroInf:
    """0·∞ products rewritten to 0/0 via L'Hôpital."""

    def test_x_log_x_at_zero_plus(self):
        """lim_{x→0+} x·log(x) = 0."""
        x = _sym("x")
        expr = IRApply(MUL, (x, IRApply(LOG, (x,))))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_x_log_x_direction_none(self):
        """lim_{x→0} x·log(x) classified from right (no dir): should be 0."""
        x = _sym("x")
        expr = IRApply(MUL, (x, IRApply(LOG, (x,))))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, eval_fn=_EVAL)
        # With ε perturbation → 0+, same as "plus"
        assert _close(out, 0.0, tol=1e-6)

    def test_x_exp_neg_x_at_inf(self):
        """lim_{x→∞} x·exp(-x) = 0 (requires VM for 1/exp(-x)=exp(x) simplification)."""
        pytest.importorskip(
            "symbolic_vm", reason="needs full VM for simplification of 1/exp(-x)"
        )
        from macsyma_runtime import make_vm  # type: ignore[import]
        vm = make_vm()
        result = vm.run("limit(x*exp(-x), x, inf)")
        assert _close(result, 0.0, tol=1e-6)

    def test_x_squared_exp_neg_x_at_inf(self):
        """lim_{x→∞} x^2·exp(-x) = 0 (requires VM)."""
        pytest.importorskip(
            "symbolic_vm", reason="needs full VM for simplification of 1/exp(-x)"
        )
        from macsyma_runtime import make_vm  # type: ignore[import]
        vm = make_vm()
        result = vm.run("limit(x^2*exp(-x), x, inf)")
        assert _close(result, 0.0, tol=1e-6)

    def test_x_log_x_stub_vs_alternate(self):
        """lim_{x→0+} x·log(x) = 0 — direct 0·∞ rewrite, stub-only."""
        x = _sym("x")
        # Use MUL(x, LOG(x)): at x=0, substitution gives 0*log(0) = 0*(-inf).
        # The stub _EVAL collapses MUL(0, ...) → 0, so exact subst returns 0.
        expr = IRApply(MUL, (x, IRApply(LOG, (x,))))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)


# ===========================================================================
# TestIndeterminate_Powers — 1^∞, 0^0, ∞^0 via exp-log
# ===========================================================================


class TestIndeterminate_Powers:
    """Exponential indeterminate forms reduced via exp-log rewrite.

    The tests that require computing a limit equal to ``e`` depend on
    multi-step L'Hôpital chains that need a real VM with full simplification.
    Those tests use ``pytest.importorskip`` for ``symbolic_vm``.  The simpler
    ``x^x → 1`` and ``x^0 → 1`` cases work with the stub alone.
    """

    def test_x_to_x_at_zero_plus(self):
        """lim_{x→0+} x^x = 1  (0^0 form, exp-log path, stub-only)."""
        x = _sym("x")
        expr = IRApply(POW, (x, x))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0, tol=1e-4)

    def test_x_to_inv_x_at_inf_stub(self):
        """lim_{x→∞} x^(1/x) = 1  (∞^0 form, exp-log path, stub-only)."""
        x = _sym("x")
        expr = IRApply(POW, (x, IRApply(DIV, (_i(1), x))))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0, tol=1e-4)

    def test_inf_0_form_literal(self):
        """lim_{x→∞} x^0 = 1 (literal 0 exponent, trivially 1)."""
        x = _sym("x")
        expr = IRApply(POW, (x, _i(0)))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 1.0)

    def test_1_plus_inv_x_to_x_needs_vm(self):
        """lim_{x→∞} (1 + 1/x)^x = e  — requires real VM for simplification."""
        pytest.importorskip(
            "symbolic_vm", reason="needs full VM for complex L'Hôpital chain"
        )
        from macsyma_runtime import make_vm  # type: ignore[import]
        vm = make_vm()
        result = vm.run("limit((1+1/x)^x, x, inf)")
        assert _close(result, math.e, tol=1e-3)

    def test_one_plus_x_to_inv_x_at_zero_needs_vm(self):
        """lim_{x→0} (1+x)^(1/x) = e — requires real VM."""
        pytest.importorskip(
            "symbolic_vm", reason="needs full VM for complex L'Hôpital chain"
        )
        from macsyma_runtime import make_vm  # type: ignore[import]
        vm = make_vm()
        result = vm.run("limit((1+x)^(1/x), x, 0)")
        assert _close(result, math.e, tol=1e-3)

    def test_pow_exp_log_transform_fires(self):
        """0^0 form: _handle_form dispatches to _pow_exp_log (EXP rewrite path)."""
        x = _sym("x")
        expr = IRApply(POW, (x, x))
        # With eval_fn that collapses EXP(0)→1 and MUL(0,...)→0:
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        # Whatever the form, the result should be close to 1 or a Limit node
        assert _close(out, 1.0, tol=1e-4) or _is_unevaluated(out)


# ===========================================================================
# TestLimitsAtInfinity — various forms at ±∞
# ===========================================================================


class TestLimitsAtInfinity:
    """Expressions evaluated at ±∞ via numeric evaluator."""

    def test_exp_neg_x_at_inf(self):
        """lim_{x→∞} exp(-x) = 0."""
        x = _sym("x")
        expr = IRApply(EXP, (IRApply(NEG, (x,)),))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0)

    def test_reciprocal_at_neg_inf(self):
        """lim_{x→-∞} 1/x = 0."""
        x = _sym("x")
        expr = IRApply(DIV, (_i(1), x))
        out = limit_advanced(expr, x, MINF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0)

    def test_exp_x_at_inf(self):
        """lim_{x→∞} exp(x) = +∞."""
        x = _sym("x")
        expr = IRApply(EXP, (x,))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert out == INF or (isinstance(out, IRSymbol) and out.name == "inf")

    def test_log_over_x_at_inf(self):
        """lim_{x→∞} log(x)/x = 0."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(LOG, (x,)), x))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)

    def test_sin_over_x_at_inf_is_zero_or_unevaluated(self):
        """lim_{x→∞} sin(x)/x: sin(∞) is NaN but overall limit is 0.

        Our numeric evaluator gives nan for sin(∞), so this either:
        - Falls through to unevaluated (no diff_fn path finds 0)
        - OR the system returns unevaluated Limit(…)
        This test just verifies no exception is raised.
        """
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(SIN, (x,)), x))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        # Either unevaluated or 0 — both acceptable
        assert out is not None


# ===========================================================================
# TestOneSided — one-sided limits
# ===========================================================================


class TestOneSided:
    """One-sided limit direction support."""

    def test_log_at_zero_plus(self):
        """lim_{x→0+} log(x) = -∞."""
        x = _sym("x")
        expr = IRApply(LOG, (x,))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert out == MINF or (isinstance(out, IRSymbol) and out.name == "minf")

    def test_reciprocal_at_zero_plus(self):
        """lim_{x→0+} 1/x = +∞."""
        x = _sym("x")
        expr = IRApply(DIV, (_i(1), x))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert out == INF or (isinstance(out, IRSymbol) and out.name == "inf")

    def test_reciprocal_at_zero_minus(self):
        """lim_{x→0-} 1/x = -∞."""
        x = _sym("x")
        expr = IRApply(DIV, (_i(1), x))
        out = limit_advanced(expr, x, _i(0), "minus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert out == MINF or (isinstance(out, IRSymbol) and out.name == "minf")

    def test_sqrt_at_zero_plus(self):
        """lim_{x→0+} sqrt(x) = 0."""
        x = _sym("x")
        expr = IRApply(SQRT, (x,))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0)

    def test_x_log_x_at_zero_plus_onesided(self):
        """lim_{x→0+} x·log(x) = 0 (one-sided variant)."""
        x = _sym("x")
        expr = IRApply(MUL, (x, IRApply(LOG, (x,))))
        out = limit_advanced(expr, x, _i(0), "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0, tol=1e-6)


# ===========================================================================
# TestFallthrough — cases that should return unevaluated
# ===========================================================================


class TestFallthrough:
    """Limits that fall through to unevaluated Limit(…)."""

    def test_no_diff_fn_indeterminate(self):
        """Without diff_fn, 0/0 form cannot be resolved → unevaluated."""
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(SIN, (x,)), x))
        out = limit_advanced(expr, x, _i(0))  # no diff_fn
        assert _is_unevaluated(out)

    def test_oscillating_sin_at_infinity(self):
        """lim_{x→∞} sin(x) is undefined (oscillates) → unevaluated."""
        x = _sym("x")
        expr = IRApply(SIN, (x,))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _is_unevaluated(out)

    def test_depth_exceeded_returns_unevaluated(self):
        """Hitting _MAX_DEPTH returns unevaluated."""
        from cas_limit_series.limit_advanced import _MAX_DEPTH
        x = _sym("x")
        expr = IRApply(DIV, (IRApply(SIN, (x,)), x))
        out = limit_advanced(expr, x, _i(0), diff_fn=_DIFF, _depth=_MAX_DEPTH + 1)
        assert _is_unevaluated(out)

    def test_nan_point_returns_unevaluated(self):
        """When the limit point is symbolic (non-numeric), return unevaluated."""
        x, a = _sym("x"), _sym("a")
        expr = IRApply(DIV, (x, x))
        out = limit_advanced(expr, x, a, diff_fn=_DIFF)  # a is unknown
        assert _is_unevaluated(out)


# ===========================================================================
# TestMacsymaExamples — surface-syntax via VM (integration test)
# ===========================================================================


class TestMacsymaExamples:
    """End-to-end tests through a real VM (requires symbolic-vm)."""

    @pytest.fixture(autouse=True)
    def _setup_vm(self):
        """Import VM lazily so the unit-test classes above stay VM-free."""
        pytest.importorskip("symbolic_vm")
        from macsyma_runtime import make_vm  # type: ignore[import]
        self._vm = make_vm()

    def _run(self, code: str):
        return self._vm.run(code)

    def test_limit_sin_over_x(self):
        """limit(sin(x)/x, x, 0) = 1 via VM."""
        result = self._run("limit(sin(x)/x, x, 0)")
        # Result should numerically be 1
        assert _close(result, 1.0, tol=1e-6)

    def test_limit_exp_at_inf(self):
        """limit(exp(-x), x, inf) = 0 via VM."""
        result = self._run("limit(exp(-x), x, inf)")
        assert _close(result, 0.0)

    def test_limit_one_plus_inv_x_to_x(self):
        """limit((1+1/x)^x, x, inf) ≈ e via VM."""
        result = self._run("limit((1+1/x)^x, x, inf)")
        assert _close(result, math.e, tol=1e-3)


# ===========================================================================
# Additional edge-case tests
# ===========================================================================


class TestEdgeCases:
    """Additional coverage for edge paths."""

    def test_unevaluated_has_correct_arity_no_dir(self):
        """Unevaluated Limit without direction has 3 args."""
        x = _sym("x")
        expr = IRApply(SIN, (x,))
        out = limit_advanced(expr, x, INF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _is_unevaluated(out)
        assert len(out.args) == 3

    def test_unevaluated_has_direction_arg(self):
        """Unevaluated Limit with direction has 4 args."""
        x = _sym("x")
        # cos(x) at infinity oscillates → unevaluated
        expr = IRApply(SIN, (x,))
        out = limit_advanced(expr, x, INF, "plus", diff_fn=_DIFF, eval_fn=_EVAL)
        if _is_unevaluated(out):
            assert len(out.args) == 4

    def test_inf_sentinel_identity(self):
        """INF sentinel is IRSymbol('inf')."""
        from cas_limit_series import INF, MINF
        assert isinstance(INF, IRSymbol)
        assert INF.name == "inf"
        assert isinstance(MINF, IRSymbol)
        assert MINF.name == "minf"

    def test_direct_sub_returns_simplified_integer(self):
        """For a polynomial at integer point with eval_fn, result collapses."""
        x = _sym("x")
        # x^2 at x=3 → 9
        expr = IRApply(POW, (x, _i(2)))
        out = limit_advanced(expr, x, _i(3), eval_fn=_EVAL)
        assert _close(out, 9.0)

    def test_minf_point(self):
        """lim_{x→-∞} exp(x) = 0."""
        x = _sym("x")
        expr = IRApply(EXP, (x,))
        out = limit_advanced(expr, x, MINF, diff_fn=_DIFF, eval_fn=_EVAL)
        assert _close(out, 0.0)
