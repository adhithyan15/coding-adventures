"""Tests for the ``If`` handler in both backends.

``If`` is the canonical held head: both branches appear as args but
only one should execute. We verify that by giving the unchosen
branch an expression that would raise if evaluated.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    EQUAL,
    IF,
    IRApply,
    IRInteger,
    IRSymbol,
)

from symbolic_vm import VM, StrictBackend, SymbolicBackend


def test_if_true_takes_then_branch() -> None:
    vm = VM(StrictBackend())
    expr = IRApply(IF, (IRSymbol("True"), IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == IRInteger(1)


def test_if_false_takes_else_branch() -> None:
    vm = VM(StrictBackend())
    expr = IRApply(IF, (IRSymbol("False"), IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == IRInteger(2)


def test_if_without_else_returns_false() -> None:
    vm = VM(StrictBackend())
    expr = IRApply(IF, (IRSymbol("False"), IRInteger(1)))
    assert vm.eval(expr) == IRSymbol("False")


def test_if_unchosen_branch_not_evaluated() -> None:
    # The else branch would raise (``undef + 1`` hits an unresolved
    # symbol) but we never take it, so the whole thing evaluates fine.
    vm = VM(StrictBackend())
    expr = IRApply(
        IF,
        (
            IRSymbol("True"),
            IRInteger(42),
            IRApply(ADD, (IRSymbol("undef"), IRInteger(1))),
        ),
    )
    assert vm.eval(expr) == IRInteger(42)


def test_if_with_computed_predicate() -> None:
    vm = VM(StrictBackend())
    expr = IRApply(
        IF,
        (
            IRApply(EQUAL, (IRInteger(3), IRInteger(3))),
            IRInteger(100),
            IRInteger(200),
        ),
    )
    assert vm.eval(expr) == IRInteger(100)


def test_if_symbolic_predicate_stays_unevaluated() -> None:
    vm = VM(SymbolicBackend())
    expr = IRApply(IF, (IRSymbol("x"), IRInteger(1), IRInteger(2)))
    result = vm.eval(expr)
    assert result == expr


def test_if_wrong_arity_raises() -> None:
    vm = VM(StrictBackend())
    with pytest.raises(TypeError, match="If expects 2 or 3 arguments"):
        vm.eval(IRApply(IF, (IRSymbol("True"),)))
