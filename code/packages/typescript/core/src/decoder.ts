/**
 * ISADecoder interface and MockDecoder -- the bridge between the Core and
 * any instruction set architecture.
 *
 * The Core knows how to move instructions through a pipeline, predict
 * branches, detect hazards, and access caches. But it does NOT know what
 * any instruction means. That is the ISA decoder's job.
 *
 * Our ISADecoder interface is that well-defined interface. Any ISA
 * (ARM, RISC-V, x86, or a custom teaching ISA) can implement it and
 * immediately run on any Core configuration.
 */

import type { PipelineToken } from "@coding-adventures/cpu-pipeline";
import type { RegisterFile } from "./register-file.js";

// =========================================================================
// ISADecoder interface
// =========================================================================

/**
 * ISADecoder is the protocol that any instruction set architecture must
 * implement to plug into a Core.
 *
 * The decoder has exactly two responsibilities:
 *
 *   1. decode: turn raw instruction bits into a structured PipelineToken
 *   2. execute: perform the actual computation (ALU, branch resolution)
 *
 * These map directly to the ID and EX stages of the pipeline.
 */
export interface ISADecoder {
  /**
   * Decode raw instruction bits into a structured PipelineToken.
   *
   * Fills in: opcode, rs1, rs2, rd, immediate, control signals.
   */
  decode(rawInstruction: number, token: PipelineToken): PipelineToken;

  /**
   * Execute the ALU operation for a decoded instruction.
   *
   * Fills in: aluResult, branchTaken, branchTarget, writeData.
   * The RegisterFile is passed so the executor can read source register values.
   */
  execute(token: PipelineToken, regFile: RegisterFile): PipelineToken;

  /**
   * Returns the size of one instruction in bytes.
   *
   *     ARM (A64): 4 bytes
   *     RISC-V:    4 bytes (base ISA) or 2 bytes (compressed)
   *     x86:       variable (1-15 bytes)
   */
  instructionSize(): number;
}

// =========================================================================
// MockDecoder -- a simple decoder for testing
// =========================================================================

/**
 * MockDecoder is a minimal ISA decoder for testing purposes.
 *
 * It supports a handful of instructions encoded in a simple format:
 *
 *     Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH, 5=HALT,
 *                         6=ADDI, 7=SUB)
 *     Bits 23-20: Rd  (destination register)
 *     Bits 19-16: Rs1 (first source register)
 *     Bits 15-12: Rs2 (second source register)
 *     Bits 11-0:  immediate (12-bit, sign-extended)
 *
 * This encoding does not match any real ISA. It exists solely to exercise
 * the Core's pipeline, hazard detection, branch prediction, and caches.
 */
export class MockDecoder implements ISADecoder {
  instructionSize(): number {
    return 4;
  }

  decode(raw: number, token: PipelineToken): PipelineToken {
    const opcode = (raw >> 24) & 0xff;
    const rd = (raw >> 20) & 0x0f;
    const rs1 = (raw >> 16) & 0x0f;
    const rs2 = (raw >> 12) & 0x0f;
    let imm = raw & 0xfff;

    // Sign-extend the 12-bit immediate.
    if (imm & 0x800) {
      imm |= ~0xfff;
    }

    switch (opcode) {
      case 0x00: // NOP
        token.opcode = "NOP";
        token.rd = -1;
        token.rs1 = -1;
        token.rs2 = -1;
        break;

      case 0x01: // ADD Rd, Rs1, Rs2
        token.opcode = "ADD";
        token.rd = rd;
        token.rs1 = rs1;
        token.rs2 = rs2;
        token.regWrite = true;
        break;

      case 0x02: // LOAD Rd, [Rs1 + imm]
        token.opcode = "LOAD";
        token.rd = rd;
        token.rs1 = rs1;
        token.rs2 = -1;
        token.immediate = imm;
        token.regWrite = true;
        token.memRead = true;
        break;

      case 0x03: // STORE [Rs1 + imm], Rs2
        token.opcode = "STORE";
        token.rd = -1;
        token.rs1 = rs1;
        token.rs2 = rs2;
        token.immediate = imm;
        token.memWrite = true;
        break;

      case 0x04: // BRANCH Rs1, Rs2, imm
        token.opcode = "BRANCH";
        token.rd = -1;
        token.rs1 = rs1;
        token.rs2 = rs2;
        token.immediate = imm;
        token.isBranch = true;
        break;

      case 0x05: // HALT
        token.opcode = "HALT";
        token.rd = -1;
        token.rs1 = -1;
        token.rs2 = -1;
        token.isHalt = true;
        break;

      case 0x06: // ADDI Rd, Rs1, imm
        token.opcode = "ADDI";
        token.rd = rd;
        token.rs1 = rs1;
        token.rs2 = -1;
        token.immediate = imm;
        token.regWrite = true;
        break;

      case 0x07: // SUB Rd, Rs1, Rs2
        token.opcode = "SUB";
        token.rd = rd;
        token.rs1 = rs1;
        token.rs2 = rs2;
        token.regWrite = true;
        break;

      default: // Unknown -- treat as NOP
        token.opcode = "NOP";
        token.rd = -1;
        token.rs1 = -1;
        token.rs2 = -1;
        break;
    }

    return token;
  }

