/**
 * GPUCore -- the generic, pluggable accelerator processing element.
 *
 * === What is a GPU Core? ===
 *
 * A GPU core is the smallest independently programmable compute unit on a GPU.
 * It's like a tiny, simplified CPU that does one thing well: floating-point math.
 *
 *     CPU Core (complex):                    GPU Core (simple):
 *     +-------------------------+            +-----------------------+
 *     | Branch predictor        |            |                       |
 *     | Out-of-order engine     |            | In-order execution    |
 *     | Large cache hierarchy   |            | Small register file   |
 *     | Integer + FP ALUs       |            | FP ALU only           |
 *     | Complex decoder         |            | Simple fetch-execute  |
 *     | Speculative execution   |            | No speculation        |
 *     +-------------------------+            +-----------------------+
 *
 * A single GPU core is MUCH simpler than a CPU core. GPUs achieve performance
 * not through per-core complexity, but through massive parallelism: thousands
 * of these simple cores running in parallel.
 *
 * === How This Core is Pluggable ===
 *
 * The GPUCore takes an InstructionSet as a constructor parameter. This ISA
 * object handles all the vendor-specific decode and execute logic:
 *
 *     // Generic educational ISA (this package)
 *     const core = new GPUCore({ isa: new GenericISA() });
 *
 *     // NVIDIA PTX (future package)
 *     const core = new GPUCore({ isa: new PTXISA(), numRegisters: 255 });
 *
 *     // AMD GCN (future package)
 *     const core = new GPUCore({ isa: new GCNISA(), numRegisters: 256 });
 *
 * The core itself (fetch loop, registers, memory, tracing) stays the same.
 * Only the ISA changes.
 *
 * === Execution Model ===
 *
 * The GPU core uses a simple fetch-execute loop (no separate decode stage):
 *
 *     +------------------------------------------+
 *     |              GPU Core                     |
 *     |                                           |
 *     |  +-----------+    +-------------------+   |
 *     |  | Program   |--->|   Fetch           |   |
 *     |  | Memory    |    |   instruction     |   |
 *     |  +-----------+    |   at PC           |   |
 *     |                   +--------+----------+   |
 *     |                            |              |
 *     |                   +--------v----------+   |
 *     |  +------------+   |   ISA.execute()   |   |
 *     |  | Register   |<--|   (pluggable!)    |-->| Trace
 *     |  | File       |-->|                   |   |
 *     |  +------------+   +--------+----------+   |
 *     |                            |              |
 *     |  +------------+   +--------v----------+   |
 *     |  |  Local     |<--|  Update PC        |   |
 *     |  |  Memory    |   +-------------------+   |
 *     |  +------------+                           |
 *     +------------------------------------------+
 *
 * Each step():
 * 1. Fetch: read instruction at program[PC]
 * 2. Execute: call isa.execute(instruction, registers, memory)
 * 3. Update PC: advance based on ExecuteResult (branch or +1)
 * 4. Return trace: GPUCoreTrace with full execution details
 */

import { type FloatFormat, FP32 } from "@coding-adventures/fp-arithmetic";

import { GenericISA } from "./generic-isa.js";
import { LocalMemory } from "./memory.js";
import type { Instruction } from "./opcodes.js";
import type { InstructionSet, ProcessingElement } from "./protocols.js";
import { FPRegisterFile } from "./registers.js";
import type { GPUCoreTrace } from "./trace.js";

/**
 * Configuration options for creating a GPUCore.
 *
 * All fields are optional -- reasonable defaults are used for anything
 * not specified.
 */
export interface GPUCoreOptions {
  /** The instruction set to use (default: GenericISA). */
  isa?: InstructionSet;
  /** Floating-point format for registers (default: FP32). */
  fmt?: FloatFormat;
  /** Number of FP registers (default: 32, max: 256). */
  numRegisters?: number;
  /** Local memory size in bytes (default: 4096). */
  memorySize?: number;
}

export class GPUCore implements ProcessingElement {
  /**
   * A generic GPU processing element with a pluggable instruction set.
   *
   * This is the central class of the package. It simulates a single
   * processing element -- one CUDA core, one AMD stream processor, one
   * Intel vector engine, or one ARM Mali execution engine -- depending
   * on which InstructionSet you plug in.
   *
   * Example:
   *     import { GPUCore, GenericISA, limm, fmul, halt } from "@coding-adventures/gpu-core";
   *     const core = new GPUCore({ isa: new GenericISA() });
   *     core.loadProgram([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
   *     const traces = core.run();
   *     core.registers.readFloat(2);  // 12.0
   */

