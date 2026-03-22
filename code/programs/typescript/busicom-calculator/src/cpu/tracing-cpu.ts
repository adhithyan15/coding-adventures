/**
 * Tracing CPU — instrumented wrapper around Intel4004GateLevel.
 *
 * === Why a wrapper? ===
 *
 * The base Intel4004GateLevel class provides a `step()` method that returns
 * a `GateTrace` with basic before/after state. But our visualization layers
 * need much more:
 *
 *   - Layer 2 (CPU View): Full state snapshot after each instruction
 *   - Layer 3 (ALU View): Per-full-adder intermediate values
 *   - Layer 4 (Gate View): Decoded instruction with control signals
 *   - Layer 5 (Transistor View): Which gates were active
 *
 * The TracingCPU wraps the base CPU and produces `DetailedTrace` objects by:
 *   1. Capturing CPU state before/after each step
 *   2. Decoding the instruction to identify ALU operations
 *   3. Replaying the ALU operation through halfAdder/fullAdder to capture
 *      per-adder intermediate values
 *
 * This approach avoids modifying the core library — all enhancement happens
 * in this wrapper layer.
 *
 * === ALU Replay ===
 *
 * When the instruction is ADD, SUB, INC, DAC, IAC, or similar, we know the
 * operands from the pre-step state and the decoded instruction. We replay
 * the operation through `fullAdder()` calls to capture each adder's inputs,
 * sum, and carry. This gives the ALU visualization layer its data without
 * the base CPU needing to expose internal wires.
 */

