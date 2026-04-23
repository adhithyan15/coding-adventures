from __future__ import annotations

import struct
from dataclasses import dataclass

import pytest
from clr_pe_file import decode_clr_pe_file
from clr_pe_file.testing import hello_world_dll_bytes

from clr_bytecode_disassembler import disassemble_clr_method


def test_disassemble_hello_world_entry_point() -> None:
    assembly = decode_clr_pe_file(hello_world_dll_bytes())
    body = disassemble_clr_method(assembly, assembly.get_entry_point_method())

    assert body.metadata_version == "v4.0.30319"
    assert body.name == "Main"
    assert body.declaring_type == "Program"
    assert [instruction.opcode for instruction in body.instructions] == [
        "ldstr",
        "call",
        "ret",
    ]
    assert body.instructions[0].operand == "Hello, world!"
    assert body.instructions[1].operand.declaring_type == "System.Console"
    assert body.instructions[1].operand.name == "WriteLine"


@dataclass(frozen=True)
class _FakeHeader:
    max_stack: int = 8


@dataclass(frozen=True)
class _FakeMethod:
    declaring_type: str
    name: str
    header: _FakeHeader
    local_count: int
    il_bytes: bytes


@dataclass(frozen=True)
class _FakeTarget:
    declaring_type: str
    name: str


class _FakeAssembly:
    metadata_version = "v4.0.30319"

    def resolve_user_string(self, token: int) -> str:
        assert token == 0x70000001
        return "fixture-string"

    def resolve_member_reference(self, token: int) -> _FakeTarget:
        assert token == 0x0A000001
        return _FakeTarget("System.Console", "WriteLine")

    def resolve_method_definition(self, token: int) -> _FakeTarget:
        assert token == 0x06000002
        return _FakeTarget("Program", "Helper")


def test_disassemble_synthetic_instruction_mix() -> None:
    bytecode = bytes([0x00, 0x01, 0x02, 0x05, 0x0E, 0x02, 0x10, 0x03, 0x15])
    bytecode += bytes([0x16, 0x17, 0x1F, 0x7F, 0x20])
    bytecode += struct.pack("<i", 1000)
    bytecode += bytes([0x06, 0x09, 0x0A, 0x0D, 0x11, 0x07, 0x13, 0x08, 0x28])
    bytecode += struct.pack("<I", 0x0A000001)
    bytecode += bytes([0x2B, 0x02, 0x2C, 0x02, 0x2D, 0x02])
    bytecode += bytes([0x2E, 0x02, 0x2F, 0x02, 0x30, 0x02])
    bytecode += bytes([0x31, 0x02, 0x32, 0x02, 0x33, 0x02, 0x38])
    bytecode += struct.pack("<i", 0)
    bytecode += bytes([0x39]) + struct.pack("<i", 0)
    bytecode += bytes([0x3A]) + struct.pack("<i", 0)
    bytecode += bytes([0x3B]) + struct.pack("<i", 0)
    bytecode += bytes([0x3C]) + struct.pack("<i", 0)
    bytecode += bytes([0x3D]) + struct.pack("<i", 0)
    bytecode += bytes([0x3E]) + struct.pack("<i", 0)
    bytecode += bytes([0x3F]) + struct.pack("<i", 0)
    bytecode += bytes([0x40]) + struct.pack("<i", 0)
    bytecode += bytes([0x58, 0x59, 0x5A, 0x5B, 0x72])
    bytecode += struct.pack("<I", 0x70000001)
    bytecode += bytes([0x5F, 0x60, 0x62, 0x63, 0x6F])
    bytecode += struct.pack("<I", 0x0A000001)
    bytecode += bytes([0x7E]) + struct.pack("<I", 0x04000001)
    bytecode += bytes([0x80]) + struct.pack("<I", 0x04000001)
    bytecode += bytes([0x8D]) + struct.pack("<I", 0x01000001)
    bytecode += bytes([0x91, 0x94, 0x9C, 0x9E])
    bytecode += bytes([0xFE, 0x01, 0xFE, 0x02, 0xFE, 0x04, 0x28])
    bytecode += struct.pack("<I", 0x06000002)
    bytecode += bytes([0x2A])

    body = disassemble_clr_method(
        _FakeAssembly(),
        _FakeMethod("Program", "Synthetic", _FakeHeader(), 2, bytecode),
    )

    assert [instruction.opcode for instruction in body.instructions] == [
        "nop",
        "ldnull",
        "ldarg.0",
        "ldarg.3",
        "ldarg.s",
        "starg.s",
        "ldc.i4.m1",
        "ldc.i4.0",
        "ldc.i4.1",
        "ldc.i4.s",
        "ldc.i4",
        "ldloc.0",
        "ldloc.3",
        "stloc.0",
        "stloc.3",
        "ldloc.s",
        "stloc.s",
        "call",
        "br.s",
        "brfalse.s",
        "brtrue.s",
        "beq.s",
        "bge.s",
        "bgt.s",
        "ble.s",
        "blt.s",
        "bne.un.s",
        "br",
        "brfalse",
        "brtrue",
        "beq",
        "bge",
        "bgt",
        "ble",
        "blt",
        "bne.un",
        "add",
        "sub",
        "mul",
        "div",
        "ldstr",
        "and",
        "or",
        "shl",
        "shr",
        "callvirt",
        "ldsfld",
        "stsfld",
        "newarr",
        "ldelem.u1",
        "ldelem.i4",
        "stelem.i1",
        "stelem.i4",
        "ceq",
        "cgt",
        "clt",
        "call",
        "ret",
    ]
    assert body.instructions[17].operand.name == "WriteLine"
    assert body.instructions[40].operand == "fixture-string"
    assert body.instructions[45].operand.name == "WriteLine"
    assert body.instructions[-2].operand.name == "Helper"


def test_disassemble_unknown_opcode_raises() -> None:
    with pytest.raises(ValueError, match="Unknown CLR opcode"):
        disassemble_clr_method(
            _FakeAssembly(),
            _FakeMethod("Program", "Broken", _FakeHeader(), 0, bytes([0xFF])),
        )
