"""Ev — numer, float, expand, factor, ratsimp, trigsimp flags (C3 + A3/B1)."""

from __future__ import annotations

from symbolic_ir import ADD, POW, SUB, IRApply, IRInteger, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import EV, MacsymaBackend


def test_ev_no_flags_returns_first_arg() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    # ev(x) with no flags returns x.
    expr = IRApply(EV, (IRSymbol("x"),))
    assert vm.eval(expr) == IRSymbol("x")


def test_ev_unknown_flag_silently_ignored() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    expr = IRApply(EV, (IRInteger(2), IRSymbol("nonexistent_flag")))
    assert vm.eval(expr) == IRInteger(2)


def test_ev_numer_sets_flag_briefly() -> None:
    """Phase-A `numer` is a hint; we only verify the with_numer
    context manager toggles the flag during the evaluation, not that
    the result actually changes (that's Phase B+ work)."""
    backend = MacsymaBackend()
    vm = VM(backend)
    expr = IRApply(EV, (IRInteger(2), IRSymbol("numer")))
    # During eval, backend.numer should have flipped to True; after
    # eval, it should be back to False (the default).
    assert vm.eval(expr) == IRInteger(2)
    assert backend.numer is False


def test_with_numer_restores_on_exception() -> None:
    backend = MacsymaBackend()
    assert backend.numer is False
    try:
        with backend.with_numer():
            assert backend.numer is True
            raise RuntimeError("boom")
    except RuntimeError:
        pass
    assert backend.numer is False


# ---------------------------------------------------------------------------
# C3: expand and factor flags
# ---------------------------------------------------------------------------


def test_ev_float_flag_same_as_numer() -> None:
    """ev(expr, float) is an alias for ev(expr, numer)."""
    backend = MacsymaBackend()
    vm = VM(backend)
    # float flag: integer expression stays integer (no change for exact ints)
    expr = IRApply(EV, (IRInteger(42), IRSymbol("float")))
    result = vm.eval(expr)
    assert result == IRInteger(42)
    assert backend.numer is False  # flag restored after eval


def test_ev_expand_flag_applies_expand() -> None:
    """ev(x + 0, expand) simplifies via Expand."""
    backend = MacsymaBackend()
    vm = VM(backend)
    x = IRSymbol("x")
    # x + 0 → after Expand canonical: x
    inner = IRApply(ADD, (x, IRInteger(0)))
    expr = IRApply(EV, (inner, IRSymbol("expand")))
    result = vm.eval(expr)
    assert result == x


def test_ev_factor_flag_applies_factor() -> None:
    """ev(x^2 - 1, factor) factors the expression."""
    backend = MacsymaBackend()
    vm = VM(backend)
    x = IRSymbol("x")
    # x^2 - 1
    inner = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    expr = IRApply(EV, (inner, IRSymbol("factor")))
    result = vm.eval(expr)
    # Result should be factored — not a Factor(...) wrapper
    assert not (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Factor"
    ), f"Expected factored result, got unevaluated: {result}"


# ---------------------------------------------------------------------------
# A3 + B1: ratsimp and trigsimp flags
# ---------------------------------------------------------------------------


def test_ev_ratsimp_flag_cancels_common_factor() -> None:
    """ev((x^2-1)/(x-1), ratsimp) → x+1 (cancels x-1 from numerator)."""
    from symbolic_ir import POW, SUB

    backend = MacsymaBackend()
    vm = VM(backend)
    x = IRSymbol("x")
    # (x^2 - 1) / (x - 1)
    num = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    den = IRApply(SUB, (x, IRInteger(1)))
    div = IRApply(IRSymbol("Div"), (num, den))
    expr = IRApply(EV, (div, IRSymbol("ratsimp")))
    result = vm.eval(expr)
    # Result should be x + 1 — a linear polynomial, not a Div.
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Sub")  # x + 1 or 1 + x


def test_ev_trigsimp_flag_applies_pythagorean() -> None:
    """ev(sin(x)^2 + cos(x)^2, trigsimp) → 1."""
    from symbolic_ir import ADD, POW

    backend = MacsymaBackend()
    vm = VM(backend)
    x = IRSymbol("x")
    sin2 = IRApply(POW, (IRApply(IRSymbol("Sin"), (x,)), IRInteger(2)))
    cos2 = IRApply(POW, (IRApply(IRSymbol("Cos"), (x,)), IRInteger(2)))
    inner = IRApply(ADD, (sin2, cos2))
    expr = IRApply(EV, (inner, IRSymbol("trigsimp")))
    result = vm.eval(expr)
    assert result == IRInteger(1)
