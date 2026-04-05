"""test_decoder.py --- Tests for WASM bytecode decoder.

Covers: decode_function_body, build_control_flow_map, to_vm_instructions,
and immediate decoding for various operand types.
"""

from __future__ import annotations

import struct

from wasm_execution.decoder import (
    DecodedInstruction,
    _decode_signed_64,
    build_control_flow_map,
    decode_function_body,
    to_vm_instructions,
)
from wasm_execution.types import ControlTarget


# ===========================================================================
# decode_function_body
# ===========================================================================


class TestDecodeFunctionBody:
    def test_empty_body(self) -> None:
        """An empty body produces an empty list."""
        result = decode_function_body(b"")
        assert result == []

    def test_nop(self) -> None:
        """0x01 is nop, no immediates."""
        result = decode_function_body(bytes([0x01]))
        assert len(result) == 1
        assert result[0].opcode == 0x01
        assert result[0].operand is None

    def test_end(self) -> None:
        """0x0B is end, no immediates."""
        result = decode_function_body(bytes([0x0B]))
        assert len(result) == 1
        assert result[0].opcode == 0x0B

    def test_i32_const(self) -> None:
        """0x41 followed by LEB128-encoded 42 (0x2A)."""
        result = decode_function_body(bytes([0x41, 0x2A]))
        assert len(result) == 1
        assert result[0].opcode == 0x41
        assert result[0].operand == 42

    def test_i32_const_negative(self) -> None:
        """i32.const -1: LEB128 0x7F."""
        result = decode_function_body(bytes([0x41, 0x7F]))
        assert result[0].operand == -1

    def test_i64_const(self) -> None:
        """0x42 followed by LEB128-encoded 100."""
        result = decode_function_body(bytes([0x42, 0xE4, 0x00]))
        assert result[0].opcode == 0x42
        assert result[0].operand == 100

    def test_f32_const(self) -> None:
        """0x43 followed by 4 bytes of IEEE 754 float."""
        f32_bytes = struct.pack("<f", 3.14)
        body = bytes([0x43]) + f32_bytes
        result = decode_function_body(body)
        assert result[0].opcode == 0x43
        assert abs(result[0].operand - 3.14) < 0.001

    def test_f64_const(self) -> None:
        """0x44 followed by 8 bytes of IEEE 754 double."""
        f64_bytes = struct.pack("<d", 2.718)
        body = bytes([0x44]) + f64_bytes
        result = decode_function_body(body)
        assert result[0].opcode == 0x44
        assert abs(result[0].operand - 2.718) < 0.001

    def test_local_get(self) -> None:
        """0x20 followed by unsigned LEB128 index."""
        result = decode_function_body(bytes([0x20, 0x03]))
        assert result[0].opcode == 0x20
        assert result[0].operand == 3

    def test_multiple_instructions(self) -> None:
        """Decode a sequence: i32.const 5, i32.const 3, i32.add, end."""
        body = bytes([
            0x41, 0x05,  # i32.const 5
            0x41, 0x03,  # i32.const 3
            0x6A,        # i32.add
            0x0B,        # end
        ])
        result = decode_function_body(body)
        assert len(result) == 4
        assert result[0].opcode == 0x41
        assert result[0].operand == 5
        assert result[1].opcode == 0x41
        assert result[1].operand == 3
        assert result[2].opcode == 0x6A
        assert result[3].opcode == 0x0B

    def test_block_with_blocktype(self) -> None:
        """0x02 (block) with blocktype 0x40 (void)."""
        body = bytes([0x02, 0x40, 0x0B])
        result = decode_function_body(body)
        assert result[0].opcode == 0x02
        assert result[0].operand == 0x40

    def test_block_with_i32_result(self) -> None:
        """block with i32 result type 0x7F."""
        body = bytes([0x02, 0x7F, 0x0B])
        result = decode_function_body(body)
        assert result[0].operand == 0x7F

    def test_instruction_offset_tracking(self) -> None:
        """Each instruction records its byte offset."""
        body = bytes([0x01, 0x01, 0x01])  # three nops
        result = decode_function_body(body)
        assert result[0].offset == 0
        assert result[1].offset == 1
        assert result[2].offset == 2


# ===========================================================================
# build_control_flow_map
# ===========================================================================


class TestBuildControlFlowMap:
    def test_empty(self) -> None:
        assert build_control_flow_map([]) == {}

    def test_block_end(self) -> None:
        """Block at index 0, end at index 1."""
        instrs = [
            DecodedInstruction(opcode=0x02, operand=0x40, offset=0, size=2),
            DecodedInstruction(opcode=0x0B, operand=None, offset=2, size=1),
        ]
        cf = build_control_flow_map(instrs)
        assert 0 in cf
        assert cf[0].end_pc == 1
        assert cf[0].else_pc is None

    def test_if_else_end(self) -> None:
        """if at 0, else at 1, end at 2."""
        instrs = [
            DecodedInstruction(opcode=0x04, operand=0x40, offset=0, size=2),
            DecodedInstruction(opcode=0x05, operand=None, offset=2, size=1),
            DecodedInstruction(opcode=0x0B, operand=None, offset=3, size=1),
        ]
        cf = build_control_flow_map(instrs)
        assert cf[0].end_pc == 2
        assert cf[0].else_pc == 1

    def test_nested_blocks(self) -> None:
        """Outer block (0), inner block (1), inner end (2), outer end (3)."""
        instrs = [
            DecodedInstruction(opcode=0x02, operand=0x40, offset=0, size=2),
            DecodedInstruction(opcode=0x02, operand=0x40, offset=2, size=2),
            DecodedInstruction(opcode=0x0B, operand=None, offset=4, size=1),
            DecodedInstruction(opcode=0x0B, operand=None, offset=5, size=1),
        ]
        cf = build_control_flow_map(instrs)
        assert cf[1].end_pc == 2  # inner block
        assert cf[0].end_pc == 3  # outer block

    def test_loop(self) -> None:
        """loop at 0, end at 1."""
        instrs = [
            DecodedInstruction(opcode=0x03, operand=0x40, offset=0, size=2),
            DecodedInstruction(opcode=0x0B, operand=None, offset=2, size=1),
        ]
        cf = build_control_flow_map(instrs)
        assert cf[0].end_pc == 1


# ===========================================================================
# to_vm_instructions
# ===========================================================================


class TestToVmInstructions:
    def test_converts_correctly(self) -> None:
        decoded = [
            DecodedInstruction(opcode=0x41, operand=42, offset=0, size=2),
            DecodedInstruction(opcode=0x0B, operand=None, offset=2, size=1),
        ]
        vm_instrs = to_vm_instructions(decoded)
        assert len(vm_instrs) == 2
        assert vm_instrs[0].opcode == 0x41
        assert vm_instrs[0].operand == 42
        assert vm_instrs[1].opcode == 0x0B
        assert vm_instrs[1].operand is None


# ===========================================================================
# _decode_signed_64
# ===========================================================================


class TestDecodeSigned64:
    def test_zero(self) -> None:
        val, consumed = _decode_signed_64(bytes([0x00]), 0)
        assert val == 0
        assert consumed == 1

    def test_positive(self) -> None:
        val, consumed = _decode_signed_64(bytes([0xE4, 0x00]), 0)
        assert val == 100

    def test_negative_one(self) -> None:
        val, consumed = _decode_signed_64(bytes([0x7F]), 0)
        assert val == -1

    def test_unterminated(self) -> None:
        import pytest
        with pytest.raises(ValueError, match="unterminated"):
            _decode_signed_64(bytes([0x80]), 0)
