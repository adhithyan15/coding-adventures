"""Tests for the HIR type system."""

import pytest

from hdl_ir.types import (
    TyArray,
    TyBit,
    TyEnum,
    TyFile,
    TyInteger,
    TyLogic,
    TyReal,
    TyRecord,
    TyStdLogic,
    TyString,
    TyTime,
    TyVector,
    ty_from_dict,
    width,
)


# ---- Construction & validation ----


def test_logic_round_trip():
    t = TyLogic()
    assert ty_from_dict(t.to_dict()) == t


def test_bit_round_trip():
    t = TyBit()
    assert ty_from_dict(t.to_dict()) == t


def test_std_logic_round_trip():
    t = TyStdLogic()
    assert ty_from_dict(t.to_dict()) == t


def test_real_round_trip():
    t = TyReal()
    assert ty_from_dict(t.to_dict()) == t


def test_time_round_trip():
    t = TyTime()
    assert ty_from_dict(t.to_dict()) == t


def test_string_round_trip():
    t = TyString()
    assert ty_from_dict(t.to_dict()) == t


def test_vector_round_trip():
    t = TyVector(TyLogic(), 8)
    assert ty_from_dict(t.to_dict()) == t


def test_vector_msb_first_default():
    t = TyVector(TyLogic(), 4)
    assert t.msb_first is True


def test_vector_msb_first_explicit_false():
    t = TyVector(TyLogic(), 4, msb_first=False)
    d = t.to_dict()
    assert d["msb_first"] is False
    assert ty_from_dict(d) == t


def test_vector_zero_width_rejected():
    with pytest.raises(ValueError, match="vector width"):
        TyVector(TyLogic(), 0)


def test_integer_round_trip():
    t = TyInteger(low=0, high=255)
    assert ty_from_dict(t.to_dict()) == t


def test_integer_default_range():
    t = TyInteger()
    assert t.low == -(2**31)
    assert t.high == 2**31 - 1


def test_integer_invalid_range():
    with pytest.raises(ValueError, match="low.*high"):
        TyInteger(low=10, high=5)


def test_enum_round_trip():
    t = TyEnum("color", ("red", "green", "blue"))
    assert ty_from_dict(t.to_dict()) == t


def test_enum_empty_rejected():
    with pytest.raises(ValueError, match=">= 1 member"):
        TyEnum("empty", ())


def test_enum_duplicate_rejected():
    with pytest.raises(ValueError, match="duplicate"):
        TyEnum("dup", ("a", "a"))


def test_record_round_trip():
    t = TyRecord("pixel", (("r", TyVector(TyLogic(), 8)), ("g", TyVector(TyLogic(), 8))))
    assert ty_from_dict(t.to_dict()) == t


def test_record_duplicate_field_rejected():
    with pytest.raises(ValueError, match="duplicate field"):
        TyRecord("bad", (("x", TyLogic()), ("x", TyBit())))


def test_array_round_trip():
    t = TyArray(TyLogic(), 0, 7)
    assert ty_from_dict(t.to_dict()) == t


def test_array_invalid_range():
    with pytest.raises(ValueError, match="bounds"):
        TyArray(TyLogic(), 10, 5)


def test_file_round_trip():
    t = TyFile(TyString())
    assert ty_from_dict(t.to_dict()) == t


# ---- width() ----


def test_width_logic():
    assert width(TyLogic()) == 1


def test_width_bit():
    assert width(TyBit()) == 1


def test_width_std_logic():
    assert width(TyStdLogic()) == 1


def test_width_vector_of_logic():
    assert width(TyVector(TyLogic(), 8)) == 8


def test_width_nested_vector():
    # A 4 × 8 byte array
    inner = TyVector(TyLogic(), 8)
    outer = TyVector(inner, 4)
    assert width(outer) == 32


def test_width_integer_matches_bits():
    assert width(TyInteger(low=0, high=255)) == 8
    assert width(TyInteger(low=0, high=15)) == 4
    assert width(TyInteger(low=0, high=0)) == 1
    assert width(TyInteger(low=0, high=1)) == 1


def test_width_enum():
    assert width(TyEnum("c", ("a", "b", "c", "d"))) == 2  # 4 members → 2 bits
    assert width(TyEnum("s", ("a",))) == 1


def test_width_record_sums_fields():
    t = TyRecord(
        "pix",
        (
            ("r", TyVector(TyLogic(), 8)),
            ("g", TyVector(TyLogic(), 8)),
            ("b", TyVector(TyLogic(), 8)),
        ),
    )
    assert width(t) == 24


def test_width_array():
    assert width(TyArray(TyLogic(), 0, 9)) == 10
    assert width(TyArray(TyVector(TyLogic(), 8), 0, 3)) == 32


def test_width_unsynthesizable_real_raises():
    with pytest.raises(ValueError, match="not defined"):
        width(TyReal())


def test_width_unsynthesizable_time_raises():
    with pytest.raises(ValueError):
        width(TyTime())


def test_ty_from_dict_unknown():
    with pytest.raises(ValueError, match="unknown type kind"):
        ty_from_dict({"kind": "no_such_kind"})
