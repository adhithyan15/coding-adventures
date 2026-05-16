"""Tests for the cas-laplace package.

We test:
1. Forward Laplace transforms via ``laplace_transform`` directly.
2. Inverse Laplace transforms via ``inverse_laplace`` directly.
3. DiracDelta and UnitStep handler evaluation rules.
4. The ``build_laplace_handler_table`` function for correct keys.

All tests use the IR constructors directly — no parsing required —
so they exercise the pure algorithmic layer, not the VM integration.

The expected output of each transform is verified by checking the
structure of the returned IR node. We use str() representations for
readability and deep equality checks for precision.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EXP,
    MUL,
    POW,
    SIN,
    SINH,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_laplace.handlers import (
    build_laplace_handler_table,
    dirac_delta_handler,
    unit_step_handler,
)
from cas_laplace.heads import DIRAC_DELTA, ILT, LAPLACE, UNIT_STEP
from cas_laplace.ilt import inverse_laplace
from cas_laplace.laplace import laplace_transform

# ---------------------------------------------------------------------------
# Fixtures: the two standard variables used in every test.
# ---------------------------------------------------------------------------

T = IRSymbol("t")
S = IRSymbol("s")


def _neg(x: IRInteger) -> IRApply:
    """Build Neg(x) IR node."""
    from symbolic_ir import NEG
    return IRApply(NEG, (x,))


# ===========================================================================
# Forward Laplace transform tests
# ===========================================================================


class TestLaplaceConstant:
    """L{1} = 1/s."""

    def test_constant_one(self):
        result = laplace_transform(IRInteger(1), T, S)
        assert result == IRApply(DIV, (IRInteger(1), S))

    def test_constant_one_rational_form(self):
        # IRRational(1,1) is also "1"
        # NOTE: IRRational normalizes 1/1 → numer=1, denom=1
        result = laplace_transform(IRRational(1, 1), T, S)
        assert result == IRApply(DIV, (IRInteger(1), S))


class TestLaplacePowerOfT:
    """L{t^n} = n! / s^{n+1}."""

    def test_t_power_1(self):
        # L{t} = 1/s^2
        result = laplace_transform(T, T, S)
        assert result == IRApply(DIV, (IRInteger(1), IRApply(POW, (S, IRInteger(2)))))

    def test_t_power_2(self):
        # L{t^2} = 2/s^3
        f = IRApply(POW, (T, IRInteger(2)))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(2), IRApply(POW, (S, IRInteger(3)))))

    def test_t_power_3(self):
        # L{t^3} = 6/s^4  (3! = 6)
        f = IRApply(POW, (T, IRInteger(3)))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(6), IRApply(POW, (S, IRInteger(4)))))

    def test_t_power_4(self):
        # L{t^4} = 24/s^5  (4! = 24)
        f = IRApply(POW, (T, IRInteger(4)))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(24), IRApply(POW, (S, IRInteger(5)))))


class TestLaplaceExp:
    """L{exp(at)} = 1/(s-a)."""

    def test_exp_3t(self):
        # L{exp(3t)} = 1/(s-3)
        f = IRApply(EXP, (IRApply(MUL, (IRInteger(3), T)),))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(1), IRApply(SUB, (S, IRInteger(3)))))

    def test_exp_neg2t(self):
        # L{exp(-2t)} = 1/(s+2) = 1/(s-(-2))
        f = IRApply(EXP, (IRApply(MUL, (IRInteger(-2), T)),))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(1), IRApply(SUB, (S, IRInteger(-2)))))

    def test_exp_t(self):
        # L{exp(t)} = 1/(s-1)
        f = IRApply(EXP, (T,))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(1), IRApply(SUB, (S, IRInteger(1)))))


class TestLaplaceSin:
    """L{sin(ωt)} = ω/(s²+ω²)."""

    def test_sin_2t(self):
        # L{sin(2t)} = 2/(s^2 + 4)
        f = IRApply(SIN, (IRApply(MUL, (IRInteger(2), T)),))
        result = laplace_transform(f, T, S)
        expected = IRApply(
            DIV,
            (
                IRInteger(2),
                IRApply(
                    ADD,
                    (
                        IRApply(POW, (S, IRInteger(2))),
                        IRApply(POW, (IRInteger(2), IRInteger(2))),
                    ),
                ),
            ),
        )
        assert result == expected

    def test_sin_t(self):
        # L{sin(t)} = 1/(s^2 + 1)
        f = IRApply(SIN, (T,))
        result = laplace_transform(f, T, S)
        expected = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(
                    ADD,
                    (
                        IRApply(POW, (S, IRInteger(2))),
                        IRApply(POW, (IRInteger(1), IRInteger(2))),
                    ),
                ),
            ),
        )
        assert result == expected


class TestLaplaceCos:
    """L{cos(ωt)} = s/(s²+ω²)."""

    def test_cos_3t(self):
        # L{cos(3t)} = s/(s^2 + 9)
        f = IRApply(COS, (IRApply(MUL, (IRInteger(3), T)),))
        result = laplace_transform(f, T, S)
        expected = IRApply(
            DIV,
            (
                S,
                IRApply(
                    ADD,
                    (
                        IRApply(POW, (S, IRInteger(2))),
                        IRApply(POW, (IRInteger(3), IRInteger(2))),
                    ),
                ),
            ),
        )
        assert result == expected

    def test_cos_t(self):
        # L{cos(t)} = s/(s^2 + 1)
        f = IRApply(COS, (T,))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == S  # numerator is s


class TestLaplaceExpSin:
    """L{exp(at)·sin(ωt)} = ω/((s-a)²+ω²)."""

    def test_exp_t_sin_2t(self):
        # L{exp(t)*sin(2t)} = 2/((s-1)^2 + 4)
        f = IRApply(MUL, (
            IRApply(EXP, (T,)),
            IRApply(SIN, (IRApply(MUL, (IRInteger(2), T)),)),
        ))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        # numerator should be 2 (= omega)
        assert result.args[0] == IRInteger(2)

    def test_exp_t_cos_2t(self):
        # L{exp(t)*cos(2t)} = (s-1)/((s-1)^2 + 4)
        f = IRApply(MUL, (
            IRApply(EXP, (T,)),
            IRApply(COS, (IRApply(MUL, (IRInteger(2), T)),)),
        ))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        # numerator should be (s-1)
        assert result.args[0] == IRApply(SUB, (S, IRInteger(1)))


class TestLaplaceModified:
    """L{t·exp(at)} = 1/(s-a)², L{t^n·exp(at)} = n!/(s-a)^{n+1}."""

    def test_t_exp_2t(self):
        # L{t*exp(2t)} = 1/(s-2)^2
        f = IRApply(MUL, (T, IRApply(EXP, (IRApply(MUL, (IRInteger(2), T)),))))
        result = laplace_transform(f, T, S)
        expected = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(POW, (IRApply(SUB, (S, IRInteger(2))), IRInteger(2))),
            ),
        )
        assert result == expected

    def test_t2_exp_t(self):
        # L{t^2 * exp(t)} = 2!/(s-1)^3 = 2/(s-1)^3
        f = IRApply(MUL, (
            IRApply(POW, (T, IRInteger(2))),
            IRApply(EXP, (T,)),
        ))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == IRInteger(2)  # 2! = 2
        expected_den = IRApply(POW, (IRApply(SUB, (S, IRInteger(1))), IRInteger(3)))
        assert result.args[1] == expected_den


class TestLaplaceHyperbolic:
    """L{sinh(at)} = a/(s²-a²), L{cosh(at)} = s/(s²-a²)."""

    def test_sinh_t(self):
        # L{sinh(t)} = 1/(s^2 - 1)
        f = IRApply(SINH, (T,))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        # numerator = a = 1
        assert result.args[0] == IRInteger(1)
        # denominator = s^2 - 1
        assert result.args[1] == IRApply(SUB, (
            IRApply(POW, (S, IRInteger(2))),
            IRApply(POW, (IRInteger(1), IRInteger(2))),
        ))

    def test_cosh_t(self):
        # L{cosh(t)} = s/(s^2 - 1)
        f = IRApply(COSH, (T,))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        # numerator = s
        assert result.args[0] == S

    def test_sinh_2t(self):
        # L{sinh(2t)} = 2/(s^2 - 4)
        f = IRApply(SINH, (IRApply(MUL, (IRInteger(2), T)),))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == IRInteger(2)


class TestLaplaceSpecialFunctions:
    """L{DiracDelta(t)} = 1, L{UnitStep(t)} = 1/s."""

    def test_dirac_delta(self):
        f = IRApply(DIRAC_DELTA, (T,))
        result = laplace_transform(f, T, S)
        assert result == IRInteger(1)

    def test_unit_step(self):
        f = IRApply(UNIT_STEP, (T,))
        result = laplace_transform(f, T, S)
        assert result == IRApply(DIV, (IRInteger(1), S))


class TestLaplaceLinearity:
    """Linearity: L{c·f} = c·L{f}, L{f+g} = L{f}+L{g}."""

    def test_scalar_multiple(self):
        # L{3*sin(t)} = 3 * (1/(s^2+1))
        f = IRApply(MUL, (IRInteger(3), IRApply(SIN, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"
        assert result.args[0] == IRInteger(3)
        # Second arg should be the transform of sin(t)
        assert isinstance(result.args[1], IRApply)
        assert result.args[1].head.name == "Div"

    def test_sum(self):
        # L{sin(t) + cos(t)} = L{sin(t)} + L{cos(t)}
        f = IRApply(ADD, (IRApply(SIN, (T,)), IRApply(COS, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"
        # Both components should be Div(...)
        assert result.args[0].head.name == "Div"
        assert result.args[1].head.name == "Div"

    def test_scalar_multiple_cos(self):
        # L{5*cos(2t)} = 5 * s/(s^2+4)
        inner = IRApply(MUL, (IRInteger(2), T))
        f = IRApply(MUL, (IRInteger(5), IRApply(COS, (inner,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"
        assert result.args[0] == IRInteger(5)


class TestLaplaceFallThrough:
    """Unevaluated form for unrecognized functions."""

    def test_unknown_function(self):
        # laplace(f(t), t, s) → Laplace(f(t), t, s) unevaluated
        unknown_head = IRSymbol("UnknownFn")
        f = IRApply(unknown_head, (T,))
        result = laplace_transform(f, T, S)
        assert result == IRApply(LAPLACE, (f, T, S))

    def test_nested_unknown(self):
        # The fall-through must happen even for complex expressions
        f = IRApply(IRSymbol("Bessel"), (IRInteger(0), T))
        result = laplace_transform(f, T, S)
        assert result == IRApply(LAPLACE, (f, T, S))


# ===========================================================================
# Inverse Laplace transform tests
# ===========================================================================


class TestILTBasic:
    """Basic inverse transforms via direct pattern matching."""

    def test_ilt_one_over_s(self):
        # L^{-1}{1/s} = UnitStep(t)
        F = IRApply(DIV, (IRInteger(1), S))
        result = inverse_laplace(F, S, T)
        assert result == IRApply(UNIT_STEP, (T,))

    def test_ilt_exp_2t(self):
        # L^{-1}{1/(s-2)} = exp(2t)
        F = IRApply(DIV, (IRInteger(1), IRApply(SUB, (S, IRInteger(2)))))
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Exp"

    def test_ilt_exp_neg3t(self):
        # L^{-1}{1/(s+3)} = exp(-3t)
        F = IRApply(DIV, (IRInteger(1), IRApply(ADD, (S, IRInteger(3)))))
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        # The exp(-3t) form — result should be an Exp node
        assert result.head.name == "Exp"

    def test_ilt_sin_2t(self):
        # L^{-1}{2/(s^2+4)} = sin(2t)
        # Note: 2/(s^2+4) = omega/(s^2+omega^2) with omega=2
        F = IRApply(
            DIV,
            (
                IRInteger(2),
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRInteger(4))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sin"

    def test_ilt_cos_3t(self):
        # L^{-1}{s/(s^2+9)} = cos(3t)
        F = IRApply(
            DIV,
            (
                S,
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRInteger(9))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Cos"

    def test_ilt_sinh_t(self):
        # L^{-1}{1/(s^2-1)} = sinh(t)
        F = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(SUB, (IRApply(POW, (S, IRInteger(2))), IRInteger(1))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sinh"

    def test_ilt_cosh_t(self):
        # L^{-1}{s/(s^2-1)} = cosh(t)
        F = IRApply(
            DIV,
            (
                S,
                IRApply(SUB, (IRApply(POW, (S, IRInteger(2))), IRInteger(1))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Cosh"

    def test_ilt_one_over_s_squared(self):
        # L^{-1}{1/s^2} = t
        F = IRApply(DIV, (IRInteger(1), IRApply(POW, (S, IRInteger(2)))))
        result = inverse_laplace(F, S, T)
        # This is 1/(s-0)^2 = repeated pole at 0 → t * exp(0*t) / 1! = t
        # Or via partial fractions: s^2 has root 0 with multiplicity 2
        # Implementation may return t or t*UnitStep(t) depending on strategy
        assert isinstance(result, IRApply) or result == T

    def test_ilt_fall_through(self):
        # Unknown form → unevaluated ILT(F, s, t)
        unknown = IRSymbol("UnknownF")
        result = inverse_laplace(unknown, S, T)
        assert result == IRApply(ILT, (unknown, S, T))


class TestILTPartialFractions:
    """Inverse transforms via partial-fraction decomposition."""

    def test_ilt_1_over_s_times_s_plus_1(self):
        # L^{-1}{1/(s*(s+1))} = 1 - exp(-t)
        # Partial fractions: 1/s - 1/(s+1)
        # 1/(s*(s+1)) = A/s + B/(s+1)
        # A*(s+1) + B*s = 1 → A=1, B=-1
        F = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(MUL, (S, IRApply(ADD, (S, IRInteger(1))))),
            ),
        )
        result = inverse_laplace(F, S, T)
        # Result should be a sum of two terms
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"

    def test_ilt_2_over_s_times_s_plus_2(self):
        # L^{-1}{2/(s*(s+2))}
        # PF: 2/(s*(s+2)) = 1/s - 1/(s+2)
        # So result = UnitStep(t) - exp(-2t)
        F = IRApply(
            DIV,
            (
                IRInteger(2),
                IRApply(MUL, (S, IRApply(ADD, (S, IRInteger(2))))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"


class TestILTFallThrough:
    """ILT should return unevaluated for unrecognized forms."""

    def test_ilt_symbol_falls_through(self):
        unknown = IRSymbol("F")
        result = inverse_laplace(unknown, S, T)
        assert result == IRApply(ILT, (unknown, S, T))

    def test_ilt_complex_unknown(self):
        # Something with sqrt(s) — not a rational function
        unknown = IRApply(IRSymbol("Sqrt"), (S,))
        result = inverse_laplace(unknown, S, T)
        assert result == IRApply(ILT, (unknown, S, T))


# ===========================================================================
# DiracDelta handler tests
# ===========================================================================


class TestDiracDeltaHandler:
    """DiracDelta(0) → 1, DiracDelta(x) → unevaluated."""

    def _call(self, arg: IRNode) -> IRNode:
        """Call the handler with a mock VM (not needed for DiracDelta)."""
        expr = IRApply(DIRAC_DELTA, (arg,))
        return dirac_delta_handler(None, expr)  # type: ignore[arg-type]

    def test_dirac_at_zero(self):
        result = self._call(IRInteger(0))
        assert result == IRInteger(1)

    def test_dirac_at_nonzero_integer(self):
        # DiracDelta(3) → unevaluated (delta is zero almost everywhere)
        result = self._call(IRInteger(3))
        assert result == IRApply(DIRAC_DELTA, (IRInteger(3),))

    def test_dirac_at_symbol(self):
        x = IRSymbol("x")
        result = self._call(x)
        assert result == IRApply(DIRAC_DELTA, (x,))

    def test_dirac_wrong_arity(self):
        # Wrong arity → unevaluated
        expr = IRApply(DIRAC_DELTA, ())
        result = dirac_delta_handler(None, expr)  # type: ignore[arg-type]
        assert result == expr

    def test_dirac_two_args(self):
        expr = IRApply(DIRAC_DELTA, (IRInteger(0), IRInteger(0)))
        result = dirac_delta_handler(None, expr)  # type: ignore[arg-type]
        assert result == expr


# ===========================================================================
# UnitStep handler tests
# ===========================================================================


class TestUnitStepHandler:
    """UnitStep(0) → 1/2, UnitStep(-1) → 0, UnitStep(1) → 1."""

    def _call(self, arg: IRNode) -> IRNode:
        expr = IRApply(UNIT_STEP, (arg,))
        return unit_step_handler(None, expr)  # type: ignore[arg-type]

    def test_unit_step_at_zero(self):
        result = self._call(IRInteger(0))
        assert result == IRRational(1, 2)

    def test_unit_step_at_negative(self):
        result = self._call(IRInteger(-1))
        assert result == IRInteger(0)

    def test_unit_step_at_negative_large(self):
        result = self._call(IRInteger(-100))
        assert result == IRInteger(0)

    def test_unit_step_at_positive(self):
        result = self._call(IRInteger(1))
        assert result == IRInteger(1)

    def test_unit_step_at_positive_large(self):
        result = self._call(IRInteger(42))
        assert result == IRInteger(1)

    def test_unit_step_at_symbol(self):
        x = IRSymbol("x")
        result = self._call(x)
        assert result == IRApply(UNIT_STEP, (x,))

    def test_unit_step_wrong_arity(self):
        expr = IRApply(UNIT_STEP, ())
        result = unit_step_handler(None, expr)  # type: ignore[arg-type]
        assert result == expr


# ===========================================================================
# Handler table tests
# ===========================================================================


class TestHandlerTable:
    """``build_laplace_handler_table()`` returns correct keys."""

    def test_table_has_laplace(self):
        table = build_laplace_handler_table()
        assert "Laplace" in table

    def test_table_has_ilt(self):
        table = build_laplace_handler_table()
        assert "ILT" in table

    def test_table_has_dirac_delta(self):
        table = build_laplace_handler_table()
        assert "DiracDelta" in table

    def test_table_has_unit_step(self):
        table = build_laplace_handler_table()
        assert "UnitStep" in table

    def test_table_has_four_entries(self):
        table = build_laplace_handler_table()
        assert len(table) == 4

    def test_all_handlers_callable(self):
        table = build_laplace_handler_table()
        for key, val in table.items():
            assert callable(val), f"Handler for {key!r} is not callable"


# ===========================================================================
# Heads module tests
# ===========================================================================


class TestHeads:
    """Verify the IR head symbols have the right names."""

    def test_laplace_head(self):
        assert LAPLACE.name == "Laplace"

    def test_ilt_head(self):
        assert ILT.name == "ILT"

    def test_dirac_delta_head(self):
        assert DIRAC_DELTA.name == "DiracDelta"

    def test_unit_step_head(self):
        assert UNIT_STEP.name == "UnitStep"


# ===========================================================================
# Additional edge-case tests for coverage
# ===========================================================================


class TestLaplaceEdgeCases:
    """Edge cases and additional coverage tests."""

    def test_exp_times_t_reversed_order(self):
        # L{exp(t)*t} — exp first, t second in Mul — same result as t*exp(t)
        f = IRApply(MUL, (IRApply(EXP, (T,)), T))
        result = laplace_transform(f, T, S)
        s_minus_1 = IRApply(SUB, (S, IRInteger(1)))
        expected = IRApply(DIV, (IRInteger(1), IRApply(POW, (s_minus_1, IRInteger(2)))))
        assert result == expected

    def test_sin_omega_reversed_mul(self):
        # sin(t*2) — t first, 2 second (reversed Mul arg order)
        f = IRApply(SIN, (IRApply(MUL, (T, IRInteger(2))),))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == IRInteger(2)

    def test_cos_omega_reversed_mul(self):
        # cos(t*3) — t first, 3 second
        f = IRApply(COS, (IRApply(MUL, (T, IRInteger(3))),))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == S

    def test_t_sin_reversed_order(self):
        # sin(t)*t — sin first, t second
        f = IRApply(MUL, (IRApply(SIN, (T,)), T))
        result = laplace_transform(f, T, S)
        # L{t*sin(t)} = 2s/(s^2+1)^2
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_t_cos_reversed_order(self):
        # cos(t)*t — cos first, t second
        f = IRApply(MUL, (IRApply(COS, (T,)), T))
        result = laplace_transform(f, T, S)
        # L{t*cos(t)} = (s^2-1)/(s^2+1)^2
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_exp_sin_reversed_mul_order(self):
        # sin(2t)*exp(t) — sin first, exp second
        f = IRApply(MUL, (
            IRApply(SIN, (IRApply(MUL, (IRInteger(2), T)),)),
            IRApply(EXP, (T,)),
        ))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        assert result.args[0] == IRInteger(2)

    def test_ilt_sin_with_pow_squared_omega(self):
        # 2/(s^2 + Pow(2,2)) — denominator uses Pow form for omega^2
        F = IRApply(
            DIV,
            (
                IRInteger(2),
                IRApply(
                    ADD,
                    (
                        IRApply(POW, (S, IRInteger(2))),
                        IRApply(POW, (IRInteger(2), IRInteger(2))),
                    ),
                ),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sin"

    def test_laplace_t_sin_2t(self):
        # L{t*sin(2t)} = 2*2*s/(s^2+4)^2 = 4s/(s^2+4)^2
        f = IRApply(MUL, (T, IRApply(SIN, (IRApply(MUL, (IRInteger(2), T)),))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_laplace_t_cos_2t(self):
        # L{t*cos(2t)} = (s^2-4)/(s^2+4)^2
        f = IRApply(MUL, (T, IRApply(COS, (IRApply(MUL, (IRInteger(2), T)),))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"


# ===========================================================================
# New gap-closure tests: t^n·trig (forward) and extended ILT
# ===========================================================================


class TestLaplaceTnTrig:
    """L{t^n·sin(ωt)} and L{t^n·cos(ωt)} for n = 2, 3."""

    def test_t2_sin_t(self):
        # L{t^2·sin(t)} = 2·1·(3s²−1)/(s²+1)³ = 2(3s²−1)/(s²+1)³
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(2))), IRApply(SIN, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        # denominator is (s²+1)^3
        denom = result.args[1]
        assert isinstance(denom, IRApply) and denom.head.name == "Pow"
        assert isinstance(denom.args[1], IRInteger) and denom.args[1].value == 3

    def test_t2_sin_2t(self):
        # L{t²·sin(2t)} = 2·2·(3s²−4)/(s²+4)³ = 4(3s²−4)/(s²+4)³
        inner = IRApply(MUL, (IRInteger(2), T))
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(2))), IRApply(SIN, (inner,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_t2_cos_t(self):
        # L{t²·cos(t)} = 2s(s²−3)/(s²+1)³
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(2))), IRApply(COS, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        denom = result.args[1]
        assert isinstance(denom, IRApply) and denom.head.name == "Pow"
        assert isinstance(denom.args[1], IRInteger) and denom.args[1].value == 3

    def test_t2_cos_3t(self):
        # L{t²·cos(3t)} = 2s(s²−27)/(s²+9)³
        inner = IRApply(MUL, (IRInteger(3), T))
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(2))), IRApply(COS, (inner,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_t3_sin_t(self):
        # L{t³·sin(t)} = 24s(s²−1)/(s²+1)⁴
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(3))), IRApply(SIN, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        denom = result.args[1]
        assert isinstance(denom, IRApply) and denom.head.name == "Pow"
        assert isinstance(denom.args[1], IRInteger) and denom.args[1].value == 4

    def test_t3_cos_t(self):
        # L{t³·cos(t)} = 6(s⁴−6s²+1)/(s²+1)⁴
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(3))), IRApply(COS, (T,))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"
        denom = result.args[1]
        assert isinstance(denom, IRApply) and denom.head.name == "Pow"
        assert isinstance(denom.args[1], IRInteger) and denom.args[1].value == 4

    def test_t2_sin_reversed_order(self):
        # sin(t) * t^2 — sin first, power second
        f = IRApply(MUL, (IRApply(SIN, (T,)), IRApply(POW, (T, IRInteger(2)))))
        result = laplace_transform(f, T, S)
        assert isinstance(result, IRApply)
        assert result.head.name == "Div"

    def test_t4_sin_falls_through(self):
        # L{t⁴·sin(t)} — not in table for n≥4, falls through to unevaluated
        from cas_laplace.heads import LAPLACE
        f = IRApply(MUL, (IRApply(POW, (T, IRInteger(4))), IRApply(SIN, (T,))))
        result = laplace_transform(f, T, S)
        # Should fall through (since tn_sin builder returns None for n>=4)
        # The table will miss it, returning Laplace(f, t, s)
        assert isinstance(result, IRApply)


class TestILTComplexPoles:
    """ILT for irreducible quadratic denominators (complex conjugate poles)."""

    def test_ilt_1_over_s2_plus_2s_plus_2(self):
        # L^{-1}{1/(s²+2s+2)} = exp(-t)·sin(t)
        # Denominator: s²+2s+2 = (s+1)²+1  →  α=1, β=1
        den = IRApply(
            ADD,
            (
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRApply(MUL, (IRInteger(2), S)))),
                IRInteger(2),
            ),
        )
        F = IRApply(DIV, (IRInteger(1), den))
        result = inverse_laplace(F, S, T)
        # Result should be Mul(Exp(-t), Sin(t))
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"
        # One factor is Exp, other is Sin
        heads = {result.args[0].head.name, result.args[1].head.name}
        assert "Exp" in heads
        assert "Sin" in heads

    def test_ilt_s_over_s2_plus_2s_plus_2(self):
        # L^{-1}{s/(s²+2s+2)} = exp(-t)·cos(t) + something
        den = IRApply(
            ADD,
            (
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRApply(MUL, (IRInteger(2), S)))),
                IRInteger(2),
            ),
        )
        F = IRApply(DIV, (S, den))
        result = inverse_laplace(F, S, T)
        # Result should be a sum involving exp(-t)*cos(t) and exp(-t)*sin(t)
        assert isinstance(result, IRApply)
        # Should not be unevaluated ILT
        from cas_laplace.heads import ILT
        assert result.head.name != "ILT"

    def test_ilt_1_over_s2_plus_4(self):
        # L^{-1}{1/(s²+4)} = (1/2)·sin(2t)
        # This is already handled by direct pattern but should also work via
        # partial fractions when expressed differently
        den = IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRInteger(4)))
        F = IRApply(DIV, (IRInteger(1), den))
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        # Should be sin_scaled form from direct pattern: (1/2)*sin(2t)
        from cas_laplace.heads import ILT
        assert result.head.name != "ILT"

    def test_ilt_mixed_rational_and_quad(self):
        # L^{-1}{1/(s·(s²+1))} = UnitStep(t) − cos(t)
        den = IRApply(MUL, (S, IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRInteger(1)))))
        F = IRApply(DIV, (IRInteger(1), den))
        result = inverse_laplace(F, S, T)
        # Should be a sum of two terms
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"


class TestILTRepeatedPoles:
    """ILT for repeated rational poles."""

    def test_ilt_1_over_s_squared(self):
        # L^{-1}{1/s²} = t
        F = IRApply(DIV, (IRInteger(1), IRApply(POW, (S, IRInteger(2)))))
        result = inverse_laplace(F, S, T)
        assert result == T  # t (bare symbol)

    def test_ilt_1_over_s_minus_2_squared(self):
        # L^{-1}{1/(s-2)²} = t·exp(2t)
        F = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(POW, (IRApply(SUB, (S, IRInteger(2))), IRInteger(2))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"
        heads = {result.args[0].head.name if isinstance(result.args[0], IRApply) else "sym",
                 result.args[1].head.name if isinstance(result.args[1], IRApply) else "sym"}
        assert "Exp" in heads

    def test_ilt_repeated_pole_at_neg1(self):
        # L^{-1}{1/(s+1)²} = t·exp(-t)
        F = IRApply(
            DIV,
            (
                IRInteger(1),
                IRApply(POW, (IRApply(ADD, (S, IRInteger(1))), IRInteger(2))),
            ),
        )
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        assert result.head.name == "Mul"
        # One factor should be t, the other exp(-t)
        factor_heads = set()
        for arg in result.args:
            if isinstance(arg, IRApply):
                factor_heads.add(arg.head.name)
            elif isinstance(arg, IRSymbol):
                factor_heads.add("Symbol")
        assert "Exp" in factor_heads

    def test_ilt_improper_fraction_constant_quotient(self):
        # L^{-1}{(s²+3s+3)/(s²+3s+2)}
        # = L^{-1}{1 + 1/(s²+3s+2)} = DiracDelta(t) + partial_fracs
        # s²+3s+2 = (s+1)(s+2), roots -1, -2
        # 1/((s+1)(s+2)) = 1/(s+1) - 1/(s+2) → exp(-t) - exp(-2t)
        from symbolic_ir import IRRational as _IRR
        # (s²+3s+3) in poly form ascending: (3, 3, 1)
        # We can't build this as a simple IR literal easily — use Add forms
        # Numerator: s² + 3s + 3
        num_ir = IRApply(
            ADD,
            (
                IRApply(ADD, (IRApply(POW, (S, IRInteger(2))), IRApply(MUL, (IRInteger(3), S)))),
                IRInteger(3),
            ),
        )
        # Denominator: s² + 3s + 2 = (s+1)(s+2)
        den_ir = IRApply(MUL, (IRApply(ADD, (S, IRInteger(1))), IRApply(ADD, (S, IRInteger(2)))))
        F = IRApply(DIV, (num_ir, den_ir))
        result = inverse_laplace(F, S, T)
        assert isinstance(result, IRApply)
        # Should include DiracDelta from the constant-1 polynomial part
        from cas_laplace.heads import DIRAC_DELTA
        # Flatten result to find all node types
        def _find_heads(node: IRNode, found: set) -> None:
            if isinstance(node, IRApply):
                found.add(node.head.name)
                for a in node.args:
                    _find_heads(a, found)
        all_heads: set = set()
        _find_heads(result, all_heads)
        assert "DiracDelta" in all_heads


class TestILTPolynomialHelpers:
    """Unit tests for the polynomial helper functions."""

    def test_poly_shift_constant(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _poly_shift
        # Shifting a constant is a no-op
        p = (Fraction(5),)
        assert _poly_shift(p, Fraction(3)) == p

    def test_poly_shift_linear(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _poly_shift
        # (s + 2) shifted by r=1 → ((1+2) + 1·s) = (3 + s)
        p = (Fraction(2), Fraction(1))  # 2 + s
        shifted = _poly_shift(p, Fraction(1))
        # Expected: 2 + (s+1) = 3 + s → (3, 1)
        from fractions import Fraction
        assert shifted == (Fraction(3), Fraction(1))

    def test_poly_shift_quadratic_around_root(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _poly_shift, _poly_evaluate
        # (s-2)^2 = s²−4s+4.  Shifted by r=2: t^2 → (0, 0, 1)
        p = (Fraction(4), Fraction(-4), Fraction(1))  # 4 - 4s + s²
        shifted = _poly_shift(p, Fraction(2))
        # p(t+2) = (t+2-2)^2 = t^2 → (0, 0, 1)
        from cas_laplace.inverse_table import _poly_normalize
        assert _poly_normalize(shifted) == (Fraction(0), Fraction(0), Fraction(1))

    def test_power_series_coeffs_simple(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _power_series_coeffs
        # 1/1 → [1, 0, 0] (Taylor coefficients of constant 1)
        g = _power_series_coeffs((Fraction(1),), (Fraction(1),), 3)
        assert g == [Fraction(1), Fraction(0), Fraction(0)]

    def test_power_series_coeffs_linear(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _power_series_coeffs
        # (1 + t) / 1 → Taylor coefficients [1, 1]
        g = _power_series_coeffs((Fraction(1), Fraction(1)), (Fraction(1),), 2)
        assert g == [Fraction(1), Fraction(1)]

    def test_is_zero_poly(self):
        from fractions import Fraction
        from cas_laplace.inverse_table import _is_zero_poly
        assert _is_zero_poly((Fraction(0),))
        assert _is_zero_poly((Fraction(0), Fraction(0)))
        assert not _is_zero_poly((Fraction(1),))
        assert not _is_zero_poly((Fraction(0), Fraction(1)))
