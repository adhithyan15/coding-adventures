"""Lisp dialect — always-prefix output."""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_pretty_printer import format_lisp


def test_integer() -> None:
    assert format_lisp(IRInteger(5)) == "5"


def test_rational() -> None:
    assert format_lisp(IRRational(3, 4)) == "3/4"


def test_float() -> None:
    assert format_lisp(IRFloat(2.5)) == "2.5"


def test_string() -> None:
    assert format_lisp(IRString("hi")) == '"hi"'


def test_symbol() -> None:
    assert format_lisp(IRSymbol("x")) == "x"


def test_simple_apply() -> None:
    expr = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert format_lisp(expr) == "(Add x 1)"


def test_nested_apply() -> None:
    """Add(2, Mul(3, x)) → (Add 2 (Mul 3 x))."""
    expr = IRApply(ADD, (IRInteger(2), IRApply(MUL, (IRInteger(3), IRSymbol("x")))))
    assert format_lisp(expr) == "(Add 2 (Mul 3 x))"


def test_pow_no_special_handling() -> None:
    """Pow stays prefix even though MACSYMA would give it ^."""
    expr = IRApply(POW, (IRSymbol("x"), IRInteger(2)))
    assert format_lisp(expr) == "(Pow x 2)"


def test_no_arg_apply() -> None:
    """A call with no args still gets parens around the head."""
    expr = IRApply(IRSymbol("Now"), ())
    assert format_lisp(expr) == "(Now)"
