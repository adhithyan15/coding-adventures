"""MacsymaBackend resolves %, %iN, %oN via the History."""

from __future__ import annotations

from symbolic_ir import IRInteger, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import History, MacsymaBackend


def test_history_fallback_resolves_percent() -> None:
    h = History()
    h.record_output(IRInteger(42))
    backend = MacsymaBackend(history=h)
    vm = VM(backend)
    # `%` evaluates to 42 — history fallback fires after env miss.
    assert vm.eval(IRSymbol("%")) == IRInteger(42)


def test_history_fallback_resolves_percent_o1() -> None:
    h = History()
    h.record_output(IRInteger(7))
    backend = MacsymaBackend(history=h)
    vm = VM(backend)
    assert vm.eval(IRSymbol("%o1")) == IRInteger(7)


def test_history_fallback_resolves_percent_i2() -> None:
    h = History()
    x = IRSymbol("x")
    h.record_input(IRInteger(1))
    h.record_input(x)
    backend = MacsymaBackend(history=h)
    vm = VM(backend)
    assert vm.eval(IRSymbol("%i2")) == x


def test_unbound_normal_name_stays_symbolic() -> None:
    """A bare `xyz` with no env binding and not a history reference
    stays symbolic (SymbolicBackend behavior)."""
    backend = MacsymaBackend()
    vm = VM(backend)
    assert vm.eval(IRSymbol("xyz")) == IRSymbol("xyz")


def test_env_binding_takes_precedence_over_history() -> None:
    """If a user did `%o1: 99`, the env wins over the history."""
    h = History()
    h.record_output(IRInteger(42))
    backend = MacsymaBackend(history=h)
    backend.bind("%o1", IRInteger(99))
    vm = VM(backend)
    assert vm.eval(IRSymbol("%o1")) == IRInteger(99)
