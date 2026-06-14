"""Phase 29 — Algebraic ``abs`` and ``sqrt`` simplification.

Phase 29 extends two handlers in ``symbolic-vm`` with pure algebraic rules
that require no user assumptions — they hold for all real inputs by the
structure of the expression alone.

**abs_handler extensions (Rules 4a-4d):**

- ``Abs(Abs(x))``        → ``Abs(x)``           (idempotency)
- ``Abs(Neg(x))``        → ``Abs(x)``           (even function: |−x| = |x|)
- ``Abs(Mul(-1, x))``    → ``Abs(x)``           (−x encoded as Mul after eval)
- ``Abs(Pow(x, 2k))``    → ``Pow(x, 2k)``       (x²ᵏ ≥ 0 always)

**sqrt_handler (new, overrides _elementary factory):**

- ``Sqrt(0) → 0``, ``Sqrt(1) → 1``
- Perfect-square numerics: ``Sqrt(4) → 2``, ``Sqrt(9) → 3``
- ``Sqrt(Pow(x, 2k))``:
  - k even → ``Pow(x, k)``     (x^k ≥ 0 always when k even)
  - k odd  → ``Abs(Pow(x, k))`` (sign depends on x)
- Assumption-aware: ``Sqrt(Pow(x, 2)) → x`` when ``assume(x >= 0)`` active.

All Phase 28 (assumption-aware abs/sign) and Phase 27 (inequalities) behaviour
is preserved in the regression tests.

Test structure
--------------
TestPhase29_AbsNeg           — abs(-x), abs(neg(neg(x))), abs(-3), abs(-1*x)
TestPhase29_AbsEvenPower     — abs(x^2), abs(x^4), abs(odd power) unevaluated
TestPhase29_AbsIdempotent    — abs(abs(x)), abs(abs(-x))
TestPhase29_SqrtEvenPower    — sqrt(x^2)→|x|, sqrt(x^4)→x², sqrt(x^6)→|x³|, sqrt(x^8)→x⁴
TestPhase29_SqrtNumeric      — sqrt(0/1/4/9/2.0) numeric fold
TestPhase29_SqrtAssumptions  — assume(x≥0): sqrt(x^2)→x; without: Abs(x)
TestPhase29_Regressions      — Phase 28 assumption abs, Phase 27 inequality, Phase 3 exp
TestPhase29_Macsyma          — end-to-end MACSYMA surface syntax
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    GREATER_EQUAL,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ── IR head constants ──────────────────────────────────────────────────────────

ABS = IRSymbol("Abs")
SQRT_HEAD = IRSymbol("Sqrt")
NEG = IRSymbol("Neg")
POW = IRSymbol("Pow")
MUL = IRSymbol("Mul")
ASSUME = IRSymbol("Assume")
ADD = IRSymbol("Add")

X = IRSymbol("x")
Y = IRSymbol("y")

ZERO = IRInteger(0)
ONE = IRInteger(1)
TWO = IRInteger(2)

# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_vm() -> VM:
    """Fresh VM with SymbolicBackend."""
    return VM(SymbolicBackend())


def _abs(x: IRNode) -> IRApply:
    return IRApply(ABS, (x,))


def _sqrt(x: IRNode) -> IRApply:
    return IRApply(SQRT_HEAD, (x,))


def _neg(x: IRNode) -> IRApply:
    return IRApply(NEG, (x,))


def _pow(base: IRNode, exp: int) -> IRApply:
    return IRApply(POW, (base, IRInteger(exp)))


def _mul(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(MUL, (a, b))


def _assume(pred: IRNode) -> IRApply:
    return IRApply(ASSUME, (pred,))


def _nonneg_pred(sym: IRSymbol) -> IRApply:
    """Build ``x >= 0`` predicate."""
    return IRApply(GREATER_EQUAL, (sym, ZERO))


# ── TestPhase29_AbsNeg ─────────────────────────────────────────────────────────


class TestPhase29_AbsNeg:
    """``abs(-x) = abs(x)`` — the even-function rule for negated arguments."""

    def test_abs_neg_symbol(self) -> None:
        """abs(-x) → Abs(x)  (Neg stripped, result is Abs of plain symbol)."""
        vm = _make_vm()
        result = vm.eval(_abs(_neg(X)))
        assert result == _abs(X)

    def test_abs_neg_symbol_y(self) -> None:
        """abs(-y) → Abs(y)."""
        vm = _make_vm()
        result = vm.eval(_abs(_neg(Y)))
        assert result == _abs(Y)

    def test_abs_neg_of_neg(self) -> None:
        """abs(neg(neg(x))) — the inner neg(neg(x)) evaluates before abs sees it.

        The VM evaluates ``neg(neg(x))`` first.  If the Neg handler cancels
        double-negations (Neg(Neg(x)) → x), abs sees ``x`` directly.
        Even if it doesn't, abs(neg(neg(x))) must not infinitely recurse.
        """
        vm = _make_vm()
        result = vm.eval(_abs(_neg(_neg(X))))
        # Either abs(x) or x (if double-neg cancelled); not Abs(Neg(Neg(x)))
        assert result in {_abs(X), X}

    def test_abs_neg_integer(self) -> None:
        """abs(-3) → 3  (numeric fold, not the algebraic neg-strip rule)."""
        vm = _make_vm()
        result = vm.eval(_abs(_neg(IRInteger(3))))
        assert result == IRInteger(3)

    def test_abs_neg_rational(self) -> None:
        """abs(-1/2) → 1/2."""
        from symbolic_ir import IRRational  # noqa: PLC0415

        vm = _make_vm()
        result = vm.eval(_abs(_neg(IRRational(1, 2))))
        assert result == IRRational(1, 2)

    def test_abs_mul_neg_one(self) -> None:
        """abs(Mul(-1, x)) → Abs(x)  (-x encoded as Mul after eval)."""
        vm = _make_vm()
        result = vm.eval(_abs(_mul(IRInteger(-1), X)))
        assert result == _abs(X)

    def test_abs_neg_sum(self) -> None:
        """abs(-(x + y)) → Abs(Add(x, y))  (neg of compound stripped)."""
        vm = _make_vm()
        xy = IRApply(ADD, (X, Y))
        result = vm.eval(_abs(_neg(xy)))
        # The inner neg is stripped; result is Abs of whatever neg(x+y) evaluates to
        # (which is either Neg(Add(x,y)) stripped → Abs(Add(x,y)), or Abs stays)
        assert isinstance(result, IRApply)
        assert result.head == ABS


# ── TestPhase29_AbsEvenPower ───────────────────────────────────────────────────


class TestPhase29_AbsEvenPower:
    """``abs(x^{2k}) = x^{2k}`` — even powers are always non-negative."""

    def test_abs_x_squared(self) -> None:
        """abs(x^2) → Pow(x, 2)  (x² ≥ 0 for all real x)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 2)))
        assert result == _pow(X, 2)

    def test_abs_x_fourth(self) -> None:
        """abs(x^4) → Pow(x, 4)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 4)))
        assert result == _pow(X, 4)

    def test_abs_x_sixth(self) -> None:
        """abs(x^6) → Pow(x, 6)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 6)))
        assert result == _pow(X, 6)

    def test_abs_odd_power_unevaluated(self) -> None:
        """abs(x^3) → Abs(Pow(x, 3))  (odd power, sign unknown)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 3)))
        assert isinstance(result, IRApply)
        assert result.head == ABS

    def test_abs_odd_power_5(self) -> None:
        """abs(x^5) → Abs(Pow(x, 5))  (odd power, unevaluated)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 5)))
        assert isinstance(result, IRApply)
        assert result.head == ABS

    def test_abs_power_zero_exponent(self) -> None:
        """abs(x^0) → abs(1) → 1  (numeric fold after eval)."""
        vm = _make_vm()
        result = vm.eval(_abs(_pow(X, 0)))
        # x^0 = 1, abs(1) = 1
        assert result == ONE


