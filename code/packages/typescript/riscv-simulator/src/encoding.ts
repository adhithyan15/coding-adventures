/**
 * Encoding helpers for constructing RISC-V machine code.
 *
 * Each helper constructs a 32-bit instruction word by placing fields
 * in their correct bit positions according to the RISC-V encoding format.
 */

import {
  OpcodeOpImm, OpcodeOp, OpcodeLoad, OpcodeStore, OpcodeBranch,
  OpcodeJAL, OpcodeJALR, OpcodeLUI, OpcodeAUIPC, OpcodeSystem,
  Funct3ADDI, Funct3SLTI, Funct3SLTIU, Funct3XORI, Funct3ORI,
  Funct3ANDI, Funct3SLLI, Funct3SRLI,
  Funct3ADD, Funct3SLL, Funct3SLT, Funct3SLTU, Funct3XOR,
  Funct3SRL, Funct3OR, Funct3AND,
  Funct7Normal, Funct7Alt, Funct7MRET,
  Funct3LB, Funct3LH, Funct3LW, Funct3LBU, Funct3LHU,
  Funct3SB, Funct3SH, Funct3SW,
  Funct3BEQ, Funct3BNE, Funct3BLT, Funct3BGE, Funct3BLTU, Funct3BGEU,
  Funct3CSRRW, Funct3CSRRS, Funct3CSRRC,
} from "./opcodes.js";

// === Internal helpers ===

function encodeIType(rd: number, rs1: number, imm: number, funct3: number, opcode: number): number {
  return (((imm & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode) >>> 0;
}

function encodeRType(rd: number, rs1: number, rs2: number, funct3: number, funct7: number): number {
  return ((funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OpcodeOp) >>> 0;
}

function encodeSType(rs2: number, rs1: number, imm: number, funct3: number): number {
  const immVal = imm & 0xfff;
  const immLow = immVal & 0x1f;
  const immHigh = (immVal >> 5) & 0x7f;
  return ((immHigh << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (immLow << 7) | OpcodeStore) >>> 0;
}

function encodeBType(rs1: number, rs2: number, offset: number, funct3: number): number {
  const imm = offset & 0x1ffe;
  const bit12 = (imm >> 12) & 0x1;
  const bit11 = (imm >> 11) & 0x1;
  const bits10_5 = (imm >> 5) & 0x3f;
  const bits4_1 = (imm >> 1) & 0xf;
  return ((bit12 << 31) | (bits10_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (bits4_1 << 8) | (bit11 << 7) | OpcodeBranch) >>> 0;
}

function encodeCSR(rd: number, csr: number, rs1: number, funct3: number): number {
  return (((csr & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OpcodeSystem) >>> 0;
}

// === I-type arithmetic encoders ===
export function encodeAddi(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3ADDI, OpcodeOpImm); }
export function encodeSlti(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3SLTI, OpcodeOpImm); }
export function encodeSltiu(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3SLTIU, OpcodeOpImm); }
export function encodeXori(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3XORI, OpcodeOpImm); }
export function encodeOri(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3ORI, OpcodeOpImm); }
export function encodeAndi(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3ANDI, OpcodeOpImm); }
export function encodeSlli(rd: number, rs1: number, shamt: number): number {
  return ((Funct7Normal << 25) | ((shamt & 0x1f) << 20) | (rs1 << 15) | (Funct3SLLI << 12) | (rd << 7) | OpcodeOpImm) >>> 0;
}
export function encodeSrli(rd: number, rs1: number, shamt: number): number {
  return ((Funct7Normal << 25) | ((shamt & 0x1f) << 20) | (rs1 << 15) | (Funct3SRLI << 12) | (rd << 7) | OpcodeOpImm) >>> 0;
}
export function encodeSrai(rd: number, rs1: number, shamt: number): number {
  return ((Funct7Alt << 25) | ((shamt & 0x1f) << 20) | (rs1 << 15) | (Funct3SRLI << 12) | (rd << 7) | OpcodeOpImm) >>> 0;
}

// === R-type arithmetic encoders ===
export function encodeAdd(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3ADD, Funct7Normal); }
export function encodeSub(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3ADD, Funct7Alt); }
export function encodeSll(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3SLL, Funct7Normal); }
export function encodeSlt(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3SLT, Funct7Normal); }
export function encodeSltu(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3SLTU, Funct7Normal); }
export function encodeXor(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3XOR, Funct7Normal); }
export function encodeSrl(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3SRL, Funct7Normal); }
export function encodeSra(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3SRL, Funct7Alt); }
export function encodeOr(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3OR, Funct7Normal); }
export function encodeAnd(rd: number, rs1: number, rs2: number): number { return encodeRType(rd, rs1, rs2, Funct3AND, Funct7Normal); }