  execute(token: PipelineToken, regFile: RegisterFile): PipelineToken {
    let rs1Val = 0;
    let rs2Val = 0;
    if (token.rs1 >= 0) rs1Val = regFile.read(token.rs1);
    if (token.rs2 >= 0) rs2Val = regFile.read(token.rs2);

    switch (token.opcode) {
      case "ADD":
        token.aluResult = rs1Val + rs2Val;
        token.writeData = token.aluResult;
        break;

      case "SUB":
        token.aluResult = rs1Val - rs2Val;
        token.writeData = token.aluResult;
        break;

      case "ADDI":
        token.aluResult = rs1Val + token.immediate;
        token.writeData = token.aluResult;
        break;

      case "LOAD":
        token.aluResult = rs1Val + token.immediate;
        break;

      case "STORE":
        token.aluResult = rs1Val + token.immediate;
        token.writeData = rs2Val;
        break;

      case "BRANCH": {
        const taken = rs1Val === rs2Val;
        token.branchTaken = taken;
        const target = token.pc + token.immediate * 4;
        token.branchTarget = target;
        token.aluResult = taken ? target : token.pc + 4;
        break;
      }

      case "NOP":
      case "HALT":
        break;

      default:
        break;
    }

    return token;
  }
}

// =========================================================================
// Instruction encoding helpers
// =========================================================================

export function encodeNOP(): number {
  return 0x00 << 24;
}

export function encodeADD(rd: number, rs1: number, rs2: number): number {
  return (0x01 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12);
}

export function encodeSUB(rd: number, rs1: number, rs2: number): number {
  return (0x07 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12);
}

export function encodeADDI(rd: number, rs1: number, imm: number): number {
  return (0x06 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xfff);
}

export function encodeLOAD(rd: number, rs1: number, imm: number): number {
  return (0x02 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xfff);
}

export function encodeSTORE(rs1: number, rs2: number, imm: number): number {
  return (0x03 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xfff);
}

export function encodeBRANCH(rs1: number, rs2: number, imm: number): number {
  return (0x04 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xfff);
}

export function encodeHALT(): number {
  return 0x05 << 24;
}

/**
 * Converts a sequence of raw instruction ints into a byte array
 * suitable for loadProgram. Each instruction is 4 bytes, little-endian.
 */
export function encodeProgram(...instructions: number[]): Uint8Array {
  const result = new Uint8Array(instructions.length * 4);
  for (let i = 0; i < instructions.length; i++) {
    const instr = instructions[i];
    const offset = i * 4;
    result[offset] = instr & 0xff;
    result[offset + 1] = (instr >> 8) & 0xff;
    result[offset + 2] = (instr >> 16) & 0xff;
    result[offset + 3] = (instr >> 24) & 0xff;
  }
  return result;
}