# ── TestPhase29_AbsIdempotent ──────────────────────────────────────────────────


class TestPhase29_AbsIdempotent:
    """``abs(abs(x)) = abs(x)`` — absolute value is idempotent."""

    def test_abs_abs_symbol(self) -> None:
        """abs(abs(x)) → Abs(x)."""
        vm = _make_vm()
        result = vm.eval(_abs(_abs(X)))
        assert result == _abs(X)

    def test_abs_abs_neg_symbol(self) -> None:
        """abs(abs(-x)) → Abs(x)  (outer abs idempotent, inner neg stripped)."""
        vm = _make_vm()
        result = vm.eval(_abs(_abs(_neg(X))))
        # abs(-x) = abs(x), then abs(abs(x)) = abs(x)
        assert result == _abs(X)

    def test_abs_triple_abs(self) -> None:
        """abs(abs(abs(x))) → Abs(x)  (collapses all layers)."""
        vm = _make_vm()
        result = vm.eval(_abs(_abs(_abs(X))))
        assert result == _abs(X)

    def test_abs_abs_integer(self) -> None:
        """abs(abs(3)) → 3  (numeric fold dominates idempotency)."""
        vm = _make_vm()
        result = vm.eval(_abs(_abs(IRInteger(3))))
        assert result == IRInteger(3)


# ── TestPhase29_SqrtEvenPower ──────────────────────────────────────────────────


