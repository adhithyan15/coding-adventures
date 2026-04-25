"""Display / Suppress wrappers — identity-on-inner."""

from __future__ import annotations

from symbolic_ir import ADD, IRApply, IRInteger, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import DISPLAY, SUPPRESS, MacsymaBackend


def test_display_unwraps() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    expr = IRApply(DISPLAY, (IRApply(ADD, (IRInteger(2), IRInteger(3))),))
    # The inner Add evaluates first, then Display returns the result.
    assert vm.eval(expr) == IRInteger(5)


def test_suppress_unwraps() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    expr = IRApply(SUPPRESS, (IRApply(ADD, (IRInteger(1), IRInteger(2))),))
    assert vm.eval(expr) == IRInteger(3)


def test_display_with_symbolic_inner() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    x = IRSymbol("x")
    expr = IRApply(DISPLAY, (x,))
    assert vm.eval(expr) == x


def test_display_arity_validation() -> None:
    backend = MacsymaBackend()
    vm = VM(backend)
    import pytest

    with pytest.raises(ValueError):
        vm.eval(IRApply(DISPLAY, (IRInteger(1), IRInteger(2))))