  /** The pluggable instruction set. */
  readonly isa: InstructionSet;

  /** The floating-point format used by registers. */
  readonly fmt: FloatFormat;

  /** The core's register file. */
  registers: FPRegisterFile;

  /** The core's local scratchpad memory. */
  memory: LocalMemory;

  /** Program counter -- index of the next instruction to execute. */
  pc: number = 0;

  /** Clock cycle counter -- increments with each step(). */
  cycle: number = 0;

  /** Whether the core has halted. */
  private _halted: boolean = false;

  /** The loaded program (list of instructions). */
  private _program: Instruction[] = [];

  constructor(options: GPUCoreOptions = {}) {
    this.isa = options.isa ?? new GenericISA();
    this.fmt = options.fmt ?? FP32;
    this.registers = new FPRegisterFile(options.numRegisters ?? 32, this.fmt);
    this.memory = new LocalMemory(options.memorySize ?? 4096);
  }

  /** True if the core has executed a HALT instruction. */
  get halted(): boolean {
    return this._halted;
  }

  /**
   * Load a program (list of instructions) into the core.
   *
   * This replaces any previously loaded program and resets the PC to 0,
   * but does NOT reset registers or memory. Call reset() for a full reset.
   */
  loadProgram(program: Instruction[]): void {
    this._program = [...program];
    this.pc = 0;
    this._halted = false;
    this.cycle = 0;
  }

  /**
   * Execute one instruction and return a trace of what happened.
   *
   * This is the core fetch-execute loop:
   * 1. Check if halted or PC out of range
   * 2. Fetch instruction at PC
   * 3. Call ISA.execute() to perform the operation
   * 4. Update PC based on the result
   * 5. Build and return a trace record
   */
  step(): GPUCoreTrace {
    if (this._halted) {
      throw new Error("Cannot step: core is halted");
    }

    if (this.pc < 0 || this.pc >= this._program.length) {
      throw new Error(
        `PC=${this.pc} out of program range [0, ${this._program.length})`,
      );
    }

    // Fetch
    const instruction = this._program[this.pc];
    const currentPc = this.pc;
    this.cycle += 1;

    // Execute (delegated to the pluggable ISA)
    const result = this.isa.execute(instruction, this.registers, this.memory);

    // Update PC
    let nextPc: number;
    if (result.halted) {
      this._halted = true;
      nextPc = currentPc; // PC doesn't advance on halt
    } else if (result.absoluteJump) {
      nextPc = result.nextPcOffset;
    } else {
      nextPc = currentPc + result.nextPcOffset;
    }
    this.pc = nextPc;

    // Build trace
    return {
      cycle: this.cycle,
      pc: currentPc,
      instruction,
      description: result.description,
      nextPc,
      halted: result.halted,
      registersChanged: result.registersChanged ?? {},
      memoryChanged: result.memoryChanged ?? {},
    };
  }

  /**
   * Execute the program until HALT or max_steps reached.
   *
   * This repeatedly calls step() until the core halts or the step
   * limit is reached (preventing infinite loops from hanging).
   */
  run(maxSteps: number = 10000): GPUCoreTrace[] {
    const traces: GPUCoreTrace[] = [];
    let steps = 0;

    while (!this._halted && steps < maxSteps) {
      traces.push(this.step());
      steps += 1;
    }

    if (!this._halted && steps >= maxSteps) {
      throw new Error(
        `Execution limit reached (${maxSteps} steps). ` +
          `Possible infinite loop. Last PC=${this.pc}`,
      );
    }

    return traces;
  }

  /**
   * Reset the core to its initial state.
   *
   * Clears registers, memory, PC, and cycle count. The loaded program
   * is preserved -- call loadProgram() to change it.
   */
  reset(): void {
    this.registers = new FPRegisterFile(this.registers.numRegisters, this.fmt);
    this.memory = new LocalMemory(this.memory.size);
    this.pc = 0;
    this.cycle = 0;
    this._halted = false;
  }

  /** String representation for debugging. */
  toString(): string {
    const status = this._halted
      ? "halted"
      : `running at PC=${this.pc}`;
    return (
      `GPUCore(isa=${this.isa.name}, ` +
      `regs=${this.registers.numRegisters}, ` +
      `fmt=${this.fmt.name}, ` +
      `${status})`
    );
  }
}
