"""Two reference backends: :class:`StrictBackend` and :class:`SymbolicBackend`.

They share ~90% of their behavior — the handler table, the held heads,
the environment storage. The two meaningful differences are:

+------------------+---------------------+----------------------------+
|                  | StrictBackend       | SymbolicBackend            |
+==================+=====================+============================+
| Unbound symbol   | raises ``NameError``| returns the symbol as-is   |
+------------------+---------------------+----------------------------+
| Unknown head     | raises ``NameError``| returns the expr as-is     |
+------------------+---------------------+----------------------------+
| Arith on symbols | raises ``TypeError``| folds identities, else     |
|                  |                     | leaves the expression      |
+------------------+---------------------+----------------------------+
| ``D`` handler    | not installed       | installed                  |
+------------------+---------------------+----------------------------+

Adding a new language-specific backend is typically a thin subclass:
override ``handlers()`` to add/replace a few entries, ``rules()`` if
you have custom rewrites, and leave everything else alone.
"""

from __future__ import annotations

from collections.abc import Mapping

from symbolic_ir import ASSIGN, DEFINE, IF, IRApply, IRNode, IRSymbol

from symbolic_vm.backend import Backend, Handler
from symbolic_vm.derivative import differentiate
from symbolic_vm.handlers import FALSE, TRUE, build_handler_table
from symbolic_vm.integrate import integrate

# Heads whose arguments must NOT be evaluated before dispatch. Shared
# by both backends — neither strict nor symbolic evaluation wants to
# pre-evaluate a function body being defined, or pre-evaluate the lhs
# of an assignment, or pre-evaluate both branches of an if.
_HELD_HEADS = frozenset({ASSIGN.name, DEFINE.name, IF.name})


class _BaseBackend(Backend):
    """Shared environment + held heads for the two reference backends."""

    def __init__(self) -> None:
        self._env: dict[str, IRNode] = {
            # ``True`` and ``False`` are pre-bound to themselves so that
            # unresolved-symbol policy doesn't kick in for MACSYMA's
            # ``true``/``false`` keywords. They act as inert symbols.
            "True": TRUE,
            "False": FALSE,
        }

    def lookup(self, name: str) -> IRNode | None:
        return self._env.get(name)

    def bind(self, name: str, value: IRNode) -> None:
        self._env[name] = value

    def hold_heads(self) -> frozenset[str]:
        return _HELD_HEADS


class StrictBackend(_BaseBackend):
    """Python-style numeric evaluator.

    Every name must be bound; every head must have a handler; every
    arithmetic operation must be fully numeric. Unknown cases raise.
    Useful for "calculator mode" — load a MACSYMA program with only
    numeric inputs and get numeric answers out.
    """

    def __init__(self) -> None:
        super().__init__()
        self._handlers = build_handler_table(simplify=False)

    def on_unresolved(self, symbol: IRSymbol) -> IRNode:
        raise NameError(f"undefined symbol: {symbol.name!r}")

    def on_unknown_head(self, expr: IRApply) -> IRNode:
        name = expr.head.name if isinstance(expr.head, IRSymbol) else "?"
        raise NameError(f"no handler for head: {name!r}")

    def handlers(self) -> Mapping[str, Handler]:
        return self._handlers


class SymbolicBackend(_BaseBackend):
    """Mathematica-style evaluator.

    Unbound names stay as free symbols; algebraic identities collapse
    the trivial cases; a derivative handler implements standard calculus
    rules; everything else stays in IR. The result is a tiny CAS —
    ``x + x`` won't combine (no polynomial normalization), but
    ``Add(x, 0)`` does, ``Pow(x, 0)`` is ``1``, ``D(x^2, x)`` is ``2*x``,
    and unknown functions pass through untouched.
    """

    def __init__(self) -> None:
        super().__init__()
        handlers = dict(build_handler_table(simplify=True))
        handlers["D"] = differentiate()
        handlers["Integrate"] = integrate()
        self._handlers = handlers

    def on_unresolved(self, symbol: IRSymbol) -> IRNode:
        return symbol

    def on_unknown_head(self, expr: IRApply) -> IRNode:
        return expr

    def handlers(self) -> Mapping[str, Handler]:
        return self._handlers
