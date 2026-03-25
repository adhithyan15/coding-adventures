/**
 * ==========================================================================
 * Type Definitions for ARM1 Web Simulator
 * ==========================================================================
 *
 * These types extend the base types from @coding-adventures/arm1-simulator
 * with additional context needed for rich visualization.
 */

import type { Trace, DecodedInstruction, Flags } from "@coding-adventures/arm1-simulator";

// ==========================================================================
// Barrel Shifter Detail
// ==========================================================================
//
// The ARM1's barrel shifter processes Operand2 before the ALU sees it.
// It can shift or rotate a register value by 0–31 positions in a single
// cycle — this is what "shift for free" means.
//
// For data processing instructions with a register Operand2, we capture:
//   input     — the raw Rm register value
//   shiftType — LSL/LSR/ASR/ROR/RRX
//   amount    — how many positions to shift
//   output    — the value after shifting
//   carryOut  — the carry output from the shifter (may update C flag)

export type ShiftTypeName = "LSL" | "LSR" | "ASR" | "ROR" | "RRX" | "none";

export interface ShiftDetail {
  /** The input value (Rm register, or imm8 for immediate form). */
  input: number;
  /** The shift operation applied. */
  shiftType: ShiftTypeName;
  /** Number of positions (0 for RRX which shifts by exactly 1). */
  amount: number;
  /** The output value after applying the shift. */
  output: number;
  /** Carry-out from the barrel shifter. */
  carryOut: boolean;
  /** True when LSL #0 — the shift is a pass-through with no movement. */
  isNop: boolean;
}

// ==========================================================================
// Extended Trace
// ==========================================================================
//
// The base Trace captures register/flag before-and-after snapshots.
// ExtendedTrace adds the decoded instruction structure and barrel shift
// detail, which the visualization components need.

export interface ExtendedTrace extends Trace {
  /** Decoded instruction fields (opcode, registers, shift type, etc.). */
  decoded: DecodedInstruction;
  /** Barrel shift details — only present for data processing with reg Operand2. */
  shift?: ShiftDetail;
  /** 1-based execution counter (useful for the Trace log). */
  cycle: number;
}

// ==========================================================================
// Pipeline State
// ==========================================================================
//
// The ARM1 has a 3-stage pipeline: Fetch → Decode → Execute.
// Because the pipeline runs in parallel, when instruction N is executing,
// instruction N+1 is being decoded, and N+2 is being fetched.
//
// This creates the famous ARM "PC = PC+8 during execution" behaviour:
// when the Execute stage runs, the Fetch stage has already advanced PC
// by 8 bytes (2 instructions ahead).

export interface PipelineStage {
  /** Program counter of the instruction at this stage. */
  pc: number;
  /** Raw 32-bit instruction word. */
  raw: number;
  /** Disassembled mnemonic (or "—" if stalled). */
  mnemonic: string;
  /** True when this stage contains a valid instruction. */
  valid: boolean;
}

export interface PipelineState {
  /** Stage 1: Fetch — reading the next instruction from memory. */
  fetch: PipelineStage;
  /** Stage 2: Decode — extracting instruction fields. */
  decode: PipelineStage;
  /** Stage 3: Execute — performing the operation. */
  execute: PipelineStage;
}

// ==========================================================================
// Simulator State
// ==========================================================================
//
// Top-level snapshot of everything the React components need to render.

export interface SimulatorState {
  /** All 16 visible registers (R0-R15), unsigned 32-bit. */
  registers: number[];
  /** Condition flags extracted from R15 bits 31:28. */
  flags: Flags;
  /** I (IRQ disable) flag from R15 bit 27. */
  irqDisabled: boolean;
  /** F (FIQ disable) flag from R15 bit 26. */
  fiqDisabled: boolean;
  /** Processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC). */
  mode: number;
  /** Program counter — address of next instruction to fetch. */
  pc: number;
  /** Raw R15 value (PC + status bits). */
  r15: number;
  /** Has the CPU halted (executed SWI #0x123456)? */
  halted: boolean;
  /** Execution history, most-recent-last, capped at 100 entries. */
  traces: ExtendedTrace[];
  /** 3-stage pipeline visualization. */
  pipeline: PipelineState;
  /** Total instructions executed since last reset. */
  totalCycles: number;
  /** Name of the currently loaded program. */
  programName: string;
}
