"""Phase 31 — Trig symmetry and arc-cancellation identities.

Phase 31 adds six new handlers to ``symbolic-vm`` that override the numeric-only
``_elementary``-factory ``Sin``, ``Cos``, ``Tan``, ``Sinh``, ``Cosh``, ``Tanh``
handlers from ``handlers.py``.  Two algebraic rule families are added on top of
the existing numeric fold behaviour:

**Negation symmetry:**

- ``Sin(-x)``  → ``Neg(Sin(x))``   (odd function)
- ``Cos(-x)``  → ``Cos(x)``        (even function — NEG stripped)
- ``Tan(-x)``  → ``Neg(Tan(x))``   (odd function)
- ``Sinh(-x)`` → ``Neg(Sinh(x))``  (odd function)
- ``Cosh(-x)`` → ``Cosh(x)``       (even function — NEG stripped)
- ``Tanh(-x)`` → ``Neg(Tanh(x))``  (odd function)

**Arc-function cancellation:**

- ``Sin(Asin(x))``  → ``x``
- ``Cos(Acos(x))``  → ``x``
- ``Tan(Atan(x))``  → ``x``
- ``Sinh(Asinh(x))``→ ``x``
- ``Cosh(Acosh(x))``→ ``x``
- ``Tanh(Atanh(x))``→ ``x``

All numeric fold behaviour from the ``_elementary`` factory is preserved:
``sin(0)→0``, ``cos(0)→1``, float inputs, etc.

Test structure
--------------
TestPhase31_SinSymmetry  — sin(-x), double-neg, sin(-expr), sin(-0.5)
TestPhase31_CosSymmetry  — cos(-x), double-neg, cos(-expr), cos(-0.5)
TestPhase31_TanSymmetry  — tan(-x), double-neg, tan(-expr), tan(-0.5)
TestPhase31_HypSymmetry  — sinh/cosh/tanh odd/even on -x and -0.5
TestPhase31_ArcCancel    — all 6 arc-cancellations + compound inner args
TestPhase31_Regressions  — Phase 30 log/exp, Phase 29 abs/sqrt, Phase 3 exp
TestPhase31_Macsyma      — end-to-end MACSYMA surface syntax
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

SIN_H = IRSymbol("Sin")
COS_H = IRSymbol("Cos")
TAN_H = IRSymbol("Tan")
SINH_H = IRSymbol("Sinh")
COSH_H = IRSymbol("Cosh")
TANH_H = IRSymbol("Tanh")
ASIN_H = IRSymbol("Asin")
ACOS_H = IRSymbol("Acos")
ATAN_H = IRSymbol("Atan")
ASINH_H = IRSymbol("Asinh")
ACOSH_H = IRSymbol("Acosh")
ATANH_H = IRSymbol("Atanh")
NEG_H = IRSymbol("Neg")
ADD_H = IRSymbol("Add")
MUL_H = IRSymbol("Mul")
LOG_H = IRSymbol("Log")
EXP_H = IRSymbol("Exp")
ABS_H = IRSymbol("Abs")
POW_H = IRSymbol("Pow")

X = IRSymbol("x")
Y = IRSymbol("y")
Z = IRSymbol("z")


# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_vm() -> VM:
    """Return a VM with the SymbolicBackend (which includes Phase 31 handlers)."""
    return VM(backend=SymbolicBackend())


def _eval(vm: VM, node: IRNode) -> IRNode:
    return vm.eval(node)


def _is_neg_of(result: IRNode, inner_head: IRSymbol, inner_arg: IRNode) -> bool:
    """Check result is Neg(Head(arg))."""
    return (
        isinstance(result, IRApply)
        and result.head == NEG_H
        and len(result.args) == 1
        and isinstance(result.args[0], IRApply)
        and result.args[0].head == inner_head
        and len(result.args[0].args) == 1
        and result.args[0].args[0] == inner_arg
    )


def _is_apply(result: IRNode, head: IRSymbol, arg: IRNode) -> bool:
    """Check result is Head(arg)."""
    return (
        isinstance(result, IRApply)
        and result.head == head
        and len(result.args) == 1
        and result.args[0] == arg
    )


# ── TestPhase31_SinSymmetry ────────────────────────────────────────────────────


class TestPhase31_SinSymmetry:
    """sin(-x) = -sin(x)  (odd symmetry)."""

    def test_sin_neg_symbol(self) -> None:
        """sin(-x) → Neg(Sin(x))."""
        vm = _make_vm()
        neg_x = IRApply(NEG_H, (X,))
        result = _eval(vm, IRApply(SIN_H, (neg_x,)))
        assert _is_neg_of(result, SIN_H, X)

    def test_sin_neg_symbol_y(self) -> None:
        """sin(-y) → Neg(Sin(y))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRApply(NEG_H, (Y,)),)))
        assert _is_neg_of(result, SIN_H, Y)

    def test_sin_neg_float(self) -> None:
        """sin(-0.5) → float, not unevaluated (numeric fold fires after neg strip)."""
        vm = _make_vm()
        neg_half = IRApply(NEG_H, (IRFloat(0.5),))
        result = _eval(vm, IRApply(SIN_H, (neg_half,)))
        # -0.5 is numeric so sin(-0.5) folds via Rule 2 (neg) → -sin(0.5) → -float
        # OR via numeric path directly — either way must be IRFloat
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.sin(-0.5)) < 1e-12

    def test_sin_double_neg(self) -> None:
        """sin(-(-x)) = sin(x)  (double negation collapses via recursion)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(SIN_H, (neg_neg_x,)))
        # Neg(Neg(x)) evaluates to x, so result is Sin(x)
        assert _is_apply(result, SIN_H, X)

    def test_sin_neg_add_expr(self) -> None:
        """sin(-(x+y)) → Neg(Sin(x+y))."""
        vm = _make_vm()
        add_xy = IRApply(ADD_H, (X, Y))
        neg_add = IRApply(NEG_H, (add_xy,))
        result = _eval(vm, IRApply(SIN_H, (neg_add,)))
        assert isinstance(result, IRApply)
        assert result.head == NEG_H

    def test_sin_pos_symbol_unevaluated(self) -> None:
        """sin(x) stays unevaluated — no symmetry rule should fire."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (X,)))
        assert _is_apply(result, SIN_H, X)


