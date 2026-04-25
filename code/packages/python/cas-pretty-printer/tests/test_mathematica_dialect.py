"""Mathematica dialect — bracket and spelling differences."""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    EQUAL,
    LIST,
    NOT_EQUAL,
    SIN,
    IRApply,
    IRInteger,
    IRSymbol,
)

from cas_pretty_printer import MathematicaDialect, pretty

D = MathematicaDialect()


def fmt(node: object) -> str:
    return pretty(node, D)  # type: ignore[arg-type]


def test_function_call_uses_square_brackets() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(SIN, (x,))) == "Sin[x]"


def test_function_keeps_camel_case() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(COS, (x,))) == "Cos[x]"


def test_list_uses_curly_braces() -> None:
    assert fmt(IRApply(LIST, (IRInteger(1), IRInteger(2)))) == "{1, 2}"


def test_double_equals() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(EQUAL, (a, b))) == "a == b"


def test_not_equal_bang() -> None:
    a, b = IRSymbol("a"), IRSymbol("b")
    assert fmt(IRApply(NOT_EQUAL, (a, b))) == "a != b"


def test_basic_addition() -> None:
    x = IRSymbol("x")
    assert fmt(IRApply(ADD, (x, IRInteger(1)))) == "x + 1"


def test_user_function_camel() -> None:
    f = IRSymbol("Foo")
    assert fmt(IRApply(f, (IRSymbol("x"),))) == "Foo[x]"
