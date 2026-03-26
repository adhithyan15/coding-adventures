"""Instruction decoder for all RV32I formats.

=== How decoding works ===

The decoder takes a raw 32-bit instruction and breaks it into meaningful
fields: which registers are involved, what immediate value is encoded, and
what specific operation to perform.

Step 1: read bits [6:0] to get the opcode.
Step 2: the opcode tells us which *format* the instruction uses.
Step 3: the format tells us where each field lives within the 32 bits.

=== Sign extension ===

Immediate values in RISC-V are always sign-extended. If the most significant
bit of the immediate is 1, the value is negative, and we fill the upper
bits with 1s to preserve the two's complement meaning.
"""

from cpu_simulator.pipeline import DecodeResult

from riscv_simulator.opcodes import (
    FUNCT3_ADDI,
    FUNCT3_ANDI,
    FUNCT3_BEQ,
    FUNCT3_BGE,
    FUNCT3_BGEU,
    FUNCT3_BLT,
    FUNCT3_BLTU,
    FUNCT3_BNE,
    FUNCT3_CSRRC,
    FUNCT3_CSRRS,
    FUNCT3_CSRRW,
    FUNCT3_LB,
    FUNCT3_LBU,
    FUNCT3_LH,
    FUNCT3_LHU,
    FUNCT3_LW,
    FUNCT3_ORI,
    FUNCT3_PRIV,
    FUNCT3_SB,
    FUNCT3_SH,
    FUNCT3_SLLI,
    FUNCT3_SLTI,
    FUNCT3_SLTIU,
    FUNCT3_SRLI,
    FUNCT3_SW,
    FUNCT3_XORI,
    FUNCT7_ALT,
    FUNCT7_MRET,
    FUNCT3_ADD,
    FUNCT3_SLL,
    FUNCT3_SLT,
    FUNCT3_SLTU,
    FUNCT3_XOR,
    FUNCT3_SRL,
    FUNCT3_OR,
    FUNCT3_AND,
    FUNCT7_NORMAL,
    OPCODE_AUIPC,
    OPCODE_BRANCH,
    OPCODE_JAL,
    OPCODE_JALR,
    OPCODE_LOAD,
    OPCODE_LUI,
    OPCODE_OP,
    OPCODE_OP_IMM,
    OPCODE_STORE,
    OPCODE_SYSTEM,
)


