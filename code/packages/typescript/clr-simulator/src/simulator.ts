/**
 * CLR IL Simulator -- Microsoft's answer to the JVM.
 *
 * === What is the CLR? ===
 *
 * The Common Language Runtime (CLR) is the virtual machine at the heart of
 * Microsoft's .NET framework, first released in 2002 with .NET 1.0. Just as
 * the JVM runs Java bytecode, the CLR runs Common Intermediate Language (CIL),
 * also called MSIL (Microsoft Intermediate Language). C#, F#, VB.NET, and even
 * PowerShell all compile down to CIL bytecode, which the CLR then executes.
 *
 * === CLR vs JVM: Two philosophies of stack machines ===
 *
 * Both the JVM and CLR are stack-based virtual machines, but they take different
 * approaches to type information in their instruction sets:
 *
 *     JVM approach -- type in the opcode:
 *         iconst_1    <-- "i" means int32
 *         iadd        <-- "i" means int32 addition
 *
 *     CLR approach -- type inferred from the stack:
 *         ldc.i4.1    <-- push int32 constant 1
 *         add         <-- type inferred! works for int32, int64, float...
 *
 * === Short encodings: CLR's optimization trick ===
 *
 *     JVM:  iconst_0 through iconst_5   (6 shortcuts, 0-5)
 *     CLR:  ldc.i4.0 through ldc.i4.8   (9 shortcuts, 0-8!)
 *
 * === The 0xFE prefix: Two-byte opcodes ===
 *
 * The CLR has more than 256 instructions, so it uses a prefix byte (0xFE)
 * to create a second "page" of opcodes. The comparison instructions live
 * in this extended space:
 *
 *     ceq  = 0xFE 0x01   (compare equal)
 *     cgt  = 0xFE 0x02   (compare greater than)
 *     clt  = 0xFE 0x04   (compare less than)
 *
 * === Branch offset convention ===
 *
 * CLR branch offsets are relative to the NEXT instruction's PC, not the current
 * one. So if br.s is at PC=10 (consuming 2 bytes), and the offset is +3:
 *     target = (10 + 2) + 3 = 15
 */

import {
  ExecutionResult,
  StepTrace,
} from "@coding-adventures/simulator-protocol";

import type { CLRState } from "./state.js";

// ---------------------------------------------------------------------------
// Opcode definitions -- real CLR IL opcode values
// ---------------------------------------------------------------------------

export const CLROpcode = {
  NOP: 0x00,
  LDNULL: 0x01,

  LDLOC_0: 0x06, LDLOC_1: 0x07, LDLOC_2: 0x08, LDLOC_3: 0x09,
  STLOC_0: 0x0a, STLOC_1: 0x0b, STLOC_2: 0x0c, STLOC_3: 0x0d,

  LDLOC_S: 0x11,
  STLOC_S: 0x13,

  LDC_I4_0: 0x16, LDC_I4_1: 0x17, LDC_I4_2: 0x18, LDC_I4_3: 0x19,
  LDC_I4_4: 0x1a, LDC_I4_5: 0x1b, LDC_I4_6: 0x1c, LDC_I4_7: 0x1d,
  LDC_I4_8: 0x1e,

  LDC_I4_S: 0x1f,
  LDC_I4: 0x20,

  RET: 0x2a,

  BR_S: 0x2b,
  BRFALSE_S: 0x2c,
  BRTRUE_S: 0x2d,

  ADD: 0x58,
  SUB: 0x59,
  MUL: 0x5a,
  DIV: 0x5b,

  PREFIX_FE: 0xfe,
} as const;

// Two-byte opcode second bytes (after 0xFE prefix)
export const CEQ_BYTE = 0x01;
export const CGT_BYTE = 0x02;
export const CLT_BYTE = 0x04;

// ---------------------------------------------------------------------------
// Trace dataclass
// ---------------------------------------------------------------------------

/**
 * A trace of one CLR IL instruction execution.
 */
export interface CLRTrace {
  pc: number;
  opcode: string;
  stackBefore: (number | null)[];
  stackAfter: (number | null)[];
  localsSnapshot: (number | null)[];
  description: string;
}

function freezeArray<T>(values: readonly T[]): readonly T[] {
  return Object.freeze([...values]);
}

// ---------------------------------------------------------------------------
// CLR Simulator
// ---------------------------------------------------------------------------

/**
 * A simulator for the CLR Intermediate Language (CIL/MSIL).
 *
 * This simulator executes real CLR IL bytecode, instruction by instruction,
 * producing detailed traces at each step.
 */
export class CLRSimulator {
  stack: (number | null)[] = [];
  locals: (number | null)[] = [];
  pc: number = 0;
  bytecode: Uint8Array = new Uint8Array(0);
  halted: boolean = false;

