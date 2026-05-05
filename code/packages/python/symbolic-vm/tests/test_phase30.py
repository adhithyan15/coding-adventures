"""Phase 30 — Algebraic ``log`` and ``exp`` cancellation identities.

Phase 30 adds two new handlers to ``symbolic-vm`` that override the numeric-only
``_elementary``-factory ``Log`` and ``Exp`` handlers from ``handlers.py``:

**log_handler new rules:**

- ``Log(Exp(x))``         → ``x``             (cancellation, always safe for ℝ)
- ``Log(Pow(x, n))``      → ``Mul(n, Log(x))``  (power rule, requires x ≥ 0 assumption)

**exp_handler new rules:**

- ``Exp(Log(x))``         → ``x``             (structural: log(x) requires x > 0)
- ``Exp(Mul(n, Log(x)))`` → ``Pow(x, n)``     (power form; both Mul orderings)

All numeric fold behaviour from the ``_elementary`` factory is preserved:
``exp(0)→1``, ``log(1)→0``, float inputs, etc.

Test structure
--------------
TestPhase30_LogExpCancel  — log(exp(x))→x, including nested/compound args
TestPhase30_ExpLogCancel  — exp(log(x))→x, exp(n*log(x))→x^n
TestPhase30_LogPower      — log(x^n)→n*log(x) assumption-aware
TestPhase30_LogNumeric    — log(1)→0, log(e)≈1, negative/zero unevaluated
TestPhase30_ExpNumeric    — exp(0)→1, exp(1.0)≈e, various numerics
TestPhase30_Regressions   — Phase 29 abs/sqrt, Phase 28 assume, Phase 3
TestPhase30_Macsyma       — end-to-end MACSYMA surface syntax
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    GREATER,
    GREATER_EQUAL,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ── IR head constants ──────────────────────────────────────────────────────────

LOG_HEAD = IRSymbol("Log")
EXP_HEAD = IRSymbol("Exp")
POW = IRSymbol("Pow")
MUL = IRSymbol("Mul")
ABS = IRSymbol("Abs")
NEG = IRSymbol("Neg")
ASSUME = IRSymbol("Assume")
ADD = IRSymbol("Add")

X = IRSymbol("x")
Y = IRSymbol("y")

ZERO = IRInteger(0)
ONE = IRInteger(1)
TWO = IRInteger(2)
THREE = IRInteger(3)

# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_vm() -> VM:
    """Fresh VM with SymbolicBackend."""
    return VM(SymbolicBackend())


def _log(x: IRNode) -> IRApply:
    return IRApply(LOG_HEAD, (x,))


def _exp(x: IRNode) -> IRApply:
    return IRApply(EXP_HEAD, (x,))


def _pow(base: IRNode, exp: IRNode) -> IRApply:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(MUL, (a, b))


def _assume(pred: IRNode) -> IRApply:
    return IRApply(ASSUME, (pred,))


def _pos_pred(sym: IRSymbol) -> IRApply:
    """Build ``x > 0`` predicate."""
    return IRApply(GREATER, (sym, ZERO))


def _nonneg_pred(sym: IRSymbol) -> IRApply:
    """Build ``x >= 0`` predicate."""
    return IRApply(GREATER_EQUAL, (sym, ZERO))


# ── TestPhase30_LogExpCancel ───────────────────────────────────────────────────


class TestPhase30_LogExpCancel:
    """``log(exp(x)) = x`` — cancellation identity, always safe for real domain."""

    def test_log_exp_symbol(self) -> None:
        """log(exp(x)) → x (symbolic, no assumption needed)."""
        vm = _make_vm()
        result = vm.eval(_log(_exp(X)))
        assert result == X

    def test_log_exp_symbol_y(self) -> None:
        """log(exp(y)) → y."""
        vm = _make_vm()
        assert vm.eval(_log(_exp(Y))) == Y

    def test_log_exp_zero(self) -> None:
        """log(exp(0)) → log(1) → 0  (numeric fold after exp)."""
        vm = _make_vm()
        result = vm.eval(_log(_exp(ZERO)))
        assert result == ZERO

    def test_log_exp_integer(self) -> None:
        """log(exp(2)) → 2  (cancellation after exp folds to IRFloat,
        or cancellation before numeric fold fires, depending on eval order).
        Either the cancellation fires (result = 2) or we get float ≈ 2.0."""
        vm = _make_vm()
        result = vm.eval(_log(_exp(TWO)))
        # Accept exact integer 2 or float ≈ 2.0
        if isinstance(result, IRInteger):
            assert result == TWO
        else:
            assert isinstance(result, IRFloat)
            assert abs(result.value - 2.0) < 1e-10

    def test_log_exp_neg_x(self) -> None:
        """log(exp(-x)) → -x  (cancellation applies to negated symbol too)."""
        vm = _make_vm()
        result = vm.eval(_log(_exp(IRApply(NEG, (X,)))))
        # -x evaluates first, then log(exp(-x)) → -x
        assert isinstance(result, IRApply)
        assert result.head == NEG
        assert result.args[0] == X

    def test_log_exp_mul(self) -> None:
        """log(exp(2*x)) → 2*x  (compound argument)."""
        vm = _make_vm()
        two_x = _mul(TWO, X)
        result = vm.eval(_log(_exp(two_x)))
        # The inner 2*x stays as Mul(2,x) or may simplify — we just check
        # that the outer log and exp cancel, leaving the inner intact
        inner = vm.eval(two_x)
        assert result == inner


# ── TestPhase30_ExpLogCancel ───────────────────────────────────────────────────


class TestPhase30_ExpLogCancel:
    """``exp(log(x)) = x`` and ``exp(n*log(x)) = x^n``."""

    def test_exp_log_symbol(self) -> None:
        """exp(log(x)) → x  (structural: log(x) implies x > 0)."""
        vm = _make_vm()
        assert vm.eval(_exp(_log(X))) == X

    def test_exp_log_symbol_y(self) -> None:
        """exp(log(y)) → y."""
        vm = _make_vm()
        assert vm.eval(_exp(_log(Y))) == Y

    def test_exp_log_integer(self) -> None:
        """exp(log(3)) → 3  (cancellation before numeric fold fires)."""
        vm = _make_vm()
        result = vm.eval(_exp(_log(THREE)))
        # Either exact 3 (if cancellation fires first) or float ≈ 3.0
        if isinstance(result, IRInteger):
            assert result == THREE
        else:
            assert isinstance(result, IRFloat)
            assert abs(result.value - 3.0) < 1e-10

    def test_exp_n_log_x_is_pow(self) -> None:
        """exp(2*log(x)) → Pow(x, 2)."""
        vm = _make_vm()
        result = vm.eval(_exp(_mul(TWO, _log(X))))
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == X
        assert result.args[1] == TWO

    def test_exp_log_x_n_commuted(self) -> None:
        """exp(log(x)*3) → Pow(x, 3)  (commuted Mul order)."""
        vm = _make_vm()
        result = vm.eval(_exp(_mul(_log(X), THREE)))
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == X
        assert result.args[1] == THREE

    def test_exp_n_log_y(self) -> None:
        """exp(3*log(y)) → Pow(y, 3)."""
        vm = _make_vm()
        result = vm.eval(_exp(_mul(THREE, _log(Y))))
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == Y
        assert result.args[1] == THREE

    def test_exp_log_structure(self) -> None:
        """exp(log(x)) result is exactly X (not wrapped in anything)."""
        vm = _make_vm()
        result = vm.eval(_exp(_log(X)))
        assert result == X
        assert isinstance(result, IRSymbol)


# ── TestPhase30_LogPower ───────────────────────────────────────────────────────


class TestPhase30_LogPower:
    """``log(x^n) = n * log(x)`` — assumption-aware power rule."""

    def test_log_pow_with_pos_assumption(self) -> None:
        """assume(x > 0); log(x^3) → Mul(3, Log(x))."""
        vm = _make_vm()
        vm.eval(_assume(_pos_pred(X)))
        result = vm.eval(_log(_pow(X, THREE)))
        assert isinstance(result, IRApply)
        assert result.head == MUL
        # Should be Mul(3, Log(x))
        assert THREE in result.args
        log_arg = [
            a for a in result.args if isinstance(a, IRApply) and a.head == LOG_HEAD
        ]
        assert len(log_arg) == 1
        assert log_arg[0].args[0] == X

    def test_log_pow_with_nonneg_assumption(self) -> None:
        """assume(x >= 0); log(x^2) → Mul(2, Log(x))."""
        vm = _make_vm()
        vm.eval(_assume(_nonneg_pred(X)))
        result = vm.eval(_log(_pow(X, TWO)))
        assert isinstance(result, IRApply)
        assert result.head == MUL

    def test_log_pow_no_assumption_unevaluated(self) -> None:
        """log(x^3) → Log(Pow(x,3)) when no assumption — safety first."""
        vm = _make_vm()
        result = vm.eval(_log(_pow(X, THREE)))
        assert isinstance(result, IRApply)
        assert result.head == LOG_HEAD

    def test_log_pow_y_no_assumption_unevaluated(self) -> None:
        """log(y^2) → Log(Pow(y,2)) — no assumption about y."""
        vm = _make_vm()
        result = vm.eval(_log(_pow(Y, TWO)))
        assert isinstance(result, IRApply)
        assert result.head == LOG_HEAD

    def test_log_pow_unrelated_assumption(self) -> None:
        """assume(x > 0); log(y^3) → unevaluated (assumption is for x, not y)."""
        vm = _make_vm()
        vm.eval(_assume(_pos_pred(X)))
        result = vm.eval(_log(_pow(Y, THREE)))
        assert isinstance(result, IRApply)
        assert result.head == LOG_HEAD

    def test_log_pow_power_rule_coefficient(self) -> None:
        """assume(x > 0); log(x^n) result contains n and log(x) as factors."""
        vm = _make_vm()
        vm.eval(_assume(_pos_pred(X)))
        n = IRInteger(5)
        result = vm.eval(_log(_pow(X, n)))
        assert isinstance(result, IRApply)
        assert result.head == MUL
        assert n in result.args


# ── TestPhase30_LogNumeric ─────────────────────────────────────────────────────


class TestPhase30_LogNumeric:
    """Numeric fold for ``Log`` — preserved from ``_elementary`` factory."""

    def test_log_one(self) -> None:
        """log(1) → 0."""
        vm = _make_vm()
        assert vm.eval(_log(ONE)) == ZERO

    def test_log_e_approx(self) -> None:
        """log(e) ≈ 1.0  (e = 2.718…)."""
        vm = _make_vm()
        result = vm.eval(_log(IRFloat(math.e)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0) < 1e-12

    def test_log_float(self) -> None:
        """log(2.0) ≈ 0.693."""
        vm = _make_vm()
        result = vm.eval(_log(IRFloat(2.0)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.log(2.0)) < 1e-12

    def test_log_zero_unevaluated(self) -> None:
        """log(0) → unevaluated (undefined in reals)."""
        vm = _make_vm()
        result = vm.eval(_log(ZERO))
        assert isinstance(result, IRApply)
        assert result.head == LOG_HEAD

    def test_log_negative_unevaluated(self) -> None:
        """log(-1) → unevaluated (undefined in reals)."""
        vm = _make_vm()
        result = vm.eval(_log(IRInteger(-1)))
        assert isinstance(result, IRApply)
        assert result.head == LOG_HEAD


# ── TestPhase30_ExpNumeric ─────────────────────────────────────────────────────


class TestPhase30_ExpNumeric:
    """Numeric fold for ``Exp`` — preserved from ``_elementary`` factory."""

    def test_exp_zero(self) -> None:
        """exp(0) → 1."""
        vm = _make_vm()
        assert vm.eval(_exp(ZERO)) == ONE

    def test_exp_one_float(self) -> None:
        """exp(1.0) ≈ 2.718."""
        vm = _make_vm()
        result = vm.eval(_exp(IRFloat(1.0)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.e) < 1e-12

    def test_exp_neg_float(self) -> None:
        """exp(-1.0) ≈ 0.368."""
        vm = _make_vm()
        result = vm.eval(_exp(IRFloat(-1.0)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.exp(-1.0)) < 1e-12

    def test_exp_integer_folds(self) -> None:
        """exp(2) → IRFloat ≈ 7.389 (not a perfect integer, stays float)."""
        vm = _make_vm()
        result = vm.eval(_exp(TWO))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.exp(2.0)) < 1e-10

    def test_exp_symbolic_unevaluated(self) -> None:
        """exp(x) → Exp(x) unevaluated (x is symbolic, no special pattern)."""
        vm = _make_vm()
        result = vm.eval(_exp(X))
        assert isinstance(result, IRApply)
        assert result.head == EXP_HEAD


# ── TestPhase30_Regressions ────────────────────────────────────────────────────


class TestPhase30_Regressions:
    """Ensure Phase 29 and earlier behaviour is fully preserved."""

    def test_phase29_sqrt_x_squared(self) -> None:
        """Phase 29: sqrt(x^2) → Abs(x)."""
        SQRT_HEAD = IRSymbol("Sqrt")
        vm = _make_vm()
        result = vm.eval(IRApply(SQRT_HEAD, (_pow(X, TWO),)))
        assert isinstance(result, IRApply)
        assert result.head == ABS
        assert result.args[0] == X

    def test_phase29_abs_neg_x(self) -> None:
        """Phase 29: abs(-x) → Abs(x)."""
        vm = _make_vm()
        result = vm.eval(IRApply(ABS, (IRApply(NEG, (X,)),)))
        assert result == IRApply(ABS, (X,))

    def test_phase28_abs_with_assumption(self) -> None:
        """Phase 28: assume(x > 0); abs(x) → x."""
        vm = _make_vm()
        vm.eval(_assume(_pos_pred(X)))
        assert vm.eval(IRApply(ABS, (X,))) == X

    def test_phase3_cos_zero(self) -> None:
        """Phase 3: cos(0) → 1 (elementary function regression)."""
        COS = IRSymbol("Cos")
        vm = _make_vm()
        assert vm.eval(IRApply(COS, (ZERO,))) == ONE

    def test_exp_zero_still_one(self) -> None:
        """exp(0) = 1 preserved from _elementary factory baseline."""
        vm = _make_vm()
        assert vm.eval(_exp(ZERO)) == ONE

    def test_log_one_still_zero(self) -> None:
        """log(1) = 0 preserved from _elementary factory baseline."""
        vm = _make_vm()
        assert vm.eval(_log(ONE)) == ZERO


# ── TestPhase30_Macsyma ────────────────────────────────────────────────────────


class TestPhase30_Macsyma:
    """End-to-end tests via MACSYMA surface syntax.

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
        result: IRNode = ZERO
        for stmt in stmts:
            result = vm.eval(stmt)
        return result

    @pytest.fixture()
    def vm(self) -> VM:
        return self._make_vm()

    def test_log_exp_x_macsyma(self, vm: VM) -> None:
        """log(exp(x)) → x via MACSYMA surface syntax."""
        result = self._run("log(exp(x))", vm)
        assert result == X

    def test_exp_log_x_macsyma(self, vm: VM) -> None:
        """exp(log(x)) → x via MACSYMA surface syntax."""
        result = self._run("exp(log(x))", vm)
        assert result == X

    def test_exp_2_log_x_macsyma(self, vm: VM) -> None:
        """exp(2*log(x)) → x^2 via MACSYMA."""
        result = self._run("exp(2*log(x))", vm)
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == X
        assert result.args[1] == TWO

    def test_log_one_macsyma(self, vm: VM) -> None:
        """log(1) → 0 via MACSYMA."""
        result = self._run("log(1)", vm)
        assert result == ZERO

    def test_exp_zero_macsyma(self, vm: VM) -> None:
        """exp(0) → 1 via MACSYMA."""
        result = self._run("exp(0)", vm)
        assert result == ONE

    def test_log_x_cubed_with_assume_macsyma(self, vm: VM) -> None:
        """assume(x > 0); log(x^3) → 3*log(x) via MACSYMA."""
        self._run("assume(x > 0)", vm)
        result = self._run("log(x^3)", vm)
        assert isinstance(result, IRApply)
        assert result.head == MUL
