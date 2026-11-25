"""Type naming must bypass metaclass code; truncation never splits escapes."""

from __future__ import annotations

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
    mux4,
    mux8,
    mux_n,
    nand_and,
    nand_not,
    nand_or,
    nand_xor,
    priority_encoder,
    register,
    shift_register,
)
from logic_gates.gates import _safe_type_name

META_CALLS: dict[str, int] = {"n": 0}


class _CountingMeta(type):
    def __getattribute__(cls, name: str) -> object:
        META_CALLS["n"] += 1
        return super().__getattribute__(name)


class Counted(metaclass=_CountingMeta):
    pass


GATES = [
    (AND, 2),
    (OR, 2),
    (XOR, 2),
    (NAND, 2),
    (NOR, 2),
    (XNOR, 2),
    (nand_and, 2),
    (nand_or, 2),
    (nand_xor, 2),
    (NOT, 1),
    (nand_not, 1),
]


def test_metaclass_getattribute_never_dispatched() -> None:
    obj = Counted()
    calls = [
        lambda: mux4(0, 0, 0, 0, obj),  # type: ignore[arg-type]
        lambda: mux8([0] * 8, obj),  # type: ignore[arg-type]
        lambda: decoder(obj),  # type: ignore[arg-type]
        lambda: encoder(obj),  # type: ignore[arg-type]
        lambda: priority_encoder(obj),  # type: ignore[arg-type]
        lambda: mux_n([0, 1], obj),  # type: ignore[arg-type]
        lambda: demux(1, obj, 2),  # type: ignore[arg-type]
        lambda: register(obj, 1),  # type: ignore[arg-type]
        lambda: shift_register(0, 1, obj, 2),  # type: ignore[arg-type]
        lambda: shift_register(0, 1, [], obj),  # type: ignore[arg-type]
    ]
    for gate, arity in GATES:
        calls.append(lambda g=gate, a=arity: g(*([obj] * a)))
    for call in calls:
        META_CALLS["n"] = 0
        with pytest.raises(TypeError, match="Counted"):
            call()
        assert META_CALLS["n"] == 0


def test_truncation_never_splits_an_escape() -> None:
    assert _safe_type_name(type("a" * 62 + "\n\n", (), {})()) == "a" * 62 + "..."
    assert _safe_type_name(type("a" * 60 + "\n", (), {})()) == "a" * 60 + "\\x0a"
    assert len(_safe_type_name(type("\U0001f600" * 50, (), {})())) <= 67


def test_mux_empty_lists_use_exact_length_message() -> None:
    with pytest.raises(ValueError, match="exactly 2 bits"):
        mux4(0, 0, 0, 0, [])
    with pytest.raises(ValueError, match="exactly 8 bits"):
        mux8([], [0, 0, 0])
    with pytest.raises(ValueError, match="exactly 3 bits"):
        mux8([0] * 8, [])
