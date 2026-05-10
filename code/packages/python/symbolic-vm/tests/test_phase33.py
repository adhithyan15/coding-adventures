"""Phase 33 — Trig special values at rational multiples of π.

Phase 33 extends the ``sin_handler``, ``cos_handler``, and ``tan_handler``
from Phase 31 with a fourth rule: **π-multiple exact evaluation**.

When the argument is of the form ``q · %pi`` (where ``q`` is a rational number
with denominator in {1, 2, 3, 4, 6}), the handlers return exact algebraic IR:

- Exact ``IRInteger`` for integer values: 0, ±1.
- Exact ``IRRational`` for half-integer values: ±1/2.
- Exact ``Div(Sqrt(2), 2)`` / ``Div(Sqrt(3), 2)`` for irrational 30/45/60° values.
- Exact ``Sqrt(3)`` / ``Div(Sqrt(3), 3)`` for tan at 60°/30°.

``tan(%pi/2)`` and ``tan(3*%pi/2)`` are undefined — the handler returns the
expression unevaluated rather than raising an exception.

All Phase 31 rules (symmetry, arc-cancellation) continue to work; the π-multiple
rule fires **after** numeric fold, odd/even symmetry, and arc-cancellation.

Test structure
--------------
TestPhase33_SinPi       — sin at all 12 sector angles (0, π/6, π/4, …, 11π/6)
TestPhase33_CosPi       — cos at all 12 sector angles
TestPhase33_TanPi       — tan at defined sector angles + undef at π/2 / 3π/2
TestPhase33_Extraction  — _try_pi_multiple helper for all supported patterns
TestPhase33_Interaction — π-multiple interacts correctly with odd symmetry
TestPhase33_Regressions — Phase 31 arc-cancel, Phase 32 inv-trig, Phase 30 log/exp
TestPhase33_Macsyma     — end-to-end MACSYMA surface syntax
"""

from __future__ import annotations

from fractions import Fraction