# ── TestPhase31_CosSymmetry ────────────────────────────────────────────────────


class TestPhase31_CosSymmetry:
    """cos(-x) = cos(x)  (even symmetry — NEG stripped)."""

    def test_cos_neg_symbol(self) -> None:
        """cos(-x) → Cos(x)  (NEG stripped, not negated)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(NEG_H, (X,)),)))
        assert _is_apply(result, COS_H, X)

    def test_cos_neg_symbol_z(self) -> None:
        """cos(-z) → Cos(z)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(NEG_H, (Z,)),)))
        assert _is_apply(result, COS_H, Z)

    def test_cos_neg_float(self) -> None:
        """cos(-0.5) → float matching math.cos(-0.5)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.cos(-0.5)) < 1e-12

    def test_cos_double_neg(self) -> None:
        """cos(-(-x)) = cos(x)  (strip double neg, same as cos(x))."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(COS_H, (neg_neg_x,)))
        assert _is_apply(result, COS_H, X)

    def test_cos_neg_add_expr(self) -> None:
        """cos(-(x+y)) → Cos(x+y)  (no outer NEG for even function)."""
        vm = _make_vm()
        add_xy = IRApply(ADD_H, (X, Y))
        result = _eval(vm, IRApply(COS_H, (IRApply(NEG_H, (add_xy,)),)))
        assert isinstance(result, IRApply)
        assert result.head == COS_H
        # Must NOT be wrapped in Neg
        assert result.head != NEG_H

    def test_cos_pos_symbol_unevaluated(self) -> None:
        """cos(x) stays unevaluated."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (X,)))
        assert _is_apply(result, COS_H, X)


# ── TestPhase31_TanSymmetry ────────────────────────────────────────────────────


class TestPhase31_TanSymmetry:
    """tan(-x) = -tan(x)  (odd symmetry)."""

    def test_tan_neg_symbol(self) -> None:
        """tan(-x) → Neg(Tan(x))."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TAN_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, TAN_H, X)

    def test_tan_neg_float(self) -> None:
        """tan(-0.5) → float matching math.tan(-0.5)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TAN_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.tan(-0.5)) < 1e-12

    def test_tan_double_neg(self) -> None:
        """tan(-(-x)) = tan(x)."""
        vm = _make_vm()
        neg_neg_x = IRApply(NEG_H, (IRApply(NEG_H, (X,)),))
        result = _eval(vm, IRApply(TAN_H, (neg_neg_x,)))
        assert _is_apply(result, TAN_H, X)

    def test_tan_pos_unevaluated(self) -> None:
        """tan(x) stays unevaluated."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TAN_H, (X,)))
        assert _is_apply(result, TAN_H, X)