// === Load encoders ===
export function encodeLb(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3LB, OpcodeLoad); }
export function encodeLh(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3LH, OpcodeLoad); }
export function encodeLw(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3LW, OpcodeLoad); }
export function encodeLbu(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3LBU, OpcodeLoad); }
export function encodeLhu(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, Funct3LHU, OpcodeLoad); }

// === Store encoders ===
export function encodeSb(rs2: number, rs1: number, imm: number): number { return encodeSType(rs2, rs1, imm, Funct3SB); }
export function encodeSh(rs2: number, rs1: number, imm: number): number { return encodeSType(rs2, rs1, imm, Funct3SH); }
export function encodeSw(rs2: number, rs1: number, imm: number): number { return encodeSType(rs2, rs1, imm, Funct3SW); }

// === Branch encoders ===
export function encodeBeq(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BEQ); }
export function encodeBne(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BNE); }
export function encodeBlt(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BLT); }
export function encodeBge(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BGE); }
export function encodeBltu(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BLTU); }
export function encodeBgeu(rs1: number, rs2: number, offset: number): number { return encodeBType(rs1, rs2, offset, Funct3BGEU); }

// === JAL / JALR / LUI / AUIPC ===
export function encodeJal(rd: number, offset: number): number {
  const imm = offset & 0x1ffffe;
  const bit20 = (imm >> 20) & 0x1;
  const bits10_1 = (imm >> 1) & 0x3ff;
  const bit11 = (imm >> 11) & 0x1;
  const bits19_12 = (imm >> 12) & 0xff;
  return ((bit20 << 31) | (bits10_1 << 21) | (bit11 << 20) | (bits19_12 << 12) | (rd << 7) | OpcodeJAL) >>> 0;
}

export function encodeJalr(rd: number, rs1: number, imm: number): number { return encodeIType(rd, rs1, imm, 0, OpcodeJALR); }
export function encodeLui(rd: number, imm: number): number { return (((imm & 0xfffff) << 12) | (rd << 7) | OpcodeLUI) >>> 0; }
export function encodeAuipc(rd: number, imm: number): number { return (((imm & 0xfffff) << 12) | (rd << 7) | OpcodeAUIPC) >>> 0; }

// === System instruction encoders ===
export function encodeEcall(): number { return OpcodeSystem; }
export function encodeMret(): number { return ((Funct7MRET << 25) | (0b00010 << 20) | OpcodeSystem) >>> 0; }
export function encodeCsrrw(rd: number, csr: number, rs1: number): number { return encodeCSR(rd, csr, rs1, Funct3CSRRW); }
export function encodeCsrrs(rd: number, csr: number, rs1: number): number { return encodeCSR(rd, csr, rs1, Funct3CSRRS); }
export function encodeCsrrc(rd: number, csr: number, rs1: number): number { return encodeCSR(rd, csr, rs1, Funct3CSRRC); }

/**
 * Convert a list of 32-bit instruction words to bytes (little-endian).
 */
export function assemble(instructions: number[]): number[] {
  const result: number[] = [];
  for (const instr of instructions) {
    const masked = (instr & 0xffffffff) >>> 0;
    result.push(masked & 0xff);
    result.push((masked >>> 8) & 0xff);
    result.push((masked >>> 16) & 0xff);
    result.push((masked >>> 24) & 0xff);
  }
  return result;
}
