//! Opcode constants for the RV32I base integer ISA.

// RV32I Base Integer Opcodes
pub const OPCODE_LOAD: u32    = 0b0000011;
pub const OPCODE_STORE: u32   = 0b0100011;
pub const OPCODE_BRANCH: u32  = 0b1100011;
pub const OPCODE_JAL: u32     = 0b1101111;
pub const OPCODE_JALR: u32    = 0b1100111;
pub const OPCODE_LUI: u32     = 0b0110111;
pub const OPCODE_AUIPC: u32   = 0b0010111;
pub const OPCODE_OP_IMM: u32  = 0b0010011;
pub const OPCODE_OP: u32      = 0b0110011;
pub const OPCODE_SYSTEM: u32  = 0b1110011;

// Funct3 for I-type immediate arithmetic
pub const FUNCT3_ADDI: u32  = 0;
pub const FUNCT3_SLTI: u32  = 2;
pub const FUNCT3_SLTIU: u32 = 3;
pub const FUNCT3_XORI: u32  = 4;
pub const FUNCT3_ORI: u32   = 6;
pub const FUNCT3_ANDI: u32  = 7;
pub const FUNCT3_SLLI: u32  = 1;
pub const FUNCT3_SRLI: u32  = 5;

// Funct3 for R-type arithmetic
pub const FUNCT3_ADD: u32  = 0;
pub const FUNCT3_SLL: u32  = 1;
pub const FUNCT3_SLT: u32  = 2;
pub const FUNCT3_SLTU: u32 = 3;
pub const FUNCT3_XOR: u32  = 4;
pub const FUNCT3_SRL: u32  = 5;
pub const FUNCT3_OR: u32   = 6;
pub const FUNCT3_AND: u32  = 7;

// Funct7
pub const FUNCT7_NORMAL: u32 = 0x00;
pub const FUNCT7_ALT: u32    = 0x20;

// Funct3 for loads
pub const FUNCT3_LB: u32  = 0;
pub const FUNCT3_LH: u32  = 1;
pub const FUNCT3_LW: u32  = 2;
pub const FUNCT3_LBU: u32 = 4;
pub const FUNCT3_LHU: u32 = 5;

// Funct3 for stores
pub const FUNCT3_SB: u32 = 0;
pub const FUNCT3_SH: u32 = 1;
pub const FUNCT3_SW: u32 = 2;

// Funct3 for branches
pub const FUNCT3_BEQ: u32  = 0;
pub const FUNCT3_BNE: u32  = 1;
pub const FUNCT3_BLT: u32  = 4;
pub const FUNCT3_BGE: u32  = 5;
pub const FUNCT3_BLTU: u32 = 6;
pub const FUNCT3_BGEU: u32 = 7;

// Funct3 for system
pub const FUNCT3_PRIV: u32  = 0;
pub const FUNCT3_CSRRW: u32 = 1;
pub const FUNCT3_CSRRS: u32 = 2;
pub const FUNCT3_CSRRC: u32 = 3;

// Funct7 for privileged instructions
pub const FUNCT7_ECALL: u32 = 0x00;
pub const FUNCT7_MRET: u32  = 0x18;
