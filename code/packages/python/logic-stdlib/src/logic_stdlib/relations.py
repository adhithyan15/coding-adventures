"""Reusable list relations for the host-language logic library.

These helpers are written directly in terms of ``logic-engine`` goal
expressions. That is an important architectural point:

- the standard library does not get its own evaluator
- the Prolog implementation later should still lower into the same engine
- users can learn relational programming concepts from ordinary Python imports

The functions here use the miniKanren-style ``o`` suffix to emphasize that they
describe relations, not one-way deterministic computations.
"""

from __future__ import annotations

from logic_engine import GoalExpr, conj, defer, disj, eq, fresh, logic_list, term

__all__ = [
    "appendo",
    "conso",
    "emptyo",
    "heado",
    "lasto",
    "listo",
    "membero",
    "permuteo",
    "selecto",
    "tailo",
]


def emptyo(value: object) -> GoalExpr:
    """Succeed exactly when ``value`` is the canonical empty list."""

    return eq(value, logic_list([]))


def conso(head: object, tail: object, pair: object) -> GoalExpr:
    """Relate ``pair`` to a list whose first cell is ``head`` and ``tail``."""

    return eq(pair, term(".", head, tail))


def heado(items: object, head: object) -> GoalExpr:
    """Relate ``head`` to the first element of a non-empty list."""

    return fresh(1, lambda tail: conso(head, tail, items))


def tailo(items: object, tail: object) -> GoalExpr:
    """Relate ``tail`` to everything after the first element of a list."""

    return fresh(1, lambda head: conso(head, tail, items))


def lasto(items: object, last: object) -> GoalExpr:
    """Relate ``last`` to the final element of a non-empty list."""

    return disj(
        conso(last, logic_list([]), items),
        fresh(
            2,
            lambda head, tail: conj(
                conso(head, tail, items),
                defer(lasto, tail, last),
            ),
        ),
    )


def listo(value: object) -> GoalExpr:
    """Succeed exactly when ``value`` is a proper finite list.

    A canonical list is either:

    - the empty list ``[]``
    - or a cons cell whose tail is itself a proper list
    """

    return disj(
        emptyo(value),
        fresh(
            2,
            lambda head, tail: conj(
                conso(head, tail, value),
                defer(listo, tail),
            ),
        ),
    )


def membero(member: object, items: object) -> GoalExpr:
    """Relate ``member`` to each element that appears inside ``items``.

    The relation says: an element is a member of a list if it is either the
    head of that list or a member of the tail.
    """

    return fresh(
        2,
        lambda head, tail: disj(
            conso(member, tail, items),
            conj(conso(head, tail, items), defer(membero, member, tail)),
        ),
    )


def appendo(left: object, right: object, combined: object) -> GoalExpr:
    """Relate two lists to their concatenation.

    This is the classic recursive append relation:

    - appending ``[]`` to ``right`` yields ``right``
    - otherwise peel one head cell from ``left`` and mirror it into ``combined``
    """

    return disj(
        conj(emptyo(left), eq(right, combined)),
        fresh(
            3,
            lambda head, left_tail, out_tail: conj(
                conso(head, left_tail, left),
                conso(head, out_tail, combined),
                defer(appendo, left_tail, right, out_tail),
            ),
        ),
    )


def selecto(member: object, items: object, remainder: object) -> GoalExpr:
    """Relate one chosen element to the list with that element removed."""

    return fresh(
        3,
        lambda head, tail, rest_tail: disj(
            conj(
                conso(member, tail, items),
                eq(tail, remainder),
            ),
            conj(
                conso(head, tail, items),
                conso(head, rest_tail, remainder),
                defer(selecto, member, tail, rest_tail),
            ),
        ),
    )


def permuteo(items: object, permutation: object) -> GoalExpr:
    """Relate a list to every ordering of its elements."""

    return disj(
        conj(emptyo(items), emptyo(permutation)),
        fresh(
            3,
            lambda head, remaining, perm_tail: conj(
                selecto(head, items, remaining),
                conso(head, perm_tail, permutation),
                defer(permuteo, remaining, perm_tail),
            ),
        ),
    )
