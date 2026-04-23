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
    # 0x00 — no-op; useful as an explicit placeholder in generated code
    NOP = 0x00
    # 0x02 — push the integer constant -1 onto the operand stack
    ICONST_M1 = 0x02
    # 0x03–0x08 — push the small constants 0–5 (no operand needed)
    ICONST_0 = 0x03
    ICONST_1 = 0x04
    ICONST_2 = 0x05
    ICONST_3 = 0x06
    ICONST_4 = 0x07
    ICONST_5 = 0x08
    # 0x10 — push a signed byte value (1-byte operand, sign-extended to int)
    BIPUSH = 0x10
    # 0x11 — push a signed short value (2-byte operand, sign-extended to int)
    SIPUSH = 0x11
    # 0x12 — load constant from the constant pool (1-byte index)
    LDC = 0x12
    # 0x13 — load constant from the constant pool (wide 2-byte index)
    LDC_W = 0x13
    # 0x15 — load int from a local variable at the given slot
    ILOAD = 0x15
    # 0x1A–0x1D — load int from local slots 0–3 (no operand; compact form)
    ILOAD_0 = 0x1A
    ILOAD_1 = 0x1B
    ILOAD_2 = 0x1C
    ILOAD_3 = 0x1D
    # 0x2E — load int from an int[] array; stack: (arrayref, index) → value
    IALOAD = 0x2E
    # 0x33 — load byte from a byte[] array; sign-extends to int; stack: (arrayref, index) → value
    BALOAD = 0x33
    # 0x36 — store int to local variable at the given slot
    ISTORE = 0x36
    # 0x3B–0x3E — store int to local slots 0–3 (no operand; compact form)
    ISTORE_0 = 0x3B
    ISTORE_1 = 0x3C
    ISTORE_2 = 0x3D
    ISTORE_3 = 0x3E
    # 0x4F — store int to an int[] array; stack: (arrayref, index, value) → void
    IASTORE = 0x4F
    # 0x54 — store byte to a byte[] array; stack: (arrayref, index, value) → void
    BASTORE = 0x54
    # 0x57 — discard the top operand-stack value (any category-1 type)
    POP = 0x57
    # 0x60–0x6C — four basic integer arithmetic operations
    IADD = 0x60
    ISUB = 0x64
    IMUL = 0x68
    IDIV = 0x6C
    # 0x78 — left-shift: (value, shift) → (value << (shift & 0x1F))
    ISHL = 0x78
    # 0x7A — arithmetic right-shift: (value, shift) → (value >> (shift & 0x1F)), sign-extended
    ISHR = 0x7A
    # 0x7E — bitwise AND: (a, b) → (a & b)
    IAND = 0x7E
    # 0x80 — bitwise OR: (a, b) → (a | b)
    IOR = 0x80
    # 0x91 — truncate int to signed byte, then sign-extend back to int
    I2B = 0x91
    # 0x99 — branch if top of stack == 0; 2-byte signed offset from instruction start
    IFEQ = 0x99
    # 0x9A — branch if top of stack != 0; 2-byte signed offset from instruction start
    IFNE = 0x9A
    # 0x9F — branch if two ints are equal; stack: (a, b) → void
    IF_ICMPEQ = 0x9F
    # 0xA0 — branch if two ints are not equal; stack: (a, b) → void
    IF_ICMPNE = 0xA0
    # 0xA1 — branch if first int < second int; stack: (a, b) → void
    IF_ICMPLT = 0xA1
    # 0xA3 — branch if first int > second int; stack: (a, b) → void
    IF_ICMPGT = 0xA3
    # 0xA7 — unconditional branch; 2-byte signed offset from instruction start
    GOTO = 0xA7
    # 0xAC — return int from the current method
    IRETURN = 0xAC
    # 0xB1 — return void from the current method
    RETURN = 0xB1
    # 0xB2 — push value of a static field; 2-byte unsigned cp index
    GETSTATIC = 0xB2
    # 0xB3 — store value into a static field; 2-byte unsigned cp index
    PUTSTATIC = 0xB3
    # 0xB6 — invoke an instance method via virtual dispatch; 2-byte unsigned cp index
    INVOKEVIRTUAL = 0xB6
    # 0xB8 — invoke a static method; 2-byte unsigned cp index
    INVOKESTATIC = 0xB8
    # 0xBC — allocate a new primitive array; 1-byte type tag (8=byte, 10=int)
    NEWARRAY = 0xBC


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

        # iconst_m1 is the special case for -1 (sits just below the 0–5 range)
        if opcode == JVMOpcode.ICONST_M1:
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="iconst_m1",
                    size=1,
                    literal=-1,
                )
            )
            pc += 1
            continue

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

        # ldc_w is identical to ldc but uses a 2-byte unsigned constant-pool index,
        # allowing access to entries beyond index 255.
        if opcode == JVMOpcode.LDC_W:
            index = int.from_bytes(bytecode[pc + 1 : pc + 3], "big", signed=False)
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="ldc_w",
                    size=3,
                    operands=tuple(bytecode[pc + 1 : pc + 3]),
                    constant_pool_index=index,
                )
            )
            pc += 3
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
            # No-op — occupies one byte, does nothing
            JVMOpcode.NOP,
            # Integer arithmetic — each pops two ints and pushes the result
            JVMOpcode.IADD,
            JVMOpcode.ISUB,
            JVMOpcode.IMUL,
            JVMOpcode.IDIV,
            # Integer bitwise/shift — each pops two ints and pushes the result
            JVMOpcode.ISHL,
            JVMOpcode.ISHR,
            JVMOpcode.IAND,
            JVMOpcode.IOR,
            # Array access — operands come from the stack, not the instruction stream
            JVMOpcode.IALOAD,
            JVMOpcode.BALOAD,
            JVMOpcode.IASTORE,
            JVMOpcode.BASTORE,
            # Stack manipulation
            JVMOpcode.POP,
            # Type conversion
            JVMOpcode.I2B,
            # Method returns
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

        if opcode in {
            JVMOpcode.GETSTATIC,
            JVMOpcode.PUTSTATIC,
            JVMOpcode.INVOKEVIRTUAL,
            JVMOpcode.INVOKESTATIC,
        }:
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

        if opcode in {
            # Unary branch tests — compare top of stack against zero
            JVMOpcode.IFEQ,
            JVMOpcode.IFNE,
            # Binary branch tests — compare two ints from the stack
            JVMOpcode.IF_ICMPEQ,
            JVMOpcode.IF_ICMPNE,
            JVMOpcode.IF_ICMPLT,
            JVMOpcode.IF_ICMPGT,
            # Unconditional jump
            JVMOpcode.GOTO,
        }:
            # The 2-byte signed offset is relative to the start of this instruction.
            # branch_target stores the absolute bytecode offset so callers can resolve
            # the target without re-doing the arithmetic.
            branch_offset = int.from_bytes(bytecode[pc + 1 : pc + 3], "big", signed=True)
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic=opcode.name.lower(),
                    size=3,
                    operands=tuple(bytecode[pc + 1 : pc + 3]),
                    branch_target=pc + branch_offset,
                )
            )
            pc += 3
            continue

        # newarray allocates a new primitive array of the type indicated by the
        # single-byte operand: 4=boolean, 5=char, 6=float, 7=double, 8=byte,
        # 9=short, 10=int, 11=long.  The stack provides the desired array length.
        if opcode == JVMOpcode.NEWARRAY:
            atype = bytecode[pc + 1]
            instructions.append(
                JVMInstruction(
                    offset=pc,
                    opcode=opcode,
                    mnemonic="newarray",
                    size=2,
                    operands=(atype,),
                    literal=atype,
                )
            )
            pc += 2
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
        # Integer constant pushes (no operand needed)
        JVMOpcode.NOP,
        JVMOpcode.ICONST_M1,
        JVMOpcode.ICONST_0,
        JVMOpcode.ICONST_1,
        JVMOpcode.ICONST_2,
        JVMOpcode.ICONST_3,
        JVMOpcode.ICONST_4,
        JVMOpcode.ICONST_5,
        # Local variable access (compact forms)
        JVMOpcode.ILOAD_0,
        JVMOpcode.ILOAD_1,
        JVMOpcode.ILOAD_2,
        JVMOpcode.ILOAD_3,
        JVMOpcode.ISTORE_0,
        JVMOpcode.ISTORE_1,
        JVMOpcode.ISTORE_2,
        JVMOpcode.ISTORE_3,
        # Array access (operands from stack)
        JVMOpcode.IALOAD,
        JVMOpcode.BALOAD,
        JVMOpcode.IASTORE,
        JVMOpcode.BASTORE,
        # Stack manipulation
        JVMOpcode.POP,
        # Integer arithmetic and bitwise operations
        JVMOpcode.IADD,
        JVMOpcode.ISUB,
        JVMOpcode.IMUL,
        JVMOpcode.IDIV,
        JVMOpcode.ISHL,
        JVMOpcode.ISHR,
        JVMOpcode.IAND,
        JVMOpcode.IOR,
        # Type conversion
        JVMOpcode.I2B,
        # Returns
        JVMOpcode.IRETURN,
        JVMOpcode.RETURN,
    }
    one_byte_operand_ops = {
        JVMOpcode.BIPUSH,
        JVMOpcode.LDC,
        JVMOpcode.ILOAD,
        JVMOpcode.ISTORE,
        # newarray carries the primitive type tag as its single operand byte
        JVMOpcode.NEWARRAY,
    }
    signed_short_ops = {
        JVMOpcode.SIPUSH,
        # Branch instructions carry a signed 2-byte offset
        JVMOpcode.IFEQ,
        JVMOpcode.IFNE,
        JVMOpcode.GOTO,
        JVMOpcode.IF_ICMPEQ,
        JVMOpcode.IF_ICMPNE,
        JVMOpcode.IF_ICMPLT,
        JVMOpcode.IF_ICMPGT,
    }
    unsigned_short_ops = {
        # Field/method references use unsigned 2-byte constant-pool indices
        JVMOpcode.GETSTATIC,
        JVMOpcode.PUTSTATIC,
        JVMOpcode.INVOKEVIRTUAL,
        JVMOpcode.INVOKESTATIC,
        # ldc_w uses a 2-byte unsigned constant-pool index (wider than ldc)
        JVMOpcode.LDC_W,
    }

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
                branch_ops = {
                    JVMOpcode.GOTO,
                    JVMOpcode.IFEQ,
                    JVMOpcode.IFNE,
                    JVMOpcode.IF_ICMPEQ,
                    JVMOpcode.IF_ICMPNE,
                    JVMOpcode.IF_ICMPLT,
                    JVMOpcode.IF_ICMPGT,
                }
                if op in branch_ops:
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
