"""RISC-V RV32I opcode constants.

=== How RISC-V encodes instructions ===

Every RISC-V instruction is exactly 32 bits wide. The lowest 7 bits ([6:0])
always contain the "opcode" -- a number that tells the CPU what general
category of work this instruction does. Think of it as a filing system:

    bits [6:0]  =  opcode  =  "which drawer to open"

Within each drawer, additional fields (funct3, funct7) narrow down the
exact operation. For example, opcode 0b0110011 means "R-type arithmetic,"
and then funct3=0 + funct7=0 means "add," while funct3=0 + funct7=0x20
means "sub."

=== The six instruction formats ===

RISC-V uses only six encoding formats:

    R-type:  [funct7 | rs2 | rs1 | funct3 | rd | opcode]  -- register-to-register
    I-type:  [imm[11:0]   | rs1 | funct3 | rd | opcode]  -- immediate operand
    S-type:  [imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode]  -- stores
    B-type:  [imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode]  -- branches
    U-type:  [imm[31:12]                       | rd | opcode]  -- upper immediate
    J-type:  [imm[20|10:1|11|19:12]            | rd | opcode]  -- jumps
"""

# === RV32I Base Integer Opcodes ===

OPCODE_LOAD = 0b0000011  # lb, lh, lw, lbu, lhu (I-type)
OPCODE_STORE = 0b0100011  # sb, sh, sw (S-type)
OPCODE_BRANCH = 0b1100011  # beq, bne, blt, bge, bltu, bgeu (B-type)
OPCODE_JAL = 0b1101111  # jal (J-type)
OPCODE_JALR = 0b1100111  # jalr (I-type)
OPCODE_LUI = 0b0110111  # lui (U-type)
OPCODE_AUIPC = 0b0010111  # auipc (U-type)
OPCODE_OP_IMM = 0b0010011  # addi, slti, etc. (I-type)
OPCODE_OP = 0b0110011  # add, sub, etc. (R-type)
OPCODE_SYSTEM = 0b1110011  # ecall, csrrw, csrrs, csrrc, mret

# === Funct3 constants for I-type immediate arithmetic (OPCODE_OP_IMM) ===

FUNCT3_ADDI = 0
FUNCT3_SLTI = 2
FUNCT3_SLTIU = 3
FUNCT3_XORI = 4
FUNCT3_ORI = 6
FUNCT3_ANDI = 7
FUNCT3_SLLI = 1
FUNCT3_SRLI = 5  # also SRAI -- distinguished by funct7

# === Funct3 constants for R-type arithmetic (OPCODE_OP) ===

FUNCT3_ADD = 0  # also SUB -- distinguished by funct7
FUNCT3_SLL = 1
FUNCT3_SLT = 2
FUNCT3_SLTU = 3
FUNCT3_XOR = 4
FUNCT3_SRL = 5  # also SRA -- distinguished by funct7
FUNCT3_OR = 6
FUNCT3_AND = 7

# === Funct7 constants ===

FUNCT7_NORMAL = 0x00  # add, srl, slli, srli
FUNCT7_ALT = 0x20  # sub, sra, srai

# === Funct3 constants for load instructions ===

FUNCT3_LB = 0  # load byte, sign-extend
FUNCT3_LH = 1  # load halfword, sign-extend
FUNCT3_LW = 2  # load word
FUNCT3_LBU = 4  # load byte, zero-extend
FUNCT3_LHU = 5  # load halfword, zero-extend

# === Funct3 constants for store instructions ===

FUNCT3_SB = 0  # store byte
FUNCT3_SH = 1  # store halfword
FUNCT3_SW = 2  # store word

# === Funct3 constants for branch instructions ===

FUNCT3_BEQ = 0
FUNCT3_BNE = 1
FUNCT3_BLT = 4
FUNCT3_BGE = 5
FUNCT3_BLTU = 6
FUNCT3_BGEU = 7

# === Funct3 constants for system instructions ===

FUNCT3_PRIV = 0  # ecall, ebreak, mret
FUNCT3_CSRRW = 1  # CSR read-write
FUNCT3_CSRRS = 2  # CSR read-set
FUNCT3_CSRRC = 3  # CSR read-clear

# === Funct7 for privileged instructions (funct3=0) ===

FUNCT7_ECALL = 0x00
FUNCT7_MRET = 0x18
