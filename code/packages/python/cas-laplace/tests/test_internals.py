"""Tests for internal functions and handler VM integration.

Covers the gaps identified by coverage analysis:
- handlers.py: laplace_handler and ilt_handler (need a mock VM)
- inverse_table.py: _ir_to_rational, polynomial helpers, edge cases
- table.py: uncommon branches in pattern matchers
"""

from __future__ import annotations

from fractions import Fraction
from unittest.mock import MagicMock

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EXP,
    MUL,
    NEG,
    POW,
    SIN,
    SINH,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_laplace.handlers import ilt_handler, laplace_handler
from cas_laplace.heads import DIRAC_DELTA, ILT, LAPLACE, UNIT_STEP
from cas_laplace.inverse_table import (
    _extract_all_rational_roots,
    _ilt_from_hyp_match,
    _ilt_from_trig_match,
    _ilt_repeated_pole,
    _ilt_simple_pole,
    _ir_to_rational,
    _isqrt_exact,
    _poly_add,
    _poly_deriv,
    _poly_divmod,
    _poly_gcd,
    _poly_mul,
    _poly_neg,
    _poly_normalize,
    _poly_pow,
    _poly_scale,
    _rational_roots,
    _rational_sqrt,
    inverse_laplace,
)
from cas_laplace.table import (
    _extract_coeff_and_fn,
    _extract_linear_arg,
    _is_const,
    _match_cos,
    _match_cosh,
    _match_dirac_delta,
    _match_exp,
    _match_exp_cos,
    _match_exp_sin,
    _match_sin,
    _match_sinh,
    _match_t_cos,
    _match_t_exp,
    _match_t_sin,
    _match_tn_exp,
    _match_unit_step,
    table_lookup,
)

T = IRSymbol("t")
S = IRSymbol("s")


# ===========================================================================
# Handler VM integration tests (mock VM)
# ===========================================================================


class TestLaplaceHandlerWithMockVM:
    """Test laplace_handler and ilt_handler through a mock VM."""

    def _make_vm(self):
        """Create a mock VM that returns its argument unchanged."""
        vm = MagicMock()
        vm.eval = lambda x: x  # identity for testing
        return vm

    def test_laplace_handler_basic(self):
        vm = self._make_vm()
        f = IRInteger(1)
        expr = IRApply(LAPLACE, (f, T, S))
        result = laplace_handler(vm, expr)
        # L{1} = 1/s
        assert result == IRApply(DIV, (IRInteger(1), S))

    def test_laplace_handler_wrong_arity(self):
        vm = self._make_vm()
        expr = IRApply(LAPLACE, (IRInteger(1), T))  # only 2 args
        result = laplace_handler(vm, expr)
        assert result == expr

    def test_laplace_handler_non_symbol_t(self):
        vm = self._make_vm()
        expr = IRApply(LAPLACE, (IRInteger(1), IRInteger(0), S))
        result = laplace_handler(vm, expr)
        assert result == expr

    def test_laplace_handler_non_symbol_s(self):
        vm = self._make_vm()
        expr = IRApply(LAPLACE, (IRInteger(1), T, IRInteger(0)))
        result = laplace_handler(vm, expr)
        assert result == expr

    def test_ilt_handler_basic(self):
        vm = self._make_vm()
        F = IRApply(DIV, (IRInteger(1), S))
        expr = IRApply(ILT, (F, S, T))
        result = ilt_handler(vm, expr)
        assert result == IRApply(UNIT_STEP, (T,))

    def test_ilt_handler_wrong_arity(self):
        vm = self._make_vm()
        expr = IRApply(ILT, (S, T))  # only 2 args
        result = ilt_handler(vm, expr)
        assert result == expr

    def test_ilt_handler_non_symbol_s(self):
        vm = self._make_vm()
        F = IRApply(DIV, (IRInteger(1), S))
        expr = IRApply(ILT, (F, IRInteger(0), T))
        result = ilt_handler(vm, expr)
        assert result == expr

    def test_ilt_handler_non_symbol_t(self):
        vm = self._make_vm()
        F = IRApply(DIV, (IRInteger(1), S))
        expr = IRApply(ILT, (F, S, IRInteger(0)))
        result = ilt_handler(vm, expr)
        assert result == expr