import pytest
from symbolic_ir import (
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ── IR head constants ──────────────────────────────────────────────────────────

SIN_H   = IRSymbol("Sin")
COS_H   = IRSymbol("Cos")
TAN_H   = IRSymbol("Tan")
ASIN_H  = IRSymbol("Asin")
ACOS_H  = IRSymbol("Acos")
ATAN_H  = IRSymbol("Atan")
LOG_H   = IRSymbol("Log")
EXP_H   = IRSymbol("Exp")
NEG_H   = IRSymbol("Neg")
DIV_H   = IRSymbol("Div")
MUL_H   = IRSymbol("Mul")
SQRT_H  = IRSymbol("Sqrt")
SUB_H   = IRSymbol("Sub")
PI_SYM  = IRSymbol("%pi")

X = IRSymbol("x")
Y = IRSymbol("y")

_ZERO = IRInteger(0)
_ONE  = IRInteger(1)
_NEG1 = IRInteger(-1)
_HALF = IRRational(1, 2)
_NEG_HALF = IRRational(-1, 2)


# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_vm() -> VM:
    """VM with the SymbolicBackend (includes Phase 33 handlers)."""
    return VM(backend=SymbolicBackend())


def _eval(vm: VM, node: IRNode) -> IRNode:
    return vm.eval(node)


def _pi_div(n: int) -> IRNode:
    """Construct ``Div(%pi, n)``."""
    return IRApply(DIV_H, (PI_SYM, IRInteger(n)))


def _pi_mul(n: int) -> IRNode:
    """Construct ``Mul(n, %pi)``."""
    return IRApply(MUL_H, (IRInteger(n), PI_SYM))


def _sin(arg: IRNode) -> IRNode:
    return IRApply(SIN_H, (arg,))


def _cos(arg: IRNode) -> IRNode:
    return IRApply(COS_H, (arg,))


def _tan(arg: IRNode) -> IRNode:
    return IRApply(TAN_H, (arg,))


def _is_sqrt_div(result: IRNode, n: int, d: int) -> bool:
    """Check result == Div(Sqrt(n), d)."""
    return (
        isinstance(result, IRApply)
        and result.head == DIV_H
        and len(result.args) == 2
        and isinstance(result.args[0], IRApply)
        and result.args[0].head == SQRT_H
        and len(result.args[0].args) == 1
        and result.args[0].args[0] == IRInteger(n)
        and result.args[1] == IRInteger(d)
    )


def _is_neg_sqrt_div(result: IRNode, n: int, d: int) -> bool:
    """Check result == Neg(Div(Sqrt(n), d))."""
    return (
        isinstance(result, IRApply)
        and result.head == NEG_H
        and len(result.args) == 1
        and _is_sqrt_div(result.args[0], n, d)
    )


def _is_sqrt(result: IRNode, n: int) -> bool:
    """Check result == Sqrt(n)."""
    return (
        isinstance(result, IRApply)
        and result.head == SQRT_H
        and len(result.args) == 1
        and result.args[0] == IRInteger(n)
    )


def _is_neg_sqrt(result: IRNode, n: int) -> bool:
    """Check result == Neg(Sqrt(n))."""
    return (
        isinstance(result, IRApply)
        and result.head == NEG_H
        and len(result.args) == 1
        and _is_sqrt(result.args[0], n)
    )


# ── TestPhase33_SinPi ──────────────────────────────────────────────────────────


class TestPhase33_SinPi:
    """sin at all 12 sector angles of the unit circle."""

    def test_sin_pi(self) -> None:
        """sin(%pi) = 0."""
        vm = _make_vm()
        assert _eval(vm, _sin(PI_SYM)) == _ZERO

    def test_sin_pi_over_2(self) -> None:
        """sin(%pi/2) = 1."""
        vm = _make_vm()
        assert _eval(vm, _sin(_pi_div(2))) == _ONE

    def test_sin_pi_over_6(self) -> None:
        """sin(%pi/6) = 1/2."""
        vm = _make_vm()
        assert _eval(vm, _sin(_pi_div(6))) == _HALF

    def test_sin_pi_over_4(self) -> None:
        """sin(%pi/4) = Div(Sqrt(2), 2)."""
        vm = _make_vm()
        result = _eval(vm, _sin(_pi_div(4)))
        assert _is_sqrt_div(result, 2, 2)

    def test_sin_pi_over_3(self) -> None:
        """sin(%pi/3) = Div(Sqrt(3), 2)."""
        vm = _make_vm()
        result = _eval(vm, _sin(_pi_div(3)))
        assert _is_sqrt_div(result, 3, 2)

    def test_sin_2pi_over_3(self) -> None:
        """sin(2*%pi/3) = Div(Sqrt(3), 2)."""
        vm = _make_vm()
        # 2*%pi/3 = Div(Mul(2,%pi), 3)
        arg = IRApply(DIV_H, (_pi_mul(2), IRInteger(3)))
        result = _eval(vm, _sin(arg))
        assert _is_sqrt_div(result, 3, 2)

    def test_sin_2pi(self) -> None:
        """sin(2*%pi) = 0  (period reduction)."""
        vm = _make_vm()
        assert _eval(vm, _sin(_pi_mul(2))) == _ZERO

    def test_sin_3pi_over_2(self) -> None:
        """sin(3*%pi/2) = -1."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(3), IRInteger(2)))
        assert _eval(vm, _sin(arg)) == _NEG1

    def test_sin_7pi_over_6(self) -> None:
        """sin(7*%pi/6) = -1/2."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(7), IRInteger(6)))
        assert _eval(vm, _sin(arg)) == _NEG_HALF

    def test_sin_5pi_over_4(self) -> None:
        """sin(5*%pi/4) = Neg(Div(Sqrt(2), 2))."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(5), IRInteger(4)))
        result = _eval(vm, _sin(arg))
        assert _is_neg_sqrt_div(result, 2, 2)

    def test_sin_4pi_over_3(self) -> None:
        """sin(4*%pi/3) = Neg(Div(Sqrt(3), 2))."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(4), IRInteger(3)))
        result = _eval(vm, _sin(arg))
        assert _is_neg_sqrt_div(result, 3, 2)

    def test_sin_5pi_over_6(self) -> None:
        """sin(5*%pi/6) = 1/2."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(5), IRInteger(6)))
        assert _eval(vm, _sin(arg)) == _HALF


# ── TestPhase33_CosPi ──────────────────────────────────────────────────────────


class TestPhase33_CosPi:
    """cos at all 12 sector angles of the unit circle."""

    def test_cos_zero(self) -> None:
        """cos(0) = 1 (numeric fold via IRInteger(0))."""
        vm = _make_vm()
        assert _eval(vm, _cos(_ZERO)) == _ONE

    def test_cos_pi(self) -> None:
        """cos(%pi) = -1."""
        vm = _make_vm()
        assert _eval(vm, _cos(PI_SYM)) == _NEG1

    def test_cos_pi_over_2(self) -> None:
        """cos(%pi/2) = 0."""
        vm = _make_vm()
        assert _eval(vm, _cos(_pi_div(2))) == _ZERO

    def test_cos_pi_over_3(self) -> None:
        """cos(%pi/3) = 1/2."""
        vm = _make_vm()
        assert _eval(vm, _cos(_pi_div(3))) == _HALF

    def test_cos_pi_over_4(self) -> None:
        """cos(%pi/4) = Div(Sqrt(2), 2)."""
        vm = _make_vm()
        result = _eval(vm, _cos(_pi_div(4)))
        assert _is_sqrt_div(result, 2, 2)

    def test_cos_pi_over_6(self) -> None:
        """cos(%pi/6) = Div(Sqrt(3), 2)."""
        vm = _make_vm()
        result = _eval(vm, _cos(_pi_div(6)))
        assert _is_sqrt_div(result, 3, 2)

    def test_cos_2pi_over_3(self) -> None:
        """cos(2*%pi/3) = -1/2."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(2), IRInteger(3)))
        assert _eval(vm, _cos(arg)) == _NEG_HALF

    def test_cos_2pi(self) -> None:
        """cos(2*%pi) = 1  (period reduction)."""
        vm = _make_vm()
        assert _eval(vm, _cos(_pi_mul(2))) == _ONE

    def test_cos_3pi_over_2(self) -> None:
        """cos(3*%pi/2) = 0."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(3), IRInteger(2)))
        assert _eval(vm, _cos(arg)) == _ZERO

    def test_cos_5pi_over_6(self) -> None:
        """cos(5*%pi/6) = Neg(Div(Sqrt(3), 2))."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(5), IRInteger(6)))
        result = _eval(vm, _cos(arg))
        assert _is_neg_sqrt_div(result, 3, 2)


# ── TestPhase33_TanPi ──────────────────────────────────────────────────────────


class TestPhase33_TanPi:
    """tan at defined sector angles plus undef at π/2."""

    def test_tan_pi(self) -> None:
        """tan(%pi) = 0  (integer multiple of π)."""
        vm = _make_vm()
        assert _eval(vm, _tan(PI_SYM)) == _ZERO

    def test_tan_pi_over_4(self) -> None:
        """tan(%pi/4) = 1."""
        vm = _make_vm()
        assert _eval(vm, _tan(_pi_div(4))) == _ONE

    def test_tan_pi_over_3(self) -> None:
        """tan(%pi/3) = Sqrt(3)."""
        vm = _make_vm()
        result = _eval(vm, _tan(_pi_div(3)))
        assert _is_sqrt(result, 3)

    def test_tan_pi_over_6(self) -> None:
        """tan(%pi/6) = Div(Sqrt(3), 3)  (rationalised 1/√3)."""
        vm = _make_vm()
        result = _eval(vm, _tan(_pi_div(6)))
        assert _is_sqrt_div(result, 3, 3)

    def test_tan_3pi_over_4(self) -> None:
        """tan(3*%pi/4) = -1."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(3), IRInteger(4)))
        assert _eval(vm, _tan(arg)) == _NEG1

    def test_tan_2pi_over_3(self) -> None:
        """tan(2*%pi/3) = Neg(Sqrt(3))."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(2), IRInteger(3)))
        result = _eval(vm, _tan(arg))
        assert _is_neg_sqrt(result, 3)

    def test_tan_pi_over_2_undef(self) -> None:
        """tan(%pi/2) stays unevaluated — tan is undefined there."""
        vm = _make_vm()
        result = _eval(vm, _tan(_pi_div(2)))
        # Should be Tan(Div(%pi, 2)) — not an integer or rational
        assert isinstance(result, IRApply)
        assert result.head == TAN_H

    def test_tan_3pi_over_2_undef(self) -> None:
        """tan(3*%pi/2) stays unevaluated — tan is undefined there."""
        vm = _make_vm()
        arg = IRApply(DIV_H, (_pi_mul(3), IRInteger(2)))
        result = _eval(vm, _tan(arg))
        assert isinstance(result, IRApply)
        assert result.head == TAN_H


# ── TestPhase33_Extraction ────────────────────────────────────────────────────


class TestPhase33_Extraction:
    """_try_pi_multiple helper handles all documented structural patterns."""

    def _extract(self, arg: IRNode) -> Fraction | None:
        # Import the helper directly from the module
        from symbolic_vm.cas_handlers import _try_pi_multiple  # noqa: PLC0415
        return _try_pi_multiple(arg)

    def test_bare_pi(self) -> None:
        """_try_pi_multiple(%pi) = Fraction(1)."""
        assert self._extract(PI_SYM) == Fraction(1)

    def test_neg_pi(self) -> None:
        """_try_pi_multiple(Neg(%pi)) = Fraction(-1)."""
        assert self._extract(IRApply(NEG_H, (PI_SYM,))) == Fraction(-1)

    def test_mul_n_pi(self) -> None:
        """_try_pi_multiple(Mul(3, %pi)) = Fraction(3)."""
        assert self._extract(IRApply(MUL_H, (IRInteger(3), PI_SYM))) == Fraction(3)

    def test_mul_pi_n(self) -> None:
        """_try_pi_multiple(Mul(%pi, 2)) = Fraction(2)."""
        assert self._extract(IRApply(MUL_H, (PI_SYM, IRInteger(2)))) == Fraction(2)

    def test_div_pi_n(self) -> None:
        """_try_pi_multiple(Div(%pi, 6)) = Fraction(1, 6)."""
        assert self._extract(IRApply(DIV_H, (PI_SYM, IRInteger(6)))) == Fraction(1, 6)

    def test_neg_mul_pi(self) -> None:
        """_try_pi_multiple(Neg(Mul(2, %pi))) = Fraction(-2)."""
        inner = IRApply(MUL_H, (IRInteger(2), PI_SYM))
        assert self._extract(IRApply(NEG_H, (inner,))) == Fraction(-2)

    def test_div_mul_pi_n(self) -> None:
        """_try_pi_multiple(Div(Mul(3, %pi), 4)) = Fraction(3, 4)."""
        inner = IRApply(MUL_H, (IRInteger(3), PI_SYM))
        assert self._extract(IRApply(DIV_H, (inner, IRInteger(4)))) == Fraction(3, 4)

    def test_unsupported_symbol(self) -> None:
        """_try_pi_multiple(x) = None for unknown symbol."""
        assert self._extract(X) is None

    def test_unsupported_add(self) -> None:
        """_try_pi_multiple(Add(%pi, 1)) = None (sum form not supported)."""
        assert self._extract(IRApply(IRSymbol("Add"), (PI_SYM, IRInteger(1)))) is None


# ── TestPhase33_Interaction ───────────────────────────────────────────────────


class TestPhase33_Interaction:
    """π-multiple interacts correctly with Phase 31 symmetry rules."""

    def test_sin_neg_pi_sixth(self) -> None:
        """sin(-(%pi/6)) = -1/2.

        The π-multiple rule fires on the raw arg: _try_pi_multiple(Neg(Div(%pi,6)))
        returns -1/6, which mod 2 = 11/6, and _SIN_PI_TABLE[11/6] = IRRational(-1,2).
        """
        vm = _make_vm()
        result = _eval(vm, _sin(IRApply(NEG_H, (_pi_div(6),))))
        assert result == _NEG_HALF

    def test_cos_neg_pi_third(self) -> None:
        """cos(-(%pi/3)) = cos(%pi/3) = 1/2.

        The π-multiple rule: _try_pi_multiple(Neg(Div(%pi,3))) = -1/3,
        mod 2 = 5/3, _COS_PI_TABLE[5/3] = 1/2 = cos(5π/3) = cos(-π/3) = cos(π/3).
        """
        vm = _make_vm()
        result = _eval(vm, _cos(IRApply(NEG_H, (_pi_div(3),))))
        assert result == _HALF

    def test_tan_neg_pi_over_4(self) -> None:
        """tan(-(%pi/4)) = -1.

        The π-multiple rule handles Neg: sign=-1, q_abs=1/4, q_mod=1/4,
        _TAN_PI_TABLE[1/4]=1, final result = Neg(1).
        """
        vm = _make_vm()
        result = _eval(vm, _tan(IRApply(NEG_H, (_pi_div(4),))))
        assert isinstance(result, IRApply)
        assert result.head == NEG_H
        assert result.args[0] == _ONE

    def test_sin_unevaluated_symbol(self) -> None:
        """sin(x) stays unevaluated — x is not a π-multiple."""
        vm = _make_vm()
        result = _eval(vm, _sin(X))
        assert isinstance(result, IRApply)
        assert result.head == SIN_H


# ── TestPhase33_Regressions ───────────────────────────────────────────────────


class TestPhase33_Regressions:
    """Phase 30/31/32 rules remain intact after Phase 33."""

    def test_log_exp_cancel_regression(self) -> None:
        """log(exp(x)) → x  (Phase 30)."""
        vm = _make_vm()
        assert _eval(vm, IRApply(LOG_H, (IRApply(EXP_H, (X,)),))) == X

    def test_sin_arc_cancel_regression(self) -> None:
        """sin(asin(x)) → x  (Phase 31)."""
        vm = _make_vm()
        assert _eval(vm, _sin(IRApply(ASIN_H, (X,)))) == X

    def test_cos_arc_cancel_regression(self) -> None:
        """cos(acos(x)) → x  (Phase 31)."""
        vm = _make_vm()
        assert _eval(vm, _cos(IRApply(ACOS_H, (X,)))) == X

    def test_asin_neg_regression(self) -> None:
        """asin(-x) → Neg(Asin(x))  (Phase 32)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (IRApply(NEG_H, (X,)),)))
        assert isinstance(result, IRApply) and result.head == NEG_H
        assert result.args[0].head == ASIN_H

    def test_acos_reflection_regression(self) -> None:
        """acos(-x) → Sub(%pi, Acos(x))  (Phase 32)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (X,)),)))
        assert isinstance(result, IRApply)
        assert result.head == SUB_H
        assert result.args[0] == PI_SYM