  constructor() {
    this.locals = new Array(16).fill(null);
  }

  /**
   * Load a CLR IL bytecode program into the simulator.
   * Resets all simulator state.
   */
  load(bytecode: Uint8Array, numLocals: number = 16): void {
    this.bytecode = bytecode;
    this.stack = [];
    this.locals = new Array(numLocals).fill(null);
    this.pc = 0;
    this.halted = false;
  }

  /**
   * Execute one CLR IL instruction and return a trace.
   */
  step(): CLRTrace {
    if (this.halted) {
      throw new Error("CLR simulator has halted -- no more instructions to execute");
    }
    if (this.pc >= this.bytecode.length) {
      throw new Error(`PC (${this.pc}) is beyond the end of bytecode (length ${this.bytecode.length})`);
    }

    const stackBefore = [...this.stack];
    const opcodeByte = this.bytecode[this.pc];

    // --- Two-byte opcodes (0xFE prefix) ---
    if (opcodeByte === CLROpcode.PREFIX_FE) {
      return this._executeTwoByteOpcode(stackBefore);
    }

    // --- NOP ---
    if (opcodeByte === CLROpcode.NOP) {
      const origPc = this.pc;
      this.pc += 1;
      return { pc: origPc, opcode: "nop", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: "no operation" };
    }

    // --- LDNULL ---
    if (opcodeByte === CLROpcode.LDNULL) {
      const origPc = this.pc;
      this.stack.push(null);
      this.pc += 1;
      return { pc: origPc, opcode: "ldnull", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: "push null" };
    }

    // --- LDC.I4.0 through LDC.I4.8 ---
    if (opcodeByte >= CLROpcode.LDC_I4_0 && opcodeByte <= CLROpcode.LDC_I4_8) {
      const value = opcodeByte - CLROpcode.LDC_I4_0;
      const origPc = this.pc;
      this.stack.push(value);
      this.pc += 1;
      return { pc: origPc, opcode: `ldc.i4.${value}`, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `push ${value}` };
    }

    // --- LDC.I4.S ---
    if (opcodeByte === CLROpcode.LDC_I4_S) {
      const origPc = this.pc;
      const raw = this.bytecode[this.pc + 1];
      const value = raw < 128 ? raw : raw - 256;
      this.stack.push(value);
      this.pc += 2;
      return { pc: origPc, opcode: "ldc.i4.s", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `push ${value}` };
    }

    // --- LDC.I4 ---
    if (opcodeByte === CLROpcode.LDC_I4) {
      const origPc = this.pc;
      const view = new DataView(this.bytecode.buffer, this.bytecode.byteOffset + this.pc + 1, 4);
      const value = view.getInt32(0, true); // little-endian
      this.stack.push(value);
      this.pc += 5;
      return { pc: origPc, opcode: "ldc.i4", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `push ${value}` };
    }

    // --- LDLOC.0 through LDLOC.3 ---
    if (opcodeByte >= CLROpcode.LDLOC_0 && opcodeByte <= CLROpcode.LDLOC_3) {
      const slot = opcodeByte - CLROpcode.LDLOC_0;
      const origPc = this.pc;
      const value = this.locals[slot];
      if (value === null || value === undefined) {
        throw new Error(`Local variable ${slot} is uninitialized`);
      }
      this.stack.push(value);
      this.pc += 1;
      return { pc: origPc, opcode: `ldloc.${slot}`, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `push locals[${slot}] = ${value}` };
    }

    // --- STLOC.0 through STLOC.3 ---
    if (opcodeByte >= CLROpcode.STLOC_0 && opcodeByte <= CLROpcode.STLOC_3) {
      const slot = opcodeByte - CLROpcode.STLOC_0;
      const origPc = this.pc;
      const value = this.stack.pop()!;
      this.locals[slot] = value;
      this.pc += 1;
      return { pc: origPc, opcode: `stloc.${slot}`, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${value}, store in locals[${slot}]` };
    }

    // --- LDLOC.S ---
    if (opcodeByte === CLROpcode.LDLOC_S) {
      const origPc = this.pc;
      const slot = this.bytecode[this.pc + 1];
      const value = this.locals[slot];
      if (value === null || value === undefined) {
        throw new Error(`Local variable ${slot} is uninitialized`);
      }
      this.stack.push(value);
      this.pc += 2;
      return { pc: origPc, opcode: "ldloc.s", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `push locals[${slot}] = ${value}` };
    }

    // --- STLOC.S ---
    if (opcodeByte === CLROpcode.STLOC_S) {
      const origPc = this.pc;
      const slot = this.bytecode[this.pc + 1];
      const value = this.stack.pop()!;
      this.locals[slot] = value;
      this.pc += 2;
      return { pc: origPc, opcode: "stloc.s", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${value}, store in locals[${slot}]` };
    }

    // --- Arithmetic ---
    if (opcodeByte === CLROpcode.ADD) return this._executeArithmetic(stackBefore, "add", (a, b) => a + b);
    if (opcodeByte === CLROpcode.SUB) return this._executeArithmetic(stackBefore, "sub", (a, b) => a - b);
    if (opcodeByte === CLROpcode.MUL) return this._executeArithmetic(stackBefore, "mul", (a, b) => a * b);
    if (opcodeByte === CLROpcode.DIV) return this._executeDiv(stackBefore);

    // --- RET ---
    if (opcodeByte === CLROpcode.RET) {
      const origPc = this.pc;
      this.pc += 1;
      this.halted = true;
      return { pc: origPc, opcode: "ret", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: "return" };
    }

    // --- BR.S ---
    if (opcodeByte === CLROpcode.BR_S) return this._executeBranchS(stackBefore, "br.s");

    // --- BRFALSE.S ---
    if (opcodeByte === CLROpcode.BRFALSE_S) return this._executeConditionalBranchS(stackBefore, "brfalse.s", true);

    // --- BRTRUE.S ---
    if (opcodeByte === CLROpcode.BRTRUE_S) return this._executeConditionalBranchS(stackBefore, "brtrue.s", false);

    throw new Error(`Unknown CLR opcode: 0x${opcodeByte.toString(16).toUpperCase().padStart(2, "0")} at PC=${this.pc}`);
  }