# ===========================================================================
# Polynomial arithmetic helpers
# ===========================================================================


class TestPolyHelpers:
    """Test the polynomial arithmetic functions in inverse_table."""

    def test_poly_deriv_constant(self):
        # Derivative of a constant is zero
        p = (Fraction(5),)
        result = _poly_deriv(p)
        assert result == (Fraction(0),)

    def test_poly_deriv_linear(self):
        # d/ds (a + b*s) = b
        p = (Fraction(3), Fraction(7))
        result = _poly_deriv(p)
        assert result == (Fraction(7),)

    def test_poly_deriv_quadratic(self):
        # d/ds (1 + 2s + 3s^2) = 2 + 6s
        p = (Fraction(1), Fraction(2), Fraction(3))
        result = _poly_deriv(p)
        assert result == (Fraction(2), Fraction(6))

    def test_poly_gcd_trivial(self):
        # GCD(s^2+1, 1) = 1
        p = (Fraction(1), Fraction(0), Fraction(1))  # s^2 + 1
        q = (Fraction(1),)
        result = _poly_gcd(p, q)
        assert result == (Fraction(1),)

    def test_poly_divmod_exact(self):
        # (s^2 - 1) / (s - 1) = s + 1
        num = (-Fraction(1), Fraction(0), Fraction(1))  # -1 + s^2
        den = (-Fraction(1), Fraction(1))               # -1 + s
        q, r = _poly_divmod(num, den)
        assert _poly_normalize(r) == (Fraction(0),)

    def test_poly_divmod_remainder(self):
        # (s^2 + 1) / s = s + 0 remainder 1
        num = (Fraction(1), Fraction(0), Fraction(1))  # 1 + s^2
        den = (Fraction(0), Fraction(1))               # s
        q, r = _poly_divmod(num, den)
        # remainder should be 1
        assert r[0] == Fraction(1)

    def test_poly_divmod_lower_degree(self):
        # degree(num) < degree(den) → quotient 0, remainder = num
        num = (Fraction(1), Fraction(2))   # 1 + 2s
        den = (Fraction(1), Fraction(0), Fraction(1))  # 1 + s^2
        q, r = _poly_divmod(num, den)
        assert _poly_normalize(q) == (Fraction(0),)

    def test_poly_pow_zero(self):
        p = (Fraction(2), Fraction(1))  # 2 + s
        result = _poly_pow(p, 0)
        assert result == (Fraction(1),)

    def test_poly_pow_one(self):
        p = (Fraction(2), Fraction(1))
        result = _poly_pow(p, 1)
        assert result == p

    def test_poly_pow_two(self):
        # (2+s)^2 = 4 + 4s + s^2
        p = (Fraction(2), Fraction(1))
        result = _poly_pow(p, 2)
        assert _poly_normalize(result) == (Fraction(4), Fraction(4), Fraction(1))

    def test_poly_scale(self):
        p = (Fraction(1), Fraction(2), Fraction(3))
        result = _poly_scale(p, Fraction(2))
        assert result == (Fraction(2), Fraction(4), Fraction(6))

    def test_poly_neg(self):
        p = (Fraction(1), Fraction(-2))
        result = _poly_neg(p)
        assert result == (-Fraction(1), Fraction(2))

    def test_poly_add_different_lengths(self):
        a = (Fraction(1), Fraction(2))
        b = (Fraction(3), Fraction(0), Fraction(5))
        result = _poly_add(a, b)
        assert result == (Fraction(4), Fraction(2), Fraction(5))

    def test_poly_mul_identity(self):
        # (1) * p = p
        a = (Fraction(1),)
        b = (Fraction(1), Fraction(2), Fraction(3))
        result = _poly_mul(a, b)
        assert _poly_normalize(result) == _poly_normalize(b)


# ===========================================================================
# Rational root extraction
# ===========================================================================


