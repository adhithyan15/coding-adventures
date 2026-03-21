/**
 * CPU Simulator -- Layer 3 of the computing stack.
 *
 * Simulates the core of a processor: registers, memory, program counter,
 * and the fetch-decode-execute cycle that drives all computation.
 *
 * This is a generic CPU model -- not tied to any specific architecture.
 * The ISA simulators (RISC-V, ARM, WASM, Intel 4004) build on top of this
 * by providing their own instruction decoders.
 */

export { CPU } from "./cpu.js";
export type { CPUState, InstructionDecoder, InstructionExecutor } from "./cpu.js";
export { Memory } from "./memory.js";
export {
  PipelineStage,
  formatPipeline,
} from "./pipeline.js";
export type {
  DecodeResult,
  ExecuteResult,
  FetchResult,
  PipelineTrace,
} from "./pipeline.js";
export { RegisterFile } from "./registers.js";
export { SparseMemory } from "./sparse-memory.js";
export type { MemoryRegionConfig } from "./sparse-memory.js";
