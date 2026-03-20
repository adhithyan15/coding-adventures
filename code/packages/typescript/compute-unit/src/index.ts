/**
 * Compute Unit -- Layer 7 of the accelerator computing stack.
 *
 * This package implements five different compute unit architectures, showing
 * how different vendors organize parallel execution engines, schedulers,
 * shared memory, and caches into working computational building blocks.
 *
 *     Layer 9:  gpu-core (one core, one instruction at a time)
 *         |
 *     Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
 *         |
 *     Layer 7:  compute-unit (THIS PACKAGE)
 *         |
 *         +-- StreamingMultiprocessor  -- NVIDIA SM
 *         +-- AMDComputeUnit           -- AMD CU (GCN/RDNA)
 *         +-- MatrixMultiplyUnit       -- Google TPU MXU
 *         +-- XeCore                   -- Intel Xe Core
 *         +-- NeuralEngineCore         -- Apple ANE Core
 *
 * Basic usage:
 *     import { StreamingMultiprocessor, makeSMConfig, makeWorkItem } from "@coding-adventures/compute-unit";
 *     import { limm, fmul, halt } from "@coding-adventures/gpu-core";
 *     const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 8 }));
 *     sm.dispatch(makeWorkItem({
 *         workId: 0,
 *         program: [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
 *         threadCount: 64,
 *     }));
 *     const traces = sm.run();
 *     console.log(`Completed in ${traces.length} cycles, occupancy: ${(sm.occupancy * 100).toFixed(1)}%`);
 */

// Protocols and types
export {
  type ComputeUnit,
  Architecture,
  WarpState,
  SchedulingPolicy,
  type WorkItem,
  makeWorkItem,
  type ComputeUnitTrace,
  makeComputeUnitTrace,
  formatComputeUnitTrace,
  SharedMemory,
} from "./protocols.js";

// NVIDIA SM
export {
  StreamingMultiprocessor,
  type SMConfig,
  makeSMConfig,
  type WarpSlot,
  WarpScheduler,
  ResourceError,
} from "./streaming-multiprocessor.js";

// AMD CU
export {
  AMDComputeUnit,
  type AMDCUConfig,
  makeAMDCUConfig,
  type WavefrontSlot,
} from "./amd-compute-unit.js";

// Google TPU MXU
export {
  MatrixMultiplyUnit,
  type MXUConfig,
  makeMXUConfig,
  applyActivation,
} from "./matrix-multiply-unit.js";

// Intel Xe Core
export {
  XeCore,
  type XeCoreConfig,
  makeXeCoreConfig,
} from "./xe-core.js";

// Apple ANE Core
export {
  NeuralEngineCore,
  type ANECoreConfig,
  makeANECoreConfig,
} from "./neural-engine-core.js";
