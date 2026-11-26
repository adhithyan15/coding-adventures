"""Exhaustive validation of the 11 basic gates.

Contract under test (see ``_validate_bit``):
  * a value whose exact type is not ``int`` (bool, float, str, None, int
    subclasses, ...) raises TypeError;
  * a genuine ``int`` other than 0 or 1 raises ValueError;
  * every valid input yields exactly the ``int`` 0 or 1 (never a bool).
Every argument position is checked, with the other argument both valid and
invalid.  Expected truth tables are written as literals.
"""

from __future__ import annotations

import itertools
from collections.abc import Callable
from decimal import Decimal
from fractions import Fraction

import pytest

from logic_gates import (
    AND,
    NAND,
    NOR,
    NOT,
    OR,
    XNOR,
    XOR,
    nand_and,
    nand_not,
    nand_or,
    nand_xor,
)

Gate = Callable[..., int]

# (a, b) -> out, as literals.
TABLES_2: dict[str, tuple[Gate, dict[tuple[int, int], int]]] = {
    "AND": (AND, {(0, 0): 0, (0, 1): 0, (1, 0): 0, (1, 1): 1}),
    "OR": (OR, {(0, 0): 0, (0, 1): 1, (1, 0): 1, (1, 1): 1}),
    "XOR": (XOR, {(0, 0): 0, (0, 1): 1, (1, 0): 1, (1, 1): 0}),
    "NAND": (NAND, {(0, 0): 1, (0, 1): 1, (1, 0): 1, (1, 1): 0}),
    "NOR": (NOR, {(0, 0): 1, (0, 1): 0, (1, 0): 0, (1, 1): 0}),
    "XNOR": (XNOR, {(0, 0): 1, (0, 1): 0, (1, 0): 0, (1, 1): 1}),
    "nand_and": (nand_and, {(0, 0): 0, (0, 1): 0, (1, 0): 0, (1, 1): 1}),
    "nand_or": (nand_or, {(0, 0): 0, (0, 1): 1, (1, 0): 1, (1, 1): 1}),
    "nand_xor": (nand_xor, {(0, 0): 0, (0, 1): 1, (1, 0): 1, (1, 1): 0}),
}
TABLES_1: dict[str, tuple[Gate, dict[int, int]]] = {
    "NOT": (NOT, {0: 1, 1: 0}),
    "nand_not": (nand_not, {0: 1, 1: 0}),
}
ARITY: dict[str, tuple[Gate, int]] = {
    **{n: (g, 2) for n, (g, _) in TABLES_2.items()},
    **{n: (g, 1) for n, (g, _) in TABLES_1.items()},
}
ALL_NAMES = list(ARITY)


class MyInt(int):
    """An int subclass; must be rejected (exact-type rule)."""


class LyingInt(int):
    """Subclass whose __eq__ claims to be 0/1 while really being 7."""

    def __eq__(self, other: object) -> bool:
        return other in (0, 1)

    __hash__ = int.__hash__


@pytest.mark.parametrize("name", list(TABLES_2))
def test_literal_truth_table_2(name: str) -> None:
    gate, table = TABLES_2[name]
    assert len(table) == 4
    for (a, b), want in table.items():
        assert gate(a, b) == want, (name, a, b)


@pytest.mark.parametrize("name", list(TABLES_1))
def test_literal_truth_table_1(name: str) -> None:
    gate, table = TABLES_1[name]
    for a, want in table.items():
        assert gate(a) == want, (name, a)


@pytest.mark.parametrize(
    ("built", "direct"),
    [(nand_and, AND), (nand_or, OR), (nand_xor, XOR)],
)
def test_nand_built_equals_direct_2(built: Gate, direct: Gate) -> None:
    for a, b in itertools.product((0, 1), repeat=2):
        assert built(a, b) == direct(a, b)


def test_nand_not_equals_not() -> None:
    for a in (0, 1):
        assert nand_not(a) == NOT(a)


def test_valid_inputs_never_raise_and_return_exact_int() -> None:
    for name in ALL_NAMES:
        gate, arity = ARITY[name]
        for args in itertools.product((0, 1), repeat=arity):
            r = gate(*args)
            assert type(r) is int, (name, args, type(r))
            assert r in (0, 1)


# ---------------------------------------------------------------- rejection

BAD_INTS = [i for i in range(-1024, 1025) if i not in (0, 1)] + [
    2**31,
    -(2**31),
    2**63,
    -(2**63),
    2**64,
    -(2**64),
    10**100,
    -(10**100),
]

BAD_TYPES: list[object] = [
    True,
    False,
    None,
    1.0,
    0.0,
    -0.0,
    float("nan"),
    float("inf"),
    "1",
    "0",
    "1abc",
    "",
    b"1",
    [1],
    (1,),
    {1},
    {},
    object(),
    1 + 0j,
    Decimal(1),
    Fraction(1),
    MyInt(1),
    MyInt(0),
    LyingInt(7),
]


def _call_with(name: str, pos: int, bad: object, other: object) -> int:
    gate, arity = ARITY[name]
    args = [other] * arity
    args[pos] = bad
    return gate(*args)


@pytest.mark.parametrize("name", ALL_NAMES)
def test_every_bad_int_raises_value_error(name: str) -> None:
    arity = ARITY[name][1]
    for pos in range(arity):
        for other in (0, 1, 2, -7):
            for bad in BAD_INTS:
                # Both ints: the first invalid position reports; all are
                # ValueError regardless of which one wins.
                with pytest.raises(ValueError, match="must be 0 or 1"):
                    _call_with(name, pos, bad, other)


@pytest.mark.parametrize("name", ALL_NAMES)
def test_type_confusion_matrix_raises_type_error(name: str) -> None:
    arity = ARITY[name][1]
    for pos in range(arity):
        for bad in BAD_TYPES:
            for other in (0, 1, None):
                with pytest.raises(TypeError, match="must be an int"):
                    _call_with(name, pos, bad, other)
            # other argument is an out-of-range int: whichever argument is
            # checked first decides, but it must be one of the two errors.
            with pytest.raises((TypeError, ValueError)):
                _call_with(name, pos, bad, 2)


@pytest.mark.parametrize("name", [n for n in ALL_NAMES if ARITY[n][1] == 2])
def test_first_invalid_argument_is_reported(name: str) -> None:
    gate, _ = ARITY[name]
    with pytest.raises(ValueError, match="a must be 0 or 1"):
        gate(5, "x")
    with pytest.raises(TypeError, match="a must be an int"):
        gate("x", 5)


def test_xor_n_literal_cases_for_full_gates_coverage() -> None:
    from logic_gates.gates import XOR_N

    assert XOR_N() == 0
    assert XOR_N(1) == 1
    assert XOR_N(0, 0, 0, 0, 0, 0, 1, 1) == 0
    assert XOR_N(0, 0, 0, 0, 0, 0, 0, 1) == 1
    with pytest.raises(TypeError):
        XOR_N(1, True)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        XOR_N(1, 2)


def test_error_messages_name_argument_and_type() -> None:
    with pytest.raises(TypeError, match="b must be an int, got NoneType"):
        AND(1, None)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="a must be an int, got MyInt"):
        OR(MyInt(1), 0)
    with pytest.raises(ValueError, match="b must be 0 or 1, got 2"):
        NOR(0, 2)
