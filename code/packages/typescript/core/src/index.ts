/**
 * Core -- a complete processor core composing pipeline, caches, branch
 * predictor, and hazard detection into a working CPU.
 *
 * This package integrates all D-series micro-architectural components:
 *
 * - Pipeline (D04): moves instructions through stages
 * - Branch Predictor (D02): guesses branch directions
 * - Hazard Detection (D03): detects data, control, and structural hazards
 * - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
 * - Register File: fast operand storage
 * - Clock: cycle-accurate timing
 *
 * @example
 * ```ts
 * import { Core, simpleConfig, MockDecoder, encodeProgram, encodeADDI, encodeHALT } from "@coding-adventures/core";
 *
 * const core = Core.create(simpleConfig(), new MockDecoder());
 * const program = encodeProgram(encodeADDI(1, 0, 42), encodeHALT());
 * core.loadProgram(program, 0);
 * const stats = core.run(1000);
 * console.log(`IPC: ${stats.ipc().toFixed(3)}`);
 * ```
 */

export {
  cortexA78LikeConfig,
  defaultCoreConfig,
  defaultMultiCoreConfig,
  defaultRegisterFileConfig,
  simpleConfig,
} from "./config.js";
export type {
  CoreConfig,
  FPUnitConfig,
  MultiCoreConfig,
  RegisterFileConfig,
} from "./config.js";
export { Core } from "./core.js";
export {
  MockDecoder,
  encodeADD,
  encodeADDI,
  encodeBRANCH,
  encodeHALT,
  encodeLOAD,
  encodeNOP,
  encodeProgram,
  encodeSTORE,
  encodeSUB,
} from "./decoder.js";
export type { ISADecoder } from "./decoder.js";
export { InterruptController } from "./interrupt-controller.js";
export type {
  AcknowledgedInterrupt,
  PendingInterrupt,
} from "./interrupt-controller.js";
export { MemoryController } from "./memory-controller.js";
export type { MemoryReadResult } from "./memory-controller.js";
export { MultiCoreCPU } from "./multi-core.js";
export { RegisterFile } from "./register-file.js";
export { CoreStats } from "./stats.js";
