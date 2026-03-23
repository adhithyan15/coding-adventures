"""Adapter to run RISC-V on a D05 Core pipeline.

=== Bridging Two Worlds ===

The riscv-simulator package has its own decoder and executor that produce
DecodeResult and ExecuteResult structs. The D05 Core (from the core package)
expects an ISADecoder interface that works with PipelineToken dicts.

This adapter bridges the gap:

    RISC-V world                     Core world
    ---------------                  ----------
    uint32 instructions              int instructions
    DecodeResult (mnemonic, fields)  PipelineToken (rs1, rs2, rd, signals)
    ExecuteResult (changes, nextPC)  PipelineToken (alu_result, branch_taken)
    cpu.RegisterFile (uint32)        core.RegisterFile (int)

The adapter translates between these representations at two points:

    1. Decode: RISC-V decoder fills a DecodeResult -> adapter copies fields
       into the PipelineToken (rs1, rs2, rd, control signals).

    2. Execute: adapter reads register values from the Core's RegisterFile,
       computes ALU results using RISC-V semantics, and fills the token's
       alu_result, branch_taken, branch_target, and write_data fields.

=== Why Not Just Reuse the RISC-V Executor Directly? ===

The existing RiscVExecutor.execute() does everything in one shot: reads
registers, computes results, modifies registers, AND accesses memory.
But the Core's pipeline separates these into distinct stages:

    ID stage:  decode instruction, identify registers
    EX stage:  compute ALU result, resolve branches
    MEM stage: access memory (loads/stores) -- handled by Core
    WB stage:  write registers -- handled by Core

The adapter must NOT read/write registers or access memory during Execute.
It only computes ALU results. The Core handles memory and writeback.
"""

from __future__ import annotations

from riscv_simulator.csr import (
    CAUSE_ECALL_M_MODE,
    CSR_MCAUSE,
    CSR_MEPC,
    CSR_MSTATUS,
    CSR_MTVEC,
    MIE,
    CSRFile,
)
from riscv_simulator.decode import RiscVDecoder


# Mask to keep values within 32 bits, matching RISC-V's 32-bit register width.
MASK32 = 0xFFFFFFFF


def _to_signed32(val: int) -> int:
    """Interpret a 32-bit unsigned value as signed (two's complement)."""
    val = val & MASK32
    if val & 0x80000000:
        return val - 0x100000000
    return val


def _to_unsigned32(val: int) -> int:
    """Ensure a value is in the unsigned 32-bit range."""
    return val & MASK32


def _get_field(fields: dict[str, int], key: str, default: int) -> int:
    """Retrieve a field from a decoded fields dict, with a default."""
    return fields.get(key, default)


