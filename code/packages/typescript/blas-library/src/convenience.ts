/**
 * Convenience API -- simple functions for common usage.
 *
 * === The Simplest Way to Use BLAS ===
 *
 * Instead of manually creating backends and calling methods:
 *
 *     import { CpuBlas } from "@coding-adventures/blas-library";
 *     const blas = new CpuBlas();
 *     const result = blas.sgemm(...);
 *
 * You can use the convenience API:
 *
 *     import { createBlas } from "@coding-adventures/blas-library";
 *
 *     const blas = createBlas("auto");     // Best available backend
 *     const blas = createBlas("cuda");     // Specific backend
 *     const blas = createBlas("cpu");      // CPU fallback
 */

import type { BlasBackend } from "./protocol.js";
import { globalRegistry } from "./registry.js";

/**
 * Create a BLAS instance with the specified backend.
 *
 * ================================================================
 * CREATE A BLAS BACKEND INSTANCE
 * ================================================================
 *
 * This is the main entry point for the BLAS library. It creates
 * and returns a backend instance:
 *
 *     "auto"   -- selects the best available backend by priority
 *     "cuda"   -- NVIDIA GPU
 *     "metal"  -- Apple Silicon
 *     "vulkan" -- any Vulkan-capable GPU
 *     "opencl" -- any OpenCL device
 *     "webgpu" -- WebGPU-capable device
 *     "opengl" -- OpenGL 4.3+ device
 *     "cpu"    -- pure TypeScript fallback (always works)
 *
 * @param backendName - Which backend to use. Default "auto".
 * @returns An instantiated BlasBackend.
 * @throws Error if the requested backend is not available.
 * ================================================================
 */
export function createBlas(backendName: string = "auto"): BlasBackend {
  if (backendName === "auto") {
    return globalRegistry.getBest();
  }
  return globalRegistry.get(backendName);
}

/**
 * Create a backend for temporary use.
 *
 * ================================================================
 * BACKEND CREATION FOR TEMPORARY USE
 * ================================================================
 *
 * TypeScript doesn't have Python's context managers, so we simply
 * return the backend. The caller can use it in a block scope:
 *
 *     {
 *         const blas = useBackend("cpu");
 *         const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 *     }  // blas goes out of scope
 *
 * ================================================================
 */
export function useBackend(name: string): BlasBackend {
  return createBlas(name);
}