class TestPhase29_SqrtEvenPower:
    """``Sqrt(x^{2k})`` algebraic reduction formulas."""

    def test_sqrt_x_squared_is_abs(self) -> None:
        """sqrt(x^2) → Abs(x)  (k=1 odd → |x|)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 2)))
        assert result == _abs(X)

    def test_sqrt_x_fourth_is_x_squared(self) -> None:
        """sqrt(x^4) → Pow(x, 2)  (k=2 even → x² ≥ 0)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 4)))
        assert result == _pow(X, 2)

    def test_sqrt_x_sixth_is_abs_x_cubed(self) -> None:
        """sqrt(x^6) → Abs(Pow(x, 3))  (k=3 odd → |x³|)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 6)))
        assert isinstance(result, IRApply)
        assert result.head == ABS
        inner = result.args[0]
        assert isinstance(inner, IRApply)
        assert inner.head == POW
        assert inner.args[1] == IRInteger(3)

    def test_sqrt_x_eighth_is_x_fourth(self) -> None:
        """sqrt(x^8) → Pow(x, 4)  (k=4 even)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 8)))
        assert result == _pow(X, 4)

    def test_sqrt_x_tenth_is_abs_x_fifth(self) -> None:
        """sqrt(x^10) → Abs(Pow(x, 5))  (k=5 odd)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 10)))
        assert isinstance(result, IRApply)
        assert result.head == ABS

    def test_sqrt_odd_exponent_unevaluated(self) -> None:
        """sqrt(x^3) → Sqrt(Pow(x, 3)) unevaluated (odd exponent)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 3)))
        assert isinstance(result, IRApply)
        assert result.head == SQRT_HEAD

    def test_sqrt_x_squared_structure(self) -> None:
        """sqrt(x^2) result has correct ABS head and X as inner arg."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 2)))
        assert isinstance(result, IRApply)
        assert result.head == ABS
        assert result.args[0] == X

    def test_sqrt_x_fourth_structure(self) -> None:
        """sqrt(x^4) → Pow(x,2) — check structure carefully."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 4)))
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == X
        assert result.args[1] == IRInteger(2)


# ── TestPhase29_SqrtNumeric ────────────────────────────────────────────────────