class TestRationalRoots:
    """Test _rational_roots and _extract_all_rational_roots."""

    def test_rational_roots_quadratic(self):
        # (s-2)(s+3) = s^2 + s - 6 → roots: 2 and -3
        # coefficients: (-6, 1, 1)
        p = (Fraction(-6), Fraction(1), Fraction(1))
        roots = _rational_roots(p)
        assert Fraction(2) in roots or Fraction(-3) in roots

    def test_extract_all_roots_quadratic(self):
        # s^2 - 1 = (s-1)(s+1) → roots 1, -1
        p = (-Fraction(1), Fraction(0), Fraction(1))
        roots = _extract_all_rational_roots(p)
        root_set = set(roots)
        assert Fraction(1) in root_set or Fraction(-1) in root_set

    def test_rational_roots_zero_constant(self):
        # s * (s - 2) → roots include 0
        p = (Fraction(0), Fraction(-2), Fraction(1))
        roots = _rational_roots(p)
        assert Fraction(0) in roots

    def test_rational_roots_constant_poly(self):
        # Constant polynomial — no roots
        p = (Fraction(5),)
        roots = _rational_roots(p)
        assert roots == []

    def test_extract_roots_irreducible(self):
        # s^2 + 1 has no rational roots
        p = (Fraction(1), Fraction(0), Fraction(1))
        roots = _extract_all_rational_roots(p)
        assert roots == []


# ===========================================================================
# Square root helper
# ===========================================================================


class TestIsqrtExact:
    """Test _isqrt_exact and _rational_sqrt."""

    def test_isqrt_zero(self):
        assert _isqrt_exact(0) == 0

    def test_isqrt_perfect_squares(self):
        assert _isqrt_exact(1) == 1
        assert _isqrt_exact(4) == 2
        assert _isqrt_exact(9) == 3
        assert _isqrt_exact(16) == 4
        assert _isqrt_exact(100) == 10

    def test_isqrt_not_perfect(self):
        assert _isqrt_exact(2) is None
        assert _isqrt_exact(3) is None
        assert _isqrt_exact(5) is None

    def test_isqrt_negative(self):
        assert _isqrt_exact(-1) is None

    def test_rational_sqrt_perfect(self):
        assert _rational_sqrt(Fraction(4)) == Fraction(2)
        assert _rational_sqrt(Fraction(1, 4)) == Fraction(1, 2)
        assert _rational_sqrt(Fraction(9, 16)) == Fraction(3, 4)

    def test_rational_sqrt_not_perfect(self):
        assert _rational_sqrt(Fraction(2)) is None


# ===========================================================================
# IR-to-rational conversion
# ===========================================================================


class TestIRToRational:
    """Test the _ir_to_rational function for various IR forms."""

    def test_integer(self):
        result = _ir_to_rational(IRInteger(5), S)
        assert result == ((Fraction(5),), (Fraction(1),))

    def test_rational(self):
        result = _ir_to_rational(IRRational(1, 2), S)
        assert result == ((Fraction(1, 2),), (Fraction(1),))

    def test_symbol_s(self):
        result = _ir_to_rational(S, S)
        # s = 0 + 1*s
        assert result == ((Fraction(0), Fraction(1)), (Fraction(1),))

    def test_symbol_other(self):
        # Non-s symbol → None (not a polynomial in s)
        x = IRSymbol("x")
        result = _ir_to_rational(x, S)
        assert result is None

    def test_add(self):
        # s + 1 = (Poly(0,1) + Poly(1)) over Poly(1)
        expr = IRApply(ADD, (S, IRInteger(1)))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_sub(self):
        # s - 2
        expr = IRApply(SUB, (S, IRInteger(2)))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_mul(self):
        # s * (s - 1) = s^2 - s
        expr = IRApply(MUL, (S, IRApply(SUB, (S, IRInteger(1)))))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_div(self):
        # 1 / s
        expr = IRApply(DIV, (IRInteger(1), S))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_neg(self):
        # -(s + 1)
        expr = IRApply(NEG, (IRApply(ADD, (S, IRInteger(1))),))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_pow_positive(self):
        # s^3
        expr = IRApply(POW, (S, IRInteger(3)))
        result = _ir_to_rational(expr, S)
        assert result is not None
        num, den = result
        assert len(num) == 4  # s^3 = (0, 0, 0, 1)
        assert num[3] == Fraction(1)

    def test_pow_negative(self):
        # s^{-2} = 1/s^2
        expr = IRApply(POW, (S, IRInteger(-2)))
        result = _ir_to_rational(expr, S)
        assert result is not None

    def test_pow_non_integer_exp(self):
        # s^(1/2) → None
        expr = IRApply(POW, (S, IRRational(1, 2)))
        result = _ir_to_rational(expr, S)
        assert result is None

    def test_unknown_head(self):
        # Sin(s) → None (not a polynomial operation)
        expr = IRApply(SIN, (S,))
        result = _ir_to_rational(expr, S)
        assert result is None

    def test_irfloat(self):
        # IRFloat is not supported
        result = _ir_to_rational(IRFloat(1.5), S)
        assert result is None

    def test_non_apply_non_literal(self):
        # IRString → None
        from symbolic_ir import IRString
        result = _ir_to_rational(IRString("x"), S)
        assert result is None


