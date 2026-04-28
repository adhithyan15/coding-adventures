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

from collections.abc import Iterator

from logic_engine import (
    Atom,
    Compound,
    GoalExpr,
    LogicVar,
    Number,
    Program,
    State,
    Term,
    conj,
    defer,
    disj,
    eq,
    fresh,
    logic_list,
    native_goal,
    num,
    solve_from,
    term,
)

__all__ = [
    "appendo",
    "conso",
    "emptyo",
    "heado",
    "lasto",
    "lengtho",
    "listo",
    "membero",
    "permuteo",
    "reverseo",
    "selecto",
    "subsequenceo",
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


def lengtho(items: object, length: object) -> GoalExpr:
    """Relate a proper finite list to its length.

    This intentionally covers the practical finite cases first:

    - counting an already proper list
    - validating a list against a known non-negative integer length
    - creating a fresh list skeleton for a known non-negative integer length

    When both sides are unknown, the relation fails instead of enumerating an
    infinite stream of longer and longer lists. Full open-ended generation
    belongs with a future CLP(FD)/fair-search story.
    """

    return native_goal(_lengtho_runner, items, length)


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


def _lengtho_runner(
    program_value: Program,
    state: State,
    args: tuple[Term, ...],
) -> Iterator[State]:
    items, length = args
    walked_length = state.substitution.walk(length)

    if isinstance(walked_length, Number):
        length_value = walked_length.value
        if not isinstance(length_value, int) or length_value < 0:
            return
        yield from solve_from(
            program_value,
            fresh(
                length_value,
                lambda *elements: eq(items, logic_list(list(elements))),
            ),
            state,
        )
        return

    known_length = _proper_list_length(items, state)
    if known_length is None:
        return

    yield from solve_from(program_value, eq(length, num(known_length)), state)


def _proper_list_length(items: Term, state: State) -> int | None:
    count = 0
    current = state.substitution.walk(items)

    while True:
        if isinstance(current, Atom) and current.symbol.name == "[]":
            return count
        if (
            isinstance(current, Compound)
            and current.functor.name == "."
            and len(current.args) == 2
        ):
            count += 1
            current = state.substitution.walk(current.args[1])
            continue
        if isinstance(current, LogicVar):
            return None
        return None


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


def subsequenceo(items: object, subsequence: object) -> GoalExpr:
    """Relate ``subsequence`` to any order-preserving deletion of ``items``."""

    return disj(
        conj(emptyo(items), emptyo(subsequence)),
        fresh(
            3,
            lambda head, tail, sub_tail: conj(
                conso(head, tail, items),
                disj(
                    conj(
                        conso(head, sub_tail, subsequence),
                        defer(subsequenceo, tail, sub_tail),
                    ),
                    defer(subsequenceo, tail, subsequence),
                ),
            ),
        ),
    )


def reverseo(items: object, reversed_items: object) -> GoalExpr:
    """Relate ``reversed_items`` to the reverse ordering of ``items``."""

    return disj(
        conj(emptyo(items), emptyo(reversed_items)),
        fresh(
            3,
            lambda head, tail, reversed_tail: conj(
                conso(head, tail, items),
                defer(reverseo, tail, reversed_tail),
                appendo(reversed_tail, logic_list([head]), reversed_items),
            ),
        ),
    )