class RiscVDecoder:
    """Decodes RISC-V RV32I instructions from 32-bit binary to structured fields."""

    def decode(self, raw: int, pc: int) -> DecodeResult:
        """Decode a 32-bit RISC-V instruction."""
        opcode = raw & 0x7F

        if opcode == OPCODE_OP_IMM:
            return self._decode_op_imm(raw)
        elif opcode == OPCODE_OP:
            return self._decode_r_type(raw)
        elif opcode == OPCODE_LOAD:
            return self._decode_load(raw)
        elif opcode == OPCODE_STORE:
            return self._decode_s_type(raw)
        elif opcode == OPCODE_BRANCH:
            return self._decode_b_type(raw)
        elif opcode == OPCODE_JAL:
            return self._decode_j_type(raw, pc)
        elif opcode == OPCODE_JALR:
            return self._decode_jalr(raw)
        elif opcode == OPCODE_LUI:
            return self._decode_u_type(raw, "lui")
        elif opcode == OPCODE_AUIPC:
            return self._decode_u_type(raw, "auipc")
        elif opcode == OPCODE_SYSTEM:
            return self._decode_system(raw)
        else:
            return DecodeResult(
                mnemonic=f"UNKNOWN(0x{opcode:02x})",
                fields={"opcode": opcode},
                raw_instruction=raw,
            )

    def _decode_op_imm(self, raw: int) -> DecodeResult:
        """Decode I-type arithmetic (OpcodeOpImm)."""
        rd = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        imm = (raw >> 20) & 0xFFF

        # Sign-extend 12-bit immediate
        if imm & 0x800:
            imm -= 0x1000

        mnemonic_map = {
            FUNCT3_ADDI: "addi",
            FUNCT3_SLTI: "slti",
            FUNCT3_SLTIU: "sltiu",
            FUNCT3_XORI: "xori",
            FUNCT3_ORI: "ori",
            FUNCT3_ANDI: "andi",
        }

        if funct3 in mnemonic_map:
            mnemonic = mnemonic_map[funct3]
        elif funct3 == FUNCT3_SLLI:
            mnemonic = "slli"
            imm = imm & 0x1F  # shift amount only
        elif funct3 == FUNCT3_SRLI:
            funct7 = (raw >> 25) & 0x7F
            mnemonic = "srai" if funct7 == FUNCT7_ALT else "srli"
            imm = imm & 0x1F
        else:
            mnemonic = f"opimm(f3={funct3})"

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rd": rd, "rs1": rs1, "imm": imm, "funct3": funct3},
            raw_instruction=raw,
        )

    def _decode_r_type(self, raw: int) -> DecodeResult:
        """Decode R-type (OpcodeOp)."""
        rd = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        rs2 = (raw >> 20) & 0x1F
        funct7 = (raw >> 25) & 0x7F

        if funct3 == FUNCT3_ADD and funct7 == FUNCT7_NORMAL:
            mnemonic = "add"
        elif funct3 == FUNCT3_ADD and funct7 == FUNCT7_ALT:
            mnemonic = "sub"
        elif funct3 == FUNCT3_SLL:
            mnemonic = "sll"
        elif funct3 == FUNCT3_SLT:
            mnemonic = "slt"
        elif funct3 == FUNCT3_SLTU:
            mnemonic = "sltu"
        elif funct3 == FUNCT3_XOR:
            mnemonic = "xor"
        elif funct3 == FUNCT3_SRL and funct7 == FUNCT7_NORMAL:
            mnemonic = "srl"
        elif funct3 == FUNCT3_SRL and funct7 == FUNCT7_ALT:
            mnemonic = "sra"
        elif funct3 == FUNCT3_OR:
            mnemonic = "or"
        elif funct3 == FUNCT3_AND:
            mnemonic = "and"
        else:
            mnemonic = f"r_op(f3={funct3},f7={funct7})"

        return DecodeResult(
            mnemonic=mnemonic,
            fields={
                "rd": rd,
                "rs1": rs1,
                "rs2": rs2,
                "funct3": funct3,
                "funct7": funct7,
            },
            raw_instruction=raw,
        )

    def _decode_load(self, raw: int) -> DecodeResult:
        """Decode load instructions (I-type format)."""
        rd = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        imm = (raw >> 20) & 0xFFF

        if imm & 0x800:
            imm -= 0x1000

        load_mnemonics = {
            FUNCT3_LB: "lb",
            FUNCT3_LH: "lh",
            FUNCT3_LW: "lw",
            FUNCT3_LBU: "lbu",
            FUNCT3_LHU: "lhu",
        }
        mnemonic = load_mnemonics.get(funct3, f"load(f3={funct3})")

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rd": rd, "rs1": rs1, "imm": imm, "funct3": funct3},
            raw_instruction=raw,
        )

    def _decode_s_type(self, raw: int) -> DecodeResult:
        """Decode S-type (store) instructions."""
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        rs2 = (raw >> 20) & 0x1F

        # Reconstruct 12-bit immediate from two pieces
        imm_low = (raw >> 7) & 0x1F
        imm_high = (raw >> 25) & 0x7F
        imm = (imm_high << 5) | imm_low

        if imm & 0x800:
            imm -= 0x1000

        store_mnemonics = {FUNCT3_SB: "sb", FUNCT3_SH: "sh", FUNCT3_SW: "sw"}
        mnemonic = store_mnemonics.get(funct3, f"store(f3={funct3})")

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rs1": rs1, "rs2": rs2, "imm": imm, "funct3": funct3},
            raw_instruction=raw,
        )

    def _decode_b_type(self, raw: int) -> DecodeResult:
        """Decode B-type (branch) instructions."""
        funct3 = (raw >> 12) & 0x7
        rs1 = (raw >> 15) & 0x1F
        rs2 = (raw >> 20) & 0x1F

        # Reconstruct the 13-bit immediate (bit 0 is implicitly 0)
        imm12 = (raw >> 31) & 0x1
        imm11 = (raw >> 7) & 0x1
        imm10_5 = (raw >> 25) & 0x3F
        imm4_1 = (raw >> 8) & 0xF

        imm = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1)

        # Sign-extend from 13 bits
        if imm & 0x1000:
            imm -= 0x2000

        branch_mnemonics = {
            FUNCT3_BEQ: "beq",
            FUNCT3_BNE: "bne",
            FUNCT3_BLT: "blt",
            FUNCT3_BGE: "bge",
            FUNCT3_BLTU: "bltu",
            FUNCT3_BGEU: "bgeu",
        }
        mnemonic = branch_mnemonics.get(funct3, f"branch(f3={funct3})")

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rs1": rs1, "rs2": rs2, "imm": imm, "funct3": funct3},
            raw_instruction=raw,
        )

    def _decode_j_type(self, raw: int, pc: int) -> DecodeResult:
        """Decode J-type (JAL) instructions."""
        rd = (raw >> 7) & 0x1F

        # Reconstruct 21-bit immediate (bit 0 is implicitly 0)
        imm20 = (raw >> 31) & 0x1
        imm10_1 = (raw >> 21) & 0x3FF
        imm11 = (raw >> 20) & 0x1
        imm19_12 = (raw >> 12) & 0xFF

        imm = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1)

        # Sign-extend from 21 bits
        if imm & 0x100000:
            imm -= 0x200000

        return DecodeResult(
            mnemonic="jal",
            fields={"rd": rd, "imm": imm},
            raw_instruction=raw,
        )

    def _decode_jalr(self, raw: int) -> DecodeResult:
        """Decode JALR (I-type)."""
        rd = (raw >> 7) & 0x1F
        rs1 = (raw >> 15) & 0x1F
        imm = (raw >> 20) & 0xFFF

        if imm & 0x800:
            imm -= 0x1000

        return DecodeResult(
            mnemonic="jalr",
            fields={"rd": rd, "rs1": rs1, "imm": imm},
            raw_instruction=raw,
        )

    def _decode_u_type(self, raw: int, mnemonic: str) -> DecodeResult:
        """Decode U-type (LUI / AUIPC)."""
        rd = (raw >> 7) & 0x1F
        imm = raw >> 12

        # Sign-extend from 20 bits
        if imm & 0x80000:
            imm -= 0x100000

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rd": rd, "imm": imm},
            raw_instruction=raw,
        )

    def _decode_system(self, raw: int) -> DecodeResult:
        """Decode system instructions (ecall, mret, CSR ops)."""
        funct3 = (raw >> 12) & 0x7

        if funct3 == FUNCT3_PRIV:
            funct7 = (raw >> 25) & 0x7F
            if funct7 == FUNCT7_MRET:
                return DecodeResult(
                    mnemonic="mret",
                    fields={"funct7": funct7},
                    raw_instruction=raw,
                )
            return DecodeResult(
                mnemonic="ecall",
                fields={"funct7": funct7},
                raw_instruction=raw,
            )

        # CSR instructions
        rd = (raw >> 7) & 0x1F
        rs1 = (raw >> 15) & 0x1F
        csr = (raw >> 20) & 0xFFF

        csr_mnemonics = {
            FUNCT3_CSRRW: "csrrw",
            FUNCT3_CSRRS: "csrrs",
            FUNCT3_CSRRC: "csrrc",
        }
        mnemonic = csr_mnemonics.get(funct3, f"system(f3={funct3})")

        return DecodeResult(
            mnemonic=mnemonic,
            fields={"rd": rd, "rs1": rs1, "csr": csr, "funct3": funct3},
            raw_instruction=raw,
        )
