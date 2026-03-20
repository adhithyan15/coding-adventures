/**
 * Vendor API Simulators -- six GPU programming APIs over one runtime.
 *
 * This package provides six vendor API simulators, each wrapping the same
 * Vulkan-inspired compute runtime (Layer 5) with different programming models:
 *
 *     CUDA    -- NVIDIA's implicit, "just launch it" model
 *     OpenCL  -- Khronos cross-platform, event-based dependencies
 *     Metal   -- Apple's unified memory, command encoder model
 *     Vulkan  -- Ultra-explicit, maximum control
 *     WebGPU  -- Safe, browser-first, single queue
 *     OpenGL  -- Legacy global state machine
 *
 * === Quick Start ===
 *
 *     // CUDA style (simplest)
 *     import { CUDARuntime, makeCUDAKernel, makeDim3 } from "@coding-adventures/vendor-api-simulators";
 *     const cuda = new CUDARuntime();
 *     const d_x = cuda.malloc(256);
 *     cuda.launchKernel(kernel, makeDim3(1,1,1), makeDim3(32,1,1), [d_x]);
 *     cuda.deviceSynchronize();
 *     cuda.free(d_x);
 *
 *     // Metal style (unified memory)
 *     import { MTLDevice } from "@coding-adventures/vendor-api-simulators";
 *     const device = new MTLDevice();
 *     const buf = device.makeBuffer(256);
 *     buf.writeBytes(data);
 *     const result = buf.contents();
 */

// Base class
export { BaseVendorSimulator } from "./base.js";

// CUDA
export {
  CUDARuntime,
  CUDAStream,
  CUDAEvent,
  CUDAMemcpyKind,
  makeCUDAKernel,
  makeDim3,
  type CUDAKernel,
  type CUDADevicePtr,
  type CUDADeviceProperties,
  type dim3,
} from "./cuda.js";

// OpenCL
export {
  CLPlatform,
  CLDevice,
  CLContext,
  CLCommandQueue,
  CLProgram,
  CLKernel,
  CLBuffer,
  CLEvent,
  CLMemFlags,
  CLDeviceType,
  CLBuildStatus,
  CLEventStatus,
  CLDeviceInfo,
} from "./opencl.js";

// Metal
export {
  MTLDevice,
  MTLCommandQueue,
  MTLCommandBuffer,
  MTLComputeCommandEncoder,
  MTLBlitCommandEncoder,
  MTLBuffer,
  MTLLibrary,
  MTLFunction,
  MTLComputePipelineState,
  MTLResourceOptions,
  MTLCommandBufferStatus,
  makeMTLSize,
  type MTLSize,
} from "./metal.js";

// Vulkan
export {
  VkInstance,
  VkPhysicalDevice,
  VkDevice,
  VkQueue,
  VkCommandPool,
  VkCommandBuffer,
  VkBuffer,
  VkDeviceMemory,
  VkShaderModule,
  VkPipeline,
  VkDescriptorSetLayout,
  VkPipelineLayout,
  VkDescriptorSet,
  VkFence,
  VkSemaphore,
  VkResult,
  VkPipelineBindPoint,
  VkBufferUsageFlagBits,
  VkMemoryPropertyFlagBits,
  VkSharingMode,
  type VkBufferCreateInfo,
  type VkMemoryAllocateInfo,
  type VkShaderModuleCreateInfo,
  type VkComputePipelineCreateInfo,
  type VkPipelineShaderStageCreateInfo,
  type VkSubmitInfo,
  type VkBufferCopy,
  type VkWriteDescriptorSet,
  type VkDescriptorBufferInfo,
  type VkCommandPoolCreateInfo,
  type VkDescriptorSetLayoutCreateInfo,
  type VkDescriptorSetLayoutBinding,
  type VkPipelineLayoutCreateInfo,
  type VkDescriptorSetAllocateInfo,
} from "./vulkan.js";

// WebGPU
export {
  GPU,
  GPUAdapter,
  GPUDevice,
  GPUQueue,
  GPUCommandEncoder,
  GPUComputePassEncoder,
  GPUCommandBuffer,
  GPUBuffer,
  GPUShaderModule,
  GPUComputePipeline,
  GPUBindGroup,
  GPUBindGroupLayout,
  GPUPipelineLayout,
  GPUBufferUsage,
  GPUMapMode,
  type GPUBufferDescriptor,
  type GPUShaderModuleDescriptor,
  type GPUComputePipelineDescriptor,
  type GPUProgrammableStage,
  type GPUBindGroupDescriptor,
  type GPUBindGroupEntry,
  type GPUBindGroupLayoutDescriptor,
  type GPUBindGroupLayoutEntry,
  type GPUBufferBindingLayout,
  type GPUPipelineLayoutDescriptor,
  type GPURequestAdapterOptions,
  type GPUDeviceDescriptor,
  type GPUAdapterLimits,
  type GPUDeviceLimits,
  type GPUComputePassDescriptor,
  type GPUCommandEncoderDescriptor,
} from "./webgpu.js";

// OpenGL
export {
  GLContext,
  GL_COMPUTE_SHADER,
  GL_SHADER_STORAGE_BUFFER,
  GL_ARRAY_BUFFER,
  GL_UNIFORM_BUFFER,
  GL_STATIC_DRAW,
  GL_DYNAMIC_DRAW,
  GL_STREAM_DRAW,
  GL_MAP_READ_BIT,
  GL_MAP_WRITE_BIT,
  GL_SHADER_STORAGE_BARRIER_BIT,
  GL_BUFFER_UPDATE_BARRIER_BIT,
  GL_ALL_BARRIER_BITS,
  GL_ALREADY_SIGNALED,
  GL_CONDITION_SATISFIED,
  GL_TIMEOUT_EXPIRED,
  GL_WAIT_FAILED,
  GL_SYNC_FLUSH_COMMANDS_BIT,
  GL_SYNC_GPU_COMMANDS_COMPLETE,
} from "./opengl.js";
