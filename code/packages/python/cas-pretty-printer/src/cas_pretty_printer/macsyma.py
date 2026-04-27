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
    - ``Add(x, Mul(a, Neg(b)))`` also displays as ``x - a*b`` (one
      level of recursive sugar on the second Add argument).
    - ``Add(-n, x)`` with a negative integer literal displays as
      ``x - n`` (swaps operands so the minus goes on the right).
    - ``Mul(x, Inv(y))`` displays as ``x / y``.
    - ``Mul(-1, x)`` displays as ``-x``.
    - ``Mul(a, Neg(b))`` displays as ``-(a*b)`` (unary minus out front).
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_pretty_printer.dialect import _DEFAULT_FUNCTION_NAMES, BaseDialect

_NEG = IRSymbol("Neg")
_INV = IRSymbol("Inv")
_SUB = IRSymbol("Sub")
_DIV = IRSymbol("Div")
_ADD = IRSymbol("Add")
_MUL = IRSymbol("Mul")


class MacsymaDialect(BaseDialect):
    """Maxima/MACSYMA flavor of CAS source text."""

    name = "macsyma"

    # Extend the default function-name table with MACSYMA-specific entries.
    function_names: dict[str, str] = {
        **_DEFAULT_FUNCTION_NAMES,
        "Atan2": "atan2",
    }

    # Symbol aliases for MACSYMA surface syntax.
    _SYMBOL_MAP: dict[str, str] = {
        "ImaginaryUnit": "%i",
    }

    def format_symbol(self, name: str) -> str:
        return self._SYMBOL_MAP.get(name, name)

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

        # Add(a, Neg(b)) → Sub(a, b).  Apply only when there's exactly
        # one negated trailing argument, to keep the rule predictable.
        #
        # Extended: peek one level of sugar on the second argument first.
        # This catches ``Add(x, Mul(a, Neg(b)))`` → ``Sub(x, Mul(a, b))``
        # (i.e. ``x - a*b``) without requiring a separate walker pass.
        #
        # Also handles Add(n<0, b) → Sub(b, -n) so that ``Add(-1, y)``
        # prints as ``y - 1`` rather than ``(-1) + y``.
        if head.name == "Add" and len(node.args) == 2:
            a, b = node.args
            # Peek: try one level of sugar on b to expose a Neg wrapper.
            b_effective: IRNode = b
            if isinstance(b, IRApply):
                sugared_b = self.try_sugar(b)
                if sugared_b is not None:
                    b_effective = sugared_b
            if (
                isinstance(b_effective, IRApply)
                and isinstance(b_effective.head, IRSymbol)
                and b_effective.head.name == "Neg"
                and len(b_effective.args) == 1
            ):
                return IRApply(_SUB, (a, b_effective.args[0]))
            # Negative integer literal as first Add argument: -n + b → b - n.
            if isinstance(a, IRInteger) and a.value < 0:
                return IRApply(_SUB, (b, IRInteger(-a.value)))

        # Mul(a, Inv(b)) → Div(a, b). Only the 2-arg case.
        # Mul(a, Neg(b)) → Neg(Mul(a, b)) — pulls unary minus to the front
        #   so that e.g. ``sin(x)*-sin(x)`` becomes ``-(sin(x)*sin(x))``.
        if head.name == "Mul" and len(node.args) == 2:
            a, b = node.args
            if (
                isinstance(b, IRApply)
                and isinstance(b.head, IRSymbol)
                and b.head.name == "Inv"
                and len(b.args) == 1
            ):
                return IRApply(_DIV, (a, b.args[0]))
            if (
                isinstance(b, IRApply)
                and isinstance(b.head, IRSymbol)
                and b.head.name == "Neg"
                and len(b.args) == 1
            ):
                return IRApply(_NEG, (IRApply(_MUL, (a, b.args[0])),))

        return None
