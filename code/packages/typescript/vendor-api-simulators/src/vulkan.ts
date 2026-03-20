/**
 * Vulkan Runtime Simulator -- the thinnest wrapper over Layer 5.
 *
 * === What is Vulkan? ===
 *
 * Vulkan is the Khronos Group's low-level, cross-platform GPU API. It's the
 * most explicit GPU API -- you manage everything: memory types, command buffer
 * recording, queue submission, synchronization barriers, descriptor set layouts.
 *
 * Because our Layer 5 compute runtime is already Vulkan-inspired, this
 * simulator is the **thinnest wrapper** of all six. It mainly adds:
 *
 *     1. Vulkan naming conventions (the `vk` prefix on all methods)
 *     2. Vulkan-specific structures (VkBufferCreateInfo, VkSubmitInfo, etc.)
 *     3. VkResult return codes instead of exceptions
 *     4. VkCommandPool for grouping command buffers
 */

import {
  type Buffer,
  type CommandBuffer,
  type CommandQueue,
  type DescriptorSet,
  type DescriptorSetLayout,
  type Fence,
  type LogicalDevice,
  type PhysicalDevice,
  type Pipeline,
  type PipelineLayout,
  type Semaphore,
  type ShaderModule,
  BufferUsage,
  MemoryType,
  PipelineStage,
  RuntimeInstance,
  makePipelineBarrier,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// Vulkan enums
// =========================================================================

/** Vulkan function return codes. */
export enum VkResult {
  SUCCESS = 0,
  NOT_READY = 1,
  TIMEOUT = 2,
  ERROR_OUT_OF_DEVICE_MEMORY = -3,
  ERROR_DEVICE_LOST = -4,
  ERROR_INITIALIZATION_FAILED = -5,
}

/** Which pipeline type to bind. */
export enum VkPipelineBindPoint {
  COMPUTE = "compute",
}

/** Vulkan buffer usage flags. */
export enum VkBufferUsageFlagBits {
  STORAGE_BUFFER = 1,
  UNIFORM_BUFFER = 2,
  TRANSFER_SRC = 4,
  TRANSFER_DST = 8,
}

/** Vulkan memory property flags. */
export enum VkMemoryPropertyFlagBits {
  DEVICE_LOCAL = 1,
  HOST_VISIBLE = 2,
  HOST_COHERENT = 4,
  HOST_CACHED = 8,
}

/** Sharing mode for resources. */
export enum VkSharingMode {
  EXCLUSIVE = "exclusive",
  CONCURRENT = "concurrent",
}

// =========================================================================
// Vulkan create-info structures
// =========================================================================

export interface VkBufferCreateInfo {
  size: number;
  usage: VkBufferUsageFlagBits;
  sharingMode: VkSharingMode;
}

export interface VkMemoryAllocateInfo {
  size: number;
  memoryTypeIndex: number;
}

export interface VkShaderModuleCreateInfo {
  code: unknown[] | null;
}

export interface VkComputePipelineCreateInfo {
  shaderStage: VkPipelineShaderStageCreateInfo | null;
  layout: VkPipelineLayout | null;
}

export interface VkPipelineShaderStageCreateInfo {
  stage: string;
  module: VkShaderModule | null;
  entryPoint: string;
}

export interface VkSubmitInfo {
  commandBuffers: VkCommandBuffer[];
  waitSemaphores: VkSemaphore[];
  signalSemaphores: VkSemaphore[];
}

export interface VkBufferCopy {
  srcOffset: number;
  dstOffset: number;
  size: number;
}

export interface VkWriteDescriptorSet {
  dstSet: VkDescriptorSet | null;
  dstBinding: number;
  descriptorType: string;
  bufferInfo: VkDescriptorBufferInfo | null;
}

export interface VkDescriptorBufferInfo {
  buffer: VkBuffer | null;
  offset: number;
  range: number;
}

export interface VkCommandPoolCreateInfo {
  queueFamilyIndex: number;
}

export interface VkDescriptorSetLayoutCreateInfo {
  bindings: VkDescriptorSetLayoutBinding[];
}

export interface VkDescriptorSetLayoutBinding {
  binding: number;
  descriptorType: string;
  descriptorCount: number;
}

export interface VkPipelineLayoutCreateInfo {
  setLayouts: VkDescriptorSetLayout[];
  pushConstantSize: number;
}

export interface VkDescriptorSetAllocateInfo {
  setLayouts: VkDescriptorSetLayout[];
}

// =========================================================================
// Vulkan wrapper objects
// =========================================================================

/** Vulkan physical device. */
export class VkPhysicalDevice {
  /** @internal */ readonly _physical: PhysicalDevice;

  constructor(physical: PhysicalDevice) {
    this._physical = physical;
  }

  vkGetPhysicalDeviceProperties(): Record<string, unknown> {
    return {
      deviceName: this._physical.name,
      deviceType: this._physical.deviceType,
      vendor: this._physical.vendor,
    };
  }

  vkGetPhysicalDeviceMemoryProperties(): Record<string, unknown> {
    const mp = this._physical.memoryProperties;
    return {
      heapCount: mp.heaps.length,
      heaps: mp.heaps.map((h) => ({ size: h.size, flags: String(h.flags) })),
      isUnified: mp.isUnified,
    };
  }

  vkGetPhysicalDeviceQueueFamilyProperties(): Array<Record<string, unknown>> {
    return this._physical.queueFamilies.map((qf) => ({
      queueType: qf.queueType,
      queueCount: qf.count,
    }));
  }
}

/** Vulkan buffer. */
export class VkBuffer {
  /** @internal */ readonly _buffer: Buffer;

  constructor(buffer: Buffer) {
    this._buffer = buffer;
  }

  get size(): number {
    return this._buffer.size;
  }
}

/** Vulkan device memory. */
export class VkDeviceMemory {
  /** @internal */ readonly _buffer: Buffer;
  /** @internal */ readonly _mm: import("@coding-adventures/compute-runtime").MemoryManager;

  constructor(buffer: Buffer, memoryManager: import("@coding-adventures/compute-runtime").MemoryManager) {
    this._buffer = buffer;
    this._mm = memoryManager;
  }
}

/** Vulkan shader module. */
export class VkShaderModule {
  /** @internal */ readonly _shader: ShaderModule;

  constructor(shader: ShaderModule) {
    this._shader = shader;
  }
}

/** Vulkan pipeline. */
export class VkPipeline {
  /** @internal */ readonly _pipeline: Pipeline;

  constructor(pipeline: Pipeline) {
    this._pipeline = pipeline;
  }
}

/** Vulkan descriptor set layout. */
export class VkDescriptorSetLayout {
  /** @internal */ readonly _layout: DescriptorSetLayout;

  constructor(layout: DescriptorSetLayout) {
    this._layout = layout;
  }
}

/** Vulkan pipeline layout. */
export class VkPipelineLayout {
  /** @internal */ readonly _layout: PipelineLayout;

  constructor(layout: PipelineLayout) {
    this._layout = layout;
  }
}

/** Vulkan descriptor set. */
export class VkDescriptorSet {
  /** @internal */ readonly _ds: DescriptorSet;

  constructor(descriptorSet: DescriptorSet) {
    this._ds = descriptorSet;
  }
}

/** Vulkan fence. */
export class VkFence {
  /** @internal */ readonly _fence: Fence;

  constructor(fence: Fence) {
    this._fence = fence;
  }

  get signaled(): boolean {
    return this._fence.signaled;
  }
}

/** Vulkan semaphore. */
export class VkSemaphore {
  /** @internal */ readonly _semaphore: Semaphore;

  constructor(semaphore: Semaphore) {
    this._semaphore = semaphore;
  }
}

/** Vulkan command pool -- groups command buffers. */
export class VkCommandPool {
  private readonly _device: VkDevice;
  private readonly _queueFamilyIndex: number;
  /** @internal */ readonly _commandBuffers: VkCommandBuffer[] = [];

  constructor(device: VkDevice, queueFamilyIndex: number) {
    this._device = device;
    this._queueFamilyIndex = queueFamilyIndex;
  }

  /** Allocate command buffers from this pool. */
  vkAllocateCommandBuffers(count: number): VkCommandBuffer[] {
    const cbs: VkCommandBuffer[] = [];
    for (let i = 0; i < count; i++) {
      const innerCb = this._device._logical.createCommandBuffer();
      const vkCb = new VkCommandBuffer(innerCb);
      cbs.push(vkCb);
      this._commandBuffers.push(vkCb);
    }
    return cbs;
  }

  /** Reset all command buffers in this pool. */
  vkResetCommandPool(): void {
    for (const vkCb of this._commandBuffers) {
      vkCb._cb.reset();
    }
  }

  /** Free specific command buffers. */
  vkFreeCommandBuffers(buffers: VkCommandBuffer[]): void {
    for (const buf of buffers) {
      const idx = this._commandBuffers.indexOf(buf);
      if (idx !== -1) {
        this._commandBuffers.splice(idx, 1);
      }
    }
  }
}

/** Vulkan command buffer -- wraps Layer 5 CommandBuffer with vk_ prefix. */
export class VkCommandBuffer {
  /** @internal */ readonly _cb: CommandBuffer;

  constructor(cb: CommandBuffer) {
    this._cb = cb;
  }

  vkBeginCommandBuffer(_flags = 0): void {
    this._cb.begin();
  }

  vkEndCommandBuffer(): void {
    this._cb.end();
  }

  vkCmdBindPipeline(_bindPoint: VkPipelineBindPoint, pipeline: VkPipeline): void {
    this._cb.cmdBindPipeline(pipeline._pipeline);
  }

  vkCmdBindDescriptorSets(
    _bindPoint: VkPipelineBindPoint,
    _layout: VkPipelineLayout,
    descriptorSets: VkDescriptorSet[],
  ): void {
    for (const ds of descriptorSets) {
      this._cb.cmdBindDescriptorSet(ds._ds);
    }
  }

  vkCmdPushConstants(_layout: VkPipelineLayout, offset: number, data: Uint8Array): void {
    this._cb.cmdPushConstants(offset, data);
  }

  vkCmdDispatch(x: number, y = 1, z = 1): void {
    this._cb.cmdDispatch(x, y, z);
  }

  vkCmdCopyBuffer(src: VkBuffer, dst: VkBuffer, regions: VkBufferCopy[]): void {
    for (const region of regions) {
      this._cb.cmdCopyBuffer(
        src._buffer,
        dst._buffer,
        region.size,
        region.srcOffset,
        region.dstOffset,
      );
    }
  }

  vkCmdFillBuffer(buffer: VkBuffer, offset: number, size: number, data: number): void {
    this._cb.cmdFillBuffer(buffer._buffer, data, offset, size);
  }

  vkCmdPipelineBarrier(
    srcStage: string,
    dstStage: string,
    _bufferBarriers?: unknown[],
  ): void {
    const barrier = makePipelineBarrier({
      srcStage: srcStage as PipelineStage,
      dstStage: dstStage as PipelineStage,
    });
    this._cb.cmdPipelineBarrier(barrier);
  }
}

/** Vulkan queue. */
export class VkQueue {
  /** @internal */ readonly _queue: CommandQueue;

  constructor(queue: CommandQueue) {
    this._queue = queue;
  }

  /** Submit work to the queue (vkQueueSubmit). */
  vkQueueSubmit(
    submits: VkSubmitInfo[],
    fence?: VkFence,
  ): VkResult {
    for (const submit of submits) {
      const cbs = submit.commandBuffers.map((vkCb) => vkCb._cb);
      const waitSems = submit.waitSemaphores.map((s) => s._semaphore);
      const signalSems = submit.signalSemaphores.map((s) => s._semaphore);

      this._queue.submit(cbs, {
        waitSemaphores: waitSems.length > 0 ? waitSems : undefined,
        signalSemaphores: signalSems.length > 0 ? signalSems : undefined,
        fence: fence ? fence._fence : undefined,
      });
    }
    return VkResult.SUCCESS;
  }

  /** Wait for all queue work to complete. */
  vkQueueWaitIdle(): void {
    this._queue.waitIdle();
  }
}

// =========================================================================
// VkDevice -- wraps LogicalDevice
// =========================================================================

/** Vulkan logical device. */
export class VkDevice {
  /** @internal */ readonly _logical: LogicalDevice;

  constructor(logical: LogicalDevice) {
    this._logical = logical;
  }

  vkGetDeviceQueue(familyIndex: number, queueIndex: number): VkQueue {
    const familyName = familyIndex === 0 ? "compute" : "transfer";
    if (familyName in this._logical.queues) {
      const queues = this._logical.queues[familyName];
      if (queueIndex < queues.length) {
        return new VkQueue(queues[queueIndex]);
      }
    }
    return new VkQueue(this._logical.queues["compute"][0]);
  }

  vkCreateCommandPool(createInfo: VkCommandPoolCreateInfo): VkCommandPool {
    return new VkCommandPool(this, createInfo.queueFamilyIndex);
  }

  vkAllocateMemory(allocInfo: VkMemoryAllocateInfo): VkDeviceMemory {
    let memType: number;
    if (allocInfo.memoryTypeIndex === 0) {
      memType =
        MemoryType.DEVICE_LOCAL |
        MemoryType.HOST_VISIBLE |
        MemoryType.HOST_COHERENT;
    } else {
      memType = MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT;
    }

    const buf = this._logical.memoryManager.allocate(
      allocInfo.size,
      memType,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
    );
    return new VkDeviceMemory(buf, this._logical.memoryManager);
  }

  vkCreateBuffer(createInfo: VkBufferCreateInfo): VkBuffer {
    const memType =
      MemoryType.DEVICE_LOCAL |
      MemoryType.HOST_VISIBLE |
      MemoryType.HOST_COHERENT;
    const buf = this._logical.memoryManager.allocate(
      createInfo.size,
      memType,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
    );
    return new VkBuffer(buf);
  }

  vkBindBufferMemory(
    _buffer: VkBuffer,
    _memory: VkDeviceMemory,
    _offset: number,
  ): void {
    // No-op in simulator
  }

  vkMapMemory(memory: VkDeviceMemory, _offset: number, _size: number): Uint8Array {
    const mapped = memory._mm.map(memory._buffer);
    return new Uint8Array(mapped.getData());
  }

  vkUnmapMemory(memory: VkDeviceMemory): void {
    if (memory._buffer.mapped) {
      memory._mm.unmap(memory._buffer);
    }
  }

  vkCreateShaderModule(createInfo: VkShaderModuleCreateInfo): VkShaderModule {
    const shader = this._logical.createShaderModule({ code: createInfo.code });
    return new VkShaderModule(shader);
  }

  vkCreateDescriptorSetLayout(
    createInfo: VkDescriptorSetLayoutCreateInfo,
  ): VkDescriptorSetLayout {
    const bindings = createInfo.bindings.map((b) =>
      makeDescriptorBinding({
        binding: b.binding,
        type: b.descriptorType,
        count: b.descriptorCount,
      }),
    );
    const layout = this._logical.createDescriptorSetLayout(bindings);
    return new VkDescriptorSetLayout(layout);
  }

  vkCreatePipelineLayout(createInfo: VkPipelineLayoutCreateInfo): VkPipelineLayout {
    const layouts = createInfo.setLayouts.map((sl) => sl._layout);
    const pl = this._logical.createPipelineLayout(layouts, createInfo.pushConstantSize);
    return new VkPipelineLayout(pl);
  }

  vkCreateComputePipelines(createInfos: VkComputePipelineCreateInfo[]): VkPipeline[] {
    const pipelines: VkPipeline[] = [];
    for (const ci of createInfos) {
      const shader =
        ci.shaderStage?.module?._shader ?? null;
      const layout = ci.layout?._layout ?? null;
      if (shader && layout) {
        const p = this._logical.createComputePipeline(shader, layout);
        pipelines.push(new VkPipeline(p));
      }
    }
    return pipelines;
  }

  vkAllocateDescriptorSets(allocInfo: VkDescriptorSetAllocateInfo): VkDescriptorSet[] {
    const sets: VkDescriptorSet[] = [];
    for (const sl of allocInfo.setLayouts) {
      const ds = this._logical.createDescriptorSet(sl._layout);
      sets.push(new VkDescriptorSet(ds));
    }
    return sets;
  }

  vkUpdateDescriptorSets(writes: VkWriteDescriptorSet[]): void {
    for (const write of writes) {
      if (write.dstSet && write.bufferInfo?.buffer) {
        write.dstSet._ds.write(
          write.dstBinding,
          write.bufferInfo.buffer._buffer,
        );
      }
    }
  }

  vkCreateFence(flags = 0): VkFence {
    const signaled = (flags & 1) !== 0;
    const fence = this._logical.createFence(signaled);
    return new VkFence(fence);
  }

  vkCreateSemaphore(): VkSemaphore {
    const sem = this._logical.createSemaphore();
    return new VkSemaphore(sem);
  }

  vkWaitForFences(
    fences: VkFence[],
    waitAll: boolean,
    _timeout: number,
  ): VkResult {
    for (const f of fences) {
      if (f._fence.signaled) {
        if (!waitAll) return VkResult.SUCCESS;
      } else if (waitAll) {
        return VkResult.NOT_READY;
      }
    }
    return VkResult.SUCCESS;
  }

  vkResetFences(fences: VkFence[]): void {
    for (const f of fences) {
      f._fence.reset();
    }
  }

  vkDeviceWaitIdle(): void {
    this._logical.waitIdle();
  }
}

// =========================================================================
// VkInstance -- the Vulkan entry point
// =========================================================================

/** Vulkan instance -- the entry point for device discovery. */
export class VkInstance extends BaseVendorSimulator {
  constructor() {
    super();
  }

  /** Enumerate all physical devices. */
  vkEnumeratePhysicalDevices(): VkPhysicalDevice[] {
    return this._physicalDevices.map((pd) => new VkPhysicalDevice(pd));
  }

  /** Create a logical device. */
  vkCreateDevice(physicalDevice: VkPhysicalDevice): VkDevice {
    const logical = this._instance.createLogicalDevice(physicalDevice._physical);
    return new VkDevice(logical);
  }
}
