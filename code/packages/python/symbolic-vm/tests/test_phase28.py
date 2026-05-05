"""Phase 28 — Assumptions framework: abs/sign simplification.

Phase 28 extends two existing handlers in symbolic-vm:

1. ``abs_handler`` — now folds ``Abs(x)`` to ``x`` or ``-x`` when the sign
   of ``x`` is known from the session's assumption context.

2. ``sign_handler`` — now folds ``Sign(n)`` for numeric literals (IRInteger,
   IRRational, IRFloat) in addition to its existing symbolic lookup.

The assumption context itself (``vm.assumptions``) is provided by
``cas_simplify.AssumptionContext`` and was implemented in Phase 21.  The
``Assume``, ``Forget``, and ``Is`` handlers that populate it were already
wired in Phase 21 as well.  Phase 28 only adds the abs/sign folding.

Test structure
--------------
TestPhase28_SignNumeric       — sign(int/rat/float) folds to 1/-1/0
TestPhase28_SignSymbolic      — sign(x) with/without assumptions
TestPhase28_AbsAssumptions    — abs(x) with pos/neg/nonneg assumptions
TestPhase28_AbsFallthrough    — abs(x) unevaluated without assumptions
TestPhase28_AssumeForgetIs    — full assume/forget/is round-trip
TestPhase28_KillResetsDB      — forget() clears assumptions
TestPhase28_Regressions       — Phase 27 inequality, Phase 23 abs numeric,
                                 Phase 21 sign symbolic, Phase 3 exp(2x)
TestPhase28_Macsyma           — end-to-end MACSYMA surface syntax
"""

from __future__ import annotations

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from macsyma_runtime import MacsymaBackend
from macsyma_runtime.name_table import extend_compiler_name_table
from symbolic_ir import (
    GREATER,
    GREATER_EQUAL,
    LESS,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ── Helpers ────────────────────────────────────────────────────────────────────

ABS = IRSymbol("Abs")
SIGN = IRSymbol("Sign")
ASSUME = IRSymbol("Assume")
FORGET = IRSymbol("Forget")
IS = IRSymbol("Is")
NEG = IRSymbol("Neg")
X = IRSymbol("x")
Y = IRSymbol("y")
ZERO = IRInteger(0)


def _make_vm() -> VM:
    """Fresh VM with SymbolicBackend (assumptions supported via vm.assumptions)."""
    return VM(SymbolicBackend())


def _make_macsyma_vm() -> VM:
    """Fresh VM with MacsymaBackend + MACSYMA name table extended."""
    extend_compiler_name_table(_STANDARD_FUNCTIONS)
    return VM(MacsymaBackend())


def _eval_macsyma(source: str, vm: VM) -> IRNode:
    """Parse + compile + eval a MACSYMA expression through the given VM."""
    ast = parse_macsyma(source + ";")
    stmts = compile_macsyma(ast)
    result: IRNode = vm.eval(ZERO)
    for stmt in stmts:
        result = vm.eval(stmt)
    return result


def _abs(x: IRNode) -> IRApply:
    return IRApply(ABS, (x,))


def _sign(x: IRNode) -> IRApply:
    return IRApply(SIGN, (x,))


def _assume(pred: IRNode) -> IRApply:
    return IRApply(ASSUME, (pred,))


def _forget(pred: IRNode) -> IRApply:
    return IRApply(FORGET, (pred,))


def _is(pred: IRNode) -> IRApply:
    return IRApply(IS, (pred,))


def _gt(var: IRSymbol = X) -> IRApply:
    return IRApply(GREATER, (var, ZERO))


def _ge(var: IRSymbol = X) -> IRApply:
    return IRApply(GREATER_EQUAL, (var, ZERO))


def _lt(var: IRSymbol = X) -> IRApply:
    return IRApply(LESS, (var, ZERO))


# ── TestPhase28_SignNumeric ────────────────────────────────────────────────────


class TestPhase28_SignNumeric:
    """sign(n) folds for all numeric IR literals."""

    def test_sign_positive_int(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRInteger(5))) == IRInteger(1)

    def test_sign_negative_int(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRInteger(-3))) == IRInteger(-1)

    def test_sign_zero_int(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRInteger(0))) == IRInteger(0)

    def test_sign_positive_rational(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRRational(3, 4))) == IRInteger(1)

    def test_sign_negative_rational(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRRational(-1, 2))) == IRInteger(-1)

    def test_sign_positive_float(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRFloat(2.5))) == IRInteger(1)

    def test_sign_negative_float(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRFloat(-0.5))) == IRInteger(-1)

    def test_sign_zero_float(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRFloat(0.0))) == IRInteger(0)

    def test_sign_large_int(self):
        vm = _make_vm()
        assert vm.eval(_sign(IRInteger(10**12))) == IRInteger(1)


