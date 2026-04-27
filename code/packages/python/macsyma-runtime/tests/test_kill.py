"""Kill handler — clear bindings."""

from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import KILL, MacsymaBackend


def test_kill_removes_binding() -> None:
    backend = MacsymaBackend()
    backend.bind("x", IRInteger(5))
    vm = VM(backend)
    # Sanity: x evaluates to 5.
    assert vm.eval(IRSymbol("x")) == IRInteger(5)
    # kill(x) clears it.
    vm.eval(IRApply(KILL, (IRSymbol("x"),)))
    # Now x is symbolic again.
    assert vm.eval(IRSymbol("x")) == IRSymbol("x")


def test_kill_multiple_args() -> None:
    backend = MacsymaBackend()
    backend.bind("x", IRInteger(1))
    backend.bind("y", IRInteger(2))
    vm = VM(backend)
    vm.eval(IRApply(KILL, (IRSymbol("x"), IRSymbol("y"))))
    assert vm.eval(IRSymbol("x")) == IRSymbol("x")
    assert vm.eval(IRSymbol("y")) == IRSymbol("y")


def test_kill_unbound_is_noop() -> None:
    """kill(x) on a never-bound name does nothing — no error."""
    backend = MacsymaBackend()
    vm = VM(backend)
    # Should not raise.
    vm.eval(IRApply(KILL, (IRSymbol("never_bound"),)))


def test_kill_all_clears_environment() -> None:
    backend = MacsymaBackend()
    backend.bind("x", IRInteger(1))
    backend.bind("y", IRInteger(2))
    vm = VM(backend)
    vm.eval(IRApply(KILL, (IRSymbol("all"),)))
    assert vm.eval(IRSymbol("x")) == IRSymbol("x")
    assert vm.eval(IRSymbol("y")) == IRSymbol("y")


def test_kill_all_clears_history() -> None:
    backend = MacsymaBackend()
    backend.history.record_input(IRInteger(1))
    backend.history.record_output(IRInteger(2))
    vm = VM(backend)
    vm.eval(IRApply(KILL, (IRSymbol("all"),)))
    assert backend.history.next_input_index() == 1
    assert backend.history.last_output() is None
