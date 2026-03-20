/**
 * Parallel Execution Engine -- Layer 8 of the accelerator computing stack.
 *
 * This package implements five different parallel execution models, showing
 * how different accelerator architectures (GPU, TPU, NPU) organize parallel
 * computation. Each engine takes many processing elements and orchestrates
 * them to execute in parallel.
 *
 *     Layer 9:  gpu-core (one core, one instruction at a time)
 *         |
 *     Layer 8:  parallel-execution-engine (THIS PACKAGE)
 *         |
 *         +-- WarpEngine      -- SIMT (NVIDIA/ARM Mali)
 *         +-- WavefrontEngine -- SIMD (AMD GCN/RDNA)
 *         +-- SystolicArray   -- Dataflow (Google TPU)
 *         +-- MACArrayEngine  -- Scheduled MAC (Apple ANE/NPU)
 *         +-- SubsliceEngine  -- Hybrid SIMD (Intel Xe)
 *
 * Basic usage:
 *     import { WarpEngine, makeWarpConfig, limm, fmul, halt } from "@coding-adventures/parallel-execution-engine";
 *     const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
 *     engine.loadProgram([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]);
 *     const traces = engine.run();
 *     engine.threads[0].core.registers.readFloat(2);  // 6.0
 */

// Protocols and types
export {
  type ParallelExecutionEngine,
  ExecutionModel,
  type EngineTrace,
  type DivergenceInfo,
  makeDivergenceInfo,
  type DataflowInfo,
  makeDataflowInfo,
  formatEngineTrace,
} from "./protocols.js";

// SIMT (NVIDIA/ARM Mali)
export {
  WarpEngine,
  type WarpConfig,
  makeWarpConfig,
  type ThreadContext,
  type DivergenceStackEntry,
} from "./warp-engine.js";

// SIMD (AMD)
export {
  WavefrontEngine,
  type WavefrontConfig,
  makeWavefrontConfig,
  VectorRegisterFile,
  ScalarRegisterFile,
} from "./wavefront-engine.js";

// Systolic (Google TPU)
export {
  SystolicArray,
  type SystolicConfig,
  makeSystolicConfig,
  SystolicPE,
} from "./systolic-array.js";

// Scheduled MAC (NPU)
export {
  MACArrayEngine,
  type MACArrayConfig,
  makeMACArrayConfig,
  type MACScheduleEntry,
  makeMACScheduleEntry,
  MACOperation,
  ActivationFunction,
} from "./mac-array-engine.js";

// Intel Xe
export {
  SubsliceEngine,
  type SubsliceConfig,
  makeSubsliceConfig,
  ExecutionUnit,
} from "./subslice-engine.js";
