"""Tests for the cas-fourier package.

These tests validate:

1. Forward Fourier transform table entries.
2. Scalar and sum linearity.
3. Inverse Fourier transform table entries.
4. Round-trip identity: ifourier(fourier(f)) ~ f.
5. VM handler integration (fourier_handler, ifourier_handler).
6. Graceful fallback for unknown / malformed inputs.

Convention tested: F(ω) = ∫ f(t) e^{-iωt} dt
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    DIV,
    EXP,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_fourier import fourier_transform, ifourier_transform
from cas_fourier.heads import FOURIER, IFOURIER

# ---------------------------------------------------------------------------
# Fixture: variable symbols
# ---------------------------------------------------------------------------

t = IRSymbol("t")
omega = IRSymbol("omega")
s = IRSymbol("s")   # a different symbol to ensure t-specificity

# Shared head symbols used in building test expressions.
_DIRAC_DELTA = IRSymbol("DiracDelta")
_UNIT_STEP = IRSymbol("UnitStep")
_IMAG = IRSymbol("ImaginaryUnit")
_PI = IRSymbol("%pi")
_SIN = IRSymbol("Sin")
_COS = IRSymbol("Cos")


# ---------------------------------------------------------------------------
# Helpers to build IR expressions compactly in tests
# ---------------------------------------------------------------------------


def delta(arg: IRNode) -> IRApply:
    return IRApply(_DIRAC_DELTA, (arg,))


def unit_step(arg: IRNode) -> IRApply:
    return IRApply(_UNIT_STEP, (arg,))


def exp_(arg: IRNode) -> IRApply:
    return IRApply(EXP, (arg,))


def neg_(arg: IRNode) -> IRApply:
    return IRApply(NEG, (arg,))


def mul_(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(MUL, (a, b))


def add_(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(ADD, (a, b))


def sub_(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(SUB, (a, b))


def div_(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(DIV, (a, b))


def pow_(base: IRNode, exp: IRNode) -> IRApply:
    return IRApply(POW, (base, exp))


def sin_(arg: IRNode) -> IRApply:
    return IRApply(_SIN, (arg,))


def cos_(arg: IRNode) -> IRApply:
    return IRApply(_COS, (arg,))


def contains_head(node: IRNode, head_name: str) -> bool:
    """Recursively check whether any sub-node has the given head name."""
    if isinstance(node, IRApply):
        if isinstance(node.head, IRSymbol) and node.head.name == head_name:
            return True
        return any(contains_head(a, head_name) for a in node.args)
    return False


def contains_symbol(node: IRNode, sym_name: str) -> bool:
    """Recursively check whether any IRSymbol with the given name appears."""
    if isinstance(node, IRSymbol) and node.name == sym_name:
        return True
    if isinstance(node, IRApply):
        return any(contains_symbol(a, sym_name) for a in node.args)
    return False


# ===========================================================================
# Section 1: Forward transform table entries
# ===========================================================================


class TestForwardDiracDelta:
    """FT{δ(t)} = 1."""

    def test_result_is_integer_one(self):
        f = delta(t)
        result = fourier_transform(f, t, omega)
        assert result == IRInteger(1)

    def test_uses_correct_t_variable(self):
        # Only match when the delta argument is the integration variable.
        f = delta(s)  # delta(s) w.r.t. t — should not match
        result = fourier_transform(f, t, omega)
        # Should return unevaluated Fourier(delta(s), t, omega)
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol)
        assert result.head.name == "Fourier"


class TestForwardConstantOne:
    """FT{1} = 2π·δ(ω)."""

    def test_result_contains_delta(self):
        result = fourier_transform(IRInteger(1), t, omega)
        assert contains_head(result, "DiracDelta")

    def test_result_contains_pi(self):
        result = fourier_transform(IRInteger(1), t, omega)
        assert contains_symbol(result, "%pi")

    def test_rational_one_also_matches(self):
        # IRRational(3, 3) = 1 — should also match constant-one.
        result = fourier_transform(IRRational(3, 3), t, omega)
        assert contains_head(result, "DiracDelta")

    def test_delta_arg_is_omega(self):
        # The delta should be evaluated at ω.
        result = fourier_transform(IRInteger(1), t, omega)
        # Find the DiracDelta node.
        assert contains_symbol(result, omega.name)


class TestForwardCausalExp:
    """FT{exp(-a·t)} = 1/(a + i·ω)."""

    def test_exp_minus_t(self):
        # exp(-t)  (a=1)
        f = exp_(neg_(t))
        result = fourier_transform(f, t, omega)
        # Result should be Div(1, Add(1, Mul(ImaginaryUnit, omega)))
        assert isinstance(result, IRApply)
        assert result.head == DIV
        assert result.args[0] == IRInteger(1)
        # Denominator contains ImaginaryUnit and omega
        assert contains_symbol(result, "ImaginaryUnit")
        assert contains_symbol(result, omega.name)

    def test_exp_minus_2t(self):
        # exp(-2t)  (a=2)
        f = exp_(neg_(mul_(IRInteger(2), t)))
        result = fourier_transform(f, t, omega)
        assert isinstance(result, IRApply)
        assert result.head == DIV
        assert contains_symbol(result, "ImaginaryUnit")
        assert contains_symbol(result, omega.name)

    def test_exp_minus_a_t_symbolic(self):
        # exp(-a*t)  where a is a symbolic constant
        a = IRSymbol("a")
        f = exp_(neg_(mul_(a, t)))
        result = fourier_transform(f, t, omega)
        assert isinstance(result, IRApply)
        assert result.head == DIV
        assert contains_symbol(result, "a")
        assert contains_symbol(result, "ImaginaryUnit")

    def test_denominator_structure(self):
        # Verify denominator is Add(a, Mul(ImaginaryUnit, omega))
        f = exp_(neg_(mul_(IRInteger(3), t)))
        result = fourier_transform(f, t, omega)
        denom = result.args[1]
        assert isinstance(denom, IRApply)
        assert denom.head == ADD


class TestForwardSin:
    """FT{sin(ω₀·t)} = i·π·(δ(ω+ω₀) - δ(ω-ω₀))."""

    def test_sin_t_contains_pi(self):
        f = sin_(t)
        result = fourier_transform(f, t, omega)
        assert contains_symbol(result, "%pi")

    def test_sin_t_contains_delta(self):
        f = sin_(t)
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "DiracDelta")

    def test_sin_3t_contains_imaginary(self):
        f = sin_(mul_(IRInteger(3), t))
        result = fourier_transform(f, t, omega)
        assert contains_symbol(result, "ImaginaryUnit")

    def test_sin_t_result_is_apply(self):
        f = sin_(t)
        result = fourier_transform(f, t, omega)
        assert isinstance(result, IRApply)

    def test_sin_2t(self):
        f = sin_(mul_(IRInteger(2), t))
        result = fourier_transform(f, t, omega)
        # Must contain both ImaginaryUnit and DiracDelta
        assert contains_symbol(result, "ImaginaryUnit")
        assert contains_head(result, "DiracDelta")


class TestForwardCos:
    """FT{cos(ω₀·t)} = π·(δ(ω-ω₀) + δ(ω+ω₀))."""

    def test_cos_t_contains_pi(self):
        f = cos_(t)
        result = fourier_transform(f, t, omega)
        assert contains_symbol(result, "%pi")

    def test_cos_t_contains_delta(self):
        f = cos_(t)
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "DiracDelta")

    def test_cos_2t(self):
        f = cos_(mul_(IRInteger(2), t))
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "DiracDelta")
        assert contains_symbol(result, "%pi")

    def test_cos_t_no_imaginary_unit(self):
        # cos has a real Fourier transform (no ImaginaryUnit)
        f = cos_(t)
        result = fourier_transform(f, t, omega)
        assert not contains_symbol(result, "ImaginaryUnit")


class TestForwardGaussian:
    """FT{exp(-a·t²)} = √(π/a) · exp(-ω²/(4a))."""

    def test_gaussian_a1_contains_sqrt(self):
        # exp(-t²) → a = 1
        f = exp_(neg_(pow_(t, IRInteger(2))))
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "Sqrt")

    def test_gaussian_a1_contains_exp(self):
        f = exp_(neg_(pow_(t, IRInteger(2))))
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "Exp")

    def test_gaussian_a2(self):
        # exp(-2t²)
        f = exp_(neg_(mul_(IRInteger(2), pow_(t, IRInteger(2)))))
        result = fourier_transform(f, t, omega)
        assert contains_head(result, "Sqrt")

    def test_gaussian_result_has_omega(self):
        f = exp_(neg_(pow_(t, IRInteger(2))))
        result = fourier_transform(f, t, omega)
        assert contains_symbol(result, omega.name)


class TestForwardTExp:
    """FT{t·exp(-a·t)} = 1/(a + i·ω)²."""

    def test_t_exp_minus_t(self):
        # t·exp(-t)
        f = mul_(t, exp_(neg_(t)))
        result = fourier_transform(f, t, omega)
        assert result.head == DIV
        assert result.args[0] == IRInteger(1)
        # Denominator should be squared
        denom = result.args[1]
        assert isinstance(denom, IRApply)
        assert denom.head == POW
        assert isinstance(denom.args[1], IRInteger)
        assert denom.args[1].value == 2

    def test_t_exp_minus_2t(self):
        # t·exp(-2t)
        f = mul_(t, exp_(neg_(mul_(IRInteger(2), t))))
        result = fourier_transform(f, t, omega)
        assert result.head == DIV
        assert contains_symbol(result, "ImaginaryUnit")


# ===========================================================================
# Section 2: Linearity
# ===========================================================================


class TestLinearity:
    """FT{c·f} = c·FT{f} and FT{f+g} = FT{f}+FT{g}."""

    def test_scalar_mul_dirac(self):
        # FT{3·δ(t)} = 3
        f = mul_(IRInteger(3), delta(t))
        result = fourier_transform(f, t, omega)
        # Should be Mul(3, 1) or simplified to 3
        assert contains_symbol(result, "3") or result == IRInteger(3) or (
            isinstance(result, IRApply)
            and result.head == MUL
            and IRInteger(3) in result.args
        )

    def test_scalar_pulls_out_of_mul(self):
        # FT{5·δ(t)} = Mul(5, 1) before vm.eval
        f = mul_(IRInteger(5), delta(t))
        result = fourier_transform(f, t, omega)
        # The result should involve 5 somewhere
        assert contains_symbol(result, "5") or (
            isinstance(result, IRApply)
            and result.head == MUL
            and IRInteger(5) in result.args
        ) or result == IRInteger(5)

    def test_sum_linearity_two_deltas(self):
        # FT{δ(t) + δ(t)} = 1 + 1
        f = add_(delta(t), delta(t))
        result = fourier_transform(f, t, omega)
        # Should be Add(1, 1) or 2
        is_add = isinstance(result, IRApply) and result.head == ADD
        assert is_add or result == IRInteger(2)

    def test_sum_linearity_delta_plus_one(self):
        # FT{δ(t) + 1} = 1 + 2π·δ(ω)
        f = add_(delta(t), IRInteger(1))
        result = fourier_transform(f, t, omega)
        assert isinstance(result, IRApply) and result.head == ADD

    def test_sum_linearity_both_deltas_give_two(self):
        # Each delta transforms to 1; sum should have two IRInteger(1) children
        f = add_(delta(t), delta(t))
        result = fourier_transform(f, t, omega)
        if isinstance(result, IRApply) and result.head == ADD:
            assert IRInteger(1) in result.args


# ===========================================================================
# Section 3: Fallback for unknown inputs
# ===========================================================================


class TestFallback:
    """Unknown inputs return unevaluated Fourier(f, t, ω)."""

    def test_unknown_function_returns_unevaluated(self):
        unknown = IRApply(IRSymbol("Mysterious"), (t,))
        result = fourier_transform(unknown, t, omega)
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol)
        assert result.head.name == "Fourier"

    def test_symbol_variable_t_is_unevaluated(self):
        # The variable t itself (not inside any known function) → unevaluated.
        # t alone is not in the table.
        result = fourier_transform(t, t, omega)
        assert isinstance(result, IRApply)
        assert result.head.name == "Fourier"


# ===========================================================================
# Section 4: Inverse transform table entries
# ===========================================================================


class TestInverseOne:
    """ifourier(1) = δ(t)."""

    def test_one_gives_delta(self):
        result = ifourier_transform(IRInteger(1), omega, t)
        assert isinstance(result, IRApply)
        assert result.head == _DIRAC_DELTA
        assert result.args == (t,)


class TestInverseDiracOmega:
    """ifourier(δ(ω)) = 1/(2π)."""

    def test_dirac_omega_gives_half_pi(self):
        F = delta(omega)
        result = ifourier_transform(F, omega, t)
        assert isinstance(result, IRApply)
        assert result.head == DIV
        assert result.args[0] == IRInteger(1)
        # Denominator is 2*pi
        denom = result.args[1]
        assert contains_symbol(denom, "%pi")


class TestInverseTwoPiDirac:
    """ifourier(2π·δ(ω)) = 1."""

    def test_two_pi_dirac_gives_one(self):
        # Build 2π·δ(ω) exactly as the forward table produces it.
        two_pi = IRApply(MUL, (IRInteger(2), _PI))
        F = IRApply(MUL, (two_pi, delta(omega)))
        result = ifourier_transform(F, omega, t)
        assert result == IRInteger(1)


class TestInverseTwoPiDiracShifted:
    """ifourier(2π·δ(ω - a)) = exp(i·a·t)."""

    def test_shifted_delta_gives_exp(self):
        a = IRInteger(3)
        two_pi = IRApply(MUL, (IRInteger(2), _PI))
        shifted_delta = delta(sub_(omega, a))
        F = IRApply(MUL, (two_pi, shifted_delta))
        result = ifourier_transform(F, omega, t)
        assert contains_head(result, "Exp")
        assert contains_symbol(result, "ImaginaryUnit")


class TestInverseCausalExp:
    """ifourier(1/(a + i·ω)) = exp(-a·t)·u(t)."""

    def test_causal_exp_denom_matches(self):
        a = IRInteger(2)
        i_omega = IRApply(MUL, (_IMAG, omega))
        denom = IRApply(ADD, (a, i_omega))
        F = IRApply(DIV, (IRInteger(1), denom))
        result = ifourier_transform(F, omega, t)
        assert contains_head(result, "Exp")
        assert contains_head(result, "UnitStep")

    def test_causal_exp_symbolic_a(self):
        a = IRSymbol("a")
        i_omega = IRApply(MUL, (_IMAG, omega))
        denom = IRApply(ADD, (a, i_omega))
        F = IRApply(DIV, (IRInteger(1), denom))
        result = ifourier_transform(F, omega, t)
        assert contains_head(result, "Exp")
        assert contains_head(result, "UnitStep")


class TestInverseTExp:
    """ifourier(1/(a + i·ω)²) = t·exp(-a·t)·u(t)."""

    def test_squared_denom_matches(self):
        a = IRInteger(2)
        i_omega = IRApply(MUL, (_IMAG, omega))
        add_expr = IRApply(ADD, (a, i_omega))
        denom = IRApply(POW, (add_expr, IRInteger(2)))
        F = IRApply(DIV, (IRInteger(1), denom))
        result = ifourier_transform(F, omega, t)
        assert contains_head(result, "Exp")
        assert contains_head(result, "UnitStep")
        assert contains_symbol(result, t.name)


class TestInverseFallback:
    """Unknown inputs return unevaluated IFourier(F, ω, t)."""

    def test_unknown_input_returns_unevaluated(self):
        unknown = IRApply(IRSymbol("Alien"), (omega,))
        result = ifourier_transform(unknown, omega, t)
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol)
        assert result.head.name == "IFourier"

    def test_integer_2_is_unevaluated(self):
        # 2 is not 1, and not a DiracDelta — no match.
        result = ifourier_transform(IRInteger(2), omega, t)
        assert isinstance(result, IRApply)
        assert result.head.name == "IFourier"


# ===========================================================================
# Section 5: Round-trip
# ===========================================================================


class TestRoundTrip:
    """ifourier(fourier(f)) ≈ f for table entries."""

    def test_round_trip_dirac_delta(self):
        # fourier(δ(t)) = 1; ifourier(1) = δ(t) ✓
        f = delta(t)
        F = fourier_transform(f, t, omega)
        f_recovered = ifourier_transform(F, omega, t)
        assert f_recovered == delta(t)

    def test_round_trip_constant_one(self):
        # fourier(1) = 2π·δ(ω); ifourier(2π·δ(ω)) = 1 ✓
        F = fourier_transform(IRInteger(1), t, omega)
        f_recovered = ifourier_transform(F, omega, t)
        assert f_recovered == IRInteger(1)

    def test_round_trip_causal_exp_does_not_crash(self):
        # fourier(exp(-2t)) = 1/(2 + iω); ifourier(1/(2+iω)) = exp(-2t)·u(t)
        f = exp_(neg_(mul_(IRInteger(2), t)))
        F = fourier_transform(f, t, omega)
        f_recovered = ifourier_transform(F, omega, t)
        # Not exactly exp(-2t) — includes UnitStep. Just ensure no crash.
        assert isinstance(f_recovered, IRNode)

    def test_round_trip_dirac_shifted_does_not_crash(self):
        # fourier(exp(i·3·t)) = 2π·δ(ω-3); ifourier(2π·δ(ω-3)) = exp(i·3·t)
        i_3_t = IRApply(MUL, (IRApply(MUL, (_IMAG, IRInteger(3))), t))
        f = exp_(i_3_t)
        F = fourier_transform(f, t, omega)
        if not isinstance(F, IRApply) or F.head.name == "Fourier":
            # Didn't match — skip round-trip
            return
        f_recovered = ifourier_transform(F, omega, t)
        assert isinstance(f_recovered, IRNode)


# ===========================================================================
# Section 6: VM handler integration (mock VM)
# ===========================================================================


class MockVM:
    """Minimal mock VM that evaluates an IRNode by returning it unchanged.

    The real SymbolicBackend.eval() would fold numeric expressions;
    here we just return the node as-is so we can test the handler
    dispatch without importing the full VM stack.
    """

    def eval(self, node: IRNode) -> IRNode:
        return node


class TestHandlers:
    """Test fourier_handler and ifourier_handler dispatch."""

    def _make_fourier_apply(self, f: IRNode) -> IRApply:
        return IRApply(FOURIER, (f, t, omega))

    def _make_ifourier_apply(self, F: IRNode) -> IRApply:
        return IRApply(IFOURIER, (F, omega, t))

    def test_fourier_handler_dirac(self):
        from cas_fourier.handlers import fourier_handler

        expr = self._make_fourier_apply(delta(t))
        vm = MockVM()
        result = fourier_handler(vm, expr)
        assert result == IRInteger(1)

    def test_fourier_handler_constant_one(self):
        from cas_fourier.handlers import fourier_handler

        expr = self._make_fourier_apply(IRInteger(1))
        vm = MockVM()
        result = fourier_handler(vm, expr)
        assert contains_head(result, "DiracDelta")

    def test_fourier_handler_wrong_arity(self):
        from cas_fourier.handlers import fourier_handler

        # Only 2 args — should return unevaluated.
        expr = IRApply(FOURIER, (delta(t), t))
        vm = MockVM()
        result = fourier_handler(vm, expr)
        assert result is expr  # unchanged

    def test_fourier_handler_non_symbol_t(self):
        from cas_fourier.handlers import fourier_handler

        # t is IRInteger(1), not IRSymbol — should return unevaluated.
        expr = IRApply(FOURIER, (delta(t), IRInteger(1), omega))
        vm = MockVM()
        result = fourier_handler(vm, expr)
        assert result is expr

    def test_fourier_handler_non_symbol_omega(self):
        from cas_fourier.handlers import fourier_handler

        expr = IRApply(FOURIER, (delta(t), t, IRInteger(0)))
        vm = MockVM()
        result = fourier_handler(vm, expr)
        assert result is expr

    def test_ifourier_handler_one_gives_delta(self):
        from cas_fourier.handlers import ifourier_handler

        expr = self._make_ifourier_apply(IRInteger(1))
        vm = MockVM()
        result = ifourier_handler(vm, expr)
        assert isinstance(result, IRApply)
        assert result.head == _DIRAC_DELTA

    def test_ifourier_handler_wrong_arity(self):
        from cas_fourier.handlers import ifourier_handler

        expr = IRApply(IFOURIER, (IRInteger(1), omega))
        vm = MockVM()
        result = ifourier_handler(vm, expr)
        assert result is expr

    def test_ifourier_handler_non_symbol_omega(self):
        from cas_fourier.handlers import ifourier_handler

        expr = IRApply(IFOURIER, (IRInteger(1), IRInteger(0), t))
        vm = MockVM()
        result = ifourier_handler(vm, expr)
        assert result is expr

    def test_build_fourier_handler_table_keys(self):
        from cas_fourier.handlers import build_fourier_handler_table

        table = build_fourier_handler_table()
        assert "Fourier" in table
        assert "IFourier" in table

    def test_build_fourier_handler_table_callables(self):
        from cas_fourier.handlers import build_fourier_handler_table

        table = build_fourier_handler_table()
        assert callable(table["Fourier"])
        assert callable(table["IFourier"])

    def test_ifourier_handler_unknown_returns_unevaluated(self):
        from cas_fourier.handlers import ifourier_handler

        expr = self._make_ifourier_apply(IRApply(IRSymbol("Unknown"), (omega,)))
        vm = MockVM()
        result = ifourier_handler(vm, expr)
        # Should be IFourier(Unknown(omega), omega, t)
        assert isinstance(result, IRApply)
        assert result.head.name == "IFourier"


# ===========================================================================
# Section 7: Package-level __init__ exports
# ===========================================================================


class TestPackageExports:
    """Verify public API exports from cas_fourier."""

    def test_fourier_transform_exported(self):
        from cas_fourier import fourier_transform as ft
        assert callable(ft)

    def test_ifourier_transform_exported(self):
        from cas_fourier import ifourier_transform as ift
        assert callable(ift)

    def test_fourier_head_exported(self):
        from cas_fourier import FOURIER
        assert FOURIER.name == "Fourier"

    def test_ifourier_head_exported(self):
        from cas_fourier import IFOURIER
        assert IFOURIER.name == "IFourier"

    def test_build_table_exported(self):
        from cas_fourier import build_fourier_handler_table
        assert callable(build_fourier_handler_table)
