"""ISADecoder -- the interface between the Core and any instruction set.

The Core knows how to move instructions through a pipeline, predict
branches, detect hazards, and access caches. But it does NOT know what
any instruction means. That is the ISA decoder's job.

This separation mirrors real CPU design:
  - ARM defines the decoder semantics (what ADD, LDR, BEQ mean)
  - Apple/Qualcomm build the pipeline and caches
  - The decoder plugs into the pipeline via a well-defined interface

Our ISADecoder protocol is that well-defined interface. Any ISA
(ARM, RISC-V, x86, or a custom teaching ISA) can implement it and
immediately run on any Core configuration.

# The Three Methods

The decoder has exactly three responsibilities:

 1. decode: turn raw instruction bits into a structured PipelineToken
    (fill in opcode, registers, control signals, immediate value)

 2. execute: perform the actual computation (ALU operation, branch
    resolution, effective address calculation)

 3. instruction_size: how many bytes each instruction occupies

These map directly to the ID and EX stages of the pipeline:

    IF stage:  fetch raw bits from memory
    ID stage:  decoder.decode(raw, token) -> fills in decoded fields
    EX stage:  decoder.execute(token, reg_file) -> computes ALU result
    MEM stage: core handles cache access
    WB stage:  core handles register writeback
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

from cpu_pipeline import PipelineToken

if TYPE_CHECKING:
    from core.register_file import RegisterFile


# =========================================================================
# ISADecoder -- the protocol any instruction set must implement
# =========================================================================


class ISADecoder(Protocol):
    """Protocol that any instruction set architecture must implement.

    The Core calls decode() in the ID stage and execute() in the EX stage.
    Any class with these three methods is automatically a valid ISADecoder
    (structural typing via Protocol -- no inheritance needed).
    """

    def decode(
        self,
        raw_instruction: int,
        token: PipelineToken,
    ) -> PipelineToken:
        """Turn raw instruction bits into a structured PipelineToken.

        The decoder fills in:
          - opcode (string name like "ADD", "LDR", "BEQ")
          - rs1, rs2 (source register numbers, -1 if unused)
          - rd (destination register number, -1 if unused)
          - immediate (sign-extended immediate value)
          - Control signals: reg_write, mem_read, mem_write, is_branch, is_halt

        Args:
            raw_instruction: The 32-bit value fetched from memory.
            token: Pre-allocated token; the decoder fills in fields.

        Returns:
            The same token, with decoded fields filled in.
        """
        ...

    def execute(
        self,
        token: PipelineToken,
        reg_file: RegisterFile,
    ) -> PipelineToken:
        """Perform the ALU operation for a decoded instruction.

        The executor fills in:
          - alu_result (computed value, or effective address for loads/stores)
          - branch_taken (was the branch actually taken?)
          - branch_target (where does the branch go?)
          - write_data (final value to write to Rd, if reg_write is True)

        Args:
            token: The decoded instruction token.
            reg_file: The register file for reading source values.

        Returns:
            The same token, with execution results filled in.
        """
        ...

    def instruction_size(self) -> int:
        """Return the size of one instruction in bytes.

        This determines how much the PC advances after each fetch:
          - ARM (A64): 4 bytes (fixed-width 32-bit instructions)
          - RISC-V:    4 bytes (base ISA) or 2 bytes (compressed)
          - x86:       variable (1-15 bytes)
        """
        ...


# =========================================================================
# MockDecoder -- a simple decoder for testing the Core
# =========================================================================


class MockDecoder:
    """Minimal ISA decoder for testing purposes.

    It supports a handful of instructions encoded in a simple format:

        Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH,
                            5=HALT, 6=ADDI, 7=SUB)
        Bits 23-20: Rd  (destination register)
        Bits 19-16: Rs1 (first source register)
        Bits 15-12: Rs2 (second source register)
        Bits 11-0:  immediate (12-bit, sign-extended)

    This encoding does not match any real ISA. It exists solely to exercise
    the Core's pipeline, hazard detection, branch prediction, and caches.

    Instruction Reference:

        NOP    (0x00): Do nothing.
        ADD    (0x01): Rd = Rs1 + Rs2
        LOAD   (0x02): Rd = Memory[Rs1 + imm]  (word load)
        STORE  (0x03): Memory[Rs1 + imm] = Rs2  (word store)
        BRANCH (0x04): If Rs1 == Rs2, PC = PC + imm (conditional branch)
        HALT   (0x05): Stop execution.
        ADDI   (0x06): Rd = Rs1 + imm
        SUB    (0x07): Rd = Rs1 - Rs2
    """

    def instruction_size(self) -> int:
        """Return 4 (all mock instructions are 32 bits)."""
        return 4

    def decode(
        self,
        raw_instruction: int,
        token: PipelineToken,
    ) -> PipelineToken:
        """Extract fields from a raw 32-bit instruction and fill in the token.

        Encoding layout::

              31      24 23    20 19    16 15    12 11           0
            +----------+--------+--------+--------+--------------+
            |  opcode  |   Rd   |  Rs1   |  Rs2   |  immediate   |
            +----------+--------+--------+--------+--------------+

        The immediate is sign-extended from 12 bits to a full int.

        Args:
            raw_instruction: Raw 32-bit instruction bits.
            token: Token to fill with decoded fields.

        Returns:
            The token with decoded fields.
        """
        # Extract fields using bit masking and shifting.
        opcode = (raw_instruction >> 24) & 0xFF
        rd = (raw_instruction >> 20) & 0x0F
        rs1 = (raw_instruction >> 16) & 0x0F
        rs2 = (raw_instruction >> 12) & 0x0F
        imm = raw_instruction & 0xFFF

        # Sign-extend the 12-bit immediate to a full int.
        # If bit 11 is set, the value is negative.
        if imm & 0x800:
            imm |= ~0xFFF  # sign-extend by filling upper bits with 1s

        # Fill in decoded fields based on opcode.
        if opcode == 0x00:  # NOP
            token.opcode = "NOP"
            token.rd = -1
            token.rs1 = -1
            token.rs2 = -1

        elif opcode == 0x01:  # ADD Rd, Rs1, Rs2
            token.opcode = "ADD"
            token.rd = rd
            token.rs1 = rs1
            token.rs2 = rs2
            token.reg_write = True

        elif opcode == 0x02:  # LOAD Rd, [Rs1 + imm]
            token.opcode = "LOAD"
            token.rd = rd
            token.rs1 = rs1
            token.rs2 = -1
            token.immediate = imm
            token.reg_write = True
            token.mem_read = True

        elif opcode == 0x03:  # STORE [Rs1 + imm], Rs2
            token.opcode = "STORE"
            token.rd = -1
            token.rs1 = rs1
            token.rs2 = rs2
            token.immediate = imm
            token.mem_write = True

        elif opcode == 0x04:  # BRANCH Rs1, Rs2, imm
            token.opcode = "BRANCH"
            token.rd = -1
            token.rs1 = rs1
            token.rs2 = rs2
            token.immediate = imm
            token.is_branch = True

        elif opcode == 0x05:  # HALT
            token.opcode = "HALT"
            token.rd = -1
            token.rs1 = -1
            token.rs2 = -1
            token.is_halt = True

        elif opcode == 0x06:  # ADDI Rd, Rs1, imm
            token.opcode = "ADDI"
            token.rd = rd
            token.rs1 = rs1
            token.rs2 = -1
            token.immediate = imm
            token.reg_write = True

        elif opcode == 0x07:  # SUB Rd, Rs1, Rs2
            token.opcode = "SUB"
            token.rd = rd
            token.rs1 = rs1
            token.rs2 = rs2
            token.reg_write = True

        else:  # Unknown opcode -- treat as NOP
            token.opcode = "NOP"
            token.rd = -1
            token.rs1 = -1
            token.rs2 = -1

        return token

    def execute(
        self,
        token: PipelineToken,
        reg_file: RegisterFile,
    ) -> PipelineToken:
        """Perform the ALU operation for a decoded instruction.

        Reads register values, computes the result, and fills in
        alu_result, branch_taken, branch_target, and write_data.

        Args:
            token: The decoded instruction token.
            reg_file: Register file for reading source register values.

        Returns:
            The token with execution results.
        """
        # Read source register values.
        rs1_val = reg_file.read(token.rs1) if token.rs1 >= 0 else 0
        rs2_val = reg_file.read(token.rs2) if token.rs2 >= 0 else 0

        if token.opcode == "ADD":
            token.alu_result = rs1_val + rs2_val
            token.write_data = token.alu_result

        elif token.opcode == "SUB":
            token.alu_result = rs1_val - rs2_val
            token.write_data = token.alu_result

        elif token.opcode == "ADDI":
            token.alu_result = rs1_val + token.immediate
            token.write_data = token.alu_result

        elif token.opcode == "LOAD":
            # Effective address = Rs1 + immediate.
            # The actual memory read happens in the MEM stage (handled by Core).
            token.alu_result = rs1_val + token.immediate

        elif token.opcode == "STORE":
            # Effective address = Rs1 + immediate.
            # The data to store comes from Rs2.
            token.alu_result = rs1_val + token.immediate
            token.write_data = rs2_val

        elif token.opcode == "BRANCH":
            # Branch condition: Rs1 == Rs2
            # Branch target: PC + (immediate * instruction_size)
            taken = rs1_val == rs2_val
            token.branch_taken = taken
            target = token.pc + (token.immediate * 4)
            token.branch_target = target
            if taken:
                token.alu_result = target
            else:
                token.alu_result = token.pc + 4

        # NOP, HALT, and unknown opcodes need no computation.

        return token


# =========================================================================
# MockInstruction -- helpers for encoding mock instructions
# =========================================================================


def encode_nop() -> int:
    """Return the raw encoding for a NOP instruction."""
    return 0x00 << 24


def encode_add(rd: int, rs1: int, rs2: int) -> int:
    """Return the raw encoding for ADD Rd, Rs1, Rs2."""
    return (0x01 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)


def encode_sub(rd: int, rs1: int, rs2: int) -> int:
    """Return the raw encoding for SUB Rd, Rs1, Rs2."""
    return (0x07 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)


def encode_addi(rd: int, rs1: int, imm: int) -> int:
    """Return the raw encoding for ADDI Rd, Rs1, imm."""
    return (0x06 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)


def encode_load(rd: int, rs1: int, imm: int) -> int:
    """Return the raw encoding for LOAD Rd, [Rs1 + imm]."""
    return (0x02 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)


def encode_store(rs1: int, rs2: int, imm: int) -> int:
    """Return the raw encoding for STORE [Rs1 + imm], Rs2."""
    return (0x03 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)


def encode_branch(rs1: int, rs2: int, imm: int) -> int:
    """Return the raw encoding for BRANCH Rs1, Rs2, imm.

    The branch is taken if Rs1 == Rs2, jumping to PC + imm*4.
    """
    return (0x04 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)


def encode_halt() -> int:
    """Return the raw encoding for a HALT instruction."""
    return 0x05 << 24


def encode_program(*instructions: int) -> bytes:
    """Convert a sequence of raw instruction ints into bytes.

    Each instruction is encoded as 4 bytes in little-endian order,
    suitable for load_program().

    Example::

        program = encode_program(encode_addi(1, 0, 42), encode_halt())
        core.load_program(program, 0)

    Args:
        *instructions: Raw instruction values to encode.

    Returns:
        Byte string with all instructions in little-endian order.
    """
    result = bytearray(len(instructions) * 4)
    for i, instr in enumerate(instructions):
        offset = i * 4
        result[offset] = instr & 0xFF
        result[offset + 1] = (instr >> 8) & 0xFF
        result[offset + 2] = (instr >> 16) & 0xFF
        result[offset + 3] = (instr >> 24) & 0xFF
    return bytes(result)
