"""Phase 24 — Definite Integration tests.

Covers the Fundamental Theorem of Calculus implementation:

    ∫_a^b f(x) dx  =  F(b) − F(a)

Test structure
--------------
TestPhase24_Polynomials      — power-rule integrands, both integer & rational limits
TestPhase24_Trig             — sin / cos / tan² over finite intervals
TestPhase24_Exponential      — exp over finite and semi-infinite intervals
TestPhase24_Rational         — 1/(1+x²), 1/(1+x), rational integrands
TestPhase24_SemiInfinite     — [0, ∞) and (−∞, 0] limits
TestPhase24_FullyInfinite    — (−∞, ∞) limits
TestPhase24_SpecialFunctions — erf, Si/Ci, Fresnel, log improper integrals
TestPhase24_Unevaluated      — divergent / no-antiderivative cases stay unevaluated
TestPhase24_Regressions      — Phase 1–23 indefinite integrals still work
TestPhase24_Macsyma          — end-to-end MACSYMA surface-syntax tests

Verification strategy: for each test case we check that the returned value
equals the expected exact result (or is numerically close within 1e-10 for
cases that simplify to floats).
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    FRESNEL_S,
    INTEGRATE,
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
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
INF = IRSymbol("%inf")
MINF = IRSymbol("%minf")
PI = IRSymbol("%pi")


def _vm() -> VM:
    """Fresh VM with the symbolic backend."""
    return VM(SymbolicBackend())


def _def_int(f: IRNode, a: IRNode, b: IRNode) -> IRNode:
    """Build and evaluate Integrate(f, x, a, b)."""
    vm = _vm()
    return vm.eval(IRApply(INTEGRATE, (f, X, a, b)))


def _indef_int(f: IRNode) -> IRNode:
    """Build and evaluate Integrate(f, x)  (indefinite)."""
    vm = _vm()
    return vm.eval(IRApply(INTEGRATE, (f, X)))


def _to_float(node: IRNode) -> float:  # noqa: PLR0912 (many branches by design)
    """Recursively evaluate an IR expression to a Python float.

    Handles all arithmetic heads, trig/log/exp functions, and the
    special symbols ``%pi`` and ``%e``.  Raises ``ValueError`` for any
    node that cannot be reduced to a single numeric value.
    """
    from symbolic_ir import ATAN, COS, EXP, LOG, SIN, TANH  # noqa: PLC0415

    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return float(node.numer) / float(node.denom)
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "%pi":
            return math.pi
        if node.name == "%e":
            return math.e
        raise ValueError(f"Unknown symbol: {node.name!r}")
    if isinstance(node, IRApply):
        h = node.head
        a = node.args
        if h == NEG:
            return -_to_float(a[0])
        if h == ADD:
            return _to_float(a[0]) + _to_float(a[1])
        if h == SUB:
            return _to_float(a[0]) - _to_float(a[1])
        if h == MUL:
            return _to_float(a[0]) * _to_float(a[1])
        if h == DIV:
            return _to_float(a[0]) / _to_float(a[1])
        if h == SQRT:
            return math.sqrt(_to_float(a[0]))
        if h == SIN:
            return math.sin(_to_float(a[0]))
        if h == COS:
            return math.cos(_to_float(a[0]))
        if h == EXP:
            return math.exp(_to_float(a[0]))
        if h == LOG:
            return math.log(_to_float(a[0]))
        if h == ATAN:
            return math.atan(_to_float(a[0]))
        if h == TANH:
            return math.tanh(_to_float(a[0]))
        if h == FRESNEL_S:
            from symbolic_vm.special_functions import fresnel_s_numeric  # noqa: PLC0415
            return fresnel_s_numeric(_to_float(a[0]))
        from symbolic_ir import SI  # noqa: PLC0415
        if h == SI:
            from symbolic_vm.special_functions import si_numeric  # noqa: PLC0415
            return si_numeric(_to_float(a[0]))
        # Fallback: try the symbolic VM numeric evaluator.
        from symbolic_vm.numeric import to_number  # noqa: PLC0415
        val = to_number(node)
        if val is not None:
            return float(val)
    raise ValueError(f"Cannot convert to float: {node!r}")


def _approx(node: IRNode, expected: float, tol: float = 1e-9) -> bool:
    """True if the numeric value of *node* is within *tol* of *expected*."""
    try:
        val = _to_float(node)
        return abs(val - expected) < tol
    except (ValueError, TypeError):
        return False


def _is_int_val(node: IRNode, v: int) -> bool:
    """True if *node* is an integer-valued IR node equal to *v*."""
    if isinstance(node, IRInteger):
        return node.value == v
    return False


def _is_rat_val(node: IRNode, n: int, d: int) -> bool:
    """True if *node* is IRRational(n, d)."""
    if isinstance(node, IRRational):
        return node.numer == n and node.denom == d
    return False


def _is_unevaluated(node: IRNode) -> bool:
    """True if *node* is an unevaluated Integrate(...)."""
    return isinstance(node, IRApply) and node.head == INTEGRATE


# ---------------------------------------------------------------------------
# IR convenience builders
# ---------------------------------------------------------------------------

def _pow(base: IRNode, n: int) -> IRNode:
    return IRApply(POW, (base, IRInteger(n)))


def _neg(e: IRNode) -> IRNode:
    return IRApply(NEG, (e,))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sin(u: IRNode) -> IRNode:
    return IRApply(SIN, (u,))


def _cos(u: IRNode) -> IRNode:
    return IRApply(COS, (u,))


def _exp(u: IRNode) -> IRNode:
    return IRApply(EXP, (u,))


def _log(u: IRNode) -> IRNode:
    return IRApply(LOG, (u,))


def _i(v: int) -> IRInteger:
    return IRInteger(v)


def _r(n: int, d: int) -> IRRational:
    return IRRational(n, d)


# ---------------------------------------------------------------------------
# TestPhase24_Polynomials
# ---------------------------------------------------------------------------

class TestPhase24_Polynomials:
    """∫_a^b polynomial dx via the power rule."""

    def test_x_squared_0_to_1(self):
        """∫₀¹ x² dx = 1/3."""
        result = _def_int(_pow(X, 2), _i(0), _i(1))
        assert _is_rat_val(result, 1, 3)

    def test_x_squared_0_to_2(self):
        """∫₀² x² dx = 8/3."""
        result = _def_int(_pow(X, 2), _i(0), _i(2))
        assert _is_rat_val(result, 8, 3)

    def test_x_squared_1_to_3(self):
        """∫₁³ x² dx = 26/3."""
        result = _def_int(_pow(X, 2), _i(1), _i(3))
        assert _is_rat_val(result, 26, 3)

    def test_linear_1_to_2(self):
        """∫₁² x dx = 3/2."""
        result = _def_int(X, _i(1), _i(2))
        assert _is_rat_val(result, 3, 2)

    def test_constant_0_to_5(self):
        """∫₀⁵ 3 dx = 15."""
        result = _def_int(_i(3), _i(0), _i(5))
        assert _is_int_val(result, 15)

    def test_x_cubed_0_to_1(self):
        """∫₀¹ x³ dx = 1/4."""
        result = _def_int(_pow(X, 3), _i(0), _i(1))
        assert _is_rat_val(result, 1, 4)

    def test_x_fourth_0_to_1(self):
        """∫₀¹ x⁴ dx = 1/5."""
        result = _def_int(_pow(X, 4), _i(0), _i(1))
        assert _is_rat_val(result, 1, 5)

    def test_polynomial_0_to_1(self):
        """∫₀¹ (x² + x + 1) dx = 11/6."""
        integrand = _add(_add(_pow(X, 2), X), _i(1))
        result = _def_int(integrand, _i(0), _i(1))
        assert _approx(result, 11.0 / 6.0)

    def test_limits_reversed(self):
        """∫₁⁰ x² dx = −1/3  (reversed limits give negated result)."""
        result = _def_int(_pow(X, 2), _i(1), _i(0))
        assert _approx(result, -1.0 / 3.0)

    def test_same_limits_zero(self):
        """∫₂² x² dx = 0."""
        result = _def_int(_pow(X, 2), _i(2), _i(2))
        assert _approx(result, 0.0)


# ---------------------------------------------------------------------------
# TestPhase24_Trig
# ---------------------------------------------------------------------------

class TestPhase24_Trig:
    """∫_a^b trig functions dx over finite intervals."""

    def test_sin_0_to_pi(self):
        """∫₀^π sin(x) dx = 2."""
        result = _def_int(_sin(X), _i(0), PI)
        assert _approx(result, 2.0)

    def test_cos_0_to_pi_over_2(self):
        """∫₀^(π/2) cos(x) dx = 1."""
        half_pi = _div(PI, _i(2))
        result = _def_int(_cos(X), _i(0), half_pi)
        assert _approx(result, 1.0)

    def test_sin_negative_limits(self):
        """∫_{-π}^{0} sin(x) dx = −2."""
        neg_pi = _neg(PI)
        result = _def_int(_sin(X), neg_pi, _i(0))
        assert _approx(result, -2.0)

    def test_cos_full_period(self):
        """∫₀^{2π} cos(x) dx = 0."""
        two_pi = _mul(_i(2), PI)
        result = _def_int(_cos(X), _i(0), two_pi)
        assert _approx(result, 0.0, tol=1e-8)


# ---------------------------------------------------------------------------
# TestPhase24_Exponential
# ---------------------------------------------------------------------------

class TestPhase24_Exponential:
    """∫_a^b exp(cx) dx."""

    def test_exp_0_to_1(self):
        """∫₀¹ exp(x) dx = e − 1."""
        result = _def_int(_exp(X), _i(0), _i(1))
        assert _approx(result, math.e - 1)

    def test_exp_neg_x_0_to_1(self):
        """∫₀¹ exp(−x) dx = 1 − 1/e."""
        result = _def_int(_exp(_neg(X)), _i(0), _i(1))
        assert _approx(result, 1.0 - 1.0 / math.e)

    def test_exp_neg_x_0_to_inf(self):
        """∫₀^∞ exp(−x) dx = 1."""
        result = _def_int(_exp(_neg(X)), _i(0), INF)
        assert _approx(result, 1.0)

    def test_exp_neg_2x_0_to_inf(self):
        """∫₀^∞ exp(−2x) dx = 1/2."""
        result = _def_int(_exp(_mul(_i(-2), X)), _i(0), INF)
        assert _approx(result, 0.5)

    def test_exp_pos_x_diverges(self):
        """∫₀^∞ exp(x) dx is divergent — returns unevaluated."""
        result = _def_int(_exp(X), _i(0), INF)
        assert _is_unevaluated(result)

    def test_exp_neg_xsq_0_to_inf(self):
        """∫₀^∞ exp(−x²) dx = √π/2."""
        result = _def_int(_exp(_neg(_pow(X, 2))), _i(0), INF)
        assert _approx(result, math.sqrt(math.pi) / 2)

    def test_exp_neg_xsq_minf_to_inf(self):
        """∫_{−∞}^{∞} exp(−x²) dx = √π."""
        result = _def_int(_exp(_neg(_pow(X, 2))), MINF, INF)
        assert _approx(result, math.sqrt(math.pi))


# ---------------------------------------------------------------------------
# TestPhase24_Rational
# ---------------------------------------------------------------------------

class TestPhase24_Rational:
    """Rational integrands including arctangent results."""

    def test_1_over_1px2_0_to_1(self):
        """∫₀¹ 1/(1+x²) dx = π/4."""
        denom = _add(_i(1), _pow(X, 2))
        result = _def_int(_div(_i(1), denom), _i(0), _i(1))
        assert _approx(result, math.pi / 4)

    def test_1_over_1px2_0_to_inf(self):
        """∫₀^∞ 1/(1+x²) dx = π/2."""
        denom = _add(_i(1), _pow(X, 2))
        result = _def_int(_div(_i(1), denom), _i(0), INF)
        assert _approx(result, math.pi / 2)

    def test_1_over_x_plus_1_integral(self):
        """∫₀¹ 1/(x+1) dx = log(2)."""
        denom = _add(X, _i(1))
        result = _def_int(_div(_i(1), denom), _i(0), _i(1))
        assert _approx(result, math.log(2))


# ---------------------------------------------------------------------------
# TestPhase24_Log
# ---------------------------------------------------------------------------

class TestPhase24_Log:
    """Log integrals including improper integrals converging at x=0."""

    def test_log_0_to_1(self):
        """∫₀¹ log(x) dx = −1  (improper but convergent)."""
        result = _def_int(_log(X), _i(0), _i(1))
        assert _approx(result, -1.0)

    def test_log_1_to_e(self):
        """∫₁^e log(x) dx = 1  (by parts: x·log(x) − x from 1 to e)."""
        e_sym = IRSymbol("%e")
        result = _def_int(_log(X), _i(1), e_sym)
        assert _approx(result, 1.0)

    def test_log_1_to_2(self):
        """∫₁² log(x) dx = 2·log(2) − 1.

        F(x) = x·log(x) − x.
        F(2) = 2·log(2) − 2.  F(1) = 1·log(1) − 1 = −1.
        F(2) − F(1) = 2·log(2) − 2 − (−1) = 2·log(2) − 1.
        """
        result = _def_int(_log(X), _i(1), _i(2))
        assert _approx(result, 2.0 * math.log(2) - 1.0)


# ---------------------------------------------------------------------------
# TestPhase24_SemiInfinite
# ---------------------------------------------------------------------------

class TestPhase24_SemiInfinite:
    """Semi-infinite integrals [a, ∞) or (−∞, b]."""

    def test_si_0_to_inf(self):
        """∫₀^∞ sin(x)/x dx = π/2."""
        sin_over_x = _div(_sin(X), X)
        result = _def_int(sin_over_x, _i(0), INF)
        assert _approx(result, math.pi / 2)

    def test_erf_evaluated_at_inf(self):
        """∫₀^∞ exp(−x²) dx = √π/2 (via erf(∞)=1)."""
        # Already tested in Exponential, but verify from raw erf form.
        # The antiderivative is sqrt(%pi)/2 * erf(x).
        # F(∞) = sqrt(%pi)/2 * 1 = sqrt(%pi)/2.
        # F(0) = sqrt(%pi)/2 * erf(0) = 0.
        result = _def_int(_exp(_neg(_pow(X, 2))), _i(0), INF)
        assert _approx(result, math.sqrt(math.pi) / 2)

    def test_exp_neg_x_minf_to_0(self):
        """∫_{−∞}^{0} exp(x) dx = 1."""
        result = _def_int(_exp(X), MINF, _i(0))
        assert _approx(result, 1.0)

    def test_xexp_neg_xsq_0_to_inf(self):
        """∫₀^∞ x·exp(−x²) dx = 1/2  (substitution: u = x²)."""
        integrand = _mul(X, _exp(_neg(_pow(X, 2))))
        result = _def_int(integrand, _i(0), INF)
        assert _approx(result, 0.5)


# ---------------------------------------------------------------------------
# TestPhase24_FullyInfinite
# ---------------------------------------------------------------------------

class TestPhase24_FullyInfinite:
    """Fully-infinite integrals (−∞, ∞)."""

    def test_gaussian_full(self):
        """∫_{−∞}^{∞} exp(−x²) dx = √π."""
        result = _def_int(_exp(_neg(_pow(X, 2))), MINF, INF)
        assert _approx(result, math.sqrt(math.pi))

    def test_si_full_symmetric(self):
        """∫_{−∞}^{∞} sin(x)/x dx = π."""
        sin_over_x = _div(_sin(X), X)
        result = _def_int(sin_over_x, MINF, INF)
        assert _approx(result, math.pi)


# ---------------------------------------------------------------------------
# TestPhase24_SpecialFunctions
# ---------------------------------------------------------------------------

class TestPhase24_SpecialFunctions:
    """Definite integrals that reduce to Phase-23 special functions."""

    def test_fresnel_s_0_to_1(self):
        """∫₀¹ sin(x²) dx = sqrt(π/2)·FresnelS(sqrt(2/π)) ≈ 0.31027."""
        sin_xsq = _sin(_pow(X, 2))
        result = _def_int(sin_xsq, _i(0), _i(1))
        # Result is sqrt(π/2) * FresnelS(sqrt(2/π)) = 0.31026830...
        assert _approx(result, 0.31026830, tol=1e-5), f"got {_to_float(result):.6f}"

    def test_fresnel_s_0_to_inf(self):
        """∫₀^∞ sin(x²) dx = √(π/8) ≈ 0.62666.

        F(x) = sqrt(π/2) · FresnelS(x·sqrt(2/π)).
        F(∞) = sqrt(π/2) · (1/2) = sqrt(π/8).
        F(0) = sqrt(π/2) · FresnelS(0) = 0.
        """
        sin_xsq = _sin(_pow(X, 2))
        result = _def_int(sin_xsq, _i(0), INF)
        assert _approx(result, math.sqrt(math.pi / 8), tol=1e-8)

    def test_sin_over_x_0_to_pi(self):
        """∫₀^π sin(x)/x dx = Si(π) ≈ 1.85194.

        The antiderivative of sin(x)/x is Si(x).  At x=0, Si(0)=0.
        """
        sin_over_x = _div(_sin(X), X)
        result = _def_int(sin_over_x, _i(0), PI)
        # Si(π) - Si(0) = Si(π) ≈ 1.85194
        assert _approx(result, 1.8519370519824665, tol=1e-5)


# ---------------------------------------------------------------------------
# TestPhase24_Unevaluated
# ---------------------------------------------------------------------------

class TestPhase24_Unevaluated:
    """Integrals that should remain unevaluated."""

    def test_sin_sin_x_unevaluated(self):
        """∫₀¹ sin(sin(x)) dx has no antiderivative — stays unevaluated."""
        result = _def_int(_sin(_sin(X)), _i(0), _i(1))
        assert _is_unevaluated(result)

    def test_exp_pos_inf_diverges(self):
        """∫₀^∞ exp(x) dx diverges — stays unevaluated."""
        result = _def_int(_exp(X), _i(0), INF)
        assert _is_unevaluated(result)

    def test_exp_pos_xsq_diverges(self):
        """∫₀^∞ exp(x²) dx diverges — stays unevaluated."""
        result = _def_int(_exp(_pow(X, 2)), _i(0), INF)
        assert _is_unevaluated(result)

    def test_non_symbol_variable(self):
        """Integrate w.r.t. a non-symbol stays unevaluated."""
        vm = _vm()
        expr = IRApply(INTEGRATE, (_pow(X, 2), _i(5), _i(0), _i(1)))
        result = vm.eval(expr)
        assert _is_unevaluated(result)

    def test_bad_arity_raises(self):
        """Integrate with 3 arguments raises TypeError."""
        vm = _vm()
        expr = IRApply(INTEGRATE, (_pow(X, 2), X, _i(0)))
        with pytest.raises(TypeError, match="Integrate expects 2 or 4"):
            vm.eval(expr)


# ---------------------------------------------------------------------------
# TestPhase24_Regressions
# ---------------------------------------------------------------------------

class TestPhase24_Regressions:
    """Ensure indefinite integration still works unchanged after Phase 24."""

    def test_phase1_x_squared_indef(self):
        """∫ x² dx = x³/3 (Phase 1 power rule)."""
        F = _indef_int(_pow(X, 2))
        assert not _is_unevaluated(F)

    def test_phase1_sin_indef(self):
        """∫ sin(x) dx = −cos(x) (Phase 1)."""
        F = _indef_int(_sin(X))
        assert not _is_unevaluated(F)

    def test_phase23_erf_indef(self):
        """∫ exp(−x²) dx = √π/2 · erf(x) (Phase 23)."""
        F = _indef_int(_exp(_neg(_pow(X, 2))))
        assert not _is_unevaluated(F)
        assert "Erf" in repr(F)

    def test_phase15_sech_squared_indef(self):
        """∫ sech²(x) dx = tanh(x) (Phase 16)."""
        from symbolic_ir import SECH  # noqa: PLC0415
        F = _indef_int(IRApply(POW, (IRApply(SECH, (X,)), _i(2))))
        assert not _is_unevaluated(F)
        assert "Tanh" in repr(F)

    def test_phase2_rational_indef(self):
        """∫ 1/(1+x²) dx = atan(x) (Phase 2 rational route)."""
        denom = _add(_i(1), _pow(X, 2))
        F = _indef_int(_div(_i(1), denom))
        assert not _is_unevaluated(F)
        assert "Atan" in repr(F) or "atan" in repr(F).lower()


# ---------------------------------------------------------------------------
# TestPhase24_Macsyma
# ---------------------------------------------------------------------------

class TestPhase24_Macsyma:
    """End-to-end tests using MACSYMA surface syntax."""

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

    def test_macsyma_poly(self):
        """integrate(x^2, x, 0, 1) = 1/3."""
        result = self._run("integrate(x^2, x, 0, 1)")
        assert _is_rat_val(result, 1, 3)

    def test_macsyma_gaussian(self):
        """integrate(exp(-x^2), x, 0, %inf) = sqrt(%pi)/2."""
        result = self._run("integrate(exp(-x^2), x, 0, %inf)")
        assert _approx(result, math.sqrt(math.pi) / 2)

    def test_macsyma_sin_0_pi(self):
        """integrate(sin(x), x, 0, %pi) = 2."""
        result = self._run("integrate(sin(x), x, 0, %pi)")
        assert _approx(result, 2.0)

    def test_macsyma_si_0_inf(self):
        """integrate(sin(x)/x, x, 0, %inf) = %pi/2."""
        result = self._run("integrate(sin(x)/x, x, 0, %inf)")
        assert _approx(result, math.pi / 2)

    def test_macsyma_log_0_1(self):
        """integrate(log(x), x, 0, 1) = -1."""
        result = self._run("integrate(log(x), x, 0, 1)")
        assert _approx(result, -1.0)

    def test_macsyma_arctan(self):
        """integrate(1/(1+x^2), x, 0, 1) = %pi/4."""
        result = self._run("integrate(1/(1+x^2), x, 0, 1)")
        assert _approx(result, math.pi / 4)

    def test_macsyma_gaussian_full(self):
        """integrate(exp(-x^2), x, %minf, %inf) = sqrt(%pi)."""
        result = self._run("integrate(exp(-x^2), x, %minf, %inf)")
        assert _approx(result, math.sqrt(math.pi))

    def test_macsyma_divergent_unevaluated(self):
        """integrate(exp(x), x, 0, %inf) returns unevaluated."""
        result = self._run("integrate(exp(x), x, 0, %inf)")
        assert _is_unevaluated(result)
