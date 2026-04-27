"""Range, Part."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger

from cas_list_operations import LIST, ListOperationError, part, range_


def L(*vals: int) -> IRApply:
    return IRApply(LIST, tuple(IRInteger(v) for v in vals))


# ---- Range ---------------------------------------------------------------


def test_range_one_arg() -> None:
    """range_(5) → [1..5] (MACSYMA convention)."""
    assert range_(5) == L(1, 2, 3, 4, 5)


def test_range_two_args() -> None:
    """range_(3, 7) → [3..7]."""
    assert range_(3, 7) == L(3, 4, 5, 6, 7)


def test_range_with_step() -> None:
    """range_(1, 10, 2) → [1, 3, 5, 7, 9]."""
    assert range_(1, 10, 2) == L(1, 3, 5, 7, 9)


def test_range_negative_step() -> None:
    """range_(10, 1, -2) → [10, 8, 6, 4, 2]."""
    assert range_(10, 1, -2) == L(10, 8, 6, 4, 2)


def test_range_step_zero_raises() -> None:
    with pytest.raises(ListOperationError):
        range_(1, 5, 0)


def test_range_zero_arg() -> None:
    assert range_(0) == L()


# ---- Part ----------------------------------------------------------------


def test_part_first() -> None:
    assert part(L(10, 20, 30), 1) == IRInteger(10)


def test_part_middle() -> None:
    assert part(L(10, 20, 30), 2) == IRInteger(20)


def test_part_negative() -> None:
    """``part(lst, -1)`` returns the last element."""
    assert part(L(10, 20, 30), -1) == IRInteger(30)


def test_part_zero_invalid() -> None:
    with pytest.raises(ListOperationError):
        part(L(10, 20, 30), 0)


def test_part_out_of_range() -> None:
    with pytest.raises(ListOperationError):
        part(L(10, 20, 30), 5)
