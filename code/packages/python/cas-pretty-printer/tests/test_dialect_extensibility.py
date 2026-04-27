"""Custom head formatter registration — package-extension surface."""

from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRSymbol

from cas_pretty_printer import (
    MacsymaDialect,
    pretty,
    register_head_formatter,
    unregister_head_formatter,
)


def test_register_custom_head_formatter() -> None:
    """A custom formatter for a new head 'Matrix' is consulted by the walker."""

    def fmt_matrix(node, dialect, fmt):
        rows = [
            "[" + ", ".join(fmt(c) for c in row.args) + "]" for row in node.args
        ]
        return "matrix(" + ", ".join(rows) + ")"

    register_head_formatter("Matrix", fmt_matrix)
    try:
        list_head = IRSymbol("List")
        matrix_head = IRSymbol("Matrix")
        row1 = IRApply(list_head, (IRInteger(1), IRInteger(2)))
        row2 = IRApply(list_head, (IRInteger(3), IRInteger(4)))
        m = IRApply(matrix_head, (row1, row2))
        assert pretty(m, MacsymaDialect()) == "matrix([1, 2], [3, 4])"
    finally:
        unregister_head_formatter("Matrix")


def test_unregister_clears_formatter() -> None:
    """After unregister, the head falls back to function-call form."""

    def noop(node, dialect, fmt):
        return "CUSTOM"

    register_head_formatter("Foo", noop)
    unregister_head_formatter("Foo")

    foo_head = IRSymbol("Foo")
    expr = IRApply(foo_head, (IRInteger(1),))
    # No registered formatter — falls back to default function-call form.
    assert pretty(expr, MacsymaDialect()) == "Foo(1)"


def test_custom_dialect_subclass() -> None:
    """A dialect subclass that overrides operator spelling works end-to-end."""
    from symbolic_ir import ADD

    from cas_pretty_printer.dialect import BaseDialect

    class Verbose(BaseDialect):
        name = "verbose"
        binary_ops = {**BaseDialect.binary_ops, "Add": " plus "}

    expr = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
    assert pretty(expr, Verbose()) == "a plus b"
