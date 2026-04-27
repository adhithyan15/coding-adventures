"""Pretty-print symbolic IR back to source text.

The package is dialect-aware: the same walker emits MACSYMA, Mathematica,
Maple, or Lisp by swapping a small :class:`Dialect` object.

Quick start::

    from cas_pretty_printer import pretty, MacsymaDialect
    from symbolic_ir import IRSymbol, IRInteger, IRApply, ADD, POW

    x = IRSymbol("x")
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    print(pretty(expr, MacsymaDialect()))
    # x^2 + 1

For the always-prefix Lisp form, use :func:`format_lisp` (which bypasses
the walker entirely)::

    from cas_pretty_printer import format_lisp
    print(format_lisp(expr))
    # (Add (Pow x 2) 1)
"""

from cas_pretty_printer.dialect import BaseDialect, Dialect
from cas_pretty_printer.lisp import LispDialect, format_lisp
from cas_pretty_printer.macsyma import MacsymaDialect
from cas_pretty_printer.maple import MapleDialect
from cas_pretty_printer.mathematica import MathematicaDialect
from cas_pretty_printer.walker import (
    pretty,
    register_head_formatter,
    unregister_head_formatter,
)

__all__ = [
    "BaseDialect",
    "Dialect",
    "LispDialect",
    "MacsymaDialect",
    "MapleDialect",
    "MathematicaDialect",
    "format_lisp",
    "pretty",
    "register_head_formatter",
    "unregister_head_formatter",
]
