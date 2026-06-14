"""Phase 32 — Inverse trig/hyperbolic odd symmetry.

Phase 32 adds five new handlers to ``symbolic-vm`` that override the numeric-only
``_elementary``-factory ``Asin``, ``Acos``, ``Atan``, ``Asinh``, ``Atanh``
handlers from ``handlers.py``.  One algebraic rule is added per handler on top
of the preserved numeric fold:

**Odd symmetry (asin, atan, asinh, atanh):**

- ``Asin(-x)``  → ``Neg(Asin(x))``
- ``Atan(-x)``  → ``Neg(Atan(x))``
- ``Asinh(-x)`` → ``Neg(Asinh(x))``
- ``Atanh(-x)`` → ``Neg(Atanh(x))``

**Reflection identity (acos):**

- ``Acos(-x)``  → ``Sub(%pi, Acos(x))``

``acosh`` is excluded: domain is ``[1, ∞)``, so ``acosh(-x)`` for positive ``x``
is undefined in the real domain.

All numeric fold behaviour from the ``_elementary`` factory is preserved:
``asin(0)→0``, ``acos(1)→0``, ``atan(0)→0``, ``asinh(0)→0``, ``atanh(0)→0``.

Test structure
--------------
TestPhase32_AsinSymmetry     — asin(-x), double-neg, float, unevaluated
TestPhase32_AcosReflection   — acos(-x)→π-acos(x), acos(-1)=π, double reflection
TestPhase32_AtanSymmetry     — atan(-x), double-neg, float, unevaluated
TestPhase32_HypInvSymmetry   — asinh(-x), atanh(-x); odd; float; double-neg
TestPhase32_Regressions      — Phase 31 sin/cos, Phase 30 log/exp, Phase 31 arc-cancel
TestPhase32_Macsyma          — end-to-end MACSYMA surface syntax
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ── IR head constants ──────────────────────────────────────────────────────────

ASIN_H = IRSymbol("Asin")
ACOS_H = IRSymbol("Acos")
ATAN_H = IRSymbol("Atan")
ASINH_H = IRSymbol("Asinh")
ATANH_H = IRSymbol("Atanh")
SIN_H = IRSymbol("Sin")
COS_H = IRSymbol("Cos")
LOG_H = IRSymbol("Log")
EXP_H = IRSymbol("Exp")
NEG_H = IRSymbol("Neg")
SUB_H = IRSymbol("Sub")
ADD_H = IRSymbol("Add")
PI_SYM = IRSymbol("%pi")

X = IRSymbol("x")
Y = IRSymbol("y")
Z = IRSymbol("z")

_ZERO = IRInteger(0)
_ONE = IRInteger(1)


# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_vm() -> VM:
    """Return a VM with the SymbolicBackend (which includes Phase 32 handlers)."""
    return VM(backend=SymbolicBackend())


def _eval(vm: VM, node: IRNode) -> IRNode:
    return vm.eval(node)


def _is_neg_of(result: IRNode, head: IRSymbol, inner: IRNode) -> bool:
    """Check result is Neg(Head(inner))."""
    return (
        isinstance(result, IRApply)
        and result.head == NEG_H
        and len(result.args) == 1
        and isinstance(result.args[0], IRApply)
        and result.args[0].head == head
        and len(result.args[0].args) == 1
        and result.args[0].args[0] == inner
    )


def _is_apply(result: IRNode, head: IRSymbol, arg: IRNode) -> bool:
    """Check result is Head(arg)."""
    return (
        isinstance(result, IRApply)
        and result.head == head
        and len(result.args) == 1
        and result.args[0] == arg
    )


def _is_pi_minus_acos(result: IRNode, inner: IRNode) -> bool:
    """Check result is Sub(%pi, Acos(inner))."""
    return (
        isinstance(result, IRApply)
        and result.head == SUB_H
        and len(result.args) == 2
        and result.args[0] == PI_SYM
        and isinstance(result.args[1], IRApply)
        and result.args[1].head == ACOS_H
        and len(result.args[1].args) == 1
        and result.args[1].args[0] == inner
    )


# ── TestPhase32_AsinSymmetry ───────────────────────────────────────────────────


class TestPhase32_AsinSymmetry:
    """asin(-x) = -asin(x)  (odd symmetry)."""

    def test_asin_neg_symbol(self) -> None:
        """asin(-x) → Neg(Asin(x))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, ASIN_H, X)

    def test_asin_neg_symbol_y(self) -> None:
        """asin(-y) → Neg(Asin(y))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (IRApply(NEG_H, (Y,)),)))
        assert _is_neg_of(result, ASIN_H, Y)

    def test_asin_neg_float(self) -> None:
        """asin(-0.5) → IRFloat matching math.asin(-0.5)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.asin(-0.5)) < 1e-12

    def test_asin_double_neg(self) -> None:
        """asin(-(-x)) → asin(x)  (double neg collapses via recursion)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(ASIN_H, (neg_neg_x,)))
        assert _is_apply(result, ASIN_H, X)

    def test_asin_numeric_zero(self) -> None:
        """asin(0) → 0 (exact special value preserved)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (_ZERO,)))
        assert result == _ZERO

    def test_asin_pos_symbol_unevaluated(self) -> None:
        """asin(x) stays unevaluated — no rule fires."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASIN_H, (X,)))
        assert _is_apply(result, ASIN_H, X)


# ── TestPhase32_AcosReflection ─────────────────────────────────────────────────


class TestPhase32_AcosReflection:
    """acos(-x) = π - acos(x)  (reflection identity)."""

    def test_acos_neg_symbol(self) -> None:
        """acos(-x) → Sub(%pi, Acos(x))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (X,)),)))
        assert _is_pi_minus_acos(result, X)

    def test_acos_neg_symbol_z(self) -> None:
        """acos(-z) → Sub(%pi, Acos(z))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (Z,)),)))
        assert _is_pi_minus_acos(result, Z)

    def test_acos_neg_float(self) -> None:
        """acos(-0.5) → IRFloat matching math.acos(-0.5) ≈ 2.094."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.acos(-0.5)) < 1e-12

    def test_acos_neg_one_numeric(self) -> None:
        """acos(-1) → IRFloat(π)  (numeric fold: acos(-1) = π)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (_ONE,)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.pi) < 1e-12

    def test_acos_one_zero(self) -> None:
        """acos(1) → 0  (exact special value preserved)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (_ONE,)))
        assert result == _ZERO

    def test_acos_pos_symbol_unevaluated(self) -> None:
        """acos(x) stays unevaluated — no rule fires."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (X,)))
        assert _is_apply(result, ACOS_H, X)

    def test_acos_reflection_structure(self) -> None:
        """Result of acos(-x) is Sub, NOT Neg — confirm it is the reflection form."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ACOS_H, (IRApply(NEG_H, (X,)),)))
        # Must be Sub, not Neg
        assert isinstance(result, IRApply)
        assert result.head == SUB_H
        assert result.head != NEG_H


