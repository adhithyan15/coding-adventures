"""Canonical-form pass over the symbolic IR.

The canonical pass performs *purely structural* normalization:

1. **Flatten** — ``Add(a, Add(b, c))`` becomes ``Add(a, b, c)`` (and
   the same for ``Mul``).
2. **Sort** — args of commutative heads are sorted by a stable
   deterministic key, so ``Add(c, a, b)`` becomes ``Add(a, b, c)``.
3. **Singleton drop** — ``Add(x)`` becomes ``x``, ``Mul(x)`` becomes
   ``x``. (The IR allows them; canonical removes them.)
4. **Empty container** — ``Add()`` becomes ``IRInteger(0)``, ``Mul()``
   becomes ``IRInteger(1)``.

No identity rules, no constant folding, no rewrite — that's
:func:`cas_simplify.simplify`'s job. Keeping the canonical pass pure
makes it cheap and idempotent: ``canonical(canonical(x)) == canonical(x)``.

The sort key is structural: a tuple ``(rank, repr)`` where ``rank``
groups by node type so the order is deterministic across runs without
depending on Python's ``id()``.
"""

from __future__ import annotations

from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_simplify.heads import is_commutative_flat

# ---------------------------------------------------------------------------
# Sort key
# ---------------------------------------------------------------------------


def _rank(node: IRNode) -> int:
    """Stable ordering by node type — integers first, then rationals,
    then floats, then symbols, then compound, then strings.

    Numeric literals come first so they cluster together, which is
    what users expect when they see ``2 + x + y``: the constant is on
    the left.
    """
    if isinstance(node, IRInteger):
        return 0
    if isinstance(node, IRRational):
        return 1
    if isinstance(node, IRFloat):
        return 2
    if isinstance(node, IRSymbol):
        return 3
    if isinstance(node, IRApply):
        return 4
    if isinstance(node, IRString):
        return 5
    return 6


def _sort_key(node: IRNode) -> tuple[int, str]:
    """Total order on IR nodes: rank, then string form."""
    return (_rank(node), repr(node))


# ---------------------------------------------------------------------------
# Public canonical pass
# ---------------------------------------------------------------------------


def canonical(node: IRNode) -> IRNode:
    """Recursively normalize ``node`` into canonical form.

    Idempotent: applying twice gives the same result as applying once.
    """
    if isinstance(node, IRApply):
        return _canonical_apply(node)
    return node


def _canonical_apply(node: IRApply) -> IRNode:
    # First, canonicalize the children.
    new_head = canonical(node.head)
    new_args = tuple(canonical(a) for a in node.args)

    if isinstance(new_head, IRSymbol) and is_commutative_flat(new_head.name):
        new_args = _flatten(new_head.name, new_args)
        new_args = tuple(sorted(new_args, key=_sort_key))

        if not new_args:
            # Add() → 0, Mul() → 1.
            return IRInteger(0 if new_head.name == "Add" else 1)
        if len(new_args) == 1:
            # Singleton drop: Add(x) → x, Mul(x) → x.
            return new_args[0]

    if new_head is node.head and new_args == node.args:
        return node
    return IRApply(new_head, new_args)


def _flatten(head_name: str, args: tuple[IRNode, ...]) -> tuple[IRNode, ...]:
    """Flatten any direct ``IRApply(head_name, ...)`` children into the args."""
    out: list[IRNode] = []
    for a in args:
        if (
            isinstance(a, IRApply)
            and isinstance(a.head, IRSymbol)
            and a.head.name == head_name
        ):
            out.extend(a.args)
        else:
            out.append(a)
    return tuple(out)
