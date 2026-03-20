/**
 * GPU Core -- generic, pluggable accelerator processing element.
 *
 * This package implements a single GPU processing element (Layer 9 of the
 * accelerator computing stack) with a pluggable instruction set architecture.
 * It sits between FP arithmetic (Layer 10) and the warp/SIMT engine (Layer 8).
 *
 * The core is designed to be vendor-agnostic: swap the InstructionSet to
 * simulate NVIDIA CUDA cores, AMD stream processors, Intel Arc vector engines,
 * ARM Mali execution engines, or any other accelerator.
 *
 * Basic usage:
 *     import { GPUCore, GenericISA, limm, fmul, halt } from "@coding-adventures/gpu-core";
 *     const core = new GPUCore({ isa: new GenericISA() });
 *     core.loadProgram([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
 *     core.run();
 *     core.registers.readFloat(2);  // 12.0
 */

// Core
export { GPUCore, type GPUCoreOptions } from "./core.js";

// ISA
export { GenericISA } from "./generic-isa.js";

// Protocols
export {
  type InstructionSet,
  type ProcessingElement,
  type ExecuteResult,
  makeExecuteResult,
} from "./protocols.js";

// Components
export { FPRegisterFile } from "./registers.js";
export { LocalMemory } from "./memory.js";

// Instructions
export {
  type Instruction,
  Opcode,
  formatInstruction,
  fadd,
  fsub,
  fmul,
  ffma,
  fneg,
  fabsOp,
  load,
  store,
  mov,
  limm,
  beq,
  blt,
  bne,
  jmp,
  nop,
  halt,
} from "./opcodes.js";

// Trace
export { type GPUCoreTrace, formatTrace } from "./trace.js";
