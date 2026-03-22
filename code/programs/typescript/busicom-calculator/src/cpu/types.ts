/**
 * Type definitions for the enhanced tracing CPU.
 *
 * These types extend the base GateTrace from the intel4004-gatelevel package
 * to capture additional detail needed by the visualization layers:
 *
 *   - DecodedInstruction: which instruction family, operands
 *   - ALU detail: inputs, carry chain, per-adder intermediate values
 *   - Memory access: which register/RAM was read or written
 *   - CPU snapshot: full state after execution
 *
 * The ALU detail is reconstructed by *replaying* the adder chain independently,
 * using the instruction's operands. This avoids modifying the core CPU library.
 */

import type { Bit } from "@coding-adventures/logic-gates";
import type { GateTrace, DecodedInstruction } from "@coding-adventures/intel4004-gatelevel";

/**
 * Snapshot of the CPU's full state at a point in time.
 *
 * Captured after each instruction executes so the CPU view layer
 * can display registers, PC, flags, and RAM.
 */
export interface CpuSnapshot {
  /** 4-bit accumulator value (0-15). */
  accumulator: number;

  /** All 16 general-purpose 4-bit registers. */
  registers: number[];

  /** Carry flag. */
  carry: boolean;

  /** 12-bit program counter (next instruction address). */
  pc: number;

  /** Hardware call stack (3 × 12-bit return addresses). */
  hwStack: number[];

  /** Currently selected RAM bank (0-3). */
  ramBank: number;

  /**
   * Full RAM contents: ramData[bank][register][character].
   * 4 banks × 4 registers × 16 characters, each a 4-bit nibble.
   */
  ramData: number[][][];

  /** RAM output ports (one per bank, 4-bit each). */
  ramOutput: number[];
}

/**
 * Per-adder intermediate state in the ripple carry chain.
 *
 * The 4004's ALU is a 4-bit ripple carry adder — four full adders
 * chained from LSB to MSB. Each full adder takes two input bits
 * and a carry-in, producing a sum bit and carry-out.
 *
 * ```
 *      A3 B3      A2 B2      A1 B1      A0 B0
 *       |  |       |  |       |  |       |  |
 *     +----+     +----+     +----+     +----+
 *     | FA |<----| FA |<----| FA |<----| FA |<-- carryIn
 *     +----+     +----+     +----+     +----+
 *       |           |          |          |
 *      S3          S2         S1         S0
 * ```
 */
export interface FullAdderState {
  /** Input bit from operand A. */
  a: Bit;
  /** Input bit from operand B (possibly complemented for SUB). */
  b: Bit;
  /** Carry in from the previous adder (or initial carry). */
  cIn: Bit;
  /** Sum output bit. */
  sum: Bit;
  /** Carry output to the next adder. */
  cOut: Bit;
}

/**
 * Detailed ALU operation trace.
 *
 * Captured when the current instruction involves the ALU:
 * ADD, SUB, INC (ISZ), DAC, IAC, DAA, TCS, CMA, etc.
 */
export interface ALUDetail {
  /** Which ALU operation was performed. */
  operation: "add" | "sub" | "inc" | "dec" | "complement" | "daa";

  /** Input A bits (4-bit, LSB first). */
  inputA: Bit[];

  /** Input B bits (4-bit, LSB first). May be complemented for SUB. */
  inputB: Bit[];

  /** Carry/borrow input to the LSB full adder. */
  carryIn: Bit;

  /** Per-adder intermediate state for the 4-bit ripple carry chain. */
  adders: FullAdderState[];

  /** Result bits (4-bit, LSB first). */
  result: Bit[];

  /** Final carry/borrow output from the MSB full adder. */
  carryOut: Bit;
}

/**
 * Memory access detail.
 *
 * Captured when the instruction reads or writes a register or RAM location.
 */
export interface MemoryAccess {
  /** Type of access. */
  type: "reg_read" | "reg_write" | "ram_read" | "ram_write";

  /** Register index (0-15) or RAM address components. */
  address: number;

  /** Value read or written (4-bit). */
  value: number;
}

/**
 * Extended trace with full visualization data.
 *
 * This is the primary data structure consumed by all five visualization
 * layers. It extends the base GateTrace with decoded instruction fields,
 * ALU intermediate values, memory access details, and a full CPU snapshot.
 */
export interface DetailedTrace extends GateTrace {
  /** Decoded instruction with all control signals. */
  decoded: DecodedInstruction;

  /** CPU state snapshot after this instruction executed. */
  snapshot: CpuSnapshot;

  /** ALU detail, present when the instruction uses the ALU. */
  aluDetail?: ALUDetail;

  /** Memory access, present when the instruction reads/writes registers or RAM. */
  memoryAccess?: MemoryAccess;
}
