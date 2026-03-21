/**
 * Backend barrel exports -- all seven BLAS backends in one import.
 *
 * === Available Backends ===
 *
 *     CpuBlas     -- pure TypeScript, works everywhere (reference impl)
 *     CudaBlas    -- NVIDIA CUDA (most popular for ML)
 *     MetalBlas   -- Apple Metal (unified memory, Apple Silicon)
 *     VulkanBlas  -- Vulkan (maximum control, cross-platform)
 *     OpenClBlas  -- OpenCL (most portable, any vendor)
 *     WebGpuBlas  -- WebGPU (safe, browser-first)
 *     OpenGlBlas  -- OpenGL (legacy state machine)
 *     GpuBlasBase -- abstract base class for GPU backends
 */

export { CpuBlas } from "./cpu.js";
export { GpuBlasBase } from "./gpu-base.js";
export { CudaBlas } from "./cuda.js";
export { MetalBlas } from "./metal.js";
export { VulkanBlas } from "./vulkan.js";
export { OpenClBlas } from "./opencl.js";
export { WebGpuBlas } from "./webgpu.js";
export { OpenGlBlas } from "./opengl.js";
