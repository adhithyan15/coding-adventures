"""Statement-terminator preservation: ``;`` vs ``$``.

The compiler wraps every top-level statement in a ``Display(...)`` or
``Suppress(...)`` IR node when ``wrap_terminators=True`` is passed to
``compile_macsyma``. The MACSYMA REPL turns that on so it can decide
whether to print results; consumers that drive the VM directly leave
it off and continue receiving raw expressions.

The wrapper heads are introduced by the macsyma-runtime layer; the
compiler emits them as plain ``IRSymbol`` literals (no import
dependency on the runtime).
"""

from __future__ import annotations

from macsyma_compiler import compile_macsyma
from macsyma_parser import parse_macsyma
from symbolic_ir import IRApply, IRInteger, IRSymbol


def _compile_all(source: str) -> list:
    return compile_macsyma(parse_macsyma(source), wrap_terminators=True)


def test_default_does_not_wrap() -> None:
    """``compile_macsyma(ast)`` without the kwarg returns raw expressions."""
    [stmt] = compile_macsyma(parse_macsyma("42;"))
    assert stmt == IRInteger(42)


def test_semicolon_emits_display_wrapper() -> None:
    [stmt] = _compile_all("42;")
    assert isinstance(stmt, IRApply)
    assert stmt.head == IRSymbol("Display")
    assert stmt.args == (IRInteger(42),)


def test_dollar_emits_suppress_wrapper() -> None:
    [stmt] = _compile_all("42$")
    assert isinstance(stmt, IRApply)
    assert stmt.head == IRSymbol("Suppress")
    assert stmt.args == (IRInteger(42),)


def test_mixed_terminators_preserve_each() -> None:
    """A program with mixed terminators yields the right wrapper per stmt."""
    stmts = _compile_all("1$ 2; 3$ 4;")
    heads = [
        s.head.name if isinstance(s, IRApply) and isinstance(s.head, IRSymbol)
        else None
        for s in stmts
    ]
    assert heads == ["Suppress", "Display", "Suppress", "Display"]


def test_inner_expression_preserved_under_wrapper() -> None:
    """``x + 1;`` wraps an `Add(x, 1)` in `Display`."""
    [stmt] = _compile_all("x + 1;")
    assert isinstance(stmt, IRApply)
    assert stmt.head == IRSymbol("Display")
    inner = stmt.args[0]
    assert isinstance(inner, IRApply)
    assert inner.head == IRSymbol("Add")
    assert inner.args == (IRSymbol("x"), IRInteger(1))


def test_function_definition_wrapped_in_display() -> None:
    """``f(x) := x^2;`` is also wrapped (non-side-effect-free statements
    get wrapped same as expressions)."""
    [stmt] = _compile_all("f(x) := x^2;")
    assert isinstance(stmt, IRApply)
    assert stmt.head == IRSymbol("Display")
    inner = stmt.args[0]
    assert isinstance(inner, IRApply)
    assert inner.head == IRSymbol("Define")
