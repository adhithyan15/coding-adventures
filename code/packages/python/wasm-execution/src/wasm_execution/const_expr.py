"""const_expr.py --- Evaluate WASM constant expressions.

Constant expressions appear in global initializers, data segment offsets,
and element segment offsets. They are tiny programs restricted to a handful
of opcodes: i32.const, i64.const, f32.const, f64.const, global.get, end.

Example: ``(i32.const 42)`` is encoded as [0x41, 0x2A, 0x0B].
"""

from __future__ import annotations

import struct

from wasm_leb128 import decode_signed, decode_unsigned

from wasm_execution.host_interface import TrapError
from wasm_execution.values import WasmValue, f32, f64, i32, i64

# ===========================================================================
# Opcode Constants
# ===========================================================================

_I32_CONST = 0x41
_I64_CONST = 0x42
_F32_CONST = 0x43
_F64_CONST = 0x44
_GLOBAL_GET = 0x23
_END = 0x0B


# ===========================================================================
# 64-bit Signed LEB128 Decoder
# ===========================================================================


def _decode_signed_64(data: bytes | bytearray, offset: int) -> tuple[int, int]:
    """Decode a signed LEB128-encoded 64-bit integer.

    Returns (value, bytes_consumed). Uses Python's arbitrary precision ints
    and then masks to 64-bit signed range.
    """
    result = 0
    shift = 0
    bytes_consumed = 0
    max_bytes = 10  # ceil(64 / 7) = 10

    for i in range(offset, len(data)):
        if bytes_consumed >= max_bytes:
            msg = f"LEB128 sequence exceeds maximum {max_bytes} bytes for a 64-bit value"
            raise TrapError(msg)

        byte = data[i]
        payload = byte & 0x7F
        result |= payload << shift
        shift += 7
        bytes_consumed += 1

        if (byte & 0x80) == 0:
            # Sign extension
            if shift < 64 and (byte & 0x40) != 0:
                result |= -(1 << shift)
            # Mask to signed 64-bit
            result = (result + (1 << 63)) % (1 << 64) - (1 << 63)
            return (result, bytes_consumed)

    msg = f"LEB128 sequence is unterminated at offset {offset + bytes_consumed}"
    raise TrapError(msg)


# ===========================================================================
# Constant Expression Evaluator
# ===========================================================================


def evaluate_const_expr(
    expr: bytes | bytearray,
    globals_list: list[WasmValue],
) -> WasmValue:
    """Evaluate a WASM constant expression and return its result.

    Args:
        expr: The raw bytes of the constant expression.
        globals_list: Global variable values available for ``global.get``.

    Returns:
        The single WasmValue produced by the expression.

    Raises:
        TrapError: If the expression is malformed or uses an illegal opcode.
    """
    result: WasmValue | None = None
    pos = 0

    while pos < len(expr):
        opcode = expr[pos]
        pos += 1

        if opcode == _I32_CONST:
            value, bytes_read = decode_signed(expr, pos)
            pos += bytes_read
            result = i32(value)

        elif opcode == _I64_CONST:
            value_64, bytes_read = _decode_signed_64(expr, pos)
            pos += bytes_read
            result = i64(value_64)

        elif opcode == _F32_CONST:
            if pos + 4 > len(expr):
                msg = f"f32.const at offset {pos - 1}: not enough bytes"
                raise TrapError(msg)
            f32_val = struct.unpack_from("<f", expr, pos)[0]
            pos += 4
            result = f32(f32_val)

        elif opcode == _F64_CONST:
            if pos + 8 > len(expr):
                msg = f"f64.const at offset {pos - 1}: not enough bytes"
                raise TrapError(msg)
            f64_val = struct.unpack_from("<d", expr, pos)[0]
            pos += 8
            result = f64(f64_val)

        elif opcode == _GLOBAL_GET:
            global_index, bytes_read = decode_unsigned(expr, pos)
            pos += bytes_read
            if global_index >= len(globals_list):
                msg = (
                    f"global.get: index {global_index} out of bounds "
                    f"({len(globals_list)} globals available)"
                )
                raise TrapError(msg)
            result = globals_list[global_index]

        elif opcode == _END:
            if result is None:
                msg = "Constant expression produced no value (empty expression)"
                raise TrapError(msg)
            return result

        else:
            msg = f"Illegal opcode 0x{opcode:02x} in constant expression at offset {pos - 1}"
            raise TrapError(msg)

    msg = "Constant expression missing end opcode (0x0B)"
    raise TrapError(msg)
