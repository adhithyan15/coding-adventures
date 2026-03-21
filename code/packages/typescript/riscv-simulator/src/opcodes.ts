/**
 * Opcode constants for the RV32I base integer ISA.
 *
 * Every RISC-V instruction is exactly 32 bits wide. The lowest 7 bits [6:0]
 * always contain the "opcode" -- a number that tells the CPU what general
 * category of work this instruction does.
 */

// === RV32I Base Integer Opcodes ===
export const OpcodeLoad = 0b0000011;
export const OpcodeStore = 0b0100011;
export const OpcodeBranch = 0b1100011;
export const OpcodeJAL = 0b1101111;
export const OpcodeJALR = 0b1100111;
export const OpcodeLUI = 0b0110111;
export const OpcodeAUIPC = 0b0010111;
export const OpcodeOpImm = 0b0010011;
export const OpcodeOp = 0b0110011;
export const OpcodeSystem = 0b1110011;

// === Funct3 for I-type immediate arithmetic ===
export const Funct3ADDI = 0;
export const Funct3SLTI = 2;
export const Funct3SLTIU = 3;
export const Funct3XORI = 4;
export const Funct3ORI = 6;
export const Funct3ANDI = 7;
export const Funct3SLLI = 1;
export const Funct3SRLI = 5; // also SRAI

// === Funct3 for R-type arithmetic ===
export const Funct3ADD = 0; // also SUB
export const Funct3SLL = 1;
export const Funct3SLT = 2;
export const Funct3SLTU = 3;
export const Funct3XOR = 4;
export const Funct3SRL = 5; // also SRA
export const Funct3OR = 6;
export const Funct3AND = 7;

// === Funct7 ===
export const Funct7Normal = 0x00;
export const Funct7Alt = 0x20;

// === Funct3 for load ===
export const Funct3LB = 0;
export const Funct3LH = 1;
export const Funct3LW = 2;
export const Funct3LBU = 4;
export const Funct3LHU = 5;

// === Funct3 for store ===
export const Funct3SB = 0;
export const Funct3SH = 1;
export const Funct3SW = 2;

// === Funct3 for branch ===
export const Funct3BEQ = 0;
export const Funct3BNE = 1;
export const Funct3BLT = 4;
export const Funct3BGE = 5;
export const Funct3BLTU = 6;
export const Funct3BGEU = 7;

// === Funct3 for system ===
export const Funct3PRIV = 0;
export const Funct3CSRRW = 1;
export const Funct3CSRRS = 2;
export const Funct3CSRRC = 3;

// === Funct7 for privileged instructions ===
export const Funct7ECALL = 0x00;
export const Funct7MRET = 0x18;
