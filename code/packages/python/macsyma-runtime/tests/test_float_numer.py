"""Phase 30 — ``float()`` function and ``numer`` evaluation mode.

Tests cover:
- :func:`_numer_fold` — the recursive exact-to-float conversion helper
- ``Float(expr)`` head (via :func:`float_handler`)
- ``ev(expr, numer)`` and ``ev(expr, float)`` evaluated through the REPL
- End-to-end pipeline: surface syntax ``float(1/2)`` → 0.5

Organisation
------------
Section A — _numer_fold unit tests (direct function call)
Section B — Float handler IR tests  (build IRApply, run through VM)
Section C — ev(expr, numer) pipeline tests
Section D — Full compiler-pipeline tests (source string → result)
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)
from symbolic_vm import VM

from macsyma_runtime import EV, MacsymaBackend
from macsyma_runtime.handlers import _numer_fold
from macsyma_runtime.name_table import FLOAT_FUNC

# ===========================================================================
# Section A — _numer_fold unit tests
# ===========================================================================


class TestNumerFold:
    """Direct tests of _numer_fold without a running VM."""

    def test_integer_becomes_float(self) -> None:
        """IRInteger(3) → IRFloat(3.0)."""
        assert _numer_fold(IRInteger(3)) == IRFloat(3.0)

    def test_integer_zero_becomes_float(self) -> None:
        """IRInteger(0) → IRFloat(0.0)."""
        assert _numer_fold(IRInteger(0)) == IRFloat(0.0)

    def test_negative_integer_becomes_float(self) -> None:
        """IRInteger(-7) → IRFloat(-7.0)."""
        assert _numer_fold(IRInteger(-7)) == IRFloat(-7.0)

    def test_rational_becomes_float(self) -> None:
        """IRRational(1, 2) → IRFloat(0.5)."""
        result = _numer_fold(IRRational(1, 2))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5) < 1e-15

    def test_rational_one_third(self) -> None:
        """IRRational(1, 3) → IRFloat(1/3)."""
        result = _numer_fold(IRRational(1, 3))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0 / 3.0) < 1e-15

    def test_float_passthrough(self) -> None:
        """IRFloat is returned unchanged (identity)."""
        f = IRFloat(2.71)
        assert _numer_fold(f) is f

    def test_symbol_passthrough(self) -> None:
        """IRSymbol is returned unchanged (identity)."""
        s = IRSymbol("x")
        assert _numer_fold(s) is s

    def test_apply_no_numerics_unchanged(self) -> None:
        """IRApply with all-symbol args: returned unchanged (same identity)."""
        x = IRSymbol("x")
        y = IRSymbol("y")
        expr = IRApply(ADD, (x, y))
        result = _numer_fold(expr)
        # No exact numerics, so the original node is returned.
        assert result is expr

    def test_apply_folds_integer_arg(self) -> None:
        """IRApply with an integer arg: that arg becomes IRFloat."""
        x = IRSymbol("x")
        expr = IRApply(ADD, (x, IRInteger(1)))
        result = _numer_fold(expr)
        assert isinstance(result, IRApply)
        assert result.args[1] == IRFloat(1.0)

    def test_apply_folds_rational_arg(self) -> None:
        """IRApply(Add, (x, 1/3)) → IRApply(Add, (x, 0.333...))."""
        x = IRSymbol("x")
        expr = IRApply(ADD, (x, IRRational(1, 3)))
        result = _numer_fold(expr)
        assert isinstance(result, IRApply)
        assert isinstance(result.args[1], IRFloat)

    def test_pow_base_folded_exponent_preserved(self) -> None:
        """Pow(2, 3): base → 2.0, exponent stays IRInteger(3).

        Preserving integer exponents is critical because downstream numeric
        routines use ``isinstance(exp, IRInteger)`` checks.
        """
        expr = IRApply(POW, (IRInteger(2), IRInteger(3)))
        result = _numer_fold(expr)
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol) and result.head.name == "Pow"
        assert result.args[0] == IRFloat(2.0)
        # Exponent stays exact.
        assert result.args[1] == IRInteger(3)

    def test_pow_symbol_base_unchanged(self) -> None:
        """Pow(x, 2): symbol base → unchanged, exponent unchanged."""
        x = IRSymbol("x")
        expr = IRApply(POW, (x, IRInteger(2)))
        result = _numer_fold(expr)
        # Nothing to fold — returned unchanged.
        assert result is expr

    def test_nested_apply_folds_all_levels(self) -> None:
        """Nested: Mul(2, Add(x, 1/2)) — all numerics folded recursively."""
        x = IRSymbol("x")
        inner = IRApply(ADD, (x, IRRational(1, 2)))
        outer = IRApply(MUL, (IRInteger(2), inner))
        result = _numer_fold(outer)
        assert isinstance(result, IRApply)
        assert result.args[0] == IRFloat(2.0)
        inner_r = result.args[1]
        assert isinstance(inner_r, IRApply)
        assert inner_r.args[1] == IRFloat(0.5)


# ===========================================================================
# Section B — Float handler IR tests
# ===========================================================================


class TestFloatHandler:
    """IR-level tests for the Float handler (no REPL parsing)."""

    def test_float_of_integer(self) -> None:
        """Float(3) → IRFloat(3.0)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(FLOAT_FUNC, (IRInteger(3),))
        result = vm.eval(expr)
        assert result == IRFloat(3.0)

    def test_float_of_rational(self) -> None:
        """Float(IRRational(1,2)) → IRFloat(0.5)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(FLOAT_FUNC, (IRRational(1, 2),))
        result = vm.eval(expr)
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5) < 1e-15

    def test_float_of_symbol_stays_symbolic(self) -> None:
        """Float(x) → x (symbol is not numeric, stays as-is)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        x = IRSymbol("x")
        expr = IRApply(FLOAT_FUNC, (x,))
        result = vm.eval(expr)
        assert result == x

    def test_float_of_pi_constant(self) -> None:
        """Float(%pi) → IRFloat(π) since %pi is pre-bound as IRFloat."""
        backend = MacsymaBackend()
        vm = VM(backend)
        pi = IRSymbol("%pi")
        expr = IRApply(FLOAT_FUNC, (pi,))
        result = vm.eval(expr)
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.pi) < 1e-12

    def test_float_of_e_constant(self) -> None:
        """Float(%e) → IRFloat(e) since %e is pre-bound as IRFloat."""
        backend = MacsymaBackend()
        vm = VM(backend)
        e_sym = IRSymbol("%e")
        expr = IRApply(FLOAT_FUNC, (e_sym,))
        result = vm.eval(expr)
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.e) < 1e-12

    def test_float_of_zero(self) -> None:
        """Float(0) → IRFloat(0.0)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(FLOAT_FUNC, (IRInteger(0),))
        result = vm.eval(expr)
        assert result == IRFloat(0.0)

    def test_float_wrong_arity_returns_unevaluated(self) -> None:
        """Float(x, y) — wrong arity returns the IRApply unevaluated."""
        backend = MacsymaBackend()
        vm = VM(backend)
        x = IRSymbol("x")
        y = IRSymbol("y")
        expr = IRApply(FLOAT_FUNC, (x, y))
        result = vm.eval(expr)
        # Head is preserved and expression is unevaluated.
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol)
        assert result.head.name == "Float"

    def test_float_of_integer_expression(self) -> None:
        """Float(Add(IRInteger(1), IRInteger(2))) → IRFloat(3.0).

        The inner Add is evaluated first (→ IRInteger(3)), then folded.
        """
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(FLOAT_FUNC, (IRApply(ADD, (IRInteger(1), IRInteger(2))),))
        result = vm.eval(expr)
        assert result == IRFloat(3.0)


# ===========================================================================
# Section C — ev(expr, numer) pipeline tests
# ===========================================================================


class TestEvNumer:
    """Tests for ev(expr, numer) and ev(expr, float) via VM."""

    def test_ev_numer_folds_integer(self) -> None:
        """ev(3, numer) → IRFloat(3.0)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(EV, (IRInteger(3), IRSymbol("numer")))
        result = vm.eval(expr)
        assert result == IRFloat(3.0)

    def test_ev_float_folds_integer(self) -> None:
        """ev(3, float) → IRFloat(3.0)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        expr = IRApply(EV, (IRInteger(3), IRSymbol("float")))
        result = vm.eval(expr)
        assert result == IRFloat(3.0)

    def test_ev_numer_folds_rational(self) -> None:
        """ev(1/2, numer) → IRFloat(0.5)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        # The VM evaluates Div(1, 2) → IRRational(1, 2) first, then numer folds.
        expr = IRApply(
            EV,
            (IRApply(IRSymbol("Div"), (IRInteger(1), IRInteger(2))), IRSymbol("numer")),
        )
        result = vm.eval(expr)
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5) < 1e-15

    def test_ev_numer_folds_sum_of_integers(self) -> None:
        """ev(1 + 2, numer) → IRFloat(3.0)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        inner = IRApply(ADD, (IRInteger(1), IRInteger(2)))
        expr = IRApply(EV, (inner, IRSymbol("numer")))
        result = vm.eval(expr)
        assert result == IRFloat(3.0)

    def test_ev_numer_symbol_stays_symbolic(self) -> None:
        """ev(x, numer) → x — unbound symbol has no numeric value to fold."""
        backend = MacsymaBackend()
        vm = VM(backend)
        x = IRSymbol("x")
        expr = IRApply(EV, (x, IRSymbol("numer")))
        result = vm.eval(expr)
        assert result == x

    def test_ev_numer_symbolic_expression_folds_constants(self) -> None:
        """ev(x + 1, numer) → x + 1.0 — integer literal folded."""
        backend = MacsymaBackend()
        vm = VM(backend)
        x = IRSymbol("x")
        inner = IRApply(ADD, (x, IRInteger(1)))
        expr = IRApply(EV, (inner, IRSymbol("numer")))
        result = vm.eval(expr)
        # Structural check: Add(x, 1.0) or Add(1.0, x).
        assert isinstance(result, IRApply)
        float_args = [a for a in result.args if isinstance(a, IRFloat)]
        assert len(float_args) == 1
        assert float_args[0] == IRFloat(1.0)

    def test_ev_numer_backend_flag_restored(self) -> None:
        """Backend.numer is False both before and after ev(…, numer)."""
        backend = MacsymaBackend()
        vm = VM(backend)
        assert backend.numer is False
        expr = IRApply(EV, (IRInteger(1), IRSymbol("numer")))
        vm.eval(expr)
        assert backend.numer is False


# ===========================================================================
# Section D — Full compiler-pipeline tests (source string → result)
# ===========================================================================
#
# These tests run the full MACSYMA pipeline:
#   source string → lexer → parser → compiler → IR → VM → result
#
# They require that MacsymaBackend + REPL compiler integration is in place.
# Use the helper used in test_cas_pipeline.py.


def _eval_source(src: str) -> IRSymbol | IRInteger | IRFloat | IRApply | IRRational:  # type: ignore[type-arg]
    """Compile and evaluate a MACSYMA source string; return the raw IR result."""
    from macsyma_compiler import compile_macsyma
    from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
    from macsyma_parser import parse_macsyma

    from macsyma_runtime import extend_compiler_name_table

    # Extend the compiler name table so MACSYMA names (float, ev, …) compile
    # to the canonical IR heads that the backend dispatches on.
    extend_compiler_name_table(_STANDARD_FUNCTIONS)

    # Strip terminator if present, then re-add for the parser.
    clean = src.strip().rstrip(";$").strip()
    ast = parse_macsyma(clean + ";")
    stmts = compile_macsyma(ast, wrap_terminators=False)
    assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}: {stmts}"

    backend = MacsymaBackend()
    vm = VM(backend)
    return vm.eval(stmts[0])


class TestFloatPipeline:
    """End-to-end tests: MACSYMA source string → IRFloat result."""

    def test_float_of_integer_literal(self) -> None:
        """``float(3)`` → IRFloat(3.0)."""
        result = _eval_source("float(3)")
        assert result == IRFloat(3.0)

    def test_float_of_fraction(self) -> None:
        """``float(1/2)`` → IRFloat(0.5)."""
        result = _eval_source("float(1/2)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5) < 1e-14

    def test_float_of_one_third(self) -> None:
        """``float(1/3)`` → IRFloat(0.333...)."""
        result = _eval_source("float(1/3)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0 / 3.0) < 1e-14

    def test_float_of_pi(self) -> None:
        """``float(%pi)`` → IRFloat(3.14159...)."""
        result = _eval_source("float(%pi)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.pi) < 1e-12

    def test_float_of_e(self) -> None:
        """``float(%e)`` → IRFloat(2.71828...)."""
        result = _eval_source("float(%e)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.e) < 1e-12

    def test_ev_numer_of_fraction(self) -> None:
        """``ev(1/2, numer)`` → IRFloat(0.5)."""
        result = _eval_source("ev(1/2, numer)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5) < 1e-14

    def test_ev_numer_of_integer(self) -> None:
        """``ev(42, numer)`` → IRFloat(42.0)."""
        result = _eval_source("ev(42, numer)")
        assert result == IRFloat(42.0)

    def test_ev_float_of_fraction(self) -> None:
        """``ev(3/4, float)`` → IRFloat(0.75)."""
        result = _eval_source("ev(3/4, float)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.75) < 1e-14

    def test_float_in_name_table(self) -> None:
        """``float`` is in MACSYMA_NAME_TABLE and maps to FLOAT_FUNC."""
        from macsyma_runtime.name_table import FLOAT_FUNC, MACSYMA_NAME_TABLE

        assert "float" in MACSYMA_NAME_TABLE
        assert MACSYMA_NAME_TABLE["float"] is FLOAT_FUNC

    def test_float_handler_in_dispatch_table(self) -> None:
        """``Float`` key is registered in the handler dispatch table."""
        from macsyma_runtime.cas_handlers import build_cas_handler_table

        table = build_cas_handler_table()
        assert "Float" in table

    def test_float_func_symbol_name(self) -> None:
        """FLOAT_FUNC is IRSymbol with name 'Float'."""
        from macsyma_runtime.name_table import FLOAT_FUNC

        assert isinstance(FLOAT_FUNC, IRSymbol)
        assert FLOAT_FUNC.name == "Float"
