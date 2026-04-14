/**
 * WebAssembly (WASM) Simulator -- a modern stack-based virtual machine.
 *
 * === What is WebAssembly? ===
 *
 * WebAssembly (WASM) is a binary instruction format designed as a portable
 * compilation target for the web. It was standardized by the W3C in 2017 and
 * is supported by all major browsers. Languages like Rust, C++, Go, and
 * AssemblyScript compile down to WASM, letting you run near-native-speed code
 * inside a browser sandbox.
 *
 * WASM is interesting because it bridges the gap between high-level languages
 * and the browser runtime. Instead of writing JavaScript, you write Rust --
 * and the compiler produces WASM bytecode that the browser executes directly.
 *
 * === Stack machines vs register machines ===
 *
 * Our RISC-V simulator is a *register machine*: instructions name specific
 * registers as operands (e.g., "add x3, x1, x2" -- read x1 and x2, write x3).
 * The CPU has a fixed set of registers, and the instruction encoding must
 * specify which registers to use.
 *
 * WASM is a *stack machine*: instructions don't name their operands. Instead,
 * operands live on an implicit *operand stack*. Push values onto the stack,
 * then invoke an operation -- it pops its inputs and pushes the result.
 *
 *     Register machine (RISC-V):        Stack machine (WASM):
 *         addi x1, x0, 1                   i32.const 1
 *         addi x2, x0, 2                   i32.const 2
 *         add  x3, x1, x2                  i32.add
 *                                           local.set 0
 *
 * Both compute "x = 1 + 2", but the stack machine never names a destination
 * register for the add. It pops 2 and 1 from the stack, pushes 3, and then
 * local.set stores it.
 *
 * Stack machines have a simpler instruction encoding (no register fields!) but
 * the CPU must manage the stack. Register machines have wider instructions but
 * can access any register in one cycle.
 *
 * Our bytecode VM (cpu-simulator) is also a stack-based design internally,
 * so WASM feels like a natural next step -- a real, production stack machine.
 *
 * === WASM instruction encoding ===
 *
 * Unlike RISC-V (where every instruction is exactly 32 bits), WASM instructions
 * are variable-width. Some are 1 byte (i32.add = 0x6A), others are 2 bytes
 * (local.get N = 0x20 N), and i32.const is 5 bytes (0x41 + 4-byte LE value).
 *
 * In real WASM, integer immediates use LEB128 variable-length encoding. For our
 * MVP, we use a simplified fixed-width encoding:
 *
 *     Instruction      Encoding              Width
 *     -------------    --------------------   -----
 *     i32.const V      0x41 V[0] V[1] V[2] V[3]   5 bytes (V as little-endian i32)
 *     i32.add          0x6A                  1 byte
 *     i32.sub          0x6B                  1 byte
 *     local.get N      0x20 N               2 bytes
 *     local.set N      0x21 N               2 bytes
 *     end              0x0B                  1 byte
 *
 * === The x = 1 + 2 program ===
 *
 *     i32.const 1    ->  stack: [1]           push 1
 *     i32.const 2    ->  stack: [1, 2]        push 2
 *     i32.add        ->  stack: [3]           pop 2 and 1, push 3
 *     local.set 0    ->  stack: [], x=3       pop 3, store in local 0
 *     end            ->  halt
 *
 * === Why standalone (not wrapping the CPU class)? ===
 *
 * The generic CPU class uses a fixed-width fetch cycle: it reads a 32-bit word
 * at PC, decodes it, and advances PC by 4. WASM instructions are variable-width
 * bytes, so the fetch cycle is fundamentally different -- we read one byte at PC
 * to determine the opcode, then read additional bytes depending on the opcode.
 *
 * Rather than forcing the CPU class to support variable-width fetch, we build
 * a standalone simulator that directly implements the WASM execution model.
 */

import {
  ExecutionResult,
  StepTrace,
} from "@coding-adventures/simulator-protocol";

