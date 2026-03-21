/**
 * Compute Runtime -- Layer 5 of the accelerator computing stack.
 *
 * A low-level Vulkan-inspired compute runtime that provides the software
 * infrastructure between user-facing APIs (CUDA, OpenCL, Metal, Vulkan)
 * and the hardware device simulators (Layer 6).
 *
 * === Quick Start ===
 *
 *     import {
 *       RuntimeInstance, MemoryType, BufferUsage, PipelineStage,
 *     } from "@coding-adventures/compute-runtime";
 *     import { limm, halt } from "@coding-adventures/gpu-core";
 *
 *     // 1. Discover devices
 *     const instance = new RuntimeInstance();
 *     const devices = instance.enumeratePhysicalDevices();
 *     const nvidia = devices.find(d => d.vendor === "nvidia")!;
 *
 *     // 2. Create logical device
 *     const device = instance.createLogicalDevice(nvidia);
 *     const queue = device.queues["compute"][0];
 *     const mm = device.memoryManager;
 *
 *     // 3. Allocate buffers
 *     const buf = mm.allocate(
 *       256,
 *       MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE,
 *       BufferUsage.STORAGE,
 *     );
 *
 *     // 4. Create pipeline
 *     const shader = device.createShaderModule({ code: [limm(0, 42.0), halt()] });
 *     const dsLayout = device.createDescriptorSetLayout([]);
 *     const plLayout = device.createPipelineLayout([dsLayout]);
 *     const pipeline = device.createComputePipeline(shader, plLayout);
 *
 *     // 5. Record and submit commands
 *     const cb = device.createCommandBuffer();
 *     cb.begin();
 *     cb.cmdBindPipeline(pipeline);
 *     cb.cmdDispatch(1, 1, 1);
 *     cb.end();
 *
 *     const fence = device.createFence();
 *     queue.submit([cb], { fence });
 *     fence.wait();
 *
 * === Architecture ===
 *
 *     RuntimeInstance
 *     +-- enumeratePhysicalDevices() -> PhysicalDevice[]
 *     +-- createLogicalDevice() -> LogicalDevice
 *         +-- queues: CommandQueue[]
 *         +-- memoryManager: MemoryManager
 *         +-- createCommandBuffer() -> CommandBuffer
 *         +-- createComputePipeline() -> Pipeline
 *         +-- createFence() -> Fence
 *         +-- createSemaphore() -> Semaphore
 */

// Protocols and types
export {
  DeviceType,
  QueueType,
  MemoryType,
  BufferUsage,
  PipelineStage,
  AccessFlags,
  CommandBufferState,
  RuntimeEventType,
  hasMemoryType,
  hasBufferUsage,
  makeDeviceLimits,
  makeDescriptorBinding,
  makePipelineBarrier,
  makeRuntimeTrace,
  formatRuntimeTrace,
  makeRuntimeStats,
  updateUtilization,
  type QueueFamily,
  type DeviceLimits,
  type MemoryHeap,
  type MemoryProperties,
  type DescriptorBinding,
  type RecordedCommand,
  type MemoryBarrier,
  type BufferBarrier,
  type PipelineBarrier,
  type RuntimeTrace,
  type RuntimeStats,
} from "./protocols.js";

// Instance and device management
export {
  PhysicalDevice,
  LogicalDevice,
  RuntimeInstance,
} from "./instance.js";

// Memory management
export {
  type Buffer,
  makeBuffer,
  MappedMemory,
  MemoryManager,
} from "./memory.js";

// Command recording and submission
export { CommandBuffer } from "./command-buffer.js";
export { CommandQueue } from "./command-queue.js";

// Pipeline and descriptors
export {
  ShaderModule,
  DescriptorSetLayout,
  PipelineLayout,
  Pipeline,
  DescriptorSet,
} from "./pipeline.js";

// Synchronization
export {
  Fence,
  Semaphore,
  Event,
} from "./sync.js";

// Validation
export {
  ValidationError,
  ValidationLayer,
} from "./validation.js";
