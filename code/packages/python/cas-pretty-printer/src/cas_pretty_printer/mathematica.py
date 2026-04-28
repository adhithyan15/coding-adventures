"""Mathematica dialect.

Surface syntax differences from MACSYMA:

- Function calls use square brackets: ``Sin[x]``, ``D[f, x]``.
- Lists use curly braces: ``{1, 2, 3}``.
- Function names are CamelCase (matches the IR head names directly).
- Equality is ``==``.
"""

from __future__ import annotations

from cas_pretty_printer.dialect import BaseDialect
from cas_pretty_printer.macsyma import MacsymaDialect


class MathematicaDialect(MacsymaDialect):
    """Mathematica flavor — reuses MACSYMA's sugar but swaps spellings."""

    name = "mathematica"

    # Override only the spellings.
    binary_ops = {
        **BaseDialect.binary_ops,
        "Equal": " == ",
        "NotEqual": " != ",
        "And": " && ",
        "Or": " || ",
    }
    unary_ops = {
        **BaseDialect.unary_ops,
        "Not": "!",
    }

    # Mathematica keeps capitalised names — `Sin`, `Cos`, etc. stay as-is.
    function_names: dict[str, str] = {}

    def list_brackets(self) -> tuple[str, str]:
        return ("{", "}")

    def call_brackets(self) -> tuple[str, str]:
        return ("[", "]")

    def function_name(self, head_name: str) -> str:
        return head_name
