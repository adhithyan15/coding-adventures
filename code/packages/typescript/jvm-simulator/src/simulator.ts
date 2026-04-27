/**
 * Java Virtual Machine (JVM) Bytecode Simulator -- a typed stack machine.
 *
 * === What is the JVM? ===
 *
 * The Java Virtual Machine was introduced by Sun Microsystems in 1995 alongside
 * the Java programming language. Its revolutionary promise was "write once, run
 * anywhere" -- compile your source code to platform-independent bytecode, and any
 * machine with a JVM can execute it.
 *
 * Today the JVM is the most widely deployed virtual machine in history. It runs
 * not just Java but also Kotlin, Scala, Clojure, Groovy, and JRuby.
 *
 * === Stack machine with typed opcodes ===
 *
 * Like our bytecode VM and WASM, the JVM is a stack-based machine. But the JVM
 * differs in a crucial way: its opcodes are *typed*. The JVM has separate
 * opcodes for each primitive type:
 *
 *     Our VM:    ADD           <-- works on whatever's on the stack
 *     JVM:       iadd          <-- integer add
 *                ladd          <-- long add
 *                fadd          <-- float add
 *                dadd          <-- double add
 *
 * The type prefix convention:
 *     i = int (32-bit signed integer)
 *     l = long (64-bit signed integer)
 *     f = float (32-bit IEEE 754)
 *     d = double (64-bit IEEE 754)
 *     a = reference (object pointer)
 *
 * Our MVP implements only the "i" (integer) variants.
 *
 * === Variable-width bytecode encoding ===
 *
 * Like WASM (and unlike RISC-V's fixed 32-bit instructions), JVM bytecode uses
 * variable-width encoding. Each instruction starts with a 1-byte opcode, followed
 * by zero or more operand bytes.
 *
 * === The x = 1 + 2 program ===
 *
 *     iconst_1          Push integer constant 1        stack: [1]
 *     iconst_2          Push integer constant 2        stack: [1, 2]
 *     iadd              Pop two ints, push sum          stack: [3]
 *     istore_0          Pop and store in local 0        stack: []  locals[0]=3
 *     return            Return void from method
 */

import {
  ExecutionResult,
  StepTrace,
} from "@coding-adventures/simulator-protocol";

import type { JVMState } from "./state.js";

// ---------------------------------------------------------------------------
// JVM Opcode definitions
// ---------------------------------------------------------------------------
// These are the REAL opcode byte values from the JVM specification.

/**
 * Real JVM opcode values from the JVM specification.
 *
 * Naming convention: the "i" prefix means "integer" (32-bit signed).
 * The opcode values here match the official JVM spec exactly.
 */
export const JVMOpcode = {
  // --- Constant-pushing opcodes ---
  ICONST_0: 0x03,
  ICONST_1: 0x04,
  ICONST_2: 0x05,
  ICONST_3: 0x06,
  ICONST_4: 0x07,
  ICONST_5: 0x08,

  // bipush pushes a signed byte value (-128 to 127) as an integer.
  BIPUSH: 0x10,

  // ldc loads a constant from the constant pool by index.
  LDC: 0x12,

  // --- Local variable load opcodes ---
  ILOAD: 0x15,
  ILOAD_0: 0x1a,
  ILOAD_1: 0x1b,
  ILOAD_2: 0x1c,
  ILOAD_3: 0x1d,

  // --- Local variable store opcodes ---
  ISTORE: 0x36,
  ISTORE_0: 0x3b,
  ISTORE_1: 0x3c,
  ISTORE_2: 0x3d,
  ISTORE_3: 0x3e,

  // --- Integer arithmetic opcodes ---
  IADD: 0x60,
  ISUB: 0x64,
  IMUL: 0x68,
  IDIV: 0x6c,

  // --- Control flow opcodes ---
  IF_ICMPEQ: 0x9f,
  IF_ICMPGT: 0xa3,
  GOTO: 0xa7,

  // --- Return opcodes ---
  IRETURN: 0xac,
  RETURN: 0xb1,
} as const;

// Map from opcode byte to mnemonic name (for trace output)
const _OPCODE_NAMES: Record<number, string> = {};
for (const [name, value] of Object.entries(JVMOpcode)) {
  _OPCODE_NAMES[value] = name.toLowerCase();
}

function freezeArray<T>(values: readonly T[]): readonly T[] {
  return Object.freeze([...values]);
}

