"""decoder.py --- WASM bytecode decoder: variable-length to fixed-format.

The decoder converts variable-length WASM bytecodes into an array of
fixed-format Instruction objects that GenericVM can execute. It also builds
the control flow map that maps block/loop/if starts to their matching ends.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import Any

from virtual_machine.vm import Instruction
from wasm_leb128 import decode_signed, decode_unsigned
from wasm_opcodes import get_opcode

from wasm_execution.types import ControlTarget


# ===========================================================================
# Decoded Instruction
# ===========================================================================


@dataclass
class DecodedInstruction:
    """A decoded WASM instruction with its byte offset and size."""

    opcode: int
    operand: Any
    offset: int
    size: int


# ===========================================================================
# 64-bit Signed LEB128 Decoder
# ===========================================================================


def _decode_signed_64(data: bytes | bytearray, offset: int) -> tuple[int, int]:
    """Decode a signed LEB128-encoded 64-bit integer."""
    result = 0
    shift = 0
    bytes_consumed = 0

    while True:
        if offset + bytes_consumed >= len(data):
            msg = "unterminated LEB128 sequence"
            raise ValueError(msg)
        byte = data[offset + bytes_consumed]
        bytes_consumed += 1
        result |= (byte & 0x7F) << shift
        shift += 7

        if (byte & 0x80) == 0:
            if shift < 64 and (byte & 0x40) != 0:
                result |= -(1 << shift)
            result = (result + (1 << 63)) % (1 << 64) - (1 << 63)
            return (result, bytes_consumed)

        if bytes_consumed >= 10:
            msg = "LEB128 sequence too long for i64"
            raise ValueError(msg)


# ===========================================================================
# Immediate Decoding
# ===========================================================================


def _decode_single_immediate(
    code: bytes | bytearray, offset: int, imm_type: str
) -> tuple[Any, int]:
    """Decode a single immediate and return (value, bytes_consumed)."""
    if imm_type == "i32":
        value, consumed = decode_signed(code, offset)
        return (value, consumed)

    if imm_type in ("labelidx", "funcidx", "typeidx", "localidx", "globalidx", "tableidx", "memidx"):
        value, consumed = decode_unsigned(code, offset)
        return (value, consumed)

    if imm_type == "i64":
        value, consumed = _decode_signed_64(code, offset)
        return (value, consumed)

    if imm_type == "f32":
        f32_val = struct.unpack_from("<f", code, offset)[0]
        return (f32_val, 4)

    if imm_type == "f64":
        f64_val = struct.unpack_from("<d", code, offset)[0]
        return (f64_val, 8)

    if imm_type == "blocktype":
        byte = code[offset]
        if byte == 0x40:
            return (0x40, 1)
        if byte in (0x7F, 0x7E, 0x7D, 0x7C):
            return (byte, 1)
        # Type index (signed LEB128) for multi-value blocks
        value, consumed = decode_signed(code, offset)
        return (value, consumed)

    if imm_type == "memarg":
        align, align_size = decode_unsigned(code, offset)
        mem_offset, offset_size = decode_unsigned(code, offset + align_size)
        return ({"align": align, "offset": mem_offset}, align_size + offset_size)

    if imm_type == "vec_labelidx":
        count, count_size = decode_unsigned(code, offset)
        pos = offset + count_size
        labels: list[int] = []
        for _ in range(count):
            label, label_size = decode_unsigned(code, pos)
            labels.append(label)
            pos += label_size
        default_label, default_size = decode_unsigned(code, pos)
        pos += default_size
        return ({"labels": labels, "default_label": default_label}, pos - offset)

    return (None, 0)


def _decode_immediates(
    code: bytes | bytearray,
    offset: int,
    immediates: tuple[str, ...],
) -> Any:
    """Decode immediate operands for an instruction."""
    if len(immediates) == 0:
        return None

    if len(immediates) == 1:
        value, _ = _decode_single_immediate(code, offset, immediates[0])
        return value

    # Multiple immediates: return as a dict
    result: dict[str, Any] = {}
    pos = offset
    for imm in immediates:
        value, size = _decode_single_immediate(code, pos, imm)
        result[imm] = value
        pos += size
    return result


def _immediates_byte_size(
    code: bytes | bytearray,
    offset: int,
    immediates: tuple[str, ...],
) -> int:
    """Calculate total byte size of immediate operands."""
    total = 0
    pos = offset
    for imm in immediates:
        _, size = _decode_single_immediate(code, pos, imm)
        total += size
        pos += size
    return total


# ===========================================================================
# Function Body Decoding
# ===========================================================================


def decode_function_body(body_code: bytes | bytearray) -> list[DecodedInstruction]:
    """Decode all instructions in a function body's bytecodes.

    Converts variable-length WASM bytecodes into a list of DecodedInstruction
    objects with decoded operands.
    """
    instructions: list[DecodedInstruction] = []
    offset = 0

    while offset < len(body_code):
        start_offset = offset
        opcode_byte = body_code[offset]
        offset += 1

        info = get_opcode(opcode_byte)
        operand: Any = None

        if info is not None:
            operand = _decode_immediates(body_code, offset, info.immediates)
            offset += _immediates_byte_size(body_code, offset, info.immediates)

        instructions.append(
            DecodedInstruction(
                opcode=opcode_byte,
                operand=operand,
                offset=start_offset,
                size=offset - start_offset,
            )
        )

    return instructions


# ===========================================================================
# Control Flow Map Construction
# ===========================================================================


def build_control_flow_map(
    instructions: list[DecodedInstruction],
) -> dict[int, ControlTarget]:
    """Build the control flow map for a function body.

    Scans through decoded instructions and maps each block/loop/if
    instruction index to its matching end (and else for if).
    """
    cf_map: dict[int, ControlTarget] = {}
    stack: list[dict[str, Any]] = []

    for i, instr in enumerate(instructions):
        if instr.opcode in (0x02, 0x03, 0x04):  # block, loop, if
            stack.append({"index": i, "opcode": instr.opcode, "else_pc": None})
        elif instr.opcode == 0x05:  # else
            if stack:
                stack[-1]["else_pc"] = i
        elif instr.opcode == 0x0B:  # end
            if stack:
                opener = stack.pop()
                cf_map[opener["index"]] = ControlTarget(
                    end_pc=i,
                    else_pc=opener["else_pc"],
                )

    return cf_map


# ===========================================================================
# Conversion to GenericVM Instructions
# ===========================================================================


def to_vm_instructions(decoded: list[DecodedInstruction]) -> list[Instruction]:
    """Convert decoded instructions to GenericVM's Instruction format."""
    return [
        Instruction(opcode=d.opcode, operand=d.operand)
        for d in decoded
    ]