# ── TestPhase32_AtanSymmetry ───────────────────────────────────────────────────


class TestPhase32_AtanSymmetry:
    """atan(-x) = -atan(x)  (odd symmetry)."""

    def test_atan_neg_symbol(self) -> None:
        """atan(-x) → Neg(Atan(x))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATAN_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, ATAN_H, X)

    def test_atan_neg_float(self) -> None:
        """atan(-1.0) → IRFloat matching math.atan(-1.0)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATAN_H, (IRApply(NEG_H, (IRFloat(1.0),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.atan(-1.0)) < 1e-12

    def test_atan_double_neg(self) -> None:
        """atan(-(-x)) → atan(x)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(ATAN_H, (neg_neg_x,)))
        assert _is_apply(result, ATAN_H, X)

    def test_atan_numeric_zero(self) -> None:
        """atan(0) → 0 (exact special value)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATAN_H, (_ZERO,)))
        assert result == _ZERO

    def test_atan_pos_unevaluated(self) -> None:
        """atan(x) stays unevaluated."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATAN_H, (X,)))
        assert _is_apply(result, ATAN_H, X)


# ── TestPhase32_HypInvSymmetry ─────────────────────────────────────────────────


class TestPhase32_HypInvSymmetry:
    """Odd symmetry for asinh and atanh."""

    def test_asinh_neg_symbol(self) -> None:
        """asinh(-x) → Neg(Asinh(x))  (odd)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASINH_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, ASINH_H, X)

    def test_asinh_neg_float(self) -> None:
        """asinh(-1.0) → float matching math.asinh(-1.0)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASINH_H, (IRApply(NEG_H, (IRFloat(1.0),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.asinh(-1.0)) < 1e-12

    def test_asinh_double_neg(self) -> None:
        """asinh(-(-x)) → asinh(x)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(ASINH_H, (neg_neg_x,)))
        assert _is_apply(result, ASINH_H, X)

    def test_asinh_numeric_zero(self) -> None:
        """asinh(0) → 0 (exact special value)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ASINH_H, (_ZERO,)))
        assert result == _ZERO

    def test_atanh_neg_symbol(self) -> None:
        """atanh(-x) → Neg(Atanh(x))  (odd)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATANH_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, ATANH_H, X)

    def test_atanh_neg_float(self) -> None:
        """atanh(-0.5) → float matching math.atanh(-0.5)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATANH_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.atanh(-0.5)) < 1e-12

    def test_atanh_double_neg(self) -> None:
        """atanh(-(-x)) → atanh(x)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(ATANH_H, (neg_neg_x,)))
        assert _is_apply(result, ATANH_H, X)

    def test_atanh_numeric_zero(self) -> None:
        """atanh(0) → 0 (exact special value)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ATANH_H, (_ZERO,)))
        assert result == _ZERO


# ── TestPhase32_Regressions ────────────────────────────────────────────────────


class TestPhase32_Regressions:
    """Phase 29/30/31 rules remain intact after Phase 32."""

    def test_sin_neg_regression(self) -> None:
        """sin(-x) → Neg(Sin(x))  (Phase 31 regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, SIN_H, X)

    def test_cos_neg_regression(self) -> None:
        """cos(-x) → cos(x)  (Phase 31 even regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(NEG_H, (X,)),)))
        assert _is_apply(result, COS_H, X)

    def test_log_exp_cancel_regression(self) -> None:
        """log(exp(x)) → x  (Phase 30 regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(LOG_H, (IRApply(EXP_H, (X,)),)))
        assert result == X

    def test_sin_asin_cancel_regression(self) -> None:
        """sin(asin(x)) → x  (Phase 31 arc-cancel regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRApply(ASIN_H, (X,)),)))
        assert result == X

    def test_tan_atan_cancel_regression(self) -> None:
        """tan(atan(x)) → x  (Phase 31 arc-cancel regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(IRSymbol("Tan"), (IRApply(ATAN_H, (X,)),)))
        assert result == X


