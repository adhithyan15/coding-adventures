"""test_values.py --- Tests for WASM typed value constructors and extractors.

Covers: i32, i64, f32, f64 constructors with wrapping semantics,
default_value for all four types, and type extractors (as_i32, as_i64,
as_f32, as_f64) including type mismatch traps.
"""

from __future__ import annotations

import math

import pytest
from wasm_types import ValueType

from wasm_execution.host_interface import TrapError
from wasm_execution.values import (
    WasmValue,
    as_f32,
    as_f64,
    as_i32,
    as_i64,
    default_value,
    f32,
    f64,
    i32,
    i64,
)


# ===========================================================================
# i32 constructor
# ===========================================================================


class TestI32Constructor:
    """i32(value) must wrap to signed 32-bit range [-2^31, 2^31-1]."""

    def test_positive(self) -> None:
        v = i32(42)
        assert v.type == ValueType.I32
        assert v.value == 42

    def test_zero(self) -> None:
        assert i32(0).value == 0

    def test_negative(self) -> None:
        assert i32(-1).value == -1

    def test_max_i32(self) -> None:
        assert i32(2**31 - 1).value == 2147483647

    def test_min_i32(self) -> None:
        assert i32(-(2**31)).value == -2147483648

    def test_overflow_wraps(self) -> None:
        """0xFFFFFFFF should wrap to -1 in signed i32."""
        assert i32(0xFFFFFFFF).value == -1

    def test_large_overflow(self) -> None:
        """2^32 should wrap to 0."""
        assert i32(2**32).value == 0

    def test_large_positive_wraps(self) -> None:
        assert i32(2**31).value == -2147483648


# ===========================================================================
# i64 constructor
# ===========================================================================


class TestI64Constructor:
    """i64(value) must wrap to signed 64-bit range [-2^63, 2^63-1]."""

    def test_positive(self) -> None:
        v = i64(100)
        assert v.type == ValueType.I64
        assert v.value == 100

    def test_zero(self) -> None:
        assert i64(0).value == 0

    def test_negative(self) -> None:
        assert i64(-1).value == -1

    def test_max_i64(self) -> None:
        assert i64(2**63 - 1).value == 2**63 - 1

    def test_min_i64(self) -> None:
        assert i64(-(2**63)).value == -(2**63)

    def test_overflow_wraps(self) -> None:
        assert i64(2**64 - 1).value == -1

    def test_large_overflow(self) -> None:
        assert i64(2**64).value == 0


# ===========================================================================
# f32 constructor
# ===========================================================================


class TestF32Constructor:
    """f32(value) should round-trip through IEEE 754 single precision."""

    def test_positive(self) -> None:
        v = f32(3.14)
        assert v.type == ValueType.F32
        # f32 rounds to single precision
        assert abs(v.value - 3.14) < 0.001

    def test_zero(self) -> None:
        assert f32(0.0).value == 0.0

    def test_negative(self) -> None:
        assert f32(-1.0).value == -1.0

    def test_precision_loss(self) -> None:
        """A value that cannot be exactly represented in f32 gets rounded."""
        v = f32(1.0000001)
        assert v.value == pytest.approx(1.0, abs=1e-5)


# ===========================================================================
# f64 constructor
# ===========================================================================


class TestF64Constructor:
    """f64(value) wraps Python float (already IEEE 754 double)."""

    def test_positive(self) -> None:
        v = f64(3.14159265358979)
        assert v.type == ValueType.F64
        assert v.value == pytest.approx(3.14159265358979)

    def test_zero(self) -> None:
        assert f64(0.0).value == 0.0

    def test_negative(self) -> None:
        assert f64(-2.718).value == pytest.approx(-2.718)

    def test_integer_converted_to_float(self) -> None:
        v = f64(42)
        assert isinstance(v.value, float)
        assert v.value == 42.0


# ===========================================================================
# default_value
# ===========================================================================


class TestDefaultValue:
    """default_value(vtype) returns the zero value for each WASM type."""

    def test_i32_default(self) -> None:
        v = default_value(ValueType.I32)
        assert v.type == ValueType.I32
        assert v.value == 0

    def test_i64_default(self) -> None:
        v = default_value(ValueType.I64)
        assert v.type == ValueType.I64
        assert v.value == 0

    def test_f32_default(self) -> None:
        v = default_value(ValueType.F32)
        assert v.type == ValueType.F32
        assert v.value == 0.0

    def test_f64_default(self) -> None:
        v = default_value(ValueType.F64)
        assert v.type == ValueType.F64
        assert v.value == 0.0

    def test_unknown_type_traps(self) -> None:
        with pytest.raises(TrapError, match="Unknown value type"):
            default_value(0x00)


# ===========================================================================
# Type extractors
# ===========================================================================


class TestAsI32:
    def test_correct_type(self) -> None:
        assert as_i32(i32(42)) == 42

    def test_wrong_type_traps(self) -> None:
        with pytest.raises(TrapError, match="Type mismatch.*expected i32"):
            as_i32(i64(42))

    def test_wrong_type_f32(self) -> None:
        with pytest.raises(TrapError, match="expected i32.*got f32"):
            as_i32(f32(1.0))


class TestAsI64:
    def test_correct_type(self) -> None:
        assert as_i64(i64(100)) == 100

    def test_wrong_type_traps(self) -> None:
        with pytest.raises(TrapError, match="expected i64"):
            as_i64(i32(42))


class TestAsF32:
    def test_correct_type(self) -> None:
        assert as_f32(f32(1.5)) == pytest.approx(1.5)

    def test_wrong_type_traps(self) -> None:
        with pytest.raises(TrapError, match="expected f32"):
            as_f32(f64(1.5))


class TestAsF64:
    def test_correct_type(self) -> None:
        assert as_f64(f64(2.5)) == pytest.approx(2.5)

    def test_wrong_type_traps(self) -> None:
        with pytest.raises(TrapError, match="expected f64"):
            as_f64(i32(42))
