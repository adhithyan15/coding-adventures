"""MACSYMA presentation helpers for evaluated runtime results."""

from __future__ import annotations

from cas_pretty_printer import MacsymaDialect, pretty
from symbolic_ir import IRApply, IRNode, IRSymbol

from macsyma_runtime.heads import EV

_DIALECT = MacsymaDialect()


def output_text_for(input_expr: IRNode, output: IRNode) -> str:
    """Return the display text selected by MACSYMA evaluation flags."""
    style = "2d" if has_ev_flag(input_expr, "display2d") else "linear"
    return pretty(output, _DIALECT, style=style)


def has_ev_flag(input_expr: IRNode, flag: str) -> bool:
    """Return whether ``input_expr`` is ``Ev(..., flag, ...)``."""
    if not isinstance(input_expr, IRApply):
        return False
    if input_expr.head != EV:
        return False
    return any(
        isinstance(arg, IRSymbol) and arg.name.lower() == flag.lower()
        for arg in input_expr.args[1:]
    )