# ── TestPhase28_SignSymbolic ───────────────────────────────────────────────────


class TestPhase28_SignSymbolic:
    """sign(x) folds for symbolic vars with known sign, else stays unevaluated."""

    def test_sign_sym_positive_assumed(self):
        """assume(x > 0) → sign(x) = 1."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        assert vm.eval(_sign(X)) == IRInteger(1)

    def test_sign_sym_nonneg_assumed(self):
        """assume(x >= 0) → sign(x) = 1 (best available: nonneg)."""
        vm = _make_vm()
        vm.eval(_assume(_ge()))
        assert vm.eval(_sign(X)) == IRInteger(1)

    def test_sign_sym_negative_assumed(self):
        """assume(x < 0) → sign(x) = -1."""
        vm = _make_vm()
        vm.eval(_assume(_lt()))
        assert vm.eval(_sign(X)) == IRInteger(-1)

    def test_sign_sym_no_assumption_unevaluated(self):
        """No assumption → sign(x) stays unevaluated."""
        vm = _make_vm()
        result = vm.eval(_sign(X))
        assert isinstance(result, IRApply)
        assert result.head == SIGN

    def test_sign_sym_different_var_no_spill(self):
        """assume(x > 0) does NOT affect sign(y)."""
        vm = _make_vm()
        vm.eval(_assume(_gt(X)))
        result = vm.eval(_sign(Y))
        assert isinstance(result, IRApply)
        assert result.head == SIGN


# ── TestPhase28_AbsAssumptions ─────────────────────────────────────────────────


class TestPhase28_AbsAssumptions:
    """abs(x) folds based on session assumptions."""

    def test_abs_positive_assumed(self):
        """assume(x > 0) → abs(x) = x."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        assert vm.eval(_abs(X)) == X

    def test_abs_nonneg_assumed(self):
        """assume(x >= 0) → abs(x) = x."""
        vm = _make_vm()
        vm.eval(_assume(_ge()))
        assert vm.eval(_abs(X)) == X

    def test_abs_negative_assumed(self):
        """assume(x < 0) → abs(x) = -x."""
        vm = _make_vm()
        vm.eval(_assume(_lt()))
        result = vm.eval(_abs(X))
        assert isinstance(result, IRApply)
        assert result.head == NEG
        assert result.args[0] == X

    def test_abs_y_positive_x_unchanged(self):
        """assume(y > 0) only affects y, not x."""
        vm = _make_vm()
        vm.eval(_assume(_gt(Y)))
        # y is simplified
        assert vm.eval(_abs(Y)) == Y
        # x is not
        result = vm.eval(_abs(X))
        assert isinstance(result, IRApply)
        assert result.head == ABS

    def test_abs_after_forget(self):
        """Forgetting the assumption restores unevaluated form."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        assert vm.eval(_abs(X)) == X  # simplified
        vm.eval(_forget(_gt()))
        result = vm.eval(_abs(X))
        assert isinstance(result, IRApply)
        assert result.head == ABS  # unevaluated again


# ── TestPhase28_AbsFallthrough ─────────────────────────────────────────────────


class TestPhase28_AbsFallthrough:
    """abs(x) stays unevaluated or uses numeric fold when no assumption."""

    def test_abs_no_assumption_unevaluated(self):
        """abs(x) with no assumption: leave as Abs(x)."""
        vm = _make_vm()
        result = vm.eval(_abs(X))
        assert isinstance(result, IRApply)
        assert result.head == ABS

    def test_abs_numeric_positive_folds(self):
        """abs(3) → 3 (numeric fold, no assumption needed)."""
        vm = _make_vm()
        assert vm.eval(_abs(IRInteger(3))) == IRInteger(3)

    def test_abs_numeric_negative_folds(self):
        """abs(-5) → 5."""
        vm = _make_vm()
        assert vm.eval(_abs(IRInteger(-5))) == IRInteger(5)

    def test_abs_rational_folds(self):
        """abs(-3/4) → 3/4."""
        vm = _make_vm()
        assert vm.eval(_abs(IRRational(-3, 4))) == IRRational(3, 4)

    def test_abs_float_folds(self):
        """abs(-2.5) → 2.5."""
        vm = _make_vm()
        result = vm.eval(_abs(IRFloat(-2.5)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 2.5) < 1e-12


# ── TestPhase28_AssumeForgetIs ─────────────────────────────────────────────────


class TestPhase28_AssumeForgetIs:
    """Full assume/forget/is round-trip via IR."""

    def test_assume_returns_done(self):
        """assume(x > 0) returns the done sentinel."""
        vm = _make_vm()
        result = vm.eval(_assume(_gt()))
        # The existing Phase 21 assume_handler returns IRSymbol("done").
        assert isinstance(result, IRSymbol)
        assert result.name == "done"

    def test_is_true_after_assume(self):
        """is(x > 0) = true after assume(x > 0)."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        result = vm.eval(_is(_gt()))
        assert isinstance(result, IRSymbol)
        assert result.name == "true"

    def test_is_unknown_before_assume(self):
        """is(x > 0) = unknown without any assumption."""
        vm = _make_vm()
        result = vm.eval(_is(_gt()))
        assert isinstance(result, IRSymbol)
        assert result.name == "unknown"

    def test_is_false_for_contradiction(self):
        """assume(x > 0) → is(x < 0) = false."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        result = vm.eval(_is(_lt()))
        assert isinstance(result, IRSymbol)
        assert result.name == "false"

    def test_is_inferred_nonneg(self):
        """assume(x > 0) → is(x >= 0) = true (inferred)."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        result = vm.eval(_is(_ge()))
        assert isinstance(result, IRSymbol)
        assert result.name == "true"

    def test_is_unknown_after_forget(self):
        """After forget(x > 0), is(x > 0) returns unknown."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        vm.eval(_forget(_gt()))
        result = vm.eval(_is(_gt()))
        assert isinstance(result, IRSymbol)
        assert result.name == "unknown"


# ── TestPhase28_KillResetsDB ───────────────────────────────────────────────────


class TestPhase28_KillResetsDB:
    """forget() clears assumptions; kill(all) resets binding env (not assumptions)."""

    def test_forget_no_args_clears_all(self):
        """Forget() with no args wipes everything."""
        vm = _make_vm()
        vm.eval(_assume(_gt()))
        vm.eval(_assume(_ge(Y)))
        vm.eval(IRApply(FORGET, ()))
        assert vm.eval(_is(_gt())).name == "unknown"
        assert vm.eval(_is(_ge(Y))).name == "unknown"


# ── TestPhase28_Regressions ────────────────────────────────────────────────────


class TestPhase28_Regressions:
    """Ensure Phase 27 inequality, Phase 21 radcan, Phase 3 exp still work."""

    def test_phase27_inequality_still_works(self):
        """solve(x^2 - 1 > 0, x) still returns interval conditions."""
        vm = _make_vm()
        SOLVE = IRSymbol("Solve")
        POW = IRSymbol("Pow")
        SUB = IRSymbol("Sub")
        result = vm.eval(
            IRApply(SOLVE, (
                IRApply(GREATER, (
                    IRApply(SUB, (IRApply(POW, (X, IRInteger(2))), IRInteger(1))),
                    ZERO,
                )),
                X,
            ))
        )
        # Should be a List with interval conditions.
        assert isinstance(result, IRApply)
        assert result.head.name == "List"  # type: ignore[union-attr]

    def test_phase3_exp_integration_still_works(self):
        """∫ exp(2x) dx = exp(2x)/2."""
        vm = _make_vm()
        INTEGRATE = IRSymbol("Integrate")
        EXP = IRSymbol("Exp")
        MUL = IRSymbol("Mul")
        result = vm.eval(
            IRApply(INTEGRATE, (IRApply(EXP, (IRApply(MUL, (IRInteger(2), X)),)), X))
        )
        # Result should contain Exp somewhere.
        assert "Exp" in repr(result)

    def test_abs_numeric_regression(self):
        """abs(-7) = 7 still folds numerically (no regression from Phase 28)."""
        vm = _make_vm()
        assert vm.eval(_abs(IRInteger(-7))) == IRInteger(7)

    def test_sign_of_rational_neg_regression(self):
        """sign(-1/3) = -1 folds numerically."""
        vm = _make_vm()
        assert vm.eval(_sign(IRRational(-1, 3))) == IRInteger(-1)


# ── TestPhase28_Macsyma ────────────────────────────────────────────────────────


class TestPhase28_Macsyma:
    """End-to-end tests through MACSYMA surface syntax."""

    @pytest.fixture()
    def vm(self):
        return _make_macsyma_vm()

    def test_assume_abs_positive(self, vm):
        """assume(x > 0); abs(x) → x via MACSYMA surface syntax."""
        _eval_macsyma("assume(x > 0)", vm)
        result = _eval_macsyma("abs(x)", vm)
        assert result == X

    def test_assume_abs_nonneg(self, vm):
        """assume(x >= 0); abs(x) → x."""
        _eval_macsyma("assume(x >= 0)", vm)
        result = _eval_macsyma("abs(x)", vm)
        assert result == X

    def test_assume_abs_negative(self, vm):
        """assume(x < 0); abs(x) → -x."""
        _eval_macsyma("assume(x < 0)", vm)
        result = _eval_macsyma("abs(x)", vm)
        assert isinstance(result, IRApply)
        assert result.head == NEG
        assert result.args[0] == X

    def test_sign_numeric_macsyma(self, vm):
        """sign(5) → 1 via MACSYMA."""
        result = _eval_macsyma("sign(5)", vm)
        assert result == IRInteger(1)

    def test_assume_sign_positive(self, vm):
        """assume(x > 0); sign(x) → 1 via MACSYMA."""
        _eval_macsyma("assume(x > 0)", vm)
        result = _eval_macsyma("sign(x)", vm)
        assert result == IRInteger(1)

    def test_is_query_macsyma(self, vm):
        """assume(x > 0); is(x > 0) → true."""
        _eval_macsyma("assume(x > 0)", vm)
        result = _eval_macsyma("is(x > 0)", vm)
        assert isinstance(result, IRSymbol)
        assert result.name == "true"

    def test_is_unknown_macsyma(self, vm):
        """is(y > 0) → unknown when no assumption about y."""
        result = _eval_macsyma("is(y > 0)", vm)
        assert isinstance(result, IRSymbol)
        assert result.name == "unknown"

    def test_forget_restores_unevaluated(self, vm):
        """assume(x > 0); abs(x) → x; forget(x > 0); abs(x) → Abs(x)."""
        _eval_macsyma("assume(x > 0)", vm)
        assert _eval_macsyma("abs(x)", vm) == X
        _eval_macsyma("forget(x > 0)", vm)
        result = _eval_macsyma("abs(x)", vm)
        assert isinstance(result, IRApply)
        assert result.head == ABS
