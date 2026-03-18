"""RISC-V RV32I Simulator — a clean, modern instruction set.

=== What is RISC-V? ===

RISC-V (pronounced "risk-five") is an open-source instruction set architecture
(ISA) designed at UC Berkeley by Patterson and Hennessy — the same people who
wrote the definitive computer architecture textbooks. It was designed from
scratch in 2010 with no historical baggage, making it the cleanest ISA to learn.

"RISC" stands for Reduced Instruction Set Computer — the philosophy that a
CPU should have a small number of simple instructions rather than many complex
ones. Each instruction does one thing well.

=== RISC-V vs other ISAs ===

    RISC-V:     Clean, regular encoding. 32 registers. No condition codes.
    ARM:        More complex encoding. 16 registers. Conditional execution.
    WASM:       Stack-based (no registers). Modern virtual machine.
    Intel 4004: 4-bit accumulator. Historical (1971).

=== Register conventions ===

RISC-V has 32 registers, each 32 bits wide:

    x0  = always 0 (hardwired — writes are ignored, reads always return 0)
    x1  = ra (return address — where to go back after a function call)
    x2  = sp (stack pointer — top of the stack)
    x3  = gp (global pointer)
    x4  = tp (thread pointer)
    x5-x7   = t0-t2 (temporary registers)
    x8-x9   = s0-s1 (saved registers)
    x10-x17 = a0-a7 (function arguments and return values)
    x18-x27 = s2-s11 (more saved registers)
    x28-x31 = t3-t6 (more temporaries)

The x0 register is special and brilliant: because it's always 0, many
operations become simpler. To load an immediate value, you just add it to x0:
    addi x1, x0, 42    →    x1 = 0 + 42 = 42

=== Instruction encoding ===

Every RISC-V instruction is exactly 32 bits. The opcode is always in bits [6:0].
Register fields are always in the same positions — this regularity makes the
decoder simpler than ARM's.

R-type (register-register):
    ┌─────────┬─────┬─────┬───────┬─────┬─────────┐
    │ funct7  │ rs2 │ rs1 │funct3 │ rd  │ opcode  │
    │ 31   25 │24 20│19 15│14   12│11  7│ 6     0 │
    └─────────┴─────┴─────┴───────┴─────┴─────────┘

I-type (immediate):
    ┌──────────────┬─────┬───────┬─────┬─────────┐
    │  imm[11:0]   │ rs1 │funct3 │ rd  │ opcode  │
    │ 31        20 │19 15│14   12│11  7│ 6     0 │
    └──────────────┴─────┴───────┴─────┴─────────┘

=== MVP instruction set (just enough for x = 1 + 2) ===

    addi x1, x0, 1    →  x1 = 0 + 1 = 1     (I-type, opcode=0010011)
    addi x2, x0, 2    →  x2 = 0 + 2 = 2     (I-type, opcode=0010011)
    add  x3, x1, x2   →  x3 = 1 + 2 = 3     (R-type, opcode=0110011)
    ecall              →  halt                 (I-type, opcode=1110011)
"""

from dataclasses import dataclass

from cpu_simulator.cpu import CPU
from cpu_simulator.memory import Memory
from cpu_simulator.pipeline import DecodeResult, ExecuteResult, PipelineTrace
from cpu_simulator.registers import RegisterFile


# ---------------------------------------------------------------------------
# Instruction encoding constants
# ---------------------------------------------------------------------------
# These are the bit patterns that identify each instruction type.
# The opcode is always in bits [6:0] of the 32-bit instruction.

OPCODE_OP_IMM = 0b0010011  # I-type arithmetic with immediate (addi, etc.)
OPCODE_OP = 0b0110011  # R-type arithmetic (add, sub, etc.)
OPCODE_SYSTEM = 0b1110011  # System instructions (ecall)


# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------