class RiscVISADecoder:
    """Adapts the RISC-V decoder and executor to a Core's ISADecoder interface.

    It holds references to the underlying RISC-V decoder (for instruction
    decoding) and a CSR file (for system instructions like ecall/mret).

    The decoder is stateless -- it just parses bits. The CSR file is stateful
    and is shared with the executor logic.
    """

    def __init__(self) -> None:
        self._decoder = RiscVDecoder()
        self._csr = CSRFile()

    @property
    def csr(self) -> CSRFile:
        """Access the CSR file (useful for tests and configuration)."""
        return self._csr

    def instruction_size(self) -> int:
        """Return 4 -- all RV32I instructions are 32 bits (4 bytes)."""
        return 4

    def decode(self, raw_instruction: int, token: dict) -> dict:
        """Translate raw RISC-V instruction bits into a PipelineToken dict.

        This is the ID (Instruction Decode) stage. The adapter:
        1. Calls the RISC-V decoder to parse raw bits into a DecodeResult.
        2. Copies decoded fields into the token dict that the Core uses.

        Control signals are derived from the mnemonic via a truth table:

            Instruction    RegWrite  MemRead  MemWrite  IsBranch  IsHalt
            -----------    --------  -------  --------  --------  ------
            add, sub, ...  True      False    False     False     False
            lw, lb, ...    True      True     False     False     False
            sw, sb, ...    False     False    True      False     False
            beq, bne, ...  False     False    False     True      False
            jal, jalr      True      False    False     True      False
            ecall (halt)   False     False    False     False     True
        """
        raw = raw_instruction & MASK32
        decoded = self._decoder.decode(raw, token.get("pc", 0))

        token["opcode"] = decoded.mnemonic
        token["rd"] = _get_field(decoded.fields, "rd", -1)
        token["rs1"] = _get_field(decoded.fields, "rs1", -1)
        token["rs2"] = _get_field(decoded.fields, "rs2", -1)
        token["immediate"] = _get_field(decoded.fields, "imm", 0)

        mnemonic = decoded.mnemonic

        # R-type arithmetic
        if mnemonic in ("add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and"):
            token["reg_write"] = True
        # I-type arithmetic
        elif mnemonic in ("addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai"):
            token["reg_write"] = True
        # Upper immediate
        elif mnemonic in ("lui", "auipc"):
            token["reg_write"] = True
        # Loads
        elif mnemonic in ("lb", "lh", "lw", "lbu", "lhu"):
            token["reg_write"] = True
            token["mem_read"] = True
        # Stores
        elif mnemonic in ("sb", "sh", "sw"):
            token["mem_write"] = True
        # Branches
        elif mnemonic in ("beq", "bne", "blt", "bge", "bltu", "bgeu"):
            token["is_branch"] = True
        # Jumps
        elif mnemonic in ("jal", "jalr"):
            token["reg_write"] = True
            token["is_branch"] = True
        # ecall
        elif mnemonic == "ecall":
            if self._csr.read(CSR_MTVEC) == 0:
                token["is_halt"] = True
        # CSR instructions
        elif mnemonic in ("csrrw", "csrrs", "csrrc"):
            token["reg_write"] = True
        # mret
        elif mnemonic == "mret":
            token["is_branch"] = True

        return token

    def execute(self, token: dict, reg_file: object) -> dict:
        """Perform the ALU computation for a decoded RISC-V instruction.

        This is the EX (Execute) stage. The adapter:
        1. Reads source register values from the Core's RegisterFile.
        2. Computes the ALU result based on the mnemonic and operands.
        3. For branches: resolves whether the branch is taken.
        4. For loads/stores: computes the effective address.

        IMPORTANT: Does NOT access memory or write registers.
        """
        rs1_val = reg_file.read(token["rs1"]) if token.get("rs1", -1) >= 0 else 0
        rs2_val = reg_file.read(token["rs2"]) if token.get("rs2", -1) >= 0 else 0

        rs1_u = _to_unsigned32(rs1_val)
        rs2_u = _to_unsigned32(rs2_val)
        imm = token.get("immediate", 0)
        pc = token.get("pc", 0)
        opcode = token.get("opcode", "")

        # === R-type arithmetic ===
        if opcode == "add":
            result = _to_signed32(rs1_u + rs2_u)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "sub":
            result = _to_signed32(rs1_u - rs2_u)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "sll":
            result = int(rs1_u << (rs2_u & 0x1F)) & MASK32
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "slt":
            result = 1 if _to_signed32(rs1_u) < _to_signed32(rs2_u) else 0
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "sltu":
            result = 1 if rs1_u < rs2_u else 0
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "xor":
            result = int(rs1_u ^ rs2_u)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "srl":
            result = int(rs1_u >> (rs2_u & 0x1F))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "sra":
            result = _to_signed32(rs1_u) >> (rs2_u & 0x1F)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "or":
            result = int(rs1_u | rs2_u)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "and":
            result = int(rs1_u & rs2_u)
            token["alu_result"] = result
            token["write_data"] = result

        # === I-type arithmetic ===
        elif opcode == "addi":
            result = _to_signed32(rs1_u + _to_unsigned32(imm))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "slti":
            result = 1 if _to_signed32(rs1_u) < _to_signed32(imm) else 0
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "sltiu":
            result = 1 if rs1_u < _to_unsigned32(imm) else 0
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "xori":
            result = int(rs1_u ^ _to_unsigned32(imm))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "ori":
            result = int(rs1_u | _to_unsigned32(imm))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "andi":
            result = int(rs1_u & _to_unsigned32(imm))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "slli":
            shamt = _to_unsigned32(imm) & 0x1F
            result = int(rs1_u << shamt) & MASK32
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "srli":
            shamt = _to_unsigned32(imm) & 0x1F
            result = int(rs1_u >> shamt)
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "srai":
            shamt = _to_unsigned32(imm) & 0x1F
            result = _to_signed32(rs1_u) >> shamt
            token["alu_result"] = result
            token["write_data"] = result

        # === Upper immediate ===
        elif opcode == "lui":
            result = int(_to_unsigned32(imm << 12))
            token["alu_result"] = result
            token["write_data"] = result

        elif opcode == "auipc":
            result = int(_to_unsigned32(pc) + _to_unsigned32(imm << 12)) & MASK32
            token["alu_result"] = result
            token["write_data"] = result

        # === Load instructions ===
        elif opcode in ("lb", "lh", "lw", "lbu", "lhu"):
            addr = _to_signed32(rs1_u) + _to_signed32(imm)
            token["alu_result"] = addr

        # === Store instructions ===
        elif opcode in ("sb", "sh", "sw"):
            addr = _to_signed32(rs1_u) + _to_signed32(imm)
            token["alu_result"] = addr
            token["write_data"] = rs2_val

        # === Branch instructions ===
        elif opcode == "beq":
            taken = rs1_u == rs2_u
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        elif opcode == "bne":
            taken = rs1_u != rs2_u
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        elif opcode == "blt":
            taken = _to_signed32(rs1_u) < _to_signed32(rs2_u)
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        elif opcode == "bge":
            taken = _to_signed32(rs1_u) >= _to_signed32(rs2_u)
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        elif opcode == "bltu":
            taken = rs1_u < rs2_u
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        elif opcode == "bgeu":
            taken = rs1_u >= rs2_u
            target = pc + imm
            token["branch_taken"] = taken
            token["branch_target"] = target
            token["alu_result"] = target if taken else pc + 4

        # === Jump instructions ===
        elif opcode == "jal":
            return_addr = pc + 4
            target = pc + imm
            token["alu_result"] = target
            token["write_data"] = return_addr
            token["branch_taken"] = True
            token["branch_target"] = target

        elif opcode == "jalr":
            return_addr = pc + 4
            target = (_to_signed32(rs1_u) + _to_signed32(imm)) & ~1
            token["alu_result"] = target
            token["write_data"] = return_addr
            token["branch_taken"] = True
            token["branch_target"] = target

        # === CSR instructions ===
        elif opcode == "csrrw":
            csr_addr = (token.get("raw_instruction", 0) >> 20) & 0xFFF
            old_val = self._csr.read_write(csr_addr, rs1_u)
            token["alu_result"] = int(old_val)
            token["write_data"] = int(old_val)

        elif opcode == "csrrs":
            csr_addr = (token.get("raw_instruction", 0) >> 20) & 0xFFF
            old_val = self._csr.read_set(csr_addr, rs1_u)
            token["alu_result"] = int(old_val)
            token["write_data"] = int(old_val)

        elif opcode == "csrrc":
            csr_addr = (token.get("raw_instruction", 0) >> 20) & 0xFFF
            old_val = self._csr.read_clear(csr_addr, rs1_u)
            token["alu_result"] = int(old_val)
            token["write_data"] = int(old_val)

        # === ecall ===
        elif opcode == "ecall":
            mtvec = self._csr.read(CSR_MTVEC)
            if mtvec != 0:
                self._csr.write(CSR_MEPC, pc)
                self._csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE)
                mstatus = self._csr.read(CSR_MSTATUS)
                self._csr.write(CSR_MSTATUS, mstatus & ~MIE)
                token["branch_taken"] = True
                token["branch_target"] = int(mtvec)
                token["alu_result"] = int(mtvec)

        # === mret ===
        elif opcode == "mret":
            mepc = self._csr.read(CSR_MEPC)
            mstatus = self._csr.read(CSR_MSTATUS)
            self._csr.write(CSR_MSTATUS, mstatus | MIE)
            token["branch_taken"] = True
            token["branch_target"] = int(mepc)
            token["alu_result"] = int(mepc)

        return token


def new_riscv_core() -> RiscVISADecoder:
    """Factory function to create a RISC-V ISA decoder for a D05 Core."""
    return RiscVISADecoder()