# ── TestPhase33_Macsyma ───────────────────────────────────────────────────────

_MACSYMA_ZERO = IRInteger(0)


class TestPhase33_Macsyma:
    """End-to-end MACSYMA surface syntax tests for Phase 33 rules.

    All tests skip gracefully when ``macsyma_runtime`` is not installed.
    """

    def _make_vm(self) -> VM:
        pytest.importorskip(
            "macsyma_runtime",
            reason="macsyma-runtime not installed; skipping MACSYMA e2e test",
        )
        from macsyma_compiler.compiler import _STANDARD_FUNCTIONS  # noqa: PLC0415
        from macsyma_runtime import MacsymaBackend  # noqa: PLC0415
        from macsyma_runtime.name_table import (
            extend_compiler_name_table,  # noqa: PLC0415
        )

        extend_compiler_name_table(_STANDARD_FUNCTIONS)
        return VM(MacsymaBackend())

    def _run(self, source: str, vm: VM) -> IRNode:
        from macsyma_compiler import compile_macsyma  # noqa: PLC0415
        from macsyma_parser import parse_macsyma  # noqa: PLC0415

        ast = parse_macsyma(source + ";")
        stmts = compile_macsyma(ast)
        result: IRNode = _MACSYMA_ZERO
        for stmt in stmts:
            result = vm.eval(stmt)
        return result

    @pytest.fixture()
    def vm(self) -> VM:
        return self._make_vm()

    def test_sin_pi_macsyma(self, vm: VM) -> None:
        """sin(%pi) → 0 via MACSYMA."""
        assert self._run("sin(%pi)", vm) == _MACSYMA_ZERO

    def test_cos_pi_macsyma(self, vm: VM) -> None:
        """cos(%pi) → -1 via MACSYMA."""
        result = self._run("cos(%pi)", vm)
        assert result == IRInteger(-1)

    def test_sin_pi_over_6_macsyma(self, vm: VM) -> None:
        """sin(%pi/6) → 1/2 via MACSYMA."""
        result = self._run("sin(%pi/6)", vm)
        assert result == IRRational(1, 2)

    def test_tan_pi_over_4_macsyma(self, vm: VM) -> None:
        """tan(%pi/4) → 1 via MACSYMA."""
        result = self._run("tan(%pi/4)", vm)
        assert result == IRInteger(1)

    def test_sin_pi_over_4_macsyma(self, vm: VM) -> None:
        """sin(%pi/4) → Div(Sqrt(2), 2) via MACSYMA."""
        result = self._run("sin(%pi/4)", vm)
        assert _is_sqrt_div(result, 2, 2)

    def test_cos_2pi_macsyma(self, vm: VM) -> None:
        """cos(2*%pi) → 1 via MACSYMA."""
        result = self._run("cos(2*%pi)", vm)
        assert result == IRInteger(1)
