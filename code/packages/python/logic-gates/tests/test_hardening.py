"""LG02 section 9 type-confusion rows, bounded error text, container pinning."""

from __future__ import annotations

import enum

import pytest

from logic_gates import (
    AND,
    NAND,
    NOR,
    NOT,
    OR,
    XNOR,
    XOR,
    decoder,
    demux,
    encoder,
    mux_n,
    nand_and,
    nand_not,
    nand_or,
    nand_xor,
    priority_encoder,
    register,
    shift_register,
)
from logic_gates.gates import _safe_int_text, _safe_type_name

GATES_2 = [AND, OR, XOR, NAND, NOR, XNOR, nand_and, nand_or, nand_xor]
GATES_1 = [NOT, nand_not]
ALL = [(g, 2) for g in GATES_2] + [(g, 1) for g in GATES_1]

CALLS: dict[str, int] = {"n": 0}


def _spy(name: str):  # noqa: ANN202
    def method(self, *a, **k):  # noqa: ANN001, ANN002, ANN003, ANN202
        CALLS["n"] += 1
        return 1

    method.__name__ = name
    return method


class Spy(int):
    """int subclass whose every dunder lies and counts calls."""

    __eq__ = _spy("__eq__")
    __index__ = _spy("__index__")
    __repr__ = _spy("__repr__")
    __str__ = _spy("__str__")
    __format__ = _spy("__format__")
    __bool__ = _spy("__bool__")
    __hash__ = _spy("__hash__")


class Color(enum.IntEnum):
    ZERO = 0
    ONE = 1


NewlineName = type("Bad\nName\x1b[31m" + "x" * 500, (), {})


class _LyingMeta(type):
    """Metaclass whose class-level ``__qualname__`` read is not a str."""

    def __getattribute__(cls, name: str) -> object:
        if name == "__qualname__":
            return 5
        return super().__getattribute__(name)


class Lying(metaclass=_LyingMeta):
    pass



def _fn() -> None:  # pragma: no cover
    return None


TYPE_ROWS: list[object] = [
    "1\n",
    " 1",
    "0x1",
    "1e0",
    "١",
    [[1]],
    _fn,
    Color.ONE,
    Color.ZERO,
    Spy(1),
    NewlineName(),
    Lying(),
]
VALUE_ROWS: list[int] = [2**32 + 1, 2**53 + 1, 2**64 + 1, 10**5000, -(10**5000)]


def _check_message(exc: BaseException) -> None:
    text = str(exc)
    assert len(text) <= 256, len(text)
    assert all(c.isprintable() for c in text), repr(text)


def _call(gate, arity: int, pos: int, bad: object):  # noqa: ANN001, ANN202
    args: list[object] = [1] * arity
    args[pos] = bad
    return gate(*args)


@pytest.mark.parametrize(("gate", "arity"), ALL)
def test_type_confusion_rows(gate, arity: int) -> None:  # noqa: ANN001
    for pos in range(arity):
        for bad in TYPE_ROWS:
            CALLS["n"] = 0
            with pytest.raises(TypeError, match="must be an int") as ei:
                _call(gate, arity, pos, bad)
            _check_message(ei.value)
            assert CALLS["n"] == 0, (gate.__name__, bad)


@pytest.mark.parametrize(("gate", "arity"), ALL)
def test_out_of_range_int_rows(gate, arity: int) -> None:  # noqa: ANN001
    for pos in range(arity):
        for bad in VALUE_ROWS:
            with pytest.raises(ValueError, match="must be 0 or 1") as ei:
                _call(gate, arity, pos, bad)
            _check_message(ei.value)


def test_huge_int_reports_bit_length() -> None:
    with pytest.raises(ValueError, match="int with 16610 bits"):
        AND(10**5000, 1)


def test_safe_helpers() -> None:
    assert _safe_int_text(5) == "5"
    assert _safe_int_text(-(10**5000)).startswith("an int with")
    assert _safe_type_name(NewlineName()).count("\n") == 0
    assert len(_safe_type_name(NewlineName())) <= 70
    assert "\\x1b" in _safe_type_name(NewlineName())
    assert _safe_type_name(Lying()) == "Lying"
    assert _safe_type_name(3) == "int"


def test_block_function_messages_are_bounded() -> None:
    for call in (
        lambda: decoder(NewlineName()),
        lambda: encoder(NewlineName()),
        lambda: priority_encoder(NewlineName()),
        lambda: mux_n([0, 1], NewlineName()),
        lambda: demux(1, NewlineName(), 2),
        lambda: demux(1, [0], NewlineName()),
        lambda: demux(1, [0], 10**5000),
        lambda: register([0], 1, [], 10**5000),
    ):
        with pytest.raises((TypeError, ValueError)) as ei:
            call()
        _check_message(ei.value)


def test_non_list_containers_are_rejected() -> None:
    """#19946 chose: exact ``list`` only; tuple/range/dict raise TypeError."""
    for bad in ((0, 1), range(2), {0: 1, 1: 0}, "01"):
        with pytest.raises(TypeError):
            decoder(bad)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            encoder(bad)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            priority_encoder(bad)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            mux_n([0, 1], bad)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            mux_n(bad, [0])  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            demux(1, bad, 2)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            register([0, 1], 1, bad, 2)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            shift_register(0, 1, bad, 2)  # type: ignore[arg-type]
