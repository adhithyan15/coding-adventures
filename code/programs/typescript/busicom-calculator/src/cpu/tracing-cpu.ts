/**
 * Tracing CPU — instrumented wrapper around Intel4004GateLevel.
 *
 * === Why a wrapper? ===
 *
 * The base Intel4004GateLevel class now provides rich `GateTrace` objects
 * from `step()` — including decoded instruction, ALU trace with per-adder
 * snapshots, and memory access details. The wrapper adds:
 *
 *   - Full CPU state snapshots after each instruction
 *   - Trace history with configurable cap
 *   - `runUntilIdle()` for the calculator's key scan loop
 *
 * === V2 simplification ===
 *
 * In V1, this wrapper had to replay the adder chain independently to
 * reconstruct ALU detail. V2 moved that capability into the CPU package
 * itself (via `rippleCarryAdderTraced` in the arithmetic package and
 * native trace emission in GateALU). The replay code is gone.
 */

import {
  Intel4004GateLevel,
} from "@coding-adventures/intel4004-gatelevel";
import type {
  DetailedTrace,
  CpuSnapshot,
} from "./types.js";

/**
 * Take a snapshot of the CPU's current state.
 *
 * This captures everything needed by the CPU view layer: registers, flags,
 * PC, hardware stack, and full RAM contents.
 */
function snapshot(cpu: Intel4004GateLevel): CpuSnapshot {
  return {
    accumulator: cpu.accumulator,
    registers: [...cpu.registers],
    carry: cpu.carry,
    pc: cpu.pc,
    hwStack: [...cpu.hwStack],
    ramBank: cpu.ramBank,
    ramData: cpu.ramData.map((bank) =>
      bank.map((reg) => [...reg]),
    ),
    ramOutput: [...cpu.ramOutput],
  };
}

/**
 * Instrumented wrapper around Intel4004GateLevel.
 *
 * Provides the same interface as the base CPU but produces DetailedTrace
 * objects with full visualization data.
 */
export class TracingCPU {
  /** The underlying gate-level CPU. */
  private _cpu: Intel4004GateLevel;

  /** History of all executed instructions with full trace data. */
  private _traceHistory: DetailedTrace[] = [];

  /** Maximum number of traces to keep in history (prevents memory issues). */
  private _maxHistory: number;

  constructor(maxHistory: number = 1000) {
    this._cpu = new Intel4004GateLevel();
    this._maxHistory = maxHistory;
  }

  // --------------------------------------------------------------------------
  // Delegated getters — expose the base CPU's state
  // --------------------------------------------------------------------------

  get accumulator(): number {
    return this._cpu.accumulator;
  }
  get registers(): number[] {
    return this._cpu.registers;
  }
  get carry(): boolean {
    return this._cpu.carry;
  }
  get pc(): number {
    return this._cpu.pc;
  }
  get halted(): boolean {
    return this._cpu.halted;
  }
  get hwStack(): number[] {
    return this._cpu.hwStack;
  }
  get ramData(): number[][][] {
    return this._cpu.ramData;
  }
  get ramStatus(): number[][][] {
    return this._cpu.ramStatus;
  }
  get ramBank(): number {
    return this._cpu.ramBank;
  }
  get ramOutput(): number[] {
    return this._cpu.ramOutput;
  }
  get romPort(): number {
    return this._cpu.romPort;
  }
  set romPort(value: number) {
    this._cpu.romPort = value;
  }

  /** Full trace history. */
  get traceHistory(): readonly DetailedTrace[] {
    return this._traceHistory;
  }

  /** Most recent trace, or undefined if no instructions executed yet. */
  get lastTrace(): DetailedTrace | undefined {
    return this._traceHistory[this._traceHistory.length - 1];
  }

  // --------------------------------------------------------------------------
  // Public API
  // --------------------------------------------------------------------------

  /** Load a program into ROM. */
  loadProgram(program: Uint8Array): void {
    this._cpu.loadProgram(program);
    this._traceHistory = [];
  }

  /** Reset CPU state and clear trace history. */
  reset(): void {
    this._cpu.reset();
    this._traceHistory = [];
  }

  /**
   * Execute one instruction and return a DetailedTrace.
   *
   * The base CPU now provides decoded, aluTrace, and memoryAccess natively.
   * We just add the CPU state snapshot.
   */
  step(): DetailedTrace {
    const baseTrace = this._cpu.step();

    const trace: DetailedTrace = {
      ...baseTrace,
      snapshot: snapshot(this._cpu),
    };

    // Add to history (with cap)
    this._traceHistory.push(trace);
    if (this._traceHistory.length > this._maxHistory) {
      this._traceHistory.shift();
    }

    return trace;
  }

  /**
   * Execute instructions until the CPU reads from the ROM port with no
   * pending key, or until maxSteps is reached.
   *
   * This is the main execution mode for the calculator: run the CPU until
   * it's idle in the keyboard scan loop, waiting for the next key press.
   *
   * @param maxSteps - Safety limit to prevent infinite loops.
   * @returns Number of instructions executed.
   */
  runUntilIdle(maxSteps: number = 10000): number {
    let steps = 0;
    while (steps < maxSteps && !this._cpu.halted) {
      this.step();
      steps++;
      // If the CPU just executed RDR and the result was 0 (no key),
      // it's idle in the scan loop
      const last = this.lastTrace;
      if (last && last.decoded.isIo && (last.raw & 0x0f) === 0x0a) {
        // RDR instruction — check if accumulator is 0 (no key pressed)
        if (last.snapshot.accumulator === 0) {
          break;
        }
      }
    }
    return steps;
  }
}