  /**
   * Execute until ret instruction, returning all traces.
   */
  run(maxSteps: number = 10000): CLRTrace[] {
    const traces: CLRTrace[] = [];
    for (let i = 0; i < maxSteps; i++) {
      if (this.halted) break;
      traces.push(this.step());
    }
    return traces;
  }

  getState(): CLRState {
    return Object.freeze({
      stack: freezeArray(this.stack),
      locals: freezeArray(this.locals),
      pc: this.pc,
      halted: this.halted,
    });
  }

  execute(
    program: Uint8Array,
    maxSteps: number = 100_000
  ): ExecutionResult<CLRState> {
    this.load(program);

    const traces: StepTrace[] = [];
    let steps = 0;
    let error: string | null = null;

    try {
      while (!this.halted && steps < maxSteps) {
        const pcBefore = this.pc;
        const trace = this.step();
        traces.push(
          new StepTrace(pcBefore, this.pc, trace.opcode, trace.description)
        );
        steps += 1;
      }
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }

    if (error === null && !this.halted) {
      error = `max_steps (${maxSteps}) exceeded`;
    }

    return new ExecutionResult({
      halted: this.halted,
      steps,
      finalState: this.getState(),
      error,
      traces,
    });
  }

  reset(): void {
    this.stack = [];
    this.locals = new Array(this.locals.length).fill(null);
    this.pc = 0;
    this.bytecode = new Uint8Array(0);
    this.halted = false;
  }

  // --- Private helper methods ---

  private _executeArithmetic(
    stackBefore: (number | null)[],
    mnemonic: string,
    op: (a: number, b: number) => number
  ): CLRTrace {
    const origPc = this.pc;
    const b = this.stack.pop() as number;
    const a = this.stack.pop() as number;
    const result = op(a, b);
    this.stack.push(result);
    this.pc += 1;
    return { pc: origPc, opcode: mnemonic, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${b} and ${a}, push ${result}` };
  }

  private _executeDiv(stackBefore: (number | null)[]): CLRTrace {
    const origPc = this.pc;
    const b = this.stack.pop() as number;
    const a = this.stack.pop() as number;
    if (b === 0) {
      throw new Error("System.DivideByZeroException: division by zero");
    }
    // CLR integer division truncates toward zero (like C's /)
    const result = Math.trunc(a / b);
    this.stack.push(result);
    this.pc += 1;
    return { pc: origPc, opcode: "div", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${b} and ${a}, push ${result}` };
  }

