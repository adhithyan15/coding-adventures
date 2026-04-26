"""Map, Apply, Select, Sort, Flatten."""

from __future__ import annotations

from symbolic_ir import ADD, IRApply, IRInteger, IRSymbol

from cas_list_operations import LIST, apply_, flatten, map_, select, sort_


def L(*args: IRInteger | IRSymbol | IRApply) -> IRApply:
    return IRApply(LIST, tuple(args))


# ---- Map -----------------------------------------------------------------


def test_map_with_symbol_head() -> None:
    """map_(f, [1, 2, 3]) → [f(1), f(2), f(3)]."""
    f = IRSymbol("f")
    out = map_(f, L(IRInteger(1), IRInteger(2), IRInteger(3)))
    expected = L(
        IRApply(f, (IRInteger(1),)),
        IRApply(f, (IRInteger(2),)),
        IRApply(f, (IRInteger(3),)),
    )
    assert out == expected


# ---- Apply ---------------------------------------------------------------


def test_apply_replaces_head() -> None:
    """apply_(Add, [1, 2, 3]) → Add(1, 2, 3)."""
    out = apply_(ADD, L(IRInteger(1), IRInteger(2), IRInteger(3)))
    expected = IRApply(ADD, (IRInteger(1), IRInteger(2), IRInteger(3)))
    assert out == expected


# ---- Select --------------------------------------------------------------


def test_select_filters() -> None:
    """select([1,2,3,4], even?) → [2, 4]."""

    def even(node: IRApply | IRInteger | IRSymbol) -> bool:
        return isinstance(node, IRInteger) and node.value % 2 == 0

    lst = L(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))
    assert select(lst, even) == L(IRInteger(2), IRInteger(4))


def test_select_drops_all() -> None:
    """If predicate rejects everything, result is empty list."""
    lst = L(IRInteger(1), IRInteger(2))
    assert select(lst, lambda _: False) == L()


# ---- Sort ----------------------------------------------------------------


def test_sort_integers() -> None:
    out = sort_(L(IRInteger(3), IRInteger(1), IRInteger(2)))
    assert out == L(IRInteger(1), IRInteger(2), IRInteger(3))


# ---- Flatten -------------------------------------------------------------


def test_flatten_one_level() -> None:
    """``[1, [2, 3], 4]`` flattens once → ``[1, 2, 3, 4]``."""
    nested = L(
        IRInteger(1),
        L(IRInteger(2), IRInteger(3)),
        IRInteger(4),
    )
    assert flatten(nested, depth=1) == L(
        IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4)
    )


def test_flatten_partial() -> None:
    """``[1, [2, [3, 4]]]`` with depth=1 → ``[1, 2, [3, 4]]``."""
    inner = L(IRInteger(3), IRInteger(4))
    nested = L(IRInteger(1), L(IRInteger(2), inner))
    out = flatten(nested, depth=1)
    assert out == L(IRInteger(1), IRInteger(2), inner)


def test_flatten_unlimited() -> None:
    """``flatten(.., -1)`` flattens fully."""
    inner = L(IRInteger(3), IRInteger(4))
    nested = L(IRInteger(1), L(IRInteger(2), inner))
    out = flatten(nested, depth=-1)
    assert out == L(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))


def test_flatten_zero_depth_unchanged() -> None:
    nested = L(IRInteger(1), L(IRInteger(2)))
    assert flatten(nested, depth=0) == nested
