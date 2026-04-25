"""Ev — phase-A skeleton (numer flag only)."""

from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRSymbol
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
