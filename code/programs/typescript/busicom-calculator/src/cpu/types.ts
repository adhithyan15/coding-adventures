/**
 * Type definitions for the enhanced tracing CPU.
 *
 * === V2 simplification ===
 *
 * In V1, ALUDetail and FullAdderState were defined here and populated by
 * replaying the adder chain in TracingCPU. In V2, the intel4004-gatelevel
 * package emits ALU traces natively via ALUTrace. The DetailedTrace type
 * now extends GateTrace (which already includes decoded, aluTrace, and
 * memoryAccess) and adds only the CPU state snapshot.
 *
 * The old ALUDetail and FullAdderState types are re-exported from the
 * package for backwards compatibility with visualization components.
 */

import type { GateTrace, ALUTrace } from "@coding-adventures/intel4004-gatelevel";
import type { FullAdderSnapshot } from "@coding-adventures/arithmetic";

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
 * Extended trace with full visualization data.
 *
 * GateTrace now provides decoded instruction, ALU trace, and memory access
 * natively. DetailedTrace adds the CPU state snapshot for the CPU view.
 */
export interface DetailedTrace extends GateTrace {
  /** CPU state snapshot after this instruction executed. */
  snapshot: CpuSnapshot;
}

// Re-export ALU types for use by visualization components
export type { ALUTrace, FullAdderSnapshot };

/**
 * Type alias for backwards compatibility.
 * Components that referenced ALUDetail can use ALUTrace instead.
 */
export type ALUDetail = ALUTrace;

/**
 * Type alias for backwards compatibility.
 * Components that referenced FullAdderState can use FullAdderSnapshot instead.
 */
export type FullAdderState = FullAdderSnapshot;

/**
 * Type alias for backwards compatibility.
 * MemoryAccess is now part of GateTrace from the package.
 */
export type { MemoryAccess } from "@coding-adventures/intel4004-gatelevel";
