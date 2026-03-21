/**
 * Device Simulator -- Layer 6 of the accelerator computing stack.
 *
 * This package simulates complete accelerator devices, assembling multiple
 * compute units (Layer 7) with global memory, L2 cache, and work distribution
 * into full devices that can launch and execute kernels.
 *
 *     Layer 9:  gpu-core (one core, one instruction at a time)
 *         |
 *     Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
 *         |
 *     Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
 *         |
 *     Layer 6:  device-simulator (THIS PACKAGE)
 *         |
 *         +-- NvidiaGPU       -- many SMs + HBM + L2 + GigaThread
 *         +-- AmdGPU          -- CUs in Shader Engines + Infinity Cache
 *         +-- GoogleTPU       -- Scalar/Vector/MXU pipeline + HBM
 *         +-- IntelGPU        -- Xe-Cores in Xe-Slices + L2
 *         +-- AppleANE        -- NE cores + SRAM + DMA + unified memory
 *
 * Basic usage:
 *     import { NvidiaGPU, makeKernelDescriptor } from "@coding-adventures/device-simulator";
 *     import { limm, halt } from "@coding-adventures/gpu-core";
 *     const gpu = new NvidiaGPU({ numSMs: 4 });
 *     gpu.launchKernel(makeKernelDescriptor({
 *         name: "test",
 *         program: [limm(0, 42.0), halt()],
 *         gridDim: [2, 1, 1],
 *         blockDim: [32, 1, 1],
 *     }));
 *     const traces = gpu.run(1000);
 *     console.log(`Completed in ${traces.length} cycles`);
 */

// Devices
export { NvidiaGPU } from "./nvidia-gpu.js";
export { AmdGPU, ShaderEngine } from "./amd-gpu.js";
export { GoogleTPU } from "./google-tpu.js";
export { IntelGPU, XeSlice } from "./intel-gpu.js";
export { AppleANE } from "./apple-ane.js";

// Protocols and types
export {
  type AcceleratorDevice,
  type DeviceConfig,
  type DeviceTrace,
  type DeviceStats,
  type KernelDescriptor,
  type GlobalMemoryStats,
  type MemoryTransaction,
  // Vendor-specific configs
  type AmdGPUConfig,
  type ShaderEngineConfig,
  type IntelGPUConfig,
  type XeSliceConfig,
  type TPUConfig,
  type ICILink,
  type ANEConfig,
  // Factory functions
  makeDeviceConfig,
  makeDeviceTrace,
  makeDeviceStats,
  makeKernelDescriptor,
  makeGlobalMemoryStats,
  makeAmdGPUConfig,
  makeIntelGPUConfig,
  makeTPUConfig,
  makeANEConfig,
  makeShaderEngineConfig,
  makeXeSliceConfig,
  // Helpers
  totalThreads,
  totalBlocks,
  threadsPerBlock,
  formatDeviceTrace,
  updateEfficiency,
} from "./protocols.js";

// Components
export { SimpleGlobalMemory } from "./global-memory.js";
export {
  GPUWorkDistributor,
  TPUSequencer,
  ANEScheduleReplayer,
  type TileOperation,
  type ScheduleEntry,
} from "./work-distributor.js";
