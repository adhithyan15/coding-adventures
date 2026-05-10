"""Named-rule store for the ``defrule`` / ``apply1`` / ``apply2`` system.

This module provides :class:`RuleStore`, a per-VM mutable dictionary that
maps rule names (Python ``str``) to compiled
``IRApply(Rule, (lhs, rhs))`` nodes produced by :func:`cas_pattern_matching.nodes.Rule`.

The store is intentionally simple — just a thin wrapper around a ``dict``
with a descriptive API.  The actual pattern compilation (substituting
declared variables with ``Pattern(name, Blank(...))`` nodes) is handled by
:class:`~cas_pattern_matching.matchdeclare.MatchDeclareContext.compile_pattern`
before the rule is passed to :meth:`RuleStore.store`.

Relationship to the rest of the VM
-----------------------------------
One :class:`RuleStore` lives on each :class:`~symbolic_vm.vm.VM` as
``vm.named_rules``.

- ``defrule_handler``  →  ``store.store(name, rule)``
- ``apply1_handler``   →  ``store.get(name)``, then ``apply_rule(rule, expr)``
- ``apply2_handler``   →  ``store.get(name)``, then ``rewrite(expr, [rule])``

Example usage::

    from cas_pattern_matching.defrule_engine import RuleStore
    from cas_pattern_matching.nodes import Blank, Pattern, Rule
    from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, POW, SIN, COS

    store = RuleStore()

    # Pre-compiled rule: sin(x)^2 + cos(x)^2 → 1
    x_pat = Pattern("x", Blank())
    lhs = IRApply(ADD, (
        IRApply(POW, (IRApply(SIN, (x_pat,)), IRInteger(2))),
        IRApply(POW, (IRApply(COS, (x_pat,)), IRInteger(2))),
    ))
    rule = Rule(lhs, IRInteger(1))
    store.store("pyth", rule)

    stored = store.get("pyth")
    assert stored is not None
"""

from __future__ import annotations

from symbolic_ir import IRApply


class RuleStore:
    """Per-VM named-rule store.

    Maps rule names to compiled :func:`~cas_pattern_matching.nodes.Rule`
    IR nodes.  Rules are installed by the ``defrule_handler`` and
    retrieved by ``apply1_handler`` / ``apply2_handler``.

    Thread-safety: none — the VM is single-threaded.
    """

    def __init__(self) -> None:
        # Maps rule name (str) → compiled IRApply(Rule, (lhs, rhs)) node.
        self._rules: dict[str, IRApply] = {}

    # ------------------------------------------------------------------
    # Mutation API
    # ------------------------------------------------------------------

    def store(self, name: str, rule: IRApply) -> None:
        """Install or replace the named rule.

        Parameters
        ----------
        name:
            The rule's identifier (e.g. ``"r1"``, ``"pythagorean"``).
        rule:
            A compiled ``IRApply(Rule, (lhs, rhs))`` node.
        """
        self._rules[name] = rule

    def remove(self, name: str) -> None:
        """Remove ``name`` from the store, if present."""
        self._rules.pop(name, None)

    def clear(self) -> None:
        """Remove every stored rule."""
        self._rules.clear()

    # ------------------------------------------------------------------
    # Query API
    # ------------------------------------------------------------------

    def get(self, name: str) -> IRApply | None:
        """Return the compiled rule for ``name``, or ``None`` if absent."""
        return self._rules.get(name)

    def names(self) -> list[str]:
        """Return a sorted list of all stored rule names."""
        return sorted(self._rules.keys())

    def __len__(self) -> int:
        return len(self._rules)

    def __contains__(self, name: object) -> bool:
        return name in self._rules

    def __repr__(self) -> str:
        return f"RuleStore({list(self._rules.keys())!r})"