class RiscVDecoder:
    """Decodes RISC-V RV32I instructions from 32-bit binary to structured fields.

    The decoder extracts the opcode, register numbers, and immediate values
    from the raw instruction bits. It doesn't execute anything — it just
    figures out what the instruction means.

    Example: decoding addi x1, x0, 1 (binary: 0x00100093)

        Bits: 000000000001 00000 000 00001 0010011
              ^^^^^^^^^^^^ ^^^^^ ^^^ ^^^^^ ^^^^^^^
              imm=1        rs1=0 f3  rd=1  opcode=OP_IMM

        Result: DecodeResult(mnemonic="addi", fields={"rd": 1, "rs1": 0, "imm": 1})
    """

    def decode(self, raw: int, pc: int) -> DecodeResult:
        """Decode a 32-bit RISC-V instruction.

        Extracts the opcode from bits [6:0], then dispatches to the
        appropriate format decoder (R-type, I-type, etc.).
        """
        opcode = raw & 0x7F  # bits [6:0]

        if opcode == OPCODE_OP_IMM:
            return self._decode_i_type(raw, "addi")
        elif opcode == OPCODE_OP:
            return self._decode_r_type(raw)
        elif opcode == OPCODE_SYSTEM:
            return DecodeResult(
                mnemonic="ecall", fields={}, raw_instruction=raw
            )
        else:
            return DecodeResult(
                mnemonic=f"UNKNOWN(0x{opcode:02x})",
                fields={"opcode": opcode},
                raw_instruction=raw,
            )

    def _decode_r_type(self, raw: int) -> DecodeResult:
        """Decode an R-type instruction (register-register operation).

        R-type format:
            [funct7 | rs2 | rs1 | funct3 | rd | opcode]
             31  25  24 20 19 15  14   12  11 7  6    0

        Example: add x3, x1, x2
            funct7=0000000, rs2=2, rs1=1, funct3=000, rd=3, opcode=0110011
        """
        rd = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        rs2 = (raw >> 20) & 0x1F
        funct7 = (raw >> 25) & 0x7F

        # Determine the specific operation from funct3 and funct7
        if funct3 == 0 and funct7 == 0:
            mnemonic = "add"
        elif funct3 == 0 and funct7 == 0x20:
            mnemonic = "sub"
        else:
            mnemonic = f"r_op(f3={funct3},f7={funct7})"

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rd": rd, "rs1": rs1, "rs2": rs2, "funct3": funct3, "funct7": funct7},
            raw_instruction=raw,
        )

    def _decode_i_type(self, raw: int, default_mnemonic: str) -> DecodeResult:
        """Decode an I-type instruction (immediate operation).

        I-type format:
            [imm[11:0] | rs1 | funct3 | rd | opcode]
             31     20  19 15  14   12  11 7  6    0

        The immediate value is sign-extended from 12 bits to 32 bits.
        This means bit 11 is the sign bit:
            0x001 = 1    (positive)
            0xFFF = -1   (negative, sign-extended)

        Example: addi x1, x0, 1
            imm=000000000001, rs1=0, funct3=000, rd=1, opcode=0010011
        """
        rd = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        imm = (raw >> 20) & 0xFFF

        # Sign-extend the 12-bit immediate to 32 bits
        # If bit 11 is set, the value is negative
        if imm & 0x800:
            imm -= 0x1000  # Convert from unsigned to signed

        return DecodeResult(
            mnemonic=default_mnemonic,
            fields={"rd": rd, "rs1": rs1, "imm": imm, "funct3": funct3},
            raw_instruction=raw,
        )


# ---------------------------------------------------------------------------
# Executor
# ---------------------------------------------------------------------------