// ---------------------------------------------------------------------------
// Trace dataclass
// ---------------------------------------------------------------------------

/**
 * A trace of one JVM instruction execution.
 *
 * Captures the complete state transition for a single step, allowing
 * you to visualize execution.
 */
export interface JVMTrace {
  pc: number;
  opcode: string; // Mnemonic like "iadd", "iconst_1", etc.
  stackBefore: number[];
  stackAfter: number[];
  localsSnapshot: (number | null)[];
  description: string;
}

// ---------------------------------------------------------------------------
// JVM Simulator
// ---------------------------------------------------------------------------

/**
 * Complete JVM bytecode simulator -- decoder, executor, and state.
 *
 * State:
 *     - stack:      The operand stack
 *     - locals:     Local variable array (numbered slots)
 *     - constants:  Constant pool (values loaded by the ldc instruction)
 *     - pc:         Program counter (byte offset into bytecode)
 *     - halted:     Whether execution has finished
 *     - returnValue: Value returned by IRETURN (null if RETURN/void)
 */
export class JVMSimulator {
  stack: number[] = [];
  locals: (number | null)[] = [];
  constants: (number | string)[] = [];
  pc: number = 0;
  halted: boolean = false;
  returnValue: number | null = null;
  private _bytecode: Uint8Array = new Uint8Array(0);
  private _numLocals: number = 16;

  constructor() {
    this.locals = new Array(16).fill(null);
  }

  /**
   * Load a JVM bytecode program.
   *
   * Resets all simulator state: stack, locals, PC, and halt flag.
   */
  load(
    bytecode: Uint8Array,
    constants?: (number | string)[],
    numLocals: number = 16
  ): void {
    this._bytecode = bytecode;
    this.constants = constants ?? [];
    this._numLocals = numLocals;
    this.stack = [];
    this.locals = new Array(numLocals).fill(null);
    this.pc = 0;
    this.halted = false;
    this.returnValue = null;
  }

  /**
   * Execute one JVM instruction and return a trace.
   */
  step(): JVMTrace {
    if (this.halted) {
      throw new Error(
        "JVM simulator has halted -- no more instructions to execute"
      );
    }

    if (this.pc >= this._bytecode.length) {
      throw new Error(
        `PC (${this.pc}) is past end of bytecode (${this._bytecode.length} bytes)`
      );
    }

    const stackBefore = [...this.stack];
    const opcodeByte = this._bytecode[this.pc];

    // Check if opcode is known
    if (_OPCODE_NAMES[opcodeByte] === undefined) {
      throw new Error(
        `Unknown JVM opcode: 0x${opcodeByte.toString(16).toUpperCase().padStart(2, "0")} at PC=${this.pc}`
      );
    }

    return this._execute(opcodeByte, stackBefore);
  }

  /**
   * Execute until RETURN/IRETURN, returning all traces.
   */
  run(maxSteps: number = 10000): JVMTrace[] {
    const traces: JVMTrace[] = [];
    for (let i = 0; i < maxSteps; i++) {
      if (this.halted) break;
      traces.push(this.step());
    }
    return traces;
  }

  getState(): JVMState {
    return Object.freeze({
      stack: freezeArray(this.stack),
      locals: freezeArray(this.locals),
      constants: freezeArray(this.constants),
      pc: this.pc,
      halted: this.halted,
      returnValue: this.returnValue,
    });
  }

  execute(
    program: Uint8Array,
    maxSteps: number = 100_000
  ): ExecutionResult<JVMState> {
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
    this.locals = new Array(this._numLocals).fill(null);
    this.constants = [];
    this.pc = 0;
    this.halted = false;
    this.returnValue = null;
    this._bytecode = new Uint8Array(0);
  }

  // -------------------------------------------------------------------
  // Private: instruction dispatch and execution
  // -------------------------------------------------------------------

