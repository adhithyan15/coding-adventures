"""mux2/mux4/mux8 must take exact lists and compute from a validated copy."""

from __future__ import annotations

import pytest

from logic_gates import mux2, mux4, mux8


class _Alternating(list):
    """List subclass whose reads alternate 0/1 regardless of content."""

    def __init__(self, *a: object) -> None:
        super().__init__(*a)
        self.flip = 0

    def __getitem__(self, i):  # noqa: ANN001, ANN204
        self.flip ^= 1
        return self.flip


def _gen(n: int):  # noqa: ANN202
    return (0 for _ in range(n))


def _bad_lists(n: int) -> list[object]:
    return [_Alternating([0] * n), tuple([0] * n), range(n), {0: 0}, _gen(n)]


def test_mux4_sel_rejects_non_exact_lists() -> None:
    for bad in _bad_lists(2):
        with pytest.raises(TypeError, match="list of bits"):
            mux4(1, 0, 0, 0, bad)  # type: ignore[arg-type]


def test_mux8_rejects_non_exact_lists() -> None:
    for bad in _bad_lists(3):
        with pytest.raises(TypeError, match="list of bits"):
            mux8([0] * 8, bad)  # type: ignore[arg-type]
    for bad in _bad_lists(8):
        with pytest.raises(TypeError, match="list of bits"):
            mux8(bad, [0, 0, 0])  # type: ignore[arg-type]


def test_mux2_sel_must_be_exact_int() -> None:
    for bad in ([1], (1,), range(1), {1: 1}, _gen(1), _Alternating([1])):
        with pytest.raises(TypeError):
            mux2(0, 1, bad)  # type: ignore[arg-type]


def test_mux4_mux8_still_work_and_length_checked() -> None:
    assert mux4(0, 0, 0, 1, [1, 1]) == 1
    assert mux8([0] * 7 + [1], [1, 1, 1]) == 1
    with pytest.raises(ValueError, match="exactly 2"):
        mux4(0, 0, 0, 0, [0])
    with pytest.raises(ValueError, match="exactly 8"):
        mux8([0] * 7, [0, 0, 0])
    with pytest.raises(ValueError, match="exactly 3"):
        mux8([0] * 8, [0, 0])
