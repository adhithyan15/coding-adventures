# frozen_string_literal: true

# === RISC-V RV32I opcode constants ===
#
# Every RISC-V instruction is exactly 32 bits wide. The lowest 7 bits ([6:0])
# always contain the "opcode" -- which tells the CPU what category of work
# this instruction does. Within each category, funct3 and funct7 fields
# narrow down the exact operation.

module CodingAdventures
  module RiscvSimulator
    # === RV32I Base Integer Opcodes ===
    OPCODE_LOAD    = 0b0000011
    OPCODE_STORE   = 0b0100011
    OPCODE_BRANCH  = 0b1100011
    OPCODE_JAL     = 0b1101111
    OPCODE_JALR    = 0b1100111
    OPCODE_LUI     = 0b0110111
    OPCODE_AUIPC   = 0b0010111
    OPCODE_OP_IMM  = 0b0010011
    OPCODE_OP      = 0b0110011
    OPCODE_SYSTEM  = 0b1110011

    # === Funct3 for I-type immediate arithmetic ===
    FUNCT3_ADDI  = 0
    FUNCT3_SLTI  = 2
    FUNCT3_SLTIU = 3
    FUNCT3_XORI  = 4
    FUNCT3_ORI   = 6
    FUNCT3_ANDI  = 7
    FUNCT3_SLLI  = 1
    FUNCT3_SRLI  = 5 # also SRAI

    # === Funct3 for R-type arithmetic ===
    FUNCT3_ADD  = 0 # also SUB
    FUNCT3_SLL  = 1
    FUNCT3_SLT  = 2
    FUNCT3_SLTU = 3
    FUNCT3_XOR  = 4
    FUNCT3_SRL  = 5 # also SRA
    FUNCT3_OR   = 6
    FUNCT3_AND  = 7

    # === Funct7 ===
    FUNCT7_NORMAL = 0x00
    FUNCT7_ALT    = 0x20

    # === Funct3 for loads ===
    FUNCT3_LB  = 0
    FUNCT3_LH  = 1
    FUNCT3_LW  = 2
    FUNCT3_LBU = 4
    FUNCT3_LHU = 5

    # === Funct3 for stores ===
    FUNCT3_SB = 0
    FUNCT3_SH = 1
    FUNCT3_SW = 2

    # === Funct3 for branches ===
    FUNCT3_BEQ  = 0
    FUNCT3_BNE  = 1
    FUNCT3_BLT  = 4
    FUNCT3_BGE  = 5
    FUNCT3_BLTU = 6
    FUNCT3_BGEU = 7

    # === Funct3 for system ===
    FUNCT3_PRIV  = 0
    FUNCT3_CSRRW = 1
    FUNCT3_CSRRS = 2
    FUNCT3_CSRRC = 3

    # === Funct7 for privileged instructions ===
    FUNCT7_ECALL = 0x00
    FUNCT7_MRET  = 0x18
  end
end