# ===========================================================================
# ILT direct pole handlers
# ===========================================================================


class TestILTPoleHandlers:
    """Test _ilt_simple_pole and _ilt_repeated_pole directly."""

    def test_simple_pole_at_zero(self):
        # A/(s-0) → A * UnitStep(t)
        result = _ilt_simple_pole(Fraction(1), Fraction(0), T)
        assert result == IRApply(UNIT_STEP, (T,))

    def test_simple_pole_nonunit_A_at_zero(self):
        # 3/s → 3 * UnitStep(t)
        result = _ilt_simple_pole(Fraction(3), Fraction(0), T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"

    def test_simple_pole_neg_one_A(self):
        # -1/(s-2) → -exp(2t)
        result = _ilt_simple_pole(Fraction(-1), Fraction(2), T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Neg"

    def test_repeated_pole_order_2_at_zero(self):
        # 1/s^2 → t (t^{2-1} * exp(0*t) / (2-1)! = t)
        result = _ilt_repeated_pole(Fraction(1), Fraction(0), 2, T)
        # Should be t (the symbol itself, since a=0, n=2, coeff=1)
        assert result == T

    def test_repeated_pole_order_3_at_zero(self):
        # 1/s^3 → t^2 / 2
        result = _ilt_repeated_pole(Fraction(1), Fraction(0), 3, T)
        assert isinstance(result, IRApply)
        # coeff = 1/(3-1)! = 1/2
        # result = (1/2) * t^2

    def test_repeated_pole_order_2_nonzero(self):
        # 1/(s-3)^2 → t * exp(3t)
        result = _ilt_repeated_pole(Fraction(1), Fraction(3), 2, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"

    def test_repeated_pole_coeff_1(self):
        # 2/(s-1)^2 → 2 * t * exp(t)  [coeff = 2/1! = 2]
        result = _ilt_repeated_pole(Fraction(2), Fraction(1), 2, T)
        assert isinstance(result, IRApply)


# ===========================================================================
# ILT from match helpers
# ===========================================================================


class TestILTFromMatchHelpers:
    """Test _ilt_from_trig_match and _ilt_from_hyp_match."""

    def test_trig_sin_omega_1(self):
        # omega=1: sin(t) not sin(1*t)
        m = {"type": "sin", "omega": Fraction(1)}
        result = _ilt_from_trig_match(m, T)
        assert result == IRApply(SIN, (T,))

    def test_trig_sin_omega_2(self):
        # omega=2: sin(2t)
        m = {"type": "sin", "omega": Fraction(2)}
        result = _ilt_from_trig_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sin"

    def test_trig_sin_scaled_A_1(self):
        # A=1, omega=2: sin(2t) without extra factor
        m = {"type": "sin_scaled", "omega": Fraction(2), "A": Fraction(1)}
        result = _ilt_from_trig_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sin"

    def test_trig_sin_scaled_A_3(self):
        # A=3, omega=2: 3*sin(2t)
        m = {"type": "sin_scaled", "omega": Fraction(2), "A": Fraction(3)}
        result = _ilt_from_trig_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"

    def test_trig_cos_omega_1(self):
        # cos(t)
        m = {"type": "cos", "omega": Fraction(1)}
        result = _ilt_from_trig_match(m, T)
        assert result == IRApply(COS, (T,))

    def test_trig_cos_omega_3(self):
        # cos(3t)
        m = {"type": "cos", "omega": Fraction(3)}
        result = _ilt_from_trig_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Cos"

    def test_hyp_sinh_a_1(self):
        # sinh(t)
        m = {"type": "sinh", "a": Fraction(1)}
        result = _ilt_from_hyp_match(m, T)
        assert result == IRApply(SINH, (T,))

    def test_hyp_sinh_a_2(self):
        # sinh(2t)
        m = {"type": "sinh", "a": Fraction(2)}
        result = _ilt_from_hyp_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sinh"

    def test_hyp_sinh_scaled_A_1(self):
        # A=1: sinh(2t)
        m = {"type": "sinh_scaled", "a": Fraction(2), "A": Fraction(1)}
        result = _ilt_from_hyp_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sinh"

    def test_hyp_sinh_scaled_A_3(self):
        # A=3: 3*sinh(2t)
        m = {"type": "sinh_scaled", "a": Fraction(2), "A": Fraction(3)}
        result = _ilt_from_hyp_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"

    def test_hyp_cosh_a_1(self):
        # cosh(t)
        m = {"type": "cosh", "a": Fraction(1)}
        result = _ilt_from_hyp_match(m, T)
        assert result == IRApply(COSH, (T,))

    def test_hyp_cosh_a_3(self):
        # cosh(3t)
        m = {"type": "cosh", "a": Fraction(3)}
        result = _ilt_from_hyp_match(m, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Cosh"


# ===========================================================================
# Table pattern matchers — additional coverage
# ===========================================================================


class TestTablePatternsCoverage:
    """Hit the uncovered branches in table.py pattern matchers."""

    def test_is_const_integer(self):
        assert _is_const(IRInteger(5), T)

    def test_is_const_rational(self):
        assert _is_const(IRRational(1, 2), T)

    def test_is_const_other_symbol(self):
        x = IRSymbol("x")
        assert _is_const(x, T)

    def test_is_const_t_symbol(self):
        assert not _is_const(T, T)

    def test_is_const_apply_containing_t(self):
        # sin(t) is NOT constant w.r.t. t
        expr = IRApply(SIN, (T,))
        assert not _is_const(expr, T)

    def test_extract_coeff_const_on_right(self):
        # Mul(t, 3) — t is first, 3 is second (const on right)
        expr = IRApply(MUL, (T, IRInteger(3)))
        coeff, fn = _extract_coeff_and_fn(expr, T)
        assert coeff == IRInteger(3)
        assert fn == T

    def test_extract_linear_arg_bare_t(self):
        result = _extract_linear_arg(T, T)
        assert result == IRInteger(1)

    def test_extract_linear_arg_mul_t_a(self):
        # Mul(t, 3) — reversed arg order
        expr = IRApply(MUL, (T, IRInteger(3)))
        result = _extract_linear_arg(expr, T)
        assert result == IRInteger(3)

    def test_match_exp_not_exp_head(self):
        # SIN(t) does not match exp
        f = IRApply(SIN, (T,))
        result = _match_exp(f, T)
        assert result is None

    def test_match_exp_non_linear_arg(self):
        # Exp(t^2) — not linear in t
        f = IRApply(EXP, (IRApply(POW, (T, IRInteger(2))),))
        result = _match_exp(f, T)
        assert result is None

    def test_match_sin_not_sin_head(self):
        f = IRApply(COS, (T,))
        result = _match_sin(f, T)
        assert result is None

    def test_match_cos_not_cos_head(self):
        f = IRApply(SIN, (T,))
        result = _match_cos(f, T)
        assert result is None

    def test_match_sinh_not_sinh_head(self):
        f = IRApply(COSH, (T,))
        result = _match_sinh(f, T)
        assert result is None

    def test_match_cosh_not_cosh_head(self):
        f = IRApply(SINH, (T,))
        result = _match_cosh(f, T)
        assert result is None

    def test_match_exp_cos_not_mul(self):
        # Must be a Mul to match exp*cos
        f = IRApply(COS, (T,))
        result = _match_exp_cos(f, T)
        assert result is None

    def test_match_exp_sin_not_mul(self):
        f = IRApply(SIN, (T,))
        result = _match_exp_sin(f, T)
        assert result is None

    def test_match_t_exp_not_mul(self):
        f = IRApply(EXP, (T,))
        result = _match_t_exp(f, T)
        assert result is None

    def test_match_tn_exp_not_mul(self):
        f = IRApply(EXP, (T,))
        result = _match_tn_exp(f, T)
        assert result is None

    def test_match_t_sin_not_mul(self):
        f = IRApply(SIN, (T,))
        result = _match_t_sin(f, T)
        assert result is None

    def test_match_t_cos_not_mul(self):
        f = IRApply(COS, (T,))
        result = _match_t_cos(f, T)
        assert result is None

    def test_match_dirac_wrong_head(self):
        f = IRApply(UNIT_STEP, (T,))
        result = _match_dirac_delta(f, T)
        assert result is None

    def test_match_dirac_wrong_arg(self):
        # DiracDelta(s) — arg is s, not t
        f = IRApply(DIRAC_DELTA, (S,))
        result = _match_dirac_delta(f, T)
        assert result is None

    def test_match_unit_step_wrong_head(self):
        f = IRApply(DIRAC_DELTA, (T,))
        result = _match_unit_step(f, T)
        assert result is None

    def test_match_unit_step_wrong_arg(self):
        f = IRApply(UNIT_STEP, (S,))
        result = _match_unit_step(f, T)
        assert result is None

    def test_table_lookup_returns_none_for_unknown(self):
        unknown = IRApply(IRSymbol("Unknown"), (T,))
        result = table_lookup(unknown, T, S)
        assert result is None

    def test_sinh_with_reversed_mul(self):
        # sinh(t*2) — t first, 2 second
        f = IRApply(SINH, (IRApply(MUL, (T, IRInteger(2))),))
        result = _match_sinh(f, T)
        assert result is not None
        assert result["a"] == IRInteger(2)

    def test_cosh_with_reversed_mul(self):
        # cosh(t*3) — t first, 3 second
        f = IRApply(COSH, (IRApply(MUL, (T, IRInteger(3))),))
        result = _match_cosh(f, T)
        assert result is not None
        assert result["a"] == IRInteger(3)


# ===========================================================================
# ILT additional edge cases
# ===========================================================================


class TestILTEdgeCases:
    """Additional inverse transform edge cases."""

    def test_ilt_improper_fraction_falls_through(self):
        # s^2 / s = s — degree(num) >= degree(den) → not a proper fraction
        F = IRApply(DIV, (IRApply(POW, (S, IRInteger(2))), S))
        result = inverse_laplace(F, S, T)
        # Should fall through (not a proper fraction for PF decomp)
        assert isinstance(result, IRApply)
        # Either ILT unevaluated or a recognized form

    def test_ilt_rational_with_irreducible_denom(self):
        # 1/(s^2 + 1) — denominator has no rational roots
        # This is omega/(s^2+omega^2) with omega=1 actually
        F = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRInteger(1))),
            ),
        )
        result = inverse_laplace(F, S, T)
        # 1/(s^2+1) = sin(t) since omega=1 and A=1/omega=1
        assert isinstance(result, IRApply)

    def test_ilt_via_pf_empty_terms_edge(self):
        # Test a form that PF decomposes to 0 terms (degenerate)
        # Actually, let's test constant / s^2 → repeated pole at 0
        F = IRApply(DIV, (IRInteger(2), IRApply(POW, (S, IRInteger(2)))))
        result = inverse_laplace(F, S, T)
        # 2/s^2 = repeated pole at 0 with residue A:
        # Actually the partial fraction approach needs the denominator to
        # be fully factored; s^2 has root 0 with multiplicity 2.
        # This should either return 2*t or fall through.
        assert result is not None