  private _execute(opcodeByte: number, stackBefore: number[]): JVMTrace {
    const pc = this.pc;

    // --- iconst_N: push small integer constants (1 byte) ---
    if (opcodeByte >= JVMOpcode.ICONST_0 && opcodeByte <= JVMOpcode.ICONST_5) {
      const value = opcodeByte - JVMOpcode.ICONST_0;
      this.stack.push(value);
      this.pc += 1;
      return {
        pc,
        opcode: _OPCODE_NAMES[opcodeByte],
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: `push ${value}`,
      };
    }

    // --- bipush: push a signed byte value (2 bytes) ---
    if (opcodeByte === JVMOpcode.BIPUSH) {
      const raw = this._bytecode[this.pc + 1];
      const value = raw < 128 ? raw : raw - 256;
      this.stack.push(value);
      this.pc += 2;
      return {
        pc,
        opcode: "bipush",
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: `push ${value}`,
      };
    }

    // --- ldc: load from constant pool (2 bytes) ---
    if (opcodeByte === JVMOpcode.LDC) {
      const index = this._bytecode[this.pc + 1];
      if (index >= this.constants.length) {
        throw new Error(
          `Constant pool index ${index} out of range (pool size: ${this.constants.length})`
        );
      }
      const value = this.constants[index];
      if (typeof value !== "number") {
        throw new Error(
          `ldc: constant pool entry ${index} is not an integer: ${String(value)}`
        );
      }
      this.stack.push(value);
      this.pc += 2;
      return {
        pc,
        opcode: "ldc",
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: `push constant[${index}] = ${value}`,
      };
    }

    // --- iload_N: load int from local slot N (1 byte) ---
    if (opcodeByte >= JVMOpcode.ILOAD_0 && opcodeByte <= JVMOpcode.ILOAD_3) {
      const slot = opcodeByte - JVMOpcode.ILOAD_0;
      return this._doIload(pc, slot, _OPCODE_NAMES[opcodeByte], stackBefore);
    }

    // --- iload: load int from arbitrary local slot (2 bytes) ---
    if (opcodeByte === JVMOpcode.ILOAD) {
      const slot = this._bytecode[this.pc + 1];
      this.pc += 1; // extra byte consumed (handler adds 1 more)
      return this._doIload(pc, slot, "iload", stackBefore);
    }

    // --- istore_N: store int to local slot N (1 byte) ---
    if (
      opcodeByte >= JVMOpcode.ISTORE_0 &&
      opcodeByte <= JVMOpcode.ISTORE_3
    ) {
      const slot = opcodeByte - JVMOpcode.ISTORE_0;
      return this._doIstore(pc, slot, _OPCODE_NAMES[opcodeByte], stackBefore);
    }

    // --- istore: store int to arbitrary local slot (2 bytes) ---
    if (opcodeByte === JVMOpcode.ISTORE) {
      const slot = this._bytecode[this.pc + 1];
      this.pc += 1; // extra byte consumed
      return this._doIstore(pc, slot, "istore", stackBefore);
    }

    // --- iadd: integer addition (1 byte) ---
    if (opcodeByte === JVMOpcode.IADD) {
      return this._doBinaryOp(pc, "iadd", (a, b) => a + b, stackBefore);
    }

    // --- isub: integer subtraction (1 byte) ---
    if (opcodeByte === JVMOpcode.ISUB) {
      return this._doBinaryOp(pc, "isub", (a, b) => a - b, stackBefore);
    }

    // --- imul: integer multiplication (1 byte) ---
    if (opcodeByte === JVMOpcode.IMUL) {
      return this._doBinaryOp(pc, "imul", (a, b) => a * b, stackBefore);
    }

    // --- idiv: integer division (1 byte) ---
    if (opcodeByte === JVMOpcode.IDIV) {
      if (this.stack.length < 2) {
        throw new Error("Stack underflow: idiv requires 2 operands");
      }
      if (this.stack[this.stack.length - 1] === 0) {
        throw new Error("ArithmeticException: division by zero");
      }
      return this._doBinaryOp(
        pc,
        "idiv",
        (a, b) => Math.trunc(a / b),
        stackBefore
      );
    }

    // --- goto: unconditional branch (3 bytes) ---
    if (opcodeByte === JVMOpcode.GOTO) {
      const view = new DataView(
        this._bytecode.buffer,
        this._bytecode.byteOffset + this.pc + 1,
        2
      );
      const offset = view.getInt16(0, false); // big-endian
      const target = this.pc + offset;
      this.pc = target;
      return {
        pc,
        opcode: "goto",
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: `jump to PC=${target} (offset ${offset >= 0 ? "+" : ""}${offset})`,
      };
    }

    // --- if_icmpeq: branch if two ints are equal (3 bytes) ---
    if (opcodeByte === JVMOpcode.IF_ICMPEQ) {
      return this._doIfIcmp(pc, "if_icmpeq", stackBefore, (a, b) => a === b);
    }

    // --- if_icmpgt: branch if first int > second (3 bytes) ---
    if (opcodeByte === JVMOpcode.IF_ICMPGT) {
      return this._doIfIcmp(pc, "if_icmpgt", stackBefore, (a, b) => a > b);
    }

    // --- ireturn: return an int value (1 byte) ---
    if (opcodeByte === JVMOpcode.IRETURN) {
      if (this.stack.length < 1) {
        throw new Error("Stack underflow: ireturn requires 1 operand");
      }
      this.returnValue = this.stack.pop()!;
      this.halted = true;
      this.pc += 1;
      return {
        pc,
        opcode: "ireturn",
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: `return ${this.returnValue}`,
      };
    }

    // --- return: return void (1 byte) ---
    if (opcodeByte === JVMOpcode.RETURN) {
      this.halted = true;
      this.pc += 1;
      return {
        pc,
        opcode: "return",
        stackBefore,
        stackAfter: [...this.stack],
        localsSnapshot: [...this.locals],
        description: "return void",
      };
    }

    throw new Error(
      `Unimplemented opcode: 0x${opcodeByte.toString(16).toUpperCase().padStart(2, "0")}`
    );
  }

