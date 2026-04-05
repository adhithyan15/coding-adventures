"""values.py --- Typed WASM values and constructor/assertion helpers.

===========================================================================
WHAT ARE WASM VALUES?
===========================================================================

Every value in WebAssembly is *typed*. The four WASM 1.0 value types are:

  +------+------------------------------------------------------+
  | Type | Description                                          |
  +------+------------------------------------------------------+
  | i32  | 32-bit integer (stored as Python int)                |
  | i64  | 64-bit integer (stored as Python int)                |
  | f32  | 32-bit IEEE 754 float (stored as Python float)       |
  | f64  | 64-bit IEEE 754 float (stored as Python float)       |
  +------+------------------------------------------------------+

Python integers have arbitrary precision, so we MUST mask to the correct
bit width when constructing i32/i64 values.

===========================================================================
WRAPPING SEMANTICS
===========================================================================

i32 wrapping:
    val = (val + 0x80000000) % 0x100000000 - 0x80000000
    This maps any Python int to the signed 32-bit range [-2^31, 2^31 - 1].

i64 wrapping:
    val = (val + 2**63) % 2**64 - 2**63
    Same idea for 64-bit.

f32 precision:
    We round-trip through struct.pack('<f') / struct.unpack('<f') to get
    IEEE 754 single-precision rounding.
"""

from __future__ import annotations

import ctypes
import struct
from typing import Any

from virtual_machine.generic_vm import TypedVMValue
from wasm_types import ValueType

from wasm_execution.host_interface import TrapError

# ===========================================================================
# WasmValue Type Alias
# ===========================================================================
#
# A WasmValue is just a TypedVMValue from the GenericVM. The ``type`` field
# holds a ValueType constant (0x7F=i32, 0x7E=i64, 0x7D=f32, 0x7C=f64) and
# ``value`` holds the raw Python numeric value.
# ===========================================================================

WasmValue = TypedVMValue


# ===========================================================================
# Constructor Functions
# ===========================================================================


def i32(value: int) -> WasmValue:
    """Create an i32 (32-bit integer) WASM value.

    Uses ctypes.c_int32 to wrap the value to the signed 32-bit range.

    Examples:
        >>> i32(42).value
        42
        >>> i32(0xFFFFFFFF).value
        -1
        >>> i32(2**32).value
        0
    """
    wrapped = ctypes.c_int32(value).value
    return TypedVMValue(type=ValueType.I32, value=wrapped)


def i64(value: int) -> WasmValue:
    """Create an i64 (64-bit integer) WASM value.

    Uses ctypes.c_int64 to wrap the value to the signed 64-bit range.
    """
    wrapped = ctypes.c_int64(value).value
    return TypedVMValue(type=ValueType.I64, value=wrapped)


def f32(value: float) -> WasmValue:
    """Create an f32 (32-bit float) WASM value.

    Round-trips through IEEE 754 single-precision encoding to ensure
    the value has f32 precision.
    """
    rounded = struct.unpack("<f", struct.pack("<f", value))[0]
    return TypedVMValue(type=ValueType.F32, value=rounded)


def f64(value: float) -> WasmValue:
    """Create an f64 (64-bit float) WASM value.

    Python floats are already IEEE 754 double-precision, so no conversion needed.
    """
    return TypedVMValue(type=ValueType.F64, value=float(value))


# ===========================================================================
# Default Value
# ===========================================================================


def default_value(vtype: int) -> WasmValue:
    """Create a zero-initialized WasmValue for a given type code.

    WASM spec (section 4.2.1): the default value of a value type is
    the respective zero.
    """
    if vtype == ValueType.I32:
        return i32(0)
    if vtype == ValueType.I64:
        return i64(0)
    if vtype == ValueType.F32:
        return f32(0.0)
    if vtype == ValueType.F64:
        return f64(0.0)
    msg = f"Unknown value type: 0x{vtype:02x}"
    raise TrapError(msg)


# ===========================================================================
# Type Extraction Helpers
# ===========================================================================

_TYPE_NAMES: dict[int, str] = {
    ValueType.I32: "i32",
    ValueType.I64: "i64",
    ValueType.F32: "f32",
    ValueType.F64: "f64",
}


def as_i32(v: WasmValue) -> int:
    """Extract the raw int from an i32 WasmValue. Traps on type mismatch."""
    if v.type != ValueType.I32:
        msg = f"Type mismatch: expected i32, got {_TYPE_NAMES.get(v.type, f'0x{v.type:02x}')}"
        raise TrapError(msg)
    return v.value  # type: ignore[return-value]


def as_i64(v: WasmValue) -> int:
    """Extract the raw int from an i64 WasmValue. Traps on type mismatch."""
    if v.type != ValueType.I64:
        msg = f"Type mismatch: expected i64, got {_TYPE_NAMES.get(v.type, f'0x{v.type:02x}')}"
        raise TrapError(msg)
    return v.value  # type: ignore[return-value]


def as_f32(v: WasmValue) -> float:
    """Extract the raw float from an f32 WasmValue. Traps on type mismatch."""
    if v.type != ValueType.F32:
        msg = f"Type mismatch: expected f32, got {_TYPE_NAMES.get(v.type, f'0x{v.type:02x}')}"
        raise TrapError(msg)
    return v.value  # type: ignore[return-value]


def as_f64(v: WasmValue) -> float:
    """Extract the raw float from an f64 WasmValue. Traps on type mismatch."""
    if v.type != ValueType.F64:
        msg = f"Type mismatch: expected f64, got {_TYPE_NAMES.get(v.type, f'0x{v.type:02x}')}"
        raise TrapError(msg)
    return v.value  # type: ignore[return-value]
