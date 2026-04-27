"""Pure-Python implementations of list operations.

Every function takes raw IR (typically an ``IRApply(LIST, args)``) and
returns raw IR. Errors raise :class:`ListOperationError` so backends
can convert them to user-facing messages.
"""

from __future__ import annotations

from collections.abc import Callable

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_list_operations.heads import LIST


class ListOperationError(ValueError):
    """Raised when an operation is given a non-List or out-of-range index."""


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _as_list(node: IRNode) -> tuple[IRNode, ...]:
    """Return the args of a List, or raise."""
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "List"
    ):
        return node.args
    raise ListOperationError(f"expected a List, got {node!r}")


def _make_list(args: tuple[IRNode, ...]) -> IRApply:
    return IRApply(LIST, args)


# ---------------------------------------------------------------------------
# Public operations
# ---------------------------------------------------------------------------


def length(lst: IRNode) -> IRInteger:
    """Number of elements."""
    return IRInteger(len(_as_list(lst)))


def first(lst: IRNode) -> IRNode:
    """First element. Empty list raises."""
    args = _as_list(lst)
    if not args:
        raise ListOperationError("first() of empty list")
    return args[0]


def rest(lst: IRNode) -> IRApply:
    """Everything but the first element. Empty list raises."""
    args = _as_list(lst)
    if not args:
        raise ListOperationError("rest() of empty list")
    return _make_list(args[1:])


def last(lst: IRNode) -> IRNode:
    """Last element. Empty list raises."""
    args = _as_list(lst)
    if not args:
        raise ListOperationError("last() of empty list")
    return args[-1]


def reverse(lst: IRNode) -> IRApply:
    """Reverse the list."""
    return _make_list(tuple(reversed(_as_list(lst))))


def append(*lsts: IRNode) -> IRApply:
    """Concatenate two or more lists."""
    out: list[IRNode] = []
    for lst in lsts:
        out.extend(_as_list(lst))
    return _make_list(tuple(out))


def join(*lsts: IRNode) -> IRApply:
    """Alias for :func:`append`. Mathematica spelling."""
    return append(*lsts)


def part(lst: IRNode, index: int) -> IRNode:
    """1-based indexed access. Negative indices count from the end."""
    args = _as_list(lst)
    if index == 0:
        raise ListOperationError("Part: index 0 is invalid (1-based)")
    py_index = index - 1 if index > 0 else index
    if not -len(args) <= py_index < len(args):
        raise ListOperationError(f"Part: index {index} out of range")
    return args[py_index]


def range_(start: int, stop: int | None = None, step: int = 1) -> IRApply:
    """Generate ``[start..stop]`` inclusive, with optional ``step``.

    Single-arg form ``range_(n)`` produces ``[1, 2, ..., n]`` (MACSYMA
    convention). Inclusive on both ends.
    """
    if stop is None:
        # range_(n): [1..n]
        return _make_list(tuple(IRInteger(i) for i in range(1, start + 1)))
    if step == 0:
        raise ListOperationError("Range: step cannot be 0")
    values = (
        range(start, stop + 1, step) if step > 0 else range(start, stop - 1, step)
    )
    return _make_list(tuple(IRInteger(i) for i in values))


def map_(f: IRNode, lst: IRNode) -> IRApply:
    """Apply ``f`` to each element.

    ``f`` is the IR head — an ``IRSymbol`` or any other ``IRNode``.
    The result is ``[f(a), f(b), f(c)]`` as an unevaluated ``IRApply``
    chain (the VM later evaluates each).
    """
    args = _as_list(lst)
    return _make_list(tuple(IRApply(f, (a,)) for a in args))


def apply_(f: IRNode, lst: IRNode) -> IRApply:
    """Replace the list's head: ``Apply(Add, [a, b, c]) → Add(a, b, c)``."""
    return IRApply(f, _as_list(lst))


def select(lst: IRNode, predicate: Callable[[IRNode], bool]) -> IRApply:
    """Keep elements where ``predicate(elem)`` is True.

    ``predicate`` is a Python callable. For symbolic predicates (e.g.,
    ``IsEven``), backends typically supply a wrapper that evaluates
    the head against each element before passing through.
    """
    return _make_list(tuple(a for a in _as_list(lst) if predicate(a)))


def sort_(lst: IRNode) -> IRApply:
    """Stable sort by ``repr`` (matches the canonical-form ordering).

    Same key as ``cas_simplify.canonical`` would use, but this package
    doesn't depend on cas-simplify — we duplicate the trivial key
    rather than introduce a circular dependency.
    """
    args = _as_list(lst)
    return _make_list(tuple(sorted(args, key=repr)))


def flatten(lst: IRNode, depth: int = 1) -> IRApply:
    """Flatten ``depth`` levels of nested lists.

    ``depth=1`` flattens once: ``[1, [2, 3], [4, [5, 6]]]`` →
    ``[1, 2, 3, 4, [5, 6]]``. Use a large depth (or ``-1`` sentinel
    for "all") to flatten completely.
    """
    if depth < 0:
        depth = 10**9  # effectively unlimited
    return _make_list(_flatten_args(_as_list(lst), depth))


def _flatten_args(
    args: tuple[IRNode, ...], depth: int
) -> tuple[IRNode, ...]:
    if depth == 0:
        return args
    out: list[IRNode] = []
    for a in args:
        if (
            isinstance(a, IRApply)
            and isinstance(a.head, IRSymbol)
            and a.head.name == "List"
        ):
            out.extend(_flatten_args(a.args, depth - 1))
        else:
            out.append(a)
    return tuple(out)
