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
        if 0x02 <= opcode <= 0x05:
            slot = opcode - 0x02
            instructions.append(CLRInstruction(start, f"ldarg.{slot}", slot))
            continue
        if opcode == 0x15:
            instructions.append(CLRInstruction(start, "ldc.i4.m1", -1))
            continue
        if 0x16 <= opcode <= 0x1E:
            value = opcode - 0x16
            instructions.append(CLRInstruction(start, f"ldc.i4.{value}", value))
            continue
        if opcode == 0x0E:
            slot = il[offset]
            offset += 1
            instructions.append(CLRInstruction(start, "ldarg.s", slot, 2))
            continue
        if opcode == 0x10:
            slot = il[offset]
            offset += 1
            instructions.append(CLRInstruction(start, "starg.s", slot, 2))
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
        if 0x2E <= opcode <= 0x33:
            delta = struct.unpack_from("b", il, offset)[0]
            offset += 1
            names = {
                0x2E: "beq.s",
                0x2F: "bge.s",
                0x30: "bgt.s",
                0x31: "ble.s",
                0x32: "blt.s",
                0x33: "bne.un.s",
            }
            instructions.append(CLRInstruction(start, names[opcode], offset + delta, 2))
            continue
        if opcode == 0x38:
            delta = struct.unpack_from("<i", il, offset)[0]
            offset += 4
            instructions.append(CLRInstruction(start, "br", offset + delta, 5))
            continue
        if 0x39 <= opcode <= 0x40:
            delta = struct.unpack_from("<i", il, offset)[0]
            offset += 4
            names = {
                0x39: "brfalse",
                0x3A: "brtrue",
                0x3B: "beq",
                0x3C: "bge",
                0x3D: "bgt",
                0x3E: "ble",
                0x3F: "blt",
                0x40: "bne.un",
            }
            instructions.append(CLRInstruction(start, names[opcode], offset + delta, 5))
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
        if opcode == 0x5F:
            instructions.append(CLRInstruction(start, "and"))
            continue
        if opcode == 0x60:
            instructions.append(CLRInstruction(start, "or"))
            continue
        if opcode == 0x62:
            instructions.append(CLRInstruction(start, "shl"))
            continue
        if opcode == 0x63:
            instructions.append(CLRInstruction(start, "shr"))
            continue
        if opcode == 0x6F:
            token = struct.unpack_from("<I", il, offset)[0]
            offset += 4
            operand = assembly.resolve_member_reference(token)
            instructions.append(CLRInstruction(start, "callvirt", operand, 5))
            continue
        if opcode == 0x72:
            token = struct.unpack_from("<I", il, offset)[0]
            offset += 4
            instructions.append(
                CLRInstruction(start, "ldstr", assembly.resolve_user_string(token), 5)
            )
            continue
        if opcode in {0x7E, 0x80, 0x8D}:
            token = struct.unpack_from("<I", il, offset)[0]
            offset += 4
            names = {0x7E: "ldsfld", 0x80: "stsfld", 0x8D: "newarr"}
            instructions.append(CLRInstruction(start, names[opcode], token, 5))
            continue
        if opcode in {0x91, 0x94, 0x9C, 0x9E}:
            names = {
                0x91: "ldelem.u1",
                0x94: "ldelem.i4",
                0x9C: "stelem.i1",
                0x9E: "stelem.i4",
            }
            instructions.append(CLRInstruction(start, names[opcode]))
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