import type { WasmState } from "./state.js";

// ---------------------------------------------------------------------------
// Opcode constants
// ---------------------------------------------------------------------------
// These are the byte values that identify each WASM instruction.
// In real WASM, these are defined by the spec in the "Binary Format" section.

export const OP_END = 0x0b; // End of function / halt
export const OP_LOCAL_GET = 0x20; // Push a local variable onto the stack
export const OP_LOCAL_SET = 0x21; // Pop the stack into a local variable
export const OP_I32_CONST = 0x41; // Push a 32-bit integer constant
export const OP_I32_ADD = 0x6a; // Pop two i32s, push their sum
export const OP_I32_SUB = 0x6b; // Pop two i32s, push their difference

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/**
 * A decoded WASM instruction with its operands and size.
 *
 * Unlike RISC-V's fixed 32-bit instructions, WASM instructions vary in
 * size. The `size` field tells the simulator how far to advance PC.
 *
 * Examples:
 *     { opcode: 0x41, mnemonic: "i32.const", operand: 42, size: 5 }
 *     { opcode: 0x6A, mnemonic: "i32.add", operand: null, size: 1 }
 *     { opcode: 0x20, mnemonic: "local.get", operand: 0, size: 2 }
 */
export interface WasmInstruction {
  opcode: number;
  mnemonic: string;
  operand: number | null; // Some instructions have no operand (add, end)
  size: number; // Number of bytes consumed (for advancing PC)
}

/**
 * Decodes WASM bytecodes from raw bytes to structured instructions.
 *
 * The decoder reads bytes from a bytecode buffer starting at a given PC,
 * determines the instruction and its operands, and returns a WasmInstruction.
 *
 * Variable-width decoding:
 *     - Read 1 byte at PC to get the opcode
 *     - Depending on the opcode, read 0, 1, or 4 more bytes for the operand
 *     - Return the total size so the simulator knows how to advance PC
 *
 * Example: decoding i32.const 42 from bytes [0x41, 0x2A, 0x00, 0x00, 0x00]
 *
 *     Byte 0: 0x41 -> opcode = i32.const (expects 4 more bytes)
 *     Bytes 1-4: 0x2A 0x00 0x00 0x00 -> value = 42 (little-endian)
 *     Result: { opcode: 0x41, mnemonic: "i32.const", operand: 42, size: 5 }
 */