class RiscVExecutor:
    """Executes decoded RISC-V instructions.

    The executor reads register values, performs the operation (often using
    the ALU), writes the result back, and determines the next PC.

    RISC-V special rule: register x0 is HARDWIRED to 0. Any write to x0
    is silently ignored. Any read from x0 always returns 0. This is
    enforced here, not in the register file (which is generic).
    """

    def execute(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        """Execute one decoded RISC-V instruction."""
        mnemonic = decoded.mnemonic

        if mnemonic == "addi":
            return self._exec_addi(decoded, registers, pc)
        elif mnemonic == "add":
            return self._exec_add(decoded, registers, pc)
        elif mnemonic == "sub":
            return self._exec_sub(decoded, registers, pc)
        elif mnemonic == "ecall":
            return ExecuteResult(
                description="System call (halt)",
                registers_changed={},
                memory_changed={},
                next_pc=pc,
                halted=True,
            )
        else:
            return ExecuteResult(
                description=f"Unknown instruction: {mnemonic}",
                registers_changed={},
                memory_changed={},
                next_pc=pc + 4,
            )

    def _exec_addi(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: addi rd, rs1, imm → rd = rs1 + imm

        Example: addi x1, x0, 1
            rs1 = x0 = 0 (always zero)
            imm = 1
            result = 0 + 1 = 1
            Write 1 to x1
        """
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        imm = decoded.fields["imm"]

        rs1_val = registers.read(rs1)
        result = (rs1_val + imm) & 0xFFFFFFFF  # Mask to 32 bits

        # x0 is hardwired to 0 — writes to x0 are silently ignored
        changes: dict[str, int] = {}
        if rd != 0:
            registers.write(rd, result)
            changes[f"x{rd}"] = result

        return ExecuteResult(
            description=f"x{rd} = x{rs1}({rs1_val}) + {imm} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_add(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: add rd, rs1, rs2 → rd = rs1 + rs2

        Example: add x3, x1, x2  (where x1=1, x2=2)
            rs1_val = 1, rs2_val = 2
            result = 1 + 2 = 3
            Write 3 to x3
        """
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        rs2 = decoded.fields["rs2"]

        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = (rs1_val + rs2_val) & 0xFFFFFFFF

        changes: dict[str, int] = {}
        if rd != 0:
            registers.write(rd, result)
            changes[f"x{rd}"] = result

        return ExecuteResult(
            description=f"x{rd} = x{rs1}({rs1_val}) + x{rs2}({rs2_val}) = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_sub(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: sub rd, rs1, rs2 → rd = rs1 - rs2"""
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        rs2 = decoded.fields["rs2"]

        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = (rs1_val - rs2_val) & 0xFFFFFFFF

        changes: dict[str, int] = {}
        if rd != 0:
            registers.write(rd, result)
            changes[f"x{rd}"] = result

        return ExecuteResult(
            description=f"x{rd} = x{rs1}({rs1_val}) - x{rs2}({rs2_val}) = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )


# ---------------------------------------------------------------------------
# Assembler helpers
# ---------------------------------------------------------------------------
# These functions encode RISC-V instructions from human-readable form
# to binary. This is a tiny assembler — just enough to create test programs.


def encode_addi(rd: int, rs1: int, imm: int) -> int:
    """Encode: addi rd, rs1, imm → 32-bit instruction.

    I-type format: [imm[11:0] | rs1 | funct3=000 | rd | opcode=0010011]

    Example:
        >>> hex(encode_addi(1, 0, 1))  # addi x1, x0, 1
        '0x93'  # actually 0x00100093
    """
    imm_bits = imm & 0xFFF
    return (imm_bits << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP_IMM


def encode_add(rd: int, rs1: int, rs2: int) -> int:
    """Encode: add rd, rs1, rs2 → 32-bit instruction.

    R-type format: [funct7=0 | rs2 | rs1 | funct3=000 | rd | opcode=0110011]

    Example:
        >>> hex(encode_add(3, 1, 2))  # add x3, x1, x2
        '0x2081b3'
    """
    return (0 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP


def encode_ecall() -> int:
    """Encode: ecall → 32-bit instruction.

    System format: [0...0 | opcode=1110011]

    Example:
        >>> hex(encode_ecall())
        '0x73'
    """
    return OPCODE_SYSTEM


# ---------------------------------------------------------------------------
# High-level simulator
# ---------------------------------------------------------------------------


class RiscVSimulator:
    """Complete RISC-V simulator — ISA + CPU in one convenient class.

    This wraps the CPU simulator with the RISC-V decoder and executor,
    providing a simple interface for running RISC-V programs.

    Example: running x = 1 + 2

        >>> sim = RiscVSimulator()
        >>> program = assemble([
        ...     encode_addi(1, 0, 1),    # x1 = 1
        ...     encode_addi(2, 0, 2),    # x2 = 2
        ...     encode_add(3, 1, 2),     # x3 = x1 + x2 = 3
        ...     encode_ecall(),           # halt
        ... ])
        >>> traces = sim.run(program)
        >>> sim.cpu.registers.read(3)
        3

        The pipeline trace for each instruction shows:
        --- Cycle 0 ---
          FETCH              | DECODE             | EXECUTE
          PC: 0x0000         | addi               | x1 = x0(0) + 1 = 1
          -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
    """

    def __init__(self, memory_size: int = 65536) -> None:
        self.decoder = RiscVDecoder()
        self.executor = RiscVExecutor()
        self.cpu = CPU(
            decoder=self.decoder,
            executor=self.executor,
            num_registers=32,  # RISC-V has 32 registers
            bit_width=32,
            memory_size=memory_size,
        )
        # Enforce x0 = 0 (it's already 0 from initialization,
        # but the executor also prevents writes to x0)

    def run(self, program: bytes) -> list[PipelineTrace]:
        """Load and run a RISC-V program, returning the pipeline trace."""
        self.cpu.load_program(program)
        return self.cpu.run()

    def step(self) -> PipelineTrace:
        """Execute one instruction and return its pipeline trace."""
        return self.cpu.step()


def assemble(instructions: list[int]) -> bytes:
    """Convert a list of 32-bit instruction words to bytes (little-endian).

    This is a convenience function for creating test programs:

        >>> program = assemble([
        ...     encode_addi(1, 0, 1),   # x1 = 1
        ...     encode_addi(2, 0, 2),   # x2 = 2
        ...     encode_add(3, 1, 2),    # x3 = x1 + x2
        ...     encode_ecall(),          # halt
        ... ])
    """
    result = b""
    for instr in instructions:
        result += (instr & 0xFFFFFFFF).to_bytes(4, byteorder="little")
    return result