# ── TestPhase31_HypSymmetry ────────────────────────────────────────────────────


class TestPhase31_HypSymmetry:
    """Odd/even symmetry for sinh, cosh, tanh."""

    def test_sinh_neg_symbol(self) -> None:
        """sinh(-x) → Neg(Sinh(x))  (odd)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SINH_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, SINH_H, X)

    def test_sinh_neg_float(self) -> None:
        """sinh(-1.0) → float matching math.sinh(-1.0)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SINH_H, (IRApply(NEG_H, (IRFloat(1.0),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.sinh(-1.0)) < 1e-12

    def test_cosh_neg_symbol(self) -> None:
        """cosh(-x) → Cosh(x)  (even — NEG stripped)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COSH_H, (IRApply(NEG_H, (X,)),)))
        assert _is_apply(result, COSH_H, X)

    def test_cosh_neg_float(self) -> None:
        """cosh(-1.0) → float matching math.cosh(-1.0) = math.cosh(1.0)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COSH_H, (IRApply(NEG_H, (IRFloat(1.0),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.cosh(1.0)) < 1e-12

    def test_tanh_neg_symbol(self) -> None:
        """tanh(-x) → Neg(Tanh(x))  (odd)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TANH_H, (IRApply(NEG_H, (X,)),)))
        assert _is_neg_of(result, TANH_H, X)

    def test_tanh_neg_float(self) -> None:
        """tanh(-0.5) → float matching math.tanh(-0.5)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TANH_H, (IRApply(NEG_H, (IRFloat(0.5),)),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.tanh(-0.5)) < 1e-12


# ── TestPhase31_ArcCancel ──────────────────────────────────────────────────────


class TestPhase31_ArcCancel:
    """f(arc_f(x)) = x  — structural arc-cancellation for all 6 functions."""

    def test_sin_asin_cancel(self) -> None:
        """sin(asin(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRApply(ASIN_H, (X,)),)))
        assert result == X

    def test_cos_acos_cancel(self) -> None:
        """cos(acos(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(ACOS_H, (X,)),)))
        assert result == X

    def test_tan_atan_cancel(self) -> None:
        """tan(atan(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TAN_H, (IRApply(ATAN_H, (X,)),)))
        assert result == X

    def test_sinh_asinh_cancel(self) -> None:
        """sinh(asinh(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SINH_H, (IRApply(ASINH_H, (X,)),)))
        assert result == X

    def test_cosh_acosh_cancel(self) -> None:
        """cosh(acosh(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COSH_H, (IRApply(ACOSH_H, (X,)),)))
        assert result == X

    def test_tanh_atanh_cancel(self) -> None:
        """tanh(atanh(x)) → x."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TANH_H, (IRApply(ATANH_H, (X,)),)))
        assert result == X

    def test_sin_asin_compound_inner(self) -> None:
        """sin(asin(x+y)) → x+y  (compound inner expression)."""
        vm = _make_vm()
        inner = IRApply(ADD_H, (X, Y))
        result = _eval(vm, IRApply(SIN_H, (IRApply(ASIN_H, (inner,)),)))
        # inner is evaluated (add stays unevaluated as symbolic), result == inner
        assert isinstance(result, IRApply)
        assert result.head == ADD_H

    def test_tanh_atanh_symbol_y(self) -> None:
        """tanh(atanh(y)) → y."""
        vm = _make_vm()
        result = _eval(vm, IRApply(TANH_H, (IRApply(ATANH_H, (Y,)),)))
        assert result == Y

    def test_sin_acos_no_cancel(self) -> None:
        """sin(acos(x)) stays unevaluated — different arc function."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRApply(ACOS_H, (X,)),)))
        # Should NOT cancel — sin(acos(x)) ≠ x in general
        assert isinstance(result, IRApply)
        assert result.head == SIN_H

    def test_cos_asin_no_cancel(self) -> None:
        """cos(asin(x)) stays unevaluated — different arc function."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRApply(ASIN_H, (X,)),)))
        assert isinstance(result, IRApply)
        assert result.head == COS_H


# ── TestPhase31_Regressions ────────────────────────────────────────────────────


class TestPhase31_Regressions:
    """Ensure Phase 29/30 rules still work after Phase 31 additions."""

    def test_log_exp_cancel_regression(self) -> None:
        """log(exp(x)) → x  (Phase 30 regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(LOG_H, (IRApply(EXP_H, (X,)),)))
        assert result == X

    def test_exp_log_cancel_regression(self) -> None:
        """exp(log(x)) → x  (Phase 30 regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(EXP_H, (IRApply(LOG_H, (X,)),)))
        assert result == X

    def test_abs_neg_regression(self) -> None:
        """abs(-x) → abs(x)  (Phase 29 regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(ABS_H, (IRApply(NEG_H, (X,)),)))
        assert isinstance(result, IRApply)
        assert result.head == ABS_H
        assert result.args[0] == X

    def test_exp_numeric_regression(self) -> None:
        """exp(0) → 1  (Phase 30 numeric fold regression)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(EXP_H, (IRInteger(0),)))
        assert result == IRInteger(1)

    def test_sin_numeric_zero(self) -> None:
        """sin(0) → 0.0  (numeric fold special value preserved)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(SIN_H, (IRInteger(0),)))
        # numeric fold returns IRFloat(0.0)
        assert isinstance(result, (IRInteger, IRFloat))
        val = result.value if isinstance(result, (IRInteger, IRFloat)) else None
        assert val is not None and abs(float(val)) < 1e-15

    def test_cos_numeric_zero(self) -> None:
        """cos(0) → 1.0  (numeric fold special value preserved)."""
        vm = _make_vm()
        result = _eval(vm, IRApply(COS_H, (IRInteger(0),)))
        assert isinstance(result, (IRInteger, IRFloat))
        assert abs(float(result.value) - 1.0) < 1e-12


# ── TestPhase31_Macsyma ────────────────────────────────────────────────────────

_ZERO = IRInteger(0)


class TestPhase31_Macsyma:
    """End-to-end MACSYMA surface syntax tests for Phase 31 rules.

    All tests skip gracefully when ``macsyma_runtime`` is not installed
    (``pytest.importorskip`` returns None and the test is skipped).
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
        result: IRNode = _ZERO
        for stmt in stmts:
            result = vm.eval(stmt)
        return result

    @pytest.fixture()
    def vm(self) -> VM:
        return self._make_vm()

    def test_sin_neg_x_macsyma(self, vm: VM) -> None:
        """sin(-x) → -sin(x) via MACSYMA surface syntax."""
        result = self._run("sin(-x)", vm)
        result_str = str(result).replace(" ", "")
        # Result should involve negation
        assert "-" in result_str or "Neg" in result_str

    def test_cos_neg_x_macsyma(self, vm: VM) -> None:
        """cos(-x) → cos(x) via MACSYMA surface syntax (even — no negation)."""
        result = self._run("cos(-x)", vm)
        result_str = str(result).replace(" ", "")
        # Should contain cos but NOT be prefixed with a negation
        assert "cos" in result_str.lower() or "Cos" in result_str
        assert not result_str.startswith("-")

    def test_sin_asin_cancel_macsyma(self, vm: VM) -> None:
        """sin(asin(x)) → x via MACSYMA surface syntax."""
        result = self._run("sin(asin(x))", vm)
        assert result == X

    def test_tanh_atanh_cancel_macsyma(self, vm: VM) -> None:
        """tanh(atanh(y)) → y via MACSYMA surface syntax."""
        result = self._run("tanh(atanh(y))", vm)
        assert result == Y

    def test_tan_neg_x_macsyma(self, vm: VM) -> None:
        """tan(-x) → -tan(x) via MACSYMA surface syntax."""
        result = self._run("tan(-x)", vm)
        result_str = str(result).replace(" ", "")
        assert "-" in result_str or "Neg" in result_str

    def test_cosh_neg_x_macsyma(self, vm: VM) -> None:
        """cosh(-x) → cosh(x) via MACSYMA surface syntax (even — no negation)."""
        result = self._run("cosh(-x)", vm)
        result_str = str(result)
        assert "cosh" in result_str.lower() or "Cosh" in result_str
        assert not str(result).strip().startswith("-")
