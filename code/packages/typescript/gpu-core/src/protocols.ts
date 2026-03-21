/**
 * Protocols -- the pluggable interfaces that make this core vendor-agnostic.
 *
 * === Why Interfaces? ===
 *
 * Every GPU vendor (NVIDIA, AMD, Intel, ARM) and every accelerator type (GPU,
 * TPU, NPU) has a processing element at its heart. They all do the same basic
 * thing: compute floating-point operations. But the details differ:
 *
 *     NVIDIA CUDA Core:     FP32 ALU + 255 registers + PTX instructions
 *     AMD Stream Processor: FP32 ALU + 256 VGPRs + GCN instructions
 *     Intel Vector Engine:  SIMD8 ALU + GRF + Xe instructions
 *     ARM Mali Exec Engine: FP32 ALU + register bank + Mali instructions
 *     TPU Processing Element: MAC unit + weight register + accumulator
 *     NPU MAC Unit:         MAC + activation function + buffer
 *
 * Instead of building separate simulators for each, we define two interfaces:
 *
 * 1. ProcessingElement -- the generic "any compute unit" interface
 * 2. InstructionSet -- the pluggable "how to decode and execute instructions"
 *
 * Any vendor-specific implementation just needs to satisfy these interfaces.
 * The core simulation infrastructure (registers, memory, tracing) is reused.
 *
 * === What is an Interface? ===
 *
 * In TypeScript, an interface defines a contract: "any class that has these
 * methods and properties can be used here." This is structural typing -- if
 * an object has the right shape, TypeScript accepts it, no explicit
 * `implements` keyword required (though you can use it for clarity).
 *
 *     interface Flyable {
 *         fly(): void;
 *     }
 *
 *     class Bird {
 *         fly(): void { console.log("flap flap"); }
 *     }
 *
 *     class Airplane {
 *         fly(): void { console.log("zoom"); }
 *     }
 *
 *     // Both Bird and Airplane satisfy Flyable -- no inheritance needed!
 */

import type { Instruction } from "./opcodes.js";
import type { FPRegisterFile } from "./registers.js";
import type { LocalMemory } from "./memory.js";

// ---------------------------------------------------------------------------
// ExecuteResult -- what an instruction execution produces
// ---------------------------------------------------------------------------

/**
 * The outcome of executing a single instruction.
 *
 * This is what the InstructionSet's execute() method returns. It tells the
 * core what changed so the core can build a complete execution trace.
 *
 * Fields:
 *   description:       Human-readable summary, e.g. "R3 = R1 * R2 = 6.0"
 *   nextPcOffset:      How to advance the program counter.
 *                      +1 for most instructions (next instruction).
 *                      Other values for branches/jumps.
 *   absoluteJump:      If true, nextPcOffset is an absolute address,
 *                      not a relative offset.
 *   registersChanged:  Map of register name -> new float value.
 *   memoryChanged:     Map of memory address -> new float value.
 *   halted:            True if this instruction stops execution.
 */
export interface ExecuteResult {
  readonly description: string;
  readonly nextPcOffset: number;
  readonly absoluteJump: boolean;
  readonly registersChanged: Record<string, number> | null;
  readonly memoryChanged: Record<number, number> | null;
  readonly halted: boolean;
}

/**
 * Create an ExecuteResult with sensible defaults.
 *
 * Most instructions just advance PC by 1, don't jump, and don't halt.
 * This helper lets you specify only the fields that differ from defaults.
 */
export function makeExecuteResult(
  partial: Partial<ExecuteResult> & { description: string },
): ExecuteResult {
  return {
    nextPcOffset: 1,
    absoluteJump: false,
    registersChanged: null,
    memoryChanged: null,
    halted: false,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// InstructionSet -- pluggable ISA (the key to vendor-agnosticism)
// ---------------------------------------------------------------------------

/**
 * A pluggable instruction set that can be swapped to simulate any vendor.
 *
 * === How it works ===
 *
 * The GPUCore calls isa.execute(instruction, registers, memory) for each
 * instruction. The ISA implementation:
 * 1. Reads the opcode to determine what operation to perform
 * 2. Reads source registers and/or memory
 * 3. Performs the computation (using fpAdd, fpMul, fpFma, etc.)
 * 4. Writes the result to the destination register and/or memory
 * 5. Returns an ExecuteResult describing what happened
 *
 * === Implementing a new ISA ===
 *
 * To add support for a new vendor (e.g., NVIDIA PTX):
 *
 *     class PTXISA implements InstructionSet {
 *         get name(): string { return "PTX"; }
 *
 *         execute(instruction, registers, memory): ExecuteResult {
 *             switch (instruction.opcode) {
 *                 case PTXOp.ADD_F32: ...
 *                 case PTXOp.FMA_RN_F32: ...
 *             }
 *         }
 *     }
 *
 *     const core = new GPUCore({ isa: new PTXISA() });
 */
export interface InstructionSet {
  /** The ISA name, e.g. 'Generic', 'PTX', 'GCN', 'Xe', 'Mali'. */
  readonly name: string;

  /** Decode and execute a single instruction. */
  execute(
    instruction: Instruction,
    registers: FPRegisterFile,
    memory: LocalMemory,
  ): ExecuteResult;
}

// ---------------------------------------------------------------------------
// ProcessingElement -- the most generic abstraction
// ---------------------------------------------------------------------------

/**
 * Any compute unit in any accelerator.
 *
 * This is the most generic interface -- a GPU core, a TPU processing element,
 * and an NPU MAC unit all satisfy this interface. It provides just enough
 * structure for a higher-level component (like a warp scheduler or systolic
 * array controller) to drive the PE.
 *
 * === Why so minimal? ===
 *
 * Different accelerators have radically different execution models:
 * - GPUs: instruction-stream + register file (step = execute one instruction)
 * - TPUs: dataflow, no instructions (step = one MAC + pass data to neighbor)
 * - NPUs: scheduled MACs (step = one MAC from the scheduler's queue)
 *
 * This interface captures only what they ALL share: the ability to advance
 * one cycle, check if done, and reset.
 */
export interface ProcessingElement {
  /** Execute one cycle. Returns a trace of what happened. */
  step(): object;

  /** True if this PE has finished execution. */
  readonly halted: boolean;

  /** Reset to initial state. */
  reset(): void;
}
