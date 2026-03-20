/**
 * BLAS Library -- pluggable linear algebra with 7 swappable backends.
 *
 * === Quick Start ===
 *
 *     import { createBlas, Matrix, Vector, Transpose } from "@coding-adventures/blas-library";
 *
 *     const blas = createBlas("auto");   // Best available backend
 *     const A = new Matrix([1,2,3,4], 2, 2);
 *     const B = new Matrix([5,6,7,8], 2, 2);
 *     const C = new Matrix([0,0,0,0], 2, 2);
 *     const result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, A, B, 0.0, C);
 *
 * === Available Backends ===
 *
 *     "cpu"     -- pure TypeScript (always works, reference implementation)
 *     "cuda"    -- NVIDIA CUDA (most popular for ML)
 *     "metal"   -- Apple Metal (unified memory)
 *     "vulkan"  -- Vulkan (maximum control)
 *     "opencl"  -- OpenCL (most portable)
 *     "webgpu"  -- WebGPU (browser-first)
 *     "opengl"  -- OpenGL (legacy)
 *
 * === Architecture ===
 *
 * The library follows a plugin architecture:
 *
 *     1. BlasBackend interface  -- the contract every backend must fulfill
 *     2. MlBlasBackend         -- optional ML extensions (activations, attention)
 *     3. BackendRegistry       -- discovers and selects backends
 *     4. createBlas()          -- convenience function for users
 *     5. Seven backends        -- CPU + 6 GPU backends using Layer 4 vendor APIs
 */

// Core types (Matrix, Vector, enums)
export {
  StorageOrder,
  Transpose,
  Side,
  Vector,
  Matrix,
  fromMatrixPkg,
  toMatrixPkg,
  type MatrixPkgLike,
} from "./types.js";

// Protocol interfaces
export type { BlasBackend, MlBlasBackend } from "./protocol.js";

// Registry
export {
  BackendRegistry,
  globalRegistry,
  type BackendFactory,
} from "./registry.js";

// Convenience API
export { createBlas, useBackend } from "./convenience.js";

// All backends
export {
  CpuBlas,
  GpuBlasBase,
  CudaBlas,
  MetalBlas,
  VulkanBlas,
  OpenClBlas,
  WebGpuBlas,
  OpenGlBlas,
} from "./backends/index.js";

// =========================================================================
// Auto-register all backends in the global registry
// =========================================================================

import { globalRegistry } from "./registry.js";
import { CpuBlas } from "./backends/cpu.js";
import { CudaBlas } from "./backends/cuda.js";
import { MetalBlas } from "./backends/metal.js";
import { VulkanBlas } from "./backends/vulkan.js";
import { OpenClBlas } from "./backends/opencl.js";
import { WebGpuBlas } from "./backends/webgpu.js";
import { OpenGlBlas } from "./backends/opengl.js";

/**
 * Register all seven backends in the global registry.
 *
 * This happens at module import time, so by the time the user calls
 * `createBlas("auto")`, all backends are ready to be tried.
 *
 * The priority order (set in BackendRegistry) is:
 *     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
 */
globalRegistry.register("cpu", CpuBlas);
globalRegistry.register("cuda", CudaBlas);
globalRegistry.register("metal", MetalBlas);
globalRegistry.register("vulkan", VulkanBlas);
globalRegistry.register("opencl", OpenClBlas);
globalRegistry.register("webgpu", WebGpuBlas);
globalRegistry.register("opengl", OpenGlBlas);