import {
  Intel4004GateLevel,
  decode,
  intToBits,
} from "@coding-adventures/intel4004-gatelevel";
import { fullAdder } from "@coding-adventures/arithmetic";
import { NOT, type Bit } from "@coding-adventures/logic-gates";
import type {
  DetailedTrace,
  CpuSnapshot,
  ALUDetail,
  FullAdderState,
  MemoryAccess,
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
 * Replay a 4-bit addition through the full adder chain.
 *
 * This produces the per-adder intermediate state that the ALU visualization
 * needs. We call `fullAdder()` from the arithmetic package — the same
 * function the actual CPU ALU uses — so the values are guaranteed to match.
 *
 * @param aBits - 4-bit input A (LSB first)
 * @param bBits - 4-bit input B (LSB first)
 * @param carryIn - Initial carry into the LSB adder
 * @returns Array of 4 FullAdderState objects + final carry out
 */
function replayAdderChain(
  aBits: Bit[],
  bBits: Bit[],
  carryIn: Bit,
): { adders: FullAdderState[]; carryOut: Bit } {
  const adders: FullAdderState[] = [];
  let carry = carryIn;

  for (let i = 0; i < 4; i++) {
    const a = aBits[i]!;
    const b = bBits[i]!;
    const [sum, cOut] = fullAdder(a, b, carry);
    adders.push({ a, b, cIn: carry, sum, cOut });
    carry = cOut;
  }

  return { adders, carryOut: carry };
}

/**
 * Complement (NOT) each bit in a 4-bit array.
 *
 * Used to show the B input transformation for SUB operations:
 * the ALU computes A + NOT(B) + 1 to perform subtraction.
 */
function complementBits(bits: Bit[]): Bit[] {
  return bits.map((b) => NOT(b));
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
   * This is the core method. It:
   *   1. Captures pre-execution state
   *   2. Calls the base CPU's step()
   *   3. Decodes the instruction
   *   4. Replays ALU operations if applicable
   *   5. Captures post-execution state
   *   6. Returns the combined DetailedTrace
   */
  step(): DetailedTrace {
    // Capture pre-execution state for ALU replay
    const preAcc = this._cpu.accumulator;
    const preCarry = this._cpu.carry;
    const preRegisters = [...this._cpu.registers];

    // Execute one instruction
    const baseTrace = this._cpu.step();

    // Decode the instruction
    const decoded = decode(baseTrace.raw, baseTrace.raw2);

    // Build the detailed trace
    const trace: DetailedTrace = {
      ...baseTrace,
      decoded,
      snapshot: snapshot(this._cpu),
    };

    // Reconstruct ALU detail if this was an ALU instruction
    const aluDetail = this._reconstructALU(
      decoded,
      preAcc,
      preCarry,
      preRegisters,
    );
    if (aluDetail) {
      trace.aluDetail = aluDetail;
    }

    // Detect memory access
    const memAccess = this._detectMemoryAccess(decoded, preRegisters);
    if (memAccess) {
      trace.memoryAccess = memAccess;
    }

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

  // --------------------------------------------------------------------------
  // Private helpers
  // --------------------------------------------------------------------------

  /**
   * Reconstruct ALU detail by replaying the operation through the adder chain.
   *
   * We determine operands from pre-execution state and the decoded instruction,
   * then call fullAdder() to get per-adder intermediate values.
   */
  private _reconstructALU(
    decoded: ReturnType<typeof decode>,
    preAcc: number,
    preCarry: boolean,
    preRegisters: number[],
  ): ALUDetail | undefined {
    // ADD Rn: acc = acc + Rn
    if (decoded.isAdd) {
      const regVal = preRegisters[decoded.regIndex]!;
      return this._replayAdd(preAcc, regVal, 0, "add");
    }

    // SUB Rn: acc = acc + NOT(Rn) + borrow
    if (decoded.isSub) {
      const regVal = preRegisters[decoded.regIndex]!;
      // 4004 SUB uses complement-add: A + NOT(B) + borrow
      // borrow = carry ? 0 : 1 (inverted carry semantics)
      const borrowIn: Bit = preCarry ? 0 : 1;
      return this._replaySub(preAcc, regVal, borrowIn);
    }

    // IAC: acc = acc + 1
    if (decoded.isAccum && decoded.lower === 0x2) {
      return this._replayAdd(preAcc, 1, 0, "inc");
    }

    // DAC: acc = acc - 1
    if (decoded.isAccum && decoded.lower === 0x8) {
      return this._replaySub(preAcc, 1, 1);
    }

    // CMA: acc = NOT(acc)
    if (decoded.isAccum && decoded.lower === 0x4) {
      const inputA = intToBits(preAcc, 4) as Bit[];
      const result = complementBits(inputA);
      return {
        operation: "complement",
        inputA,
        inputB: [0, 0, 0, 0],
        carryIn: 0,
        adders: [],
        result,
        carryOut: 0,
      };
    }

    // DAA: decimal adjust accumulator
    if (decoded.isAccum && decoded.lower === 0xB) {
      // DAA adds 6 if acc > 9 or carry was set
      if (preAcc > 9 || preCarry) {
        return this._replayAdd(preAcc, 6, 0, "daa");
      }
    }

    // INC Rn: Rn = Rn + 1 (doesn't go through accumulator ALU,
    // but we can still show the adder chain)
    if (decoded.isInc) {
      const regVal = preRegisters[decoded.regIndex]!;
      return this._replayAdd(regVal, 1, 0, "inc");
    }

    return undefined;
  }

  /**
   * Replay an addition through the full adder chain.
   */
  private _replayAdd(
    a: number,
    b: number,
    carryIn: Bit,
    operation: ALUDetail["operation"],
  ): ALUDetail {
    const inputA = intToBits(a, 4) as Bit[];
    const inputB = intToBits(b, 4) as Bit[];
    const { adders, carryOut } = replayAdderChain(inputA, inputB, carryIn);
    const result = adders.map((fa) => fa.sum);

    return {
      operation,
      inputA,
      inputB,
      carryIn,
      adders,
      result,
      carryOut,
    };
  }

  /**
   * Replay a subtraction: A + NOT(B) + borrowIn.
   */
  private _replaySub(
    a: number,
    b: number,
    borrowIn: Bit,
  ): ALUDetail {
    const inputA = intToBits(a, 4) as Bit[];
    const rawB = intToBits(b, 4) as Bit[];
    const inputB = complementBits(rawB);
    const { adders, carryOut } = replayAdderChain(inputA, inputB, borrowIn);
    const result = adders.map((fa) => fa.sum);

    return {
      operation: "sub",
      inputA,
      inputB,
      carryIn: borrowIn,
      adders,
      result,
      carryOut,
    };
  }

  /**
   * Detect memory access from the decoded instruction.
   */
  private _detectMemoryAccess(
    decoded: ReturnType<typeof decode>,
    preRegisters: number[],
  ): MemoryAccess | undefined {
    // LD Rn: read register
    if (decoded.isLd) {
      return {
        type: "reg_read",
        address: decoded.regIndex,
        value: preRegisters[decoded.regIndex]!,
      };
    }

    // XCH Rn: read+write register
    if (decoded.isXch) {
      return {
        type: "reg_write",
        address: decoded.regIndex,
        value: preRegisters[decoded.regIndex]!,
      };
    }

    // INC Rn: read+write register
    if (decoded.isInc) {
      return {
        type: "reg_write",
        address: decoded.regIndex,
        value: (preRegisters[decoded.regIndex]! + 1) & 0xf,
      };
    }

    // I/O instructions for RAM
    if (decoded.isIo) {
      const subOp = decoded.lower;
      if (subOp === 0x0) {
        // WRM
        return { type: "ram_write", address: 0, value: 0 };
      }
      if (subOp === 0x9) {
        // RDM
        return { type: "ram_read", address: 0, value: 0 };
      }
    }

    return undefined;
  }
}
