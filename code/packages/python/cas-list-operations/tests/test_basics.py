"""Length, First, Rest, Last, Reverse, Append, Join."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRSymbol

from cas_list_operations import (
    LIST,
    ListOperationError,
    append,
    first,
    join,
    last,
    length,
    rest,
    reverse,
)


def L(*args: IRInteger | IRSymbol) -> IRApply:
    """Build an ``IRApply(LIST, args)`` quickly."""
    return IRApply(LIST, tuple(args))


def test_length_simple() -> None:
    assert length(L(IRInteger(1), IRInteger(2), IRInteger(3))) == IRInteger(3)


def test_length_empty() -> None:
    assert length(L()) == IRInteger(0)


def test_length_non_list_raises() -> None:
    with pytest.raises(ListOperationError):
        length(IRInteger(5))


def test_first() -> None:
    assert first(L(IRInteger(7), IRInteger(8))) == IRInteger(7)


def test_first_empty_raises() -> None:
    with pytest.raises(ListOperationError):
        first(L())


def test_rest() -> None:
    assert rest(L(IRInteger(1), IRInteger(2), IRInteger(3))) == L(
        IRInteger(2), IRInteger(3)
    )


def test_rest_empty_raises() -> None:
    with pytest.raises(ListOperationError):
        rest(L())


def test_last() -> None:
    assert last(L(IRInteger(1), IRInteger(2), IRInteger(3))) == IRInteger(3)


def test_last_empty_raises() -> None:
    with pytest.raises(ListOperationError):
        last(L())


def test_reverse() -> None:
    assert reverse(L(IRInteger(1), IRInteger(2), IRInteger(3))) == L(
        IRInteger(3), IRInteger(2), IRInteger(1)
    )


def test_append_two_lists() -> None:
    assert append(L(IRInteger(1)), L(IRInteger(2))) == L(IRInteger(1), IRInteger(2))


def test_append_many() -> None:
    a = L(IRInteger(1))
    b = L(IRInteger(2), IRInteger(3))
    c = L(IRInteger(4))
    assert append(a, b, c) == L(
        IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4)
    )


def test_append_empty() -> None:
    assert append(L(), L(IRInteger(1)), L()) == L(IRInteger(1))


def test_join_alias() -> None:
    assert join(L(IRInteger(1)), L(IRInteger(2))) == append(
        L(IRInteger(1)), L(IRInteger(2))
    )
