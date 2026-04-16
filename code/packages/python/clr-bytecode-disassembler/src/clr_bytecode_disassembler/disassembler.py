"""Disassemble CLR CIL method bodies into reusable instruction objects."""

from __future__ import annotations

import struct
from dataclasses import dataclass

from clr_pe_file import CLRMemberReference, CLRMethodDef, CLRPEFile


@dataclass(frozen=True)
class CLRInstruction:
    """One disassembled CLR instruction."""

    offset: int
    opcode: str
    operand: object | None = None
    size: int = 1


@dataclass(frozen=True)
class CLRMethodBody:
    """Disassembled CLR method body."""

    metadata_version: str
    declaring_type: str
    name: str
    max_stack: int
    local_count: int
    instructions: tuple[CLRInstruction, ...]
    il_bytes: bytes


def disassemble_clr_method(assembly: CLRPEFile, method: CLRMethodDef) -> CLRMethodBody:
    """Disassemble a decoded CLR method definition."""
    il = method.il_bytes
    offset = 0
    instructions: list[CLRInstruction] = []
    while offset < len(il):
        start = offset
        opcode = il[offset]
        offset += 1

        if opcode == 0x00:
            instructions.append(CLRInstruction(start, "nop"))
            continue
        if opcode == 0x01:
            instructions.append(CLRInstruction(start, "ldnull"))
            continue
        if 0x16 <= opcode <= 0x1E:
            value = opcode - 0x16
            instructions.append(CLRInstruction(start, f"ldc.i4.{value}", value))
            continue
        if opcode == 0x1F:
            value = struct.unpack_from("b", il, offset)[0]
            offset += 1
            instructions.append(CLRInstruction(start, "ldc.i4.s", value, 2))
            continue
        if opcode == 0x20:
            value = struct.unpack_from("<i", il, offset)[0]
            offset += 4
            instructions.append(CLRInstruction(start, "ldc.i4", value, 5))
            continue
        if 0x06 <= opcode <= 0x09:
            slot = opcode - 0x06
            instructions.append(CLRInstruction(start, f"ldloc.{slot}", slot))
            continue
        if 0x0A <= opcode <= 0x0D:
            slot = opcode - 0x0A
            instructions.append(CLRInstruction(start, f"stloc.{slot}", slot))
            continue
        if opcode == 0x11:
            slot = il[offset]
            offset += 1
            instructions.append(CLRInstruction(start, "ldloc.s", slot, 2))
            continue
        if opcode == 0x13:
            slot = il[offset]
            offset += 1
            instructions.append(CLRInstruction(start, "stloc.s", slot, 2))
            continue
        if opcode == 0x28:
            token = struct.unpack_from("<I", il, offset)[0]
            offset += 4
            operand: CLRMemberReference | CLRMethodDef
            if token & 0xFF000000 == 0x06000000:
                operand = assembly.resolve_method_definition(token)
            else:
                operand = assembly.resolve_member_reference(token)
            instructions.append(CLRInstruction(start, "call", operand, 5))
            continue
        if opcode == 0x2A:
            instructions.append(CLRInstruction(start, "ret"))
            continue
        if opcode == 0x2B:
            delta = struct.unpack_from("b", il, offset)[0]
            offset += 1
            instructions.append(CLRInstruction(start, "br.s", offset + delta, 2))
            continue
        if opcode == 0x2C:
            delta = struct.unpack_from("b", il, offset)[0]
            offset += 1
            instructions.append(CLRInstruction(start, "brfalse.s", offset + delta, 2))
            continue
        if opcode == 0x2D:
            delta = struct.unpack_from("b", il, offset)[0]
            offset += 1
            instructions.append(CLRInstruction(start, "brtrue.s", offset + delta, 2))
            continue
        if opcode == 0x38:
            delta = struct.unpack_from("<i", il, offset)[0]
            offset += 4
            instructions.append(CLRInstruction(start, "br", offset + delta, 5))
            continue
        if opcode == 0x58:
            instructions.append(CLRInstruction(start, "add"))
            continue
        if opcode == 0x59:
            instructions.append(CLRInstruction(start, "sub"))
            continue
        if opcode == 0x5A:
            instructions.append(CLRInstruction(start, "mul"))
            continue
        if opcode == 0x5B:
            instructions.append(CLRInstruction(start, "div"))
            continue
        if opcode == 0x72:
            token = struct.unpack_from("<I", il, offset)[0]
            offset += 4
            instructions.append(
                CLRInstruction(start, "ldstr", assembly.resolve_user_string(token), 5)
            )
            continue
        if opcode == 0xFE:
            extended = il[offset]
            offset += 1
            if extended == 0x01:
                instructions.append(CLRInstruction(start, "ceq", size=2))
                continue
            if extended == 0x02:
                instructions.append(CLRInstruction(start, "cgt", size=2))
                continue
            if extended == 0x04:
                instructions.append(CLRInstruction(start, "clt", size=2))
                continue
            msg = f"Unknown extended CLR opcode 0xFE {extended:#04x}"
            raise ValueError(msg)

        msg = f"Unknown CLR opcode {opcode:#04x} at offset {start}"
        raise ValueError(msg)

    return CLRMethodBody(
        metadata_version=assembly.metadata_version,
        declaring_type=method.declaring_type,
        name=method.name,
        max_stack=method.header.max_stack,
        local_count=method.local_count,
        instructions=tuple(instructions),
        il_bytes=il,
    )
