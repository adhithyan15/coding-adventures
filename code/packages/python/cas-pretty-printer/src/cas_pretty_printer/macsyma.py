"""MACSYMA / Maxima dialect.

Surface syntax conventions:

- Function calls use parens: ``sin(x)``, ``f(x, y)``.
- Lists use square brackets: ``[1, 2, 3]``.
- Power is ``^`` (also accepts ``**`` on input — output uses ``^``).
- Equality is ``=``; not-equal is ``#``.
- Function names are lowercase: ``sin``, ``cos``, ``log``, ``exp``,
  ``diff``, ``integrate``.
- Surface sugar:
    - ``Add(x, Neg(y))`` displays as ``x - y``.
    - ``Mul(x, Inv(y))`` displays as ``x / y``.
    - ``Mul(-1, x)`` displays as ``-x``.
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_pretty_printer.dialect import BaseDialect

_NEG = IRSymbol("Neg")
_INV = IRSymbol("Inv")
_SUB = IRSymbol("Sub")
_DIV = IRSymbol("Div")
_ADD = IRSymbol("Add")
_MUL = IRSymbol("Mul")


class MacsymaDialect(BaseDialect):
    """Maxima/MACSYMA flavor of CAS source text."""

    name = "macsyma"

    def try_sugar(self, node: IRApply) -> IRNode | None:
        head = node.head
        if not isinstance(head, IRSymbol):
            return None

        # Mul(-1, x) → Neg(x)
        if head.name == "Mul" and len(node.args) >= 2:
            first = node.args[0]
            if isinstance(first, IRInteger) and first.value == -1:
                rest = node.args[1:]
                inner = rest[0] if len(rest) == 1 else IRApply(_MUL, tuple(rest))
                return IRApply(_NEG, (inner,))

        # Add(a, Neg(b)) → Sub(a, b). Apply only when there's exactly
        # one negated trailing argument, to keep the rule predictable.
        if head.name == "Add" and len(node.args) == 2:
            a, b = node.args
            if (
                isinstance(b, IRApply)
                and isinstance(b.head, IRSymbol)
                and b.head.name == "Neg"
                and len(b.args) == 1
            ):
                return IRApply(_SUB, (a, b.args[0]))

        # Mul(a, Inv(b)) → Div(a, b). Same caution: only the 2-arg case.
        if head.name == "Mul" and len(node.args) == 2:
            a, b = node.args
            if (
                isinstance(b, IRApply)
                and isinstance(b.head, IRSymbol)
                and b.head.name == "Inv"
                and len(b.args) == 1
            ):
                return IRApply(_DIV, (a, b.args[0]))

        return None