  private _doIload(
    pc: number,
    slot: number,
    mnemonic: string,
    stackBefore: number[]
  ): JVMTrace {
    const value = this.locals[slot];
    if (value === null) {
      throw new Error(`Local variable ${slot} has not been initialized`);
    }
    this.stack.push(value);
    this.pc += 1;
    return {
      pc,
      opcode: mnemonic,
      stackBefore,
      stackAfter: [...this.stack],
      localsSnapshot: [...this.locals],
      description: `push locals[${slot}] = ${value}`,
    };
  }

  private _doIstore(
    pc: number,
    slot: number,
    mnemonic: string,
    stackBefore: number[]
  ): JVMTrace {
    if (this.stack.length < 1) {
      throw new Error(`Stack underflow: ${mnemonic} requires 1 operand`);
    }
    const value = this.stack.pop()!;
    this.locals[slot] = value;
    this.pc += 1;
    return {
      pc,
      opcode: mnemonic,
      stackBefore,
      stackAfter: [...this.stack],
      localsSnapshot: [...this.locals],
      description: `pop ${value}, store in locals[${slot}]`,
    };
  }

  private _doBinaryOp(
    pc: number,
    mnemonic: string,
    op: (a: number, b: number) => number,
    stackBefore: number[]
  ): JVMTrace {
    if (this.stack.length < 2) {
      throw new Error(`Stack underflow: ${mnemonic} requires 2 operands`);
    }
    const b = this.stack.pop()!;
    const a = this.stack.pop()!;
    let result = op(a, b);
    // Wrap to 32-bit signed integer range
    result = toI32(result);
    this.stack.push(result);
    this.pc += 1;
    return {
      pc,
      opcode: mnemonic,
      stackBefore,
      stackAfter: [...this.stack],
      localsSnapshot: [...this.locals],
      description: `pop ${b} and ${a}, push ${result}`,
    };
  }

  private _doIfIcmp(
    pc: number,
    mnemonic: string,
    stackBefore: number[],
    condition: (a: number, b: number) => boolean
  ): JVMTrace {
    if (this.stack.length < 2) {
      throw new Error(`Stack underflow: ${mnemonic} requires 2 operands`);
    }

    const view = new DataView(
      this._bytecode.buffer,
      this._bytecode.byteOffset + this.pc + 1,
      2
    );
    const offset = view.getInt16(0, false); // big-endian

    const b = this.stack.pop()!;
    const a = this.stack.pop()!;
    const taken = condition(a, b);
    const cmpOp = mnemonic.includes("eq") ? "==" : ">";

    let desc: string;
    if (taken) {
      const target = pc + offset;
      this.pc = target;
      desc = `pop ${b} and ${a}, ${a} ${cmpOp} ${b} is true, jump to PC=${target}`;
    } else {
      this.pc = pc + 3; // skip past the 3-byte instruction
      desc = `pop ${b} and ${a}, ${a} ${cmpOp} ${b} is false, fall through`;
    }

    return {
      pc,
      opcode: mnemonic,
      stackBefore,
      stackAfter: [...this.stack],
      localsSnapshot: [...this.locals],
      description: desc,
    };
  }
}

/**
 * Wrap a JavaScript number to 32-bit signed range.
 *
 * The JVM specifies that integer arithmetic wraps at 32 bits using
 * two's complement.
 */