class TestPhase29_SqrtNumeric:
    """Numeric fold for ``Sqrt`` — including perfect-square detection."""

    def test_sqrt_zero(self) -> None:
        """sqrt(0) → 0."""
        vm = _make_vm()
        assert vm.eval(_sqrt(ZERO)) == ZERO

    def test_sqrt_one(self) -> None:
        """sqrt(1) → 1."""
        vm = _make_vm()
        assert vm.eval(_sqrt(ONE)) == ONE

    def test_sqrt_four(self) -> None:
        """sqrt(4) → 2  (perfect square → IRInteger, not IRFloat)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(IRInteger(4)))
        assert result == IRInteger(2)

    def test_sqrt_nine(self) -> None:
        """sqrt(9) → 3."""
        vm = _make_vm()
        assert vm.eval(_sqrt(IRInteger(9))) == IRInteger(3)

    def test_sqrt_float_irrational(self) -> None:
        """sqrt(2.0) → IRFloat ≈ 1.4142."""
        vm = _make_vm()
        result = vm.eval(_sqrt(IRFloat(2.0)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 2.0**0.5) < 1e-12

    def test_sqrt_integer_irrational(self) -> None:
        """sqrt(2) → IRFloat ≈ 1.4142  (integer 2 is not a perfect square)."""
        vm = _make_vm()
        result = vm.eval(_sqrt(IRInteger(2)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 2.0**0.5) < 1e-12


# ── TestPhase29_SqrtAssumptions ────────────────────────────────────────────────


class TestPhase29_SqrtAssumptions:
    """sqrt(x²) simplification integrates with Phase 28 assumption context."""

    def test_sqrt_x_sq_nonneg_assumed(self) -> None:
        """assume(x >= 0); sqrt(x^2) → x  (abs drops under non-negativity)."""
        vm = _make_vm()
        vm.eval(_assume(_nonneg_pred(X)))
        result = vm.eval(_sqrt(_pow(X, 2)))
        assert result == X

    def test_sqrt_x_sq_no_assumption(self) -> None:
        """sqrt(x^2) → Abs(x) when no assumption is made."""
        vm = _make_vm()
        result = vm.eval(_sqrt(_pow(X, 2)))
        assert result == _abs(X)

    def test_sqrt_x_sq_positive_assumed(self) -> None:
        """assume(x > 0); sqrt(x^2) → x  (positive implies nonneg)."""
        vm = _make_vm()
        from symbolic_ir import GREATER  # noqa: PLC0415

        vm.eval(_assume(IRApply(GREATER, (X, ZERO))))
        result = vm.eval(_sqrt(_pow(X, 2)))
        assert result == X

    def test_sqrt_y_sq_unrelated_assumption(self) -> None:
        """assume(x >= 0); sqrt(y^2) → Abs(y)  (assumption is for x, not y)."""
        vm = _make_vm()
        vm.eval(_assume(_nonneg_pred(X)))
        result = vm.eval(_sqrt(_pow(Y, 2)))
        assert result == _abs(Y)


# ── TestPhase29_Regressions ────────────────────────────────────────────────────


class TestPhase29_Regressions:
    """Ensure Phase 28 and earlier behaviour is fully preserved."""

    def test_phase28_abs_with_positive_assumption(self) -> None:
        """Phase 28: assume(x > 0); abs(x) → x (assumption fold still works)."""
        from symbolic_ir import GREATER  # noqa: PLC0415

        vm = _make_vm()
        vm.eval(_assume(IRApply(GREATER, (X, ZERO))))
        assert vm.eval(_abs(X)) == X

    def test_phase28_abs_with_negative_assumption(self) -> None:
        """Phase 28: assume(x < 0); abs(x) → -x."""
        from symbolic_ir import LESS  # noqa: PLC0415

        vm = _make_vm()
        vm.eval(_assume(IRApply(LESS, (X, ZERO))))
        result = vm.eval(_abs(X))
        assert isinstance(result, IRApply)
        assert result.head == NEG
        assert result.args[0] == X

    def test_phase28_sign_numeric(self) -> None:
        """Phase 28: sign(−5) → −1 numeric fold still works."""
        SIGN = IRSymbol("Sign")
        vm = _make_vm()
        assert vm.eval(IRApply(SIGN, (IRInteger(-5),))) == IRInteger(-1)

    def test_phase3_exp(self) -> None:
        """Phase 3: exp(0) → 1 (elementary function regression)."""
        EXP = IRSymbol("Exp")
        vm = _make_vm()
        assert vm.eval(IRApply(EXP, (ZERO,))) == ONE

    def test_abs_numeric_still_works(self) -> None:
        """Numeric fold unchanged: abs(-7) → 7."""
        vm = _make_vm()
        assert vm.eval(_abs(IRInteger(-7))) == IRInteger(7)

    def test_sqrt_zero_still_works(self) -> None:
        """sqrt(0) = 0 preserved from _elementary factory baseline."""
        vm = _make_vm()
        assert vm.eval(_sqrt(ZERO)) == ZERO


# ── TestPhase29_Macsyma ────────────────────────────────────────────────────────


class TestPhase29_Macsyma:
    """End-to-end tests via MACSYMA surface syntax.

    All tests skip gracefully when ``macsyma_runtime`` is not installed
    (e.g. when running symbolic-vm's own CI in isolation).
    """

    def _make_vm(self) -> VM:
        """Build a MacsymaBackend VM, skipping if runtime not installed."""
        pytest.importorskip(
            "macsyma_runtime",
            reason="macsyma-runtime not installed; skipping MACSYMA e2e test",
        )
        from macsyma_compiler.compiler import (  # noqa: PLC0415
            _STANDARD_FUNCTIONS,
        )
        from macsyma_runtime import MacsymaBackend  # noqa: PLC0415
        from macsyma_runtime.name_table import (  # noqa: PLC0415
            extend_compiler_name_table,
        )

        extend_compiler_name_table(_STANDARD_FUNCTIONS)
        return VM(MacsymaBackend())

    def _run(self, source: str, vm: VM) -> IRNode:
        """Parse + compile + eval a MACSYMA expression."""
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

    def test_sqrt_x_squared_macsyma(self, vm: VM) -> None:
        """sqrt(x^2) → Abs(x) via MACSYMA surface syntax."""
        result = self._run("sqrt(x^2)", vm)
        assert isinstance(result, IRApply)
        assert result.head == ABS
        assert result.args[0] == X

    def test_sqrt_x_fourth_macsyma(self, vm: VM) -> None:
        """sqrt(x^4) → Pow(x,2) via MACSYMA surface syntax."""
        result = self._run("sqrt(x^4)", vm)
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[1] == TWO

    def test_abs_neg_x_macsyma(self, vm: VM) -> None:
        """abs(-x) → Abs(x) via MACSYMA surface syntax."""
        result = self._run("abs(-x)", vm)
        assert isinstance(result, IRApply)
        assert result.head == ABS
        assert result.args[0] == X

    def test_sqrt_x_squared_with_assume_macsyma(self, vm: VM) -> None:
        """assume(x >= 0); sqrt(x^2) → x via MACSYMA."""
        self._run("assume(x >= 0)", vm)
        result = self._run("sqrt(x^2)", vm)
        assert result == X

    def test_abs_x_squared_macsyma(self, vm: VM) -> None:
        """abs(x^2) → Pow(x,2) via MACSYMA (even power rule)."""
        result = self._run("abs(x^2)", vm)
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[1] == TWO