  private _executeTwoByteOpcode(stackBefore: (number | null)[]): CLRTrace {
    const origPc = this.pc;
    if (this.pc + 1 >= this.bytecode.length) {
      throw new Error(`Incomplete two-byte opcode at PC=${this.pc}`);
    }

    const secondByte = this.bytecode[this.pc + 1];

    if (secondByte === CEQ_BYTE) {
      const b = this.stack.pop();
      const a = this.stack.pop();
      const result = a === b ? 1 : 0;
      this.stack.push(result);
      this.pc += 2;
      return { pc: origPc, opcode: "ceq", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${b} and ${a}, push ${result} (${a} == ${b})` };
    }

    if (secondByte === CGT_BYTE) {
      const b = this.stack.pop() as number;
      const a = this.stack.pop() as number;
      const result = a > b ? 1 : 0;
      this.stack.push(result);
      this.pc += 2;
      return { pc: origPc, opcode: "cgt", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${b} and ${a}, push ${result} (${a} > ${b})` };
    }

    if (secondByte === CLT_BYTE) {
      const b = this.stack.pop() as number;
      const a = this.stack.pop() as number;
      const result = a < b ? 1 : 0;
      this.stack.push(result);
      this.pc += 2;
      return { pc: origPc, opcode: "clt", stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${b} and ${a}, push ${result} (${a} < ${b})` };
    }

    throw new Error(`Unknown two-byte opcode: 0xFE 0x${secondByte.toString(16).toUpperCase().padStart(2, "0")} at PC=${this.pc}`);
  }

  private _executeBranchS(stackBefore: (number | null)[], mnemonic: string): CLRTrace {
    const origPc = this.pc;
    const raw = this.bytecode[this.pc + 1];
    const offset = raw < 128 ? raw : raw - 256;
    const nextPc = this.pc + 2;
    const target = nextPc + offset;
    this.pc = target;
    return { pc: origPc, opcode: mnemonic, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `branch to PC=${target} (offset ${offset >= 0 ? "+" : ""}${offset})` };
  }

  private _executeConditionalBranchS(
    stackBefore: (number | null)[],
    mnemonic: string,
    takeIfZero: boolean
  ): CLRTrace {
    const origPc = this.pc;
    const raw = this.bytecode[this.pc + 1];
    const offset = raw < 128 ? raw : raw - 256;
    const nextPc = this.pc + 2;
    const target = nextPc + offset;

    const value = this.stack.pop();
    // For null values, treat as 0 (false)
    const numericValue = value === null ? 0 : value as number;

    const shouldBranch = takeIfZero ? (numericValue === 0) : (numericValue !== 0);

    if (shouldBranch) {
      this.pc = target;
      return { pc: origPc, opcode: mnemonic, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${value}, branch taken to PC=${target}` };
    } else {
      this.pc = nextPc;
      return { pc: origPc, opcode: mnemonic, stackBefore, stackAfter: [...this.stack], localsSnapshot: [...this.locals], description: `pop ${value}, branch not taken` };
    }
  }
}

// ---------------------------------------------------------------------------
// Helper functions -- a mini CLR IL assembler
// ---------------------------------------------------------------------------

/**
 * Assemble CLR IL from instruction tuples or raw Uint8Arrays.
 */
export function assembleClr(...instructions: (number[] | Uint8Array)[]): Uint8Array {
  const result: number[] = [];
  for (const instr of instructions) {
    if (instr instanceof Uint8Array) {
      for (let i = 0; i < instr.length; i++) result.push(instr[i]);
    } else {
      for (const byte of instr) result.push(byte);
    }
  }
  return new Uint8Array(result);
}

/**
 * Encode pushing an int32 constant, picking the optimal encoding.
 *
 * The CLR has three ways to push an integer constant:
 *     1. ldc.i4.N (1 byte)   -- for values 0 through 8
 *     2. ldc.i4.s V (2 bytes) -- for values -128 through 127
 *     3. ldc.i4 V (5 bytes)  -- for any 32-bit integer
 */
export function encodeLdcI4(n: number): Uint8Array {
  // Short forms: 0 through 8 -> single-byte opcodes
  if (n >= 0 && n <= 8) {
    return new Uint8Array([CLROpcode.LDC_I4_0 + n]);
  }

  // Medium form: -128 through 127 -> 2-byte opcode + signed int8
  if (n >= -128 && n <= 127) {
    const raw = n >= 0 ? n : n + 256;
    return new Uint8Array([CLROpcode.LDC_I4_S, raw & 0xff]);
  }

  // General form: any 32-bit integer -> 5-byte opcode + LE int32
  const buf = new ArrayBuffer(5);
  const view = new DataView(buf);
  view.setUint8(0, CLROpcode.LDC_I4);
  view.setInt32(1, n, true); // little-endian
  return new Uint8Array(buf);
}

/**
 * Encode storing to a local variable slot.
 * Uses short form for slots 0-3.
 */
export function encodeStloc(slot: number): Uint8Array {
  if (slot >= 0 && slot <= 3) {
    return new Uint8Array([CLROpcode.STLOC_0 + slot]);
  }
  return new Uint8Array([CLROpcode.STLOC_S, slot]);
}

/**
 * Encode loading from a local variable slot.
 * Uses short form for slots 0-3.
 */
export function encodeLdloc(slot: number): Uint8Array {
  if (slot >= 0 && slot <= 3) {
    return new Uint8Array([CLROpcode.LDLOC_0 + slot]);
  }
  return new Uint8Array([CLROpcode.LDLOC_S, slot]);
}