function toI32(value: number): number {
  value = value & 0xffffffff;
  if (value >= 0x80000000) {
    value -= 0x100000000;
  }
  return value;
}

// ---------------------------------------------------------------------------
// Encoding helpers (mini assembler)
// ---------------------------------------------------------------------------

/**
 * Encode pushing a small integer constant (0-5) using iconst_N opcodes.
 * Falls back to bipush for -128 to 127 range.
 */
export function encodeIconst(n: number): Uint8Array {
  if (n >= 0 && n <= 5) {
    return new Uint8Array([JVMOpcode.ICONST_0 + n]);
  }
  if (n >= -128 && n <= 127) {
    const raw = n >= 0 ? n : n + 256;
    return new Uint8Array([JVMOpcode.BIPUSH, raw]);
  }
  throw new Error(
    `encodeIconst: value ${n} is outside signed byte range (-128 to 127). Use bipush or ldc.`
  );
}

/**
 * Encode storing to a local variable slot.
 * Uses the istore_N shortcut for slots 0-3.
 */
export function encodeIstore(slot: number): Uint8Array {
  if (slot >= 0 && slot <= 3) {
    return new Uint8Array([JVMOpcode.ISTORE_0 + slot]);
  }
  return new Uint8Array([JVMOpcode.ISTORE, slot]);
}

/**
 * Encode loading from a local variable slot.
 * Uses the iload_N shortcut for slots 0-3.
 */
export function encodeIload(slot: number): Uint8Array {
  if (slot >= 0 && slot <= 3) {
    return new Uint8Array([JVMOpcode.ILOAD_0 + slot]);
  }
  return new Uint8Array([JVMOpcode.ILOAD, slot]);
}

/**
 * Assemble JVM bytecode from instruction tuples.
 *
 * Each instruction is a number array: [opcode] or [opcode, operand, ...].
 */
export function assembleJvm(...instructions: number[][]): Uint8Array {
  const oneByteOpcodes: Set<number> = new Set([
    JVMOpcode.ICONST_0, JVMOpcode.ICONST_1, JVMOpcode.ICONST_2,
    JVMOpcode.ICONST_3, JVMOpcode.ICONST_4, JVMOpcode.ICONST_5,
    JVMOpcode.ILOAD_0, JVMOpcode.ILOAD_1, JVMOpcode.ILOAD_2, JVMOpcode.ILOAD_3,
    JVMOpcode.ISTORE_0, JVMOpcode.ISTORE_1, JVMOpcode.ISTORE_2, JVMOpcode.ISTORE_3,
    JVMOpcode.IADD, JVMOpcode.ISUB, JVMOpcode.IMUL, JVMOpcode.IDIV,
    JVMOpcode.IRETURN, JVMOpcode.RETURN,
  ]);

  const twoByteOpcodes: Set<number> = new Set([
    JVMOpcode.BIPUSH, JVMOpcode.LDC, JVMOpcode.ILOAD, JVMOpcode.ISTORE,
  ]);

  const threeByteOpcodes: Set<number> = new Set([
    JVMOpcode.GOTO, JVMOpcode.IF_ICMPEQ, JVMOpcode.IF_ICMPGT,
  ]);

  const result: number[] = [];

  for (const instr of instructions) {
    const op = instr[0];

    if (oneByteOpcodes.has(op)) {
      result.push(op);
    } else if (twoByteOpcodes.has(op)) {
      if (instr.length < 2) {
        throw new Error(`Opcode 0x${op.toString(16)} requires an operand`);
      }
      const operand = instr[1];
      if (op === JVMOpcode.BIPUSH) {
        const raw = operand >= 0 ? operand : operand + 256;
        result.push(op, raw & 0xff);
      } else {
        result.push(op, operand & 0xff);
      }
    } else if (threeByteOpcodes.has(op)) {
      if (instr.length < 2) {
        throw new Error(`Opcode 0x${op.toString(16)} requires an offset operand`);
      }
      const offset = instr[1];
      const buf = new ArrayBuffer(2);
      const view = new DataView(buf);
      view.setInt16(0, offset, false); // big-endian
      result.push(op, view.getUint8(0), view.getUint8(1));
    } else {
      throw new Error(`Unknown opcode in assembleJvm: 0x${op.toString(16)}`);
    }
  }

  return new Uint8Array(result);
}