# ── TestPhase32_Macsyma ────────────────────────────────────────────────────────

_MACSYMA_ZERO = IRInteger(0)


class TestPhase32_Macsyma:
    """End-to-end MACSYMA surface syntax tests for Phase 32 rules.

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

    def test_asin_neg_x_macsyma(self, vm: VM) -> None:
        """asin(-x) → -asin(x) via MACSYMA."""
        result = self._run("asin(-x)", vm)
        result_str = str(result).replace(" ", "")
        assert "-" in result_str or "Neg" in result_str

    def test_acos_neg_x_macsyma(self, vm: VM) -> None:
        """acos(-x) → %pi - acos(x) via MACSYMA (reflection — not negation)."""
        result = self._run("acos(-x)", vm)
        result_str = str(result)
        # Result should reference pi and acos, NOT be a raw negation
        assert "pi" in result_str.lower() or "Pi" in result_str or "%pi" in result_str
        assert not result_str.strip().startswith("-")

    def test_atan_neg_x_macsyma(self, vm: VM) -> None:
        """atan(-x) → -atan(x) via MACSYMA."""
        result = self._run("atan(-x)", vm)
        result_str = str(result).replace(" ", "")
        assert "-" in result_str or "Neg" in result_str

    def test_asinh_neg_x_macsyma(self, vm: VM) -> None:
        """asinh(-x) → -asinh(x) via MACSYMA."""
        result = self._run("asinh(-x)", vm)
        result_str = str(result).replace(" ", "")
        assert "-" in result_str or "Neg" in result_str

    def test_atanh_neg_x_macsyma(self, vm: VM) -> None:
        """atanh(-x) → -atanh(x) via MACSYMA."""
        result = self._run("atanh(-x)", vm)
        result_str = str(result).replace(" ", "")
        assert "-" in result_str or "Neg" in result_str

    def test_asin_zero_macsyma(self, vm: VM) -> None:
        """asin(0) → 0 via MACSYMA (exact special value)."""
        result = self._run("asin(0)", vm)
        assert result == _MACSYMA_ZERO
