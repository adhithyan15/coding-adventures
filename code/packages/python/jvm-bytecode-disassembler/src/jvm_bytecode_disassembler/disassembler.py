"""Version-aware JVM bytecode disassembler primitives."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import IntEnum


@dataclass(frozen=True, order=True)
class JVMVersion:
    major: int
    minor: int = 0

    @property
    def label(self) -> str:
        return {
            52: "Java 8",
            55: "Java 11",
            61: "Java 17",
            65: "Java 21",
        }.get(self.major, f"class-file {self.major}.{self.minor}")


class JVMOpcode(IntEnum):
    ICONST_0 = 0x03
    ICONST_1 = 0x04
    ICONST_2 = 0x05
    ICONST_3 = 0x06
    ICONST_4 = 0x07
    ICONST_5 = 0x08
    BIPUSH = 0x10
    SIPUSH = 0x11
    LDC = 0x12
    ILOAD = 0x15
    ILOAD_0 = 0x1A
    ILOAD_1 = 0x1B
    ILOAD_2 = 0x1C
    ILOAD_3 = 0x1D
    ISTORE = 0x36
    ISTORE_0 = 0x3B
    ISTORE_1 = 0x3C
    ISTORE_2 = 0x3D
    ISTORE_3 = 0x3E
    IADD = 0x60
    ISUB = 0x64
    IMUL = 0x68
    IDIV = 0x6C
    IF_ICMPEQ = 0x9F
    IF_ICMPGT = 0xA3
    GOTO = 0xA7
    IRETURN = 0xAC
    RETURN = 0xB1
    GETSTATIC = 0xB2
    INVOKEVIRTUAL = 0xB6


@dataclass(frozen=True)
class JVMInstruction:
    offset: int
    opcode: JVMOpcode
    mnemonic: str
    size: int
    operands: tuple[int, ...] = ()
    literal: int | None = None
    constant_pool_index: int | None = None
    local_slot: int | None = None
    branch_target: int | None = None


@dataclass(frozen=True)
class JVMMethodBody:
    version: JVMVersion
    max_stack: int
    max_locals: int
    instructions: tuple[JVMInstruction, ...]
    constant_pool: tuple[tuple[int, int | str], ...] = ()

    def instruction_at(self, offset: int) -> JVMInstruction:
        for instruction in self.instructions:
            if instruction.offset == offset:
                return instruction
        msg = f"No instruction starts at bytecode offset {offset}"
        raise KeyError(msg)

    @property
    def constant_pool_lookup(self) -> dict[int, int | str]:
        return dict(self.constant_pool)


def disassemble_method_body(
    bytecode: bytes,
    *,
    version: JVMVersion | None = None,
    max_stack: int = 0,
    max_locals: int = 16,
    constant_pool: Mapping[int, object] | Sequence[object] | None = None,
) -> JVMMethodBody:
    resolved_version = version or JVMVersion(52, 0)
    if resolved_version.major < 45:
        msg = (
            "Unsupported JVM class-file version "
            f"{resolved_version.major}.{resolved_version.minor}"
        )
        raise ValueError(msg)

    constant_lookup = _normalize_constant_pool(constant_pool)
    instructions: list[JVMInstruction] = []
    pc = 0

    while pc < len(bytecode):
        raw_opcode = bytecode[pc]
        try:
            opcode = JVMOpcode(raw_opcode)
        except ValueError as exc:
            msg = f"Unknown JVM opcode: 0x{raw_opcode:02X} at PC={pc}"
            raise ValueError(msg) from exc

        if JVMOpcode.ICONST_0 <= opcode <= JVMOpcode.ICONST_5:
            literal = opcode - JVMOpcode.ICONST_0
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=1,
                    literal=int(literal),
                )
            )
            pc += 1
            continue

        if opcode == JVMOpcode.BIPUSH:
            raw = bytecode[pc + 1]
            literal = raw if raw < 128 else raw - 256
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="bipush",
                    size=2,
                    operands=(raw,),
                    literal=literal,
                )
            )
            pc += 2
            continue

        if opcode == JVMOpcode.SIPUSH:
            literal = int.from_bytes(bytecode[pc + 1 : pc + 3], "big", signed=True)
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="sipush",
                    size=3,
                    operands=tuple(bytecode[pc + 1 : pc + 3]),
                    literal=literal,
                )
            )
            pc += 3
            continue

        if opcode == JVMOpcode.LDC:
            index = bytecode[pc + 1]
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="ldc",
                    size=2,
                    operands=(index,),
                    constant_pool_index=index,
                )
            )
            pc += 2
            continue

        if JVMOpcode.ILOAD_0 <= opcode <= JVMOpcode.ILOAD_3:
            slot = opcode - JVMOpcode.ILOAD_0
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=1,
                    local_slot=int(slot),
                )
            )
            pc += 1
            continue

        if opcode == JVMOpcode.ILOAD:
            slot = bytecode[pc + 1]
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="iload",
                    size=2,
                    operands=(slot,),
                    local_slot=slot,
                )
            )
            pc += 2
            continue

        if JVMOpcode.ISTORE_0 <= opcode <= JVMOpcode.ISTORE_3:
            slot = opcode - JVMOpcode.ISTORE_0
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=1,
                    local_slot=int(slot),
                )
            )
            pc += 1
            continue

        if opcode == JVMOpcode.ISTORE:
            slot = bytecode[pc + 1]
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="istore",
                    size=2,
                    operands=(slot,),
                    local_slot=slot,
                )
            )
            pc += 2
            continue

        if opcode in {
            JVMOpcode.IADD,
            JVMOpcode.ISUB,
            JVMOpcode.IMUL,
            JVMOpcode.IDIV,
            JVMOpcode.IRETURN,
            JVMOpcode.RETURN,
        }:
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=1,
                )
            )
            pc += 1
            continue

        if opcode in {JVMOpcode.GETSTATIC, JVMOpcode.INVOKEVIRTUAL}:
            index = int.from_bytes(bytecode[pc + 1 : pc + 3], "big", signed=False)
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=3,
                    operands=tuple(bytecode[pc + 1 : pc + 3]),
                    constant_pool_index=index,
                )
            )
            pc += 3
            continue

        if opcode in {JVMOpcode.IF_ICMPEQ, JVMOpcode.IF_ICMPGT, JVMOpcode.GOTO}:
            offset = int.from_bytes(bytecode[pc + 1 : pc + 3], "big", signed=True)
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=3,
                    operands=tuple(bytecode[pc + 1 : pc + 3]),
                    branch_target=pc + offset,
                )
            )
            pc += 3
            continue

    return JVMMethodBody(
        version=resolved_version,
        max_stack=max_stack,
        max_locals=max_locals,
        instructions=tuple(instructions),
        constant_pool=tuple(sorted(constant_lookup.items())),
    )


def encode_iconst(n: int) -> bytes:
    if 0 <= n <= 5:
        return bytes([JVMOpcode.ICONST_0 + n])
    if -128 <= n <= 127:
        raw = n if n >= 0 else n + 256
        return bytes([JVMOpcode.BIPUSH, raw])
    if -32768 <= n <= 32767:
        return bytes([JVMOpcode.SIPUSH]) + int(n).to_bytes(2, "big", signed=True)
    msg = (
        f"encode_iconst: value {n} is outside signed short range "
        "(-32768 to 32767). Use ldc."
    )
    raise ValueError(msg)


def encode_istore(slot: int) -> bytes:
    if 0 <= slot <= 3:
        return bytes([JVMOpcode.ISTORE_0 + slot])
    return bytes([JVMOpcode.ISTORE, slot])


def encode_iload(slot: int) -> bytes:
    if 0 <= slot <= 3:
        return bytes([JVMOpcode.ILOAD_0 + slot])
    return bytes([JVMOpcode.ILOAD, slot])


def assemble_jvm(*instructions: tuple[JVMOpcode, ...]) -> bytes:
    result = bytearray()

    one_byte_opcodes = {
        JVMOpcode.ICONST_0,
        JVMOpcode.ICONST_1,
        JVMOpcode.ICONST_2,
        JVMOpcode.ICONST_3,
        JVMOpcode.ICONST_4,
        JVMOpcode.ICONST_5,
        JVMOpcode.ILOAD_0,
        JVMOpcode.ILOAD_1,
        JVMOpcode.ILOAD_2,
        JVMOpcode.ILOAD_3,
        JVMOpcode.ISTORE_0,
        JVMOpcode.ISTORE_1,
        JVMOpcode.ISTORE_2,
        JVMOpcode.ISTORE_3,
        JVMOpcode.IADD,
        JVMOpcode.ISUB,
        JVMOpcode.IMUL,
        JVMOpcode.IDIV,
        JVMOpcode.IRETURN,
        JVMOpcode.RETURN,
    }
    one_byte_operand_ops = {
        JVMOpcode.BIPUSH,
        JVMOpcode.LDC,
        JVMOpcode.ILOAD,
        JVMOpcode.ISTORE,
    }
    signed_short_ops = {
        JVMOpcode.SIPUSH,
        JVMOpcode.GOTO,
        JVMOpcode.IF_ICMPEQ,
        JVMOpcode.IF_ICMPGT,
    }
    unsigned_short_ops = {JVMOpcode.GETSTATIC, JVMOpcode.INVOKEVIRTUAL}

    for instr in instructions:
        op = instr[0]
        if op in one_byte_opcodes:
            result.append(op)
        elif op in one_byte_operand_ops:
            if len(instr) < 2:
                msg = f"Opcode {op.name} requires an operand"
                raise ValueError(msg)
            operand = instr[1]
            result.append(op)
            result.append(operand & 0xFF)
        elif op in signed_short_ops:
            if len(instr) < 2:
                if op in {JVMOpcode.GOTO, JVMOpcode.IF_ICMPEQ, JVMOpcode.IF_ICMPGT}:
                    msg = f"Opcode {op.name} requires an offset operand"
                else:
                    msg = f"Opcode {op.name} requires an operand"
                raise ValueError(msg)
            operand = int(instr[1])
            result.append(op)
            result.extend(operand.to_bytes(2, "big", signed=True))
        elif op in unsigned_short_ops:
            if len(instr) < 2:
                msg = f"Opcode {op.name} requires an operand"
                raise ValueError(msg)
            operand = int(instr[1])
            result.append(op)
            result.extend(operand.to_bytes(2, "big", signed=False))
        else:
            msg = f"Unknown opcode in assemble_jvm: {op}"
            raise ValueError(msg)

    return bytes(result)


def _normalize_constant_pool(
    constant_pool: Mapping[int, object] | Sequence[object] | None,
) -> dict[int, object]:
    if constant_pool is None:
        return {}
    if isinstance(constant_pool, Mapping):
        return dict(constant_pool)
    return {index: value for index, value in enumerate(constant_pool)}
