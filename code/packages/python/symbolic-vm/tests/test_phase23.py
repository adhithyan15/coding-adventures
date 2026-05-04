"""Phase 23 integration tests — special functions as integration fallback.

Covers five special-function families introduced in Phase 23:

  23a — Error functions: erf, erfc, erfi
  23b — Trigonometric integrals: Si, Ci, Shi, Chi
  23c — Dilogarithm: Li₂
  23d — Gamma and Beta functions
  23e — Fresnel integrals: FresnelS, FresnelC

Verification strategy:
  - Integration tests: antiderivative F(x) satisfies F'(x) ≈ f(x) numerically.
  - Differentiation tests: check the derivative IR structure.
  - Handler tests: check exact-result special values and numeric evaluation.
  - Regression tests: previous phases unaffected.
  - MACSYMA e2e tests: full parse → compile → eval pipeline.
"""

from __future__ import annotations

import math

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from macsyma_runtime.name_table import extend_compiler_name_table
from symbolic_ir import (
    BETA_FUNC,
    CHI,
    CI,
    COS,
    COSH,
    DIV,
    ERF,
    ERFI,
    EXP,
    FRESNEL_C,
    FRESNEL_S,
    GAMMA_FUNC,
    INTEGRATE,
    LI2,
    LOG,
    MUL,
    NEG,
    POW,
    SHI,
    SI,
    SIN,
    SINH,
    SQRT,
    SUB,
    D,
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
# Extend the MACSYMA compiler name table so erf, gamma, etc. are recognised
# in the e2e tests.  This is the same call that macsyma-runtime makes at REPL
# startup.
# ---------------------------------------------------------------------------
extend_compiler_name_table(_STANDARD_FUNCTIONS)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

X = IRSymbol("x")
_PI_SYM = IRSymbol("%pi")

ONE = IRInteger(1)
TWO = IRInteger(2)

# Test points strictly away from special values.
_TP = (0.5, 1.2)
_TP_POS = (0.3, 0.8, 1.5)  # for Si/Ci (all positive, away from 0)
_TP_LI2 = (0.2, 0.6)       # for Li₂ (well inside (0,1))


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911, PLR0912
    """Numerically evaluate an IR tree at x = x_val, including special fns."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "x":
            return x_val
        if node.name == "%pi":
            return math.pi
        raise ValueError(f"Unknown symbol: {node.name!r}")
    if not isinstance(node, IRApply):
        raise TypeError(f"Unexpected node: {node!r}")
    head = node.head.name

    # Arithmetic
    if head == "Add":
        return _eval_ir(node.args[0], x_val) + _eval_ir(node.args[1], x_val)
    if head == "Sub":
        return _eval_ir(node.args[0], x_val) - _eval_ir(node.args[1], x_val)
    if head == "Mul":
        return _eval_ir(node.args[0], x_val) * _eval_ir(node.args[1], x_val)
    if head == "Div":
        return _eval_ir(node.args[0], x_val) / _eval_ir(node.args[1], x_val)
    if head == "Neg":
        return -_eval_ir(node.args[0], x_val)
    if head == "Inv":
        return 1.0 / _eval_ir(node.args[0], x_val)
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Sqrt":
        v = _eval_ir(node.args[0], x_val)
        return math.sqrt(abs(v))
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val)))

    # Trig / hyp
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val))
    if head == "Sinh":
        return math.sinh(_eval_ir(node.args[0], x_val))
    if head == "Cosh":
        return math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Tanh":
        return math.tanh(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))

    # Phase 23 special functions
    if head == "Erf":
        return math.erf(_eval_ir(node.args[0], x_val))
    if head == "Erfi":
        from symbolic_vm.special_functions import erfi_numeric
        return erfi_numeric(_eval_ir(node.args[0], x_val))
    if head == "Si":
        from symbolic_vm.special_functions import si_numeric
        return si_numeric(_eval_ir(node.args[0], x_val))
    if head == "Ci":
        from symbolic_vm.special_functions import ci_numeric
        return ci_numeric(_eval_ir(node.args[0], x_val))
    if head == "Shi":
        from symbolic_vm.special_functions import shi_numeric
        return shi_numeric(_eval_ir(node.args[0], x_val))
    if head == "Chi":
        from symbolic_vm.special_functions import chi_numeric
        return chi_numeric(_eval_ir(node.args[0], x_val))
    if head == "Li2":
        from symbolic_vm.special_functions import li2_numeric
        return li2_numeric(_eval_ir(node.args[0], x_val))
    if head == "FresnelS":
        from symbolic_vm.special_functions import fresnel_s_numeric
        return fresnel_s_numeric(_eval_ir(node.args[0], x_val))
    if head == "FresnelC":
        from symbolic_vm.special_functions import fresnel_c_numeric
        return fresnel_c_numeric(_eval_ir(node.args[0], x_val))

    raise ValueError(f"Unhandled head: {head!r}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    """Central-difference derivative of IR node at x_val."""
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _check_antiderivative(
    integrand: IRNode,
    antideriv: IRNode,
    test_points: tuple[float, ...] = _TP,
    atol: float = 1e-5,
    rtol: float = 1e-5,
) -> None:
    """Verify F'(x) ≈ f(x) numerically at each test point."""
    for x_val in test_points:
        expected = _eval_ir(integrand, x_val)
        actual = _numerical_deriv(antideriv, x_val)
        tol = atol + rtol * abs(expected)
        assert abs(actual - expected) < tol, (
            f"At x={x_val}: F'={actual:.8f}, f={expected:.8f}, "
            f"diff={abs(actual - expected):.2e}"
        )


def _integrate_ir(vm: VM, integrand: IRNode) -> IRNode:
    return vm.eval(IRApply(INTEGRATE, (integrand, X)))


def _was_evaluated(f: IRNode, F: IRNode) -> None:
    assert IRApply(INTEGRATE, (f, X)) != F, (
        "Expected a closed-form antiderivative, got unevaluated Integrate"
    )


def _is_unevaluated(f: IRNode, F: IRNode) -> None:
    assert IRApply(INTEGRATE, (f, X)) == F, (
        "Expected unevaluated Integrate, got a closed form"
    )


def _run_macsyma(src: str) -> IRNode:
    """Full parse → compile → VM.eval pipeline (symbolic mode)."""
    stmts = compile_macsyma(parse_macsyma(src))
    return VM(SymbolicBackend()).eval_program(stmts)


# ---------------------------------------------------------------------------
# 23a — Error function integration
# ---------------------------------------------------------------------------


class TestPhase23_ErfIntegral:
    """∫ exp(c·x²) dx → erf or erfi form."""

    def test_exp_neg_x_sq_gives_erf(self) -> None:
        """∫ exp(−x²) dx = √π/2 · erf(x)."""
        vm = _make_vm()
        f = IRApply(EXP, (IRApply(NEG, (IRApply(POW, (X, TWO)),)),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_exp_neg_x_sq_structure(self) -> None:
        """Result head contains Erf."""
        vm = _make_vm()
        f = IRApply(EXP, (IRApply(NEG, (IRApply(POW, (X, TWO)),)),))
        F = _integrate_ir(vm, f)
        src = repr(F)
        assert "Erf" in src

    def test_exp_neg_4x_sq_gives_erf_2x(self) -> None:
        """∫ exp(−4x²) dx = √π/4 · erf(2x)."""
        vm = _make_vm()
        f = IRApply(
            EXP,
            (IRApply(MUL, (IRInteger(-4), IRApply(POW, (X, TWO)))),),
        )
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_exp_pos_x_sq_gives_erfi(self) -> None:
        """∫ exp(x²) dx = √π/2 · erfi(x)."""
        vm = _make_vm()
        f = IRApply(EXP, (IRApply(POW, (X, TWO)),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        # Verify via IR structure: must contain Erfi
        assert "Erfi" in repr(F)

    def test_exp_neg_2x_sq_is_antiderivative(self) -> None:
        """∫ exp(−2x²) dx — verify F'=f numerically (coeff involves √2)."""
        vm = _make_vm()
        f = IRApply(
            EXP,
            (IRApply(MUL, (IRInteger(-2), IRApply(POW, (X, TWO)))),),
        )
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP, atol=1e-5)

    def test_exp_neg_rational_coeff_antiderivative(self) -> None:
        """∫ exp(−x²/4) dx (c = −1/4)."""
        vm = _make_vm()
        c = IRRational(-1, 4)
        f = IRApply(EXP, (IRApply(MUL, (c, IRApply(POW, (X, TWO)))),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP, atol=1e-5)

    def test_exp_non_quadratic_unevaluated(self) -> None:
        """∫ exp(x) dx does NOT trigger the erf pattern."""
        vm = _make_vm()
        f = IRApply(EXP, (X,))
        F = _integrate_ir(vm, f)
        # Should give exp(x), not erf form
        assert IRApply(INTEGRATE, (f, X)) != F  # _was_ evaluated (by Phase 3)
        assert "Erf" not in repr(F)

    def test_macsyma_integrate_exp_neg_x_sq(self) -> None:
        """End-to-end MACSYMA: integrate(exp(-x^2), x) → erf form."""
        result = _run_macsyma("integrate(exp(-x^2), x);")
        assert "Erf" in repr(result)

    def test_macsyma_erf_zero(self) -> None:
        """erf(0) = 0."""
        result = _run_macsyma("erf(0);")
        assert result == IRInteger(0)

    def test_macsyma_erf_numeric(self) -> None:
        """erf(1.0) ≈ 0.842."""
        result = _run_macsyma("erf(1.0);")
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.erf(1.0)) < 1e-10


# ---------------------------------------------------------------------------
# 23b — Trigonometric integral integration
# ---------------------------------------------------------------------------


class TestPhase23_SiCiIntegral:
    """∫ trig(ax)/x dx → Si/Ci/Shi/Chi(ax)."""

    def test_sin_over_x_gives_si(self) -> None:
        """∫ sin(x)/x dx = Si(x)."""
        vm = _make_vm()
        f = IRApply(DIV, (IRApply(SIN, (X,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(SI, (X,)) == F

    def test_cos_over_x_gives_ci(self) -> None:
        """∫ cos(x)/x dx = Ci(x)."""
        vm = _make_vm()
        f = IRApply(DIV, (IRApply(COS, (X,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(CI, (X,)) == F

    def test_sinh_over_x_gives_shi(self) -> None:
        """∫ sinh(x)/x dx = Shi(x)."""
        vm = _make_vm()
        f = IRApply(DIV, (IRApply(SINH, (X,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(SHI, (X,)) == F

    def test_cosh_over_x_gives_chi(self) -> None:
        """∫ cosh(x)/x dx = Chi(x)."""
        vm = _make_vm()
        f = IRApply(DIV, (IRApply(COSH, (X,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(CHI, (X,)) == F

    def test_sin_2x_over_x_gives_si_2x(self) -> None:
        """∫ sin(2x)/x dx = Si(2x)."""
        vm = _make_vm()
        two_x = IRApply(MUL, (TWO, X))
        f = IRApply(DIV, (IRApply(SIN, (two_x,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(SI, (two_x,)) == F

    def test_si_is_antiderivative_of_sin_over_x(self) -> None:
        """Numerically verify d/dx Si(x) = sin(x)/x."""
        integrand = IRApply(DIV, (IRApply(SIN, (X,)), X))
        antideriv = IRApply(SI, (X,))
        _check_antiderivative(integrand, antideriv, test_points=_TP_POS)

    def test_ci_is_antiderivative_of_cos_over_x(self) -> None:
        """Numerically verify d/dx Ci(x) = cos(x)/x."""
        integrand = IRApply(DIV, (IRApply(COS, (X,)), X))
        antideriv = IRApply(CI, (X,))
        _check_antiderivative(integrand, antideriv, test_points=_TP_POS)

    def test_si_2x_is_antiderivative(self) -> None:
        """d/dx Si(2x) = sin(2x)/x."""
        two_x = IRApply(MUL, (TWO, X))
        integrand = IRApply(DIV, (IRApply(SIN, (two_x,)), X))
        antideriv = IRApply(SI, (two_x,))
        _check_antiderivative(integrand, antideriv, test_points=_TP_POS)

    def test_macsyma_integrate_sin_x_over_x(self) -> None:
        """End-to-end: integrate(sin(x)/x, x) → si(x)."""
        result = _run_macsyma("integrate(sin(x)/x, x);")
        assert result == IRApply(SI, (X,))

    def test_macsyma_si_zero(self) -> None:
        """si(0) = 0."""
        result = _run_macsyma("si(0);")
        assert result == IRInteger(0)


# ---------------------------------------------------------------------------
# 23c — Dilogarithm integration
# ---------------------------------------------------------------------------


class TestPhase23_Li2Integral:
    """∫ log(1−x)/x dx → −Li₂(x);  ∫ log(x)/(1−x) dx → Li₂(1−x)."""

    def test_log_1_minus_x_over_x_gives_neg_li2(self) -> None:
        """∫ log(1−x)/x dx = −Li₂(x)."""
        vm = _make_vm()
        one_minus_x = IRApply(SUB, (ONE, X))
        f = IRApply(DIV, (IRApply(LOG, (one_minus_x,)), X))
        F = _integrate_ir(vm, f)
        assert IRApply(NEG, (IRApply(LI2, (X,)),)) == F

    def test_log_x_over_1_minus_x_gives_li2_1_minus_x(self) -> None:
        """∫ log(x)/(1−x) dx = Li₂(1−x)."""
        vm = _make_vm()
        one_minus_x = IRApply(SUB, (ONE, X))
        f = IRApply(DIV, (IRApply(LOG, (X,)), one_minus_x))
        F = _integrate_ir(vm, f)
        assert IRApply(LI2, (one_minus_x,)) == F

    def test_neg_li2_is_antiderivative_of_log_1_minus_x_over_x(self) -> None:
        """Numerically verify d/dx(−Li₂(x)) = log(1−x)/x."""
        one_minus_x = IRApply(SUB, (ONE, X))
        integrand = IRApply(DIV, (IRApply(LOG, (one_minus_x,)), X))
        antideriv = IRApply(NEG, (IRApply(LI2, (X,)),))
        _check_antiderivative(integrand, antideriv, test_points=_TP_LI2)

    def test_li2_1_minus_x_is_antiderivative(self) -> None:
        """Numerically verify d/dx Li₂(1−x) = log(x)/(1−x)."""
        one_minus_x = IRApply(SUB, (ONE, X))
        integrand = IRApply(DIV, (IRApply(LOG, (X,)), one_minus_x))
        antideriv = IRApply(LI2, (one_minus_x,))
        _check_antiderivative(integrand, antideriv, test_points=_TP_LI2)

    def test_macsyma_integrate_log_1_minus_x_over_x(self) -> None:
        """End-to-end: integrate(log(1-x)/x, x) → −li2(x)."""
        result = _run_macsyma("integrate(log(1-x)/x, x);")
        assert "Li2" in repr(result)
        assert "Neg" in repr(result)

    def test_li2_zero_is_zero(self) -> None:
        """Li₂(0) = 0."""
        vm = _make_vm()
        result = vm.eval(IRApply(LI2, (IRInteger(0),)))
        assert result == IRInteger(0)

    def test_li2_one_is_pi_sq_over_6(self) -> None:
        """Li₂(1) = π²/6."""
        vm = _make_vm()
        result = vm.eval(IRApply(LI2, (IRInteger(1),)))
        assert "Pow" in repr(result) or "pi" in repr(result).lower()

    def test_log_x_sq_unevaluated(self) -> None:
        """∫ log(x²)/x dx does NOT match the Li₂ pattern."""
        vm = _make_vm()
        x_sq = IRApply(POW, (X, TWO))
        f = IRApply(DIV, (IRApply(LOG, (x_sq,)), X))
        F = _integrate_ir(vm, f)
        # No Li₂ — should be unevaluated or elementary
        assert "Li2" not in repr(F)


# ---------------------------------------------------------------------------
# 23d — Gamma and Beta functions
# ---------------------------------------------------------------------------


class TestPhase23_GammaBeta:
    """Exact evaluation of Γ and B at integer and half-integer arguments."""

    def test_gamma_1(self) -> None:
        """Γ(1) = 0! = 1."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (ONE,)))
        assert result == IRInteger(1)

    def test_gamma_2(self) -> None:
        """Γ(2) = 1! = 1."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (TWO,)))
        assert result == IRInteger(1)

    def test_gamma_5(self) -> None:
        """Γ(5) = 4! = 24."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRInteger(5),)))
        assert result == IRInteger(24)

    def test_gamma_6(self) -> None:
        """Γ(6) = 5! = 120."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRInteger(6),)))
        assert result == IRInteger(120)

    def test_gamma_half(self) -> None:
        """Γ(1/2) = √π."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRRational(1, 2),)))
        assert result == IRApply(SQRT, (_PI_SYM,))

    def test_gamma_three_halves(self) -> None:
        """Γ(3/2) = √π/2."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRRational(3, 2),)))
        # Should be (1/2)*sqrt(pi)
        r = repr(result)
        assert "Sqrt" in r and "pi" in r

    def test_gamma_five_halves(self) -> None:
        """Γ(5/2) = 3√π/4."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRRational(5, 2),)))
        r = repr(result)
        assert "Sqrt" in r and "pi" in r

    def test_gamma_float(self) -> None:
        """Γ(2.5) ≈ 1.329 (Lanczos approximation)."""
        vm = _make_vm()
        result = vm.eval(IRApply(GAMMA_FUNC, (IRFloat(2.5),)))
        assert isinstance(result, IRFloat)
        import math
        assert abs(result.value - math.gamma(2.5)) < 1e-6

    def test_beta_half_half_is_pi(self) -> None:
        """B(1/2, 1/2) = π."""
        vm = _make_vm()
        result = vm.eval(
            IRApply(BETA_FUNC, (IRRational(1, 2), IRRational(1, 2)))
        )
        assert result == _PI_SYM

    def test_beta_2_3(self) -> None:
        """B(2, 3) = Γ(2)·Γ(3)/Γ(5) = 1·2/24 = 1/12."""
        vm = _make_vm()
        result = vm.eval(IRApply(BETA_FUNC, (TWO, IRInteger(3))))
        # 1/12
        if isinstance(result, IRRational):
            assert result.numer == 1 and result.denom == 12
        elif isinstance(result, IRFloat):
            assert abs(result.value - 1 / 12) < 1e-8
        else:
            pytest.fail(f"Unexpected type: {result!r}")

    def test_macsyma_gamma_5(self) -> None:
        """End-to-end: gamma(5) = 24."""
        result = _run_macsyma("gamma(5);")
        assert result == IRInteger(24)

    def test_macsyma_gamma_half(self) -> None:
        """End-to-end: gamma(1/2) = sqrt(%pi)."""
        result = _run_macsyma("gamma(1/2);")
        assert result == IRApply(SQRT, (_PI_SYM,))

    def test_macsyma_beta_half_half(self) -> None:
        """End-to-end: beta(1/2, 1/2) = %pi."""
        result = _run_macsyma("beta(1/2, 1/2);")
        assert result == _PI_SYM


# ---------------------------------------------------------------------------
# 23e — Fresnel integral integration
# ---------------------------------------------------------------------------


class TestPhase23_FresnelIntegral:
    """∫ sin/cos(q·π·x²) dx → scaled FresnelS/FresnelC."""

    def test_sin_pi_x_sq_over_2_gives_fresnel_s(self) -> None:
        """∫ sin(π·x²/2) dx = FresnelS(x)."""
        vm = _make_vm()
        # sin(%pi*x^2/2) — argument = MUL(%pi, DIV(POW(x,2), 2))
        arg = IRApply(
            MUL,
            (_PI_SYM, IRApply(DIV, (IRApply(POW, (X, TWO)), TWO))),
        )
        f = IRApply(SIN, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(FRESNEL_S, (X,)) == F

    def test_cos_pi_x_sq_over_2_gives_fresnel_c(self) -> None:
        """∫ cos(π·x²/2) dx = FresnelC(x)."""
        vm = _make_vm()
        arg = IRApply(
            MUL,
            (_PI_SYM, IRApply(DIV, (IRApply(POW, (X, TWO)), TWO))),
        )
        f = IRApply(COS, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(FRESNEL_C, (X,)) == F

    def test_fresnel_s_is_antiderivative(self) -> None:
        """d/dx FresnelS(x) = sin(π·x²/2)."""
        arg = IRApply(
            MUL,
            (_PI_SYM, IRApply(DIV, (IRApply(POW, (X, TWO)), TWO))),
        )
        integrand = IRApply(SIN, (arg,))
        antideriv = IRApply(FRESNEL_S, (X,))
        _check_antiderivative(integrand, antideriv, test_points=_TP)

    def test_fresnel_c_is_antiderivative(self) -> None:
        """d/dx FresnelC(x) = cos(π·x²/2)."""
        arg = IRApply(
            MUL,
            (_PI_SYM, IRApply(DIV, (IRApply(POW, (X, TWO)), TWO))),
        )
        integrand = IRApply(COS, (arg,))
        antideriv = IRApply(FRESNEL_C, (X,))
        _check_antiderivative(integrand, antideriv, test_points=_TP)

    def test_rational_quadratic_sin_gives_scaled_fresnel_s(self) -> None:
        """∫ sin(x²) dx = √(π/2)·FresnelS(x·√(2/π)) — F'=f numerically."""
        vm = _make_vm()
        f = IRApply(SIN, (IRApply(POW, (X, TWO)),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        assert "FresnelS" in repr(F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_rational_quadratic_cos_gives_scaled_fresnel_c(self) -> None:
        """∫ cos(x²) dx = √(π/2)·FresnelC(x·√(2/π)) — F'=f numerically."""
        vm = _make_vm()
        f = IRApply(COS, (IRApply(POW, (X, TWO)),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        assert "FresnelC" in repr(F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_fresnel_s_zero(self) -> None:
        """FresnelS(0) = 0."""
        vm = _make_vm()
        result = vm.eval(IRApply(FRESNEL_S, (IRInteger(0),)))
        assert result == IRInteger(0)

    def test_fresnel_c_zero(self) -> None:
        """FresnelC(0) = 0."""
        vm = _make_vm()
        result = vm.eval(IRApply(FRESNEL_C, (IRInteger(0),)))
        assert result == IRInteger(0)

    def test_macsyma_fresnel_s_via_integration(self) -> None:
        """End-to-end: integrate(sin(%pi*x^2/2), x) → fresnel_s(x)."""
        result = _run_macsyma("integrate(sin(%pi*x^2/2), x);")
        assert result == IRApply(FRESNEL_S, (X,))

    def test_macsyma_fresnel_c_via_integration(self) -> None:
        """End-to-end: integrate(cos(%pi*x^2/2), x) → fresnel_c(x)."""
        result = _run_macsyma("integrate(cos(%pi*x^2/2), x);")
        assert result == IRApply(FRESNEL_C, (X,))


# ---------------------------------------------------------------------------
# Differentiation rules for special functions
# ---------------------------------------------------------------------------


class TestPhase23_Differentiation:
    """d/dx of erf, Si, Ci, Li₂, FresnelS, FresnelC."""

    def _diff(self, vm: VM, f_node: IRNode) -> IRNode:
        return vm.eval(IRApply(D, (f_node, X)))

    def test_diff_erf_x(self) -> None:
        """d/dx erf(x) = (2/√π)·exp(−x²)."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(ERF, (X,)))
        r = repr(result)
        assert "Exp" in r and "Sqrt" in r

    def test_diff_erf_x_is_correct_numerically(self) -> None:
        """Verify the derivative of erf(x) equals (2/√π)·exp(−x²)."""
        vm = _make_vm()
        diff_expr = self._diff(vm, IRApply(ERF, (X,)))
        for x_val in _TP:
            expected = 2.0 / math.sqrt(math.pi) * math.exp(-(x_val ** 2))
            actual = _eval_ir(diff_expr, x_val)
            assert abs(actual - expected) < 1e-8, (
                f"At x={x_val}: expected {expected}, got {actual}"
            )

    def test_diff_erfi_x(self) -> None:
        """d/dx erfi(x) = (2/√π)·exp(x²)."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(ERFI, (X,)))
        r = repr(result)
        assert "Exp" in r

    def test_diff_si_x(self) -> None:
        """d/dx Si(x) = sin(x)/x."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(SI, (X,)))
        # Should be sin(x)/x
        assert result == IRApply(DIV, (IRApply(SIN, (X,)), X))

    def test_diff_ci_x(self) -> None:
        """d/dx Ci(x) = cos(x)/x."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(CI, (X,)))
        assert result == IRApply(DIV, (IRApply(COS, (X,)), X))

    def test_diff_li2_x(self) -> None:
        """d/dx Li₂(x) = −log(1−x)/x."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(LI2, (X,)))
        r = repr(result)
        assert "Log" in r and "Neg" in r

    def test_diff_fresnel_s_x(self) -> None:
        """d/dx FresnelS(x) = sin(π·x²/2)."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(FRESNEL_S, (X,)))
        r = repr(result)
        assert "Sin" in r and "pi" in r.lower()

    def test_diff_fresnel_c_x(self) -> None:
        """d/dx FresnelC(x) = cos(π·x²/2)."""
        vm = _make_vm()
        result = self._diff(vm, IRApply(FRESNEL_C, (X,)))
        r = repr(result)
        assert "Cos" in r and "pi" in r.lower()

    def test_diff_erf_chain_rule(self) -> None:
        """d/dx erf(x²) = (2/√π)·exp(−x⁴)·2x  (chain rule)."""
        vm = _make_vm()
        f = IRApply(ERF, (IRApply(POW, (X, TWO)),))
        result = self._diff(vm, f)
        # Verify numerically
        for x_val in _TP:
            expected = (
                4.0 * x_val / math.sqrt(math.pi) * math.exp(-(x_val ** 4))
            )
            actual = _eval_ir(result, x_val)
            assert abs(actual - expected) < 1e-7, (
                f"Chain rule d/dx erf(x²) at x={x_val}: {actual} ≠ {expected}"
            )

    def test_diff_si_chain_rule(self) -> None:
        """d/dx Si(2x) = sin(2x)/x."""
        vm = _make_vm()
        two_x = IRApply(MUL, (TWO, X))
        f = IRApply(SI, (two_x,))
        result = self._diff(vm, f)
        # Verify numerically
        for x_val in _TP_POS:
            expected = math.sin(2 * x_val) / x_val
            actual = _eval_ir(result, x_val)
            assert abs(actual - expected) < 1e-7, (
                f"d/dx Si(2x) at x={x_val}: {actual} ≠ {expected}"
            )

    def test_macsyma_diff_erf(self) -> None:
        """End-to-end: diff(erf(x), x) = (2/sqrt(%pi))*exp(-x^2)."""
        result = _run_macsyma("diff(erf(x), x);")
        r = repr(result)
        assert "Exp" in r
        assert "Sqrt" in r


# ---------------------------------------------------------------------------
# Fallthrough / regression tests
# ---------------------------------------------------------------------------


class TestPhase23_Regressions:
    """Verify no regressions in earlier phases."""

    def test_phase3_exp_linear_still_works(self) -> None:
        """Phase 3: ∫ exp(2x) dx = exp(2x)/2."""
        vm = _make_vm()
        two_x = IRApply(MUL, (TWO, X))
        f = IRApply(EXP, (two_x,))
        F = _integrate_ir(vm, f)
        # Must not be erf — it's a linear exponential
        assert "Erf" not in repr(F)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase1_sin_still_works(self) -> None:
        """Phase 1: ∫ sin(x) dx = −cos(x)."""
        vm = _make_vm()
        f = IRApply(SIN, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase15_sech_still_works(self) -> None:
        """Phase 15: ∫ sech(x) dx evaluates (not unevaluated)."""
        from symbolic_ir import SECH
        vm = _make_vm()
        f = IRApply(SECH, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)

    def test_sin_x_squared_unevaluated(self) -> None:
        """∫ sin(x)^2 is NOT a Fresnel pattern — no x² in trig arg."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), TWO))
        F = _integrate_ir(vm, f)
        assert "Fresnel" not in repr(F)

    def test_rational_log_div_not_li2(self) -> None:
        """∫ log(x)/x dx — primary check is that Phase 23 does NOT misidentify
        this as a Li₂ pattern (which would be wrong).  The integral log(x)·x⁻¹
        is not yet handled by any elementary rule so the result may be
        unevaluated; that is acceptable here.
        """
        vm = _make_vm()
        f = IRApply(DIV, (IRApply(LOG, (X,)), X))
        F = _integrate_ir(vm, f)
        assert "Li2" not in repr(F)

    def test_phase22_pattern_matching_unaffected(self) -> None:
        """Phase 22 matchdeclare/defrule still works after Phase 23 additions."""
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("u"),)))
        u = IRSymbol("u")
        lhs = IRApply(ERF, (u,))  # defrule using an erf head
        vm.eval(IRApply(IRSymbol("Defrule"), (
            IRSymbol("my_rule"), lhs, IRInteger(42),
        )))
        # Apply the rule to erf(x) — should fire and return 42
        target = IRApply(ERF, (X,))
        result = vm.eval(
            IRApply(IRSymbol("Apply1"), (IRSymbol("my_rule"), target))
        )
        assert result == IRInteger(42)