export class WasmDecoder {
  /**
   * Decode one instruction starting at `pc` in the bytecode buffer.
   *
   * Reads the opcode byte, then dispatches to the appropriate handler
   * based on the instruction's operand format.
   */
  decode(bytecode: Uint8Array, pc: number): WasmInstruction {
    const opcode = bytecode[pc];

    if (opcode === OP_I32_CONST) {
      // 5 bytes: opcode + 4-byte little-endian signed integer
      const view = new DataView(bytecode.buffer, bytecode.byteOffset + pc + 1, 4);
      const value = view.getInt32(0, true); // true = little-endian
      return { opcode, mnemonic: "i32.const", operand: value, size: 5 };
    } else if (opcode === OP_I32_ADD) {
      // 1 byte: just the opcode
      return { opcode, mnemonic: "i32.add", operand: null, size: 1 };
    } else if (opcode === OP_I32_SUB) {
      // 1 byte: just the opcode
      return { opcode, mnemonic: "i32.sub", operand: null, size: 1 };
    } else if (opcode === OP_LOCAL_GET) {
      // 2 bytes: opcode + 1-byte local index
      const index = bytecode[pc + 1];
      return { opcode, mnemonic: "local.get", operand: index, size: 2 };
    } else if (opcode === OP_LOCAL_SET) {
      // 2 bytes: opcode + 1-byte local index
      const index = bytecode[pc + 1];
      return { opcode, mnemonic: "local.set", operand: index, size: 2 };
    } else if (opcode === OP_END) {
      // 1 byte: end marker
      return { opcode, mnemonic: "end", operand: null, size: 1 };
    } else {
      throw new Error(
        `Unknown WASM opcode: 0x${opcode.toString(16).toUpperCase().padStart(2, "0")} at PC=${pc}`
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/**
 * A trace of one WASM instruction execution.
 *
 * This is the WASM equivalent of PipelineTrace -- it captures what happened
 * during a single step so you can visualize execution.
 *
 * The key difference from RISC-V traces: instead of register changes,
 * we show the operand stack before and after execution. This makes the
 * stack-based execution model visible.
 *
 * Example trace for i32.add when stack was [1, 2]:
 *     {
 *         pc: 10,
 *         instruction: { ..., mnemonic: "i32.add" },
 *         stackBefore: [1, 2],
 *         stackAfter: [3],
 *         localsSnapshot: [0, 0, 0, 0],
 *         description: "pop 2 and 1, push 3",
 *         halted: false,
 *     }
 */
export interface WasmStepTrace {
  pc: number;
  instruction: WasmInstruction;
  stackBefore: number[];
  stackAfter: number[];
  localsSnapshot: number[];
  description: string;
  halted: boolean;
}

function freezeArray<T>(values: readonly T[]): readonly T[] {
  return Object.freeze([...values]);
}

/**
 * Executes decoded WASM instructions against a stack and local variables.
 *
 * The executor is the "do it" phase. Given a decoded instruction, it:
 *   - Reads from the stack or locals (inputs)
 *   - Performs the operation
 *   - Writes to the stack or locals (outputs)
 *
 * Stack operations follow WASM semantics:
 *   - i32.const V: push V onto the stack
 *   - i32.add:     pop b, pop a, push (a + b)
 *   - i32.sub:     pop b, pop a, push (a - b)
 *   - local.get N: push locals[N] onto the stack
 *   - local.set N: pop the stack into locals[N]
 *   - end:         halt execution
 *
 * Note the operand order for i32.add and i32.sub: the *second* operand
 * is on top of the stack (popped first). So "push 1; push 2; i32.sub"
 * computes 1 - 2 = -1 (not 2 - 1).
 */
export class WasmExecutor {
  /**
   * Execute one decoded WASM instruction.
   *
   * Modifies `stack` and `locals` in place. Returns a trace showing
   * what happened (stack before/after, description, halt status).
   */
  execute(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number
  ): WasmStepTrace {
    const stackBefore = [...stack]; // Snapshot before mutation
    const mnemonic = instruction.mnemonic;

    if (mnemonic === "i32.const") {
      return this._execI32Const(instruction, stack, locals, pc, stackBefore);
    } else if (mnemonic === "i32.add") {
      return this._execI32Add(instruction, stack, locals, pc, stackBefore);
    } else if (mnemonic === "i32.sub") {
      return this._execI32Sub(instruction, stack, locals, pc, stackBefore);
    } else if (mnemonic === "local.get") {
      return this._execLocalGet(instruction, stack, locals, pc, stackBefore);
    } else if (mnemonic === "local.set") {
      return this._execLocalSet(instruction, stack, locals, pc, stackBefore);
    } else if (mnemonic === "end") {
      return {
        pc,
        instruction,
        stackBefore,
        stackAfter: [...stack],
        localsSnapshot: [...locals],
        description: "halt",
        halted: true,
      };
    } else {
      throw new Error(`Cannot execute: ${mnemonic}`);
    }
  }

  /**
   * Execute: i32.const V -> push V onto the stack.
   *
   * Example: i32.const 42
   *     stack before: []
   *     stack after:  [42]
   */
  private _execI32Const(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number,
    stackBefore: number[]
  ): WasmStepTrace {
    const value = instruction.operand!;
    stack.push(value);
    return {
      pc,
      instruction,
      stackBefore,
      stackAfter: [...stack],
      localsSnapshot: [...locals],
      description: `push ${value}`,
      halted: false,
    };
  }

  /**
   * Execute: i32.add -> pop b, pop a, push (a + b).
   *
   * The second-to-top value is the left operand (a), and the top
   * value is the right operand (b). This matches WASM spec semantics.
   *
   * Example: stack [1, 2] -> i32.add -> stack [3]
   *     b = pop() -> 2
   *     a = pop() -> 1
   *     push(1 + 2) -> push(3)
   */
  private _execI32Add(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number,
    stackBefore: number[]
  ): WasmStepTrace {
    const b = stack.pop()!;
    const a = stack.pop()!;
    const result = (a + b) & 0xffffffff; // Mask to 32-bit unsigned
    stack.push(result);
    return {
      pc,
      instruction,
      stackBefore,
      stackAfter: [...stack],
      localsSnapshot: [...locals],
      description: `pop ${b} and ${a}, push ${result}`,
      halted: false,
    };
  }

  /**
   * Execute: i32.sub -> pop b, pop a, push (a - b).
   *
   * Same operand order as i32.add: second-to-top minus top.
   *
   * Example: stack [5, 3] -> i32.sub -> stack [2]
   *     b = pop() -> 3
   *     a = pop() -> 5
   *     push(5 - 3) -> push(2)
   */
  private _execI32Sub(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number,
    stackBefore: number[]
  ): WasmStepTrace {
    const b = stack.pop()!;
    const a = stack.pop()!;
    const result = (a - b) & 0xffffffff; // Mask to 32-bit unsigned
    stack.push(result);
    return {
      pc,
      instruction,
      stackBefore,
      stackAfter: [...stack],
      localsSnapshot: [...locals],
      description: `pop ${b} and ${a}, push ${result}`,
      halted: false,
    };
  }

  /**
   * Execute: local.get N -> push locals[N] onto the stack.
   *
   * Local variables are like WASM's version of registers -- a fixed set
   * of named storage slots. But unlike registers, you access them through
   * the stack: local.get pushes the value, local.set pops it.
   *
   * Example: local.get 0 (where locals[0] = 42)
   *     stack before: []
   *     stack after:  [42]
   */
  private _execLocalGet(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number,
    stackBefore: number[]
  ): WasmStepTrace {
    const index = instruction.operand!;
    const value = locals[index];
    stack.push(value);
    return {
      pc,
      instruction,
      stackBefore,
      stackAfter: [...stack],
      localsSnapshot: [...locals],
      description: `push locals[${index}] = ${value}`,
      halted: false,
    };
  }

  /**
   * Execute: local.set N -> pop the stack into locals[N].
   *
   * Example: local.set 0 (stack has [3])
   *     value = pop() -> 3
   *     locals[0] = 3
   *     stack after: []
   */
  private _execLocalSet(
    instruction: WasmInstruction,
    stack: number[],
    locals: number[],
    pc: number,
    stackBefore: number[]
  ): WasmStepTrace {
    const index = instruction.operand!;
    const value = stack.pop()!;
    locals[index] = value;
    return {
      pc,
      instruction,
      stackBefore,
      stackAfter: [...stack],
      localsSnapshot: [...locals],
      description: `pop ${value}, store in locals[${index}]`,
      halted: false,
    };
  }
}

// ---------------------------------------------------------------------------
// Encoding helpers (mini assembler)
// ---------------------------------------------------------------------------
// These functions produce raw bytes for WASM instructions.
// This is a tiny assembler -- just enough to create test programs.

/**
 * Encode: i32.const value -> 5 bytes (opcode + 4-byte LE signed int).
 *
 * Example:
 *     encodeI32Const(1)  -> [0x41, 0x01, 0x00, 0x00, 0x00]
 *     encodeI32Const(-1) -> [0x41, 0xFF, 0xFF, 0xFF, 0xFF]
 */
export function encodeI32Const(value: number): Uint8Array {
  const buf = new ArrayBuffer(5);
  const view = new DataView(buf);
  view.setUint8(0, OP_I32_CONST);
  view.setInt32(1, value, true); // true = little-endian
  return new Uint8Array(buf);
}

/**
 * Encode: i32.add -> 1 byte.
 *
 * Example:
 *     encodeI32Add() -> [0x6A]
 */
export function encodeI32Add(): Uint8Array {
  return new Uint8Array([OP_I32_ADD]);
}

/**
 * Encode: i32.sub -> 1 byte.
 *
 * Example:
 *     encodeI32Sub() -> [0x6B]
 */
export function encodeI32Sub(): Uint8Array {
  return new Uint8Array([OP_I32_SUB]);
}

/**
 * Encode: local.get index -> 2 bytes (opcode + 1-byte index).
 *
 * Example:
 *     encodeLocalGet(0) -> [0x20, 0x00]
 */
export function encodeLocalGet(index: number): Uint8Array {
  return new Uint8Array([OP_LOCAL_GET, index]);
}

/**
 * Encode: local.set index -> 2 bytes (opcode + 1-byte index).
 *
 * Example:
 *     encodeLocalSet(0) -> [0x21, 0x00]
 */
export function encodeLocalSet(index: number): Uint8Array {
  return new Uint8Array([OP_LOCAL_SET, index]);
}

/**
 * Encode: end -> 1 byte.
 *
 * Example:
 *     encodeEnd() -> [0x0B]
 */
export function encodeEnd(): Uint8Array {
  return new Uint8Array([OP_END]);
}

/**
 * Concatenate encoded WASM instructions into a bytecode program.
 *
 * Unlike RISC-V's assemble() (which packs fixed-width 32-bit words),
 * this just concatenates variable-width byte sequences.
 *
 * Example:
 *     const program = assembleWasm([
 *         encodeI32Const(1),    // push 1
 *         encodeI32Const(2),    // push 2
 *         encodeI32Add(),       // pop 2 and 1, push 3
 *         encodeLocalSet(0),    // pop 3, store in local 0
 *         encodeEnd(),          // halt
 *     ]);
 */
export function assembleWasm(instructions: Uint8Array[]): Uint8Array {
  const totalLength = instructions.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const instr of instructions) {
    result.set(instr, offset);
    offset += instr.length;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Simulator
// ---------------------------------------------------------------------------

/**
 * Complete WASM simulator -- decoder, executor, and execution state.
 *
 * This is a standalone simulator (not wrapping the generic CPU class)
 * because WASM's variable-width instruction fetch is fundamentally
 * different from the CPU's fixed-width 32-bit fetch cycle.
 *
 * State:
 *     - stack:    The operand stack (values pushed/popped by instructions)
 *     - locals:   Local variables (like registers, but accessed via stack)
 *     - pc:       Program counter (byte offset into bytecode)
 *     - bytecode: The raw program bytes
 *     - halted:   Whether execution has finished
 *
 * Example: running x = 1 + 2
 *
 *     const sim = new WasmSimulator(4);
 *     const program = assembleWasm([
 *         encodeI32Const(1),    // push 1
 *         encodeI32Const(2),    // push 2
 *         encodeI32Add(),       // pop 2 and 1, push 3
 *         encodeLocalSet(0),    // pop 3, store in local 0
 *         encodeEnd(),          // halt
 *     ]);
 *     const traces = sim.run(program);
 *     sim.locals[0]; // => 3
 *
 *     Step-by-step stack evolution:
 *         Step 0: i32.const 1    stack: [] -> [1]
 *         Step 1: i32.const 2    stack: [1] -> [1, 2]
 *         Step 2: i32.add        stack: [1, 2] -> [3]
 *         Step 3: local.set 0    stack: [3] -> []       locals[0] = 3
 *         Step 4: end            halt
 */
export class WasmSimulator {
  stack: number[] = [];
  locals: number[];
  pc: number = 0;
  bytecode: Uint8Array = new Uint8Array(0);
  halted: boolean = false;
  cycle: number = 0;
  private _decoder = new WasmDecoder();
  private _executor = new WasmExecutor();

  constructor(numLocals: number = 4) {
    this.locals = new Array(numLocals).fill(0);
  }

  /**
   * Load a WASM bytecode program.
   *
   * Resets the PC to 0 but preserves the stack and locals
   * (call the constructor again for a full reset).
   */
  load(bytecode: Uint8Array): void {
    this.bytecode = bytecode;
    this.pc = 0;
    this.halted = false;
    this.cycle = 0;
    this.stack.length = 0;
    // Reset locals to zero
    for (let i = 0; i < this.locals.length; i++) {
      this.locals[i] = 0;
    }
  }

  /**
   * Execute one WASM instruction and return a trace.
   *
   * The WASM execution cycle:
   *
   *     1. DECODE: Read bytes at PC -> determine opcode and operands
   *     2. EXECUTE: Perform the operation (push/pop stack, read/write locals)
   *     3. ADVANCE: Move PC forward by the instruction's byte width
   *
   * This is simpler than RISC-V's fetch-decode-execute because there's
   * no separate "fetch a fixed-width word" stage -- the decoder reads
   * exactly the bytes it needs directly from the bytecode buffer.
   *
   * Returns:
   *     WasmStepTrace showing the instruction, stack before/after, etc.
   *
   * Throws:
   *     Error if the simulator has halted.
   */
  step(): WasmStepTrace {
    if (this.halted) {
      throw new Error(
        "WASM simulator has halted -- no more instructions to execute"
      );
    }

    // === DECODE ===
    // Read bytes at PC to determine the instruction and its operands.
    // The decoder returns a WasmInstruction with a `size` field telling
    // us how many bytes were consumed.
    const instruction = this._decoder.decode(this.bytecode, this.pc);

    // === EXECUTE ===
    // Perform the operation -- modifies stack and locals in place.
    const trace = this._executor.execute(
      instruction,
      this.stack,
      this.locals,
      this.pc
    );

    // === ADVANCE PC ===
    // Move the program counter forward by the instruction's byte width.
    // (Unlike RISC-V where PC always advances by 4.)
    this.pc += instruction.size;
    this.halted = trace.halted;
    this.cycle += 1;

    return trace;
  }

  /**
   * Load and run a WASM program, returning the execution trace.
   *
   * Returns a list of WasmStepTrace objects -- one for each instruction
   * executed. This gives you the complete execution history with stack
   * snapshots at every step.
   *
   * @param program - Raw bytecode to execute.
   * @param maxSteps - Safety limit to prevent infinite loops.
   * @returns List of WasmStepTrace objects, one per instruction.
   */
  run(program: Uint8Array, maxSteps: number = 10000): WasmStepTrace[] {
    this.load(program);
    const traces: WasmStepTrace[] = [];
    for (let i = 0; i < maxSteps; i++) {
      if (this.halted) break;
      traces.push(this.step());
    }
    return traces;
  }

  getState(): WasmState {
    return Object.freeze({
      stack: freezeArray(this.stack),
      locals: freezeArray(this.locals),
      pc: this.pc,
      halted: this.halted,
      cycle: this.cycle,
    });
  }

  execute(
    program: Uint8Array,
    maxSteps: number = 100_000
  ): ExecutionResult<WasmState> {
    this.load(program);

    const traces: StepTrace[] = [];
    let steps = 0;
    let error: string | null = null;

    try {
      while (!this.halted && steps < maxSteps) {
        const pcBefore = this.pc;
        const trace = this.step();
        traces.push(
          new StepTrace(
            pcBefore,
            this.pc,
            trace.instruction.mnemonic,
            trace.description
          )
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
    const localCount = this.locals.length;
    this.stack = [];
    this.locals = new Array(localCount).fill(0);
    this.pc = 0;
    this.bytecode = new Uint8Array(0);
    this.halted = false;
    this.cycle = 0;
  }
}
