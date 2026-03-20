/**
 * WebGPU Runtime Simulator -- safe, browser-first GPU programming.
 *
 * === What is WebGPU? ===
 *
 * WebGPU is the modern web GPU API, designed to run safely in browsers.
 * It sits on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
 * or D3D12 (Windows), providing a safe, portable abstraction.
 *
 * === Key Simplifications Over Vulkan ===
 *
 * 1. **Single queue** -- `device.queue` is all you get.
 * 2. **Automatic barriers** -- no manual pipeline barriers.
 * 3. **No memory types** -- just usage flags.
 * 4. **Always validated** -- every operation is checked.
 * 5. **Immutable command buffers** -- once finish() is called, it's frozen.
 *
 * === The WebGPU Object Hierarchy ===
 *
 *     GPU (navigator.gpu in browsers)
 *     +-- GPUAdapter (represents a physical device)
 *         +-- GPUDevice (the usable handle)
 *             +-- device.queue (GPUQueue -- single queue!)
 *             +-- createBuffer() -> GPUBuffer
 *             +-- createShaderModule() -> GPUShaderModule
 *             +-- createComputePipeline() -> GPUComputePipeline
 *             +-- createBindGroup() -> GPUBindGroup
 *             +-- createCommandEncoder() -> GPUCommandEncoder
 */

import {
  type Buffer as RuntimeBuffer,
  type DescriptorSet,
  type DescriptorSetLayout,
  type MemoryManager,
  type Pipeline,
  type PipelineLayout,
  type ShaderModule,
  BufferUsage,
  MemoryType,
  RuntimeInstance,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// WebGPU flags
// =========================================================================

/** WebGPU buffer usage flags. */
export enum GPUBufferUsage {
  MAP_READ = 1,
  MAP_WRITE = 2,
  COPY_SRC = 4,
  COPY_DST = 8,
  STORAGE = 16,
  UNIFORM = 32,
}

/** WebGPU buffer map modes. */
export enum GPUMapMode {
  READ = 1,
  WRITE = 2,
}

// =========================================================================
// WebGPU descriptor types
// =========================================================================

export interface GPUBufferDescriptor {
  size: number;
  usage: GPUBufferUsage;
  mappedAtCreation?: boolean;
}

export interface GPUShaderModuleDescriptor {
  code: unknown;
}

export interface GPUProgrammableStage {
  module: GPUShaderModule | null;
  entryPoint?: string;
}

export interface GPUComputePipelineDescriptor {
  layout: string | GPUPipelineLayout;
  compute: GPUProgrammableStage | null;
}

export interface GPUBufferBindingLayout {
  type: string;
}

export interface GPUBindGroupLayoutEntry {
  binding: number;
  visibility: number;
  buffer: GPUBufferBindingLayout;
}

export interface GPUBindGroupLayoutDescriptor {
  entries: GPUBindGroupLayoutEntry[];
}

export interface GPUBindGroupEntry {
  binding: number;
  resource: GPUBuffer | null;
}

export interface GPUBindGroupDescriptor {
  layout: GPUBindGroupLayout | null;
  entries: GPUBindGroupEntry[];
}

export interface GPUPipelineLayoutDescriptor {
  bindGroupLayouts: GPUBindGroupLayout[];
}

export interface GPURequestAdapterOptions {
  powerPreference?: string;
}

export interface GPUDeviceDescriptor {
  requiredFeatures?: string[];
}

export interface GPUAdapterLimits {
  maxBufferSize: number;
  maxComputeWorkgroupSizeX: number;
}

export interface GPUDeviceLimits {
  maxBufferSize: number;
  maxComputeWorkgroupSizeX: number;
}

export interface GPUComputePassDescriptor {
  label?: string;
}

export interface GPUCommandEncoderDescriptor {
  label?: string;
}

// =========================================================================
// WebGPU wrapper objects
// =========================================================================

/**
 * A WebGPU buffer -- memory on the device.
 *
 * WebGPU buffers don't expose memory types. Mapping is async
 * (simulated as sync).
 */
export class GPUBuffer {
  /** @internal */ readonly _buffer: RuntimeBuffer;
  private readonly _mm: MemoryManager;
  private readonly _size: number;
  private readonly _usage: GPUBufferUsage;
  private _mapped = false;
  private _mappedData: Uint8Array | null = null;
  private _destroyed = false;

  constructor(
    buffer: RuntimeBuffer,
    memoryManager: MemoryManager,
    size: number,
    usage: GPUBufferUsage,
  ) {
    this._buffer = buffer;
    this._mm = memoryManager;
    this._size = size;
    this._usage = usage;
  }

  get size(): number {
    return this._size;
  }

  get usage(): GPUBufferUsage {
    return this._usage;
  }

  /** Map the buffer for CPU access (simulated as synchronous). */
  mapAsync(_mode: GPUMapMode, offset = 0, size?: number): void {
    if (this._destroyed) {
      throw new Error("Cannot map a destroyed buffer");
    }
    const actualSize = size ?? this._size;
    this._mm.invalidate(this._buffer);
    const data = this._mm.getBufferData(this._buffer.bufferId);
    this._mappedData = new Uint8Array(data.slice(offset, offset + actualSize));
    this._mapped = true;
  }

  /** Get a view of the mapped buffer data. */
  getMappedRange(offset = 0, size?: number): Uint8Array {
    if (!this._mapped || !this._mappedData) {
      throw new Error("Buffer is not mapped. Call mapAsync() first.");
    }
    const actualSize = size ?? this._mappedData.length;
    return this._mappedData.slice(offset, offset + actualSize);
  }

  /** Unmap the buffer. */
  unmap(): void {
    if (!this._mapped) {
      throw new Error("Buffer is not mapped");
    }
    if (this._mappedData) {
      const mapped = this._mm.map(this._buffer);
      mapped.write(0, this._mappedData);
      this._mm.unmap(this._buffer);
    }
    this._mapped = false;
    this._mappedData = null;
  }

  /** Destroy this buffer. */
  destroy(): void {
    if (!this._destroyed) {
      this._mm.free(this._buffer);
      this._destroyed = true;
    }
  }
}

/** A WebGPU shader module. */
export class GPUShaderModule {
  /** @internal */ readonly _shader: ShaderModule;

  constructor(shader: ShaderModule) {
    this._shader = shader;
  }
}

/** A WebGPU bind group layout. */
export class GPUBindGroupLayout {
  /** @internal */ _layout: DescriptorSetLayout;
  /** @internal */ _isAuto: boolean;

  constructor(layout: DescriptorSetLayout, isAuto: boolean = false) {
    this._layout = layout;
    this._isAuto = isAuto;
  }
}

/** A WebGPU pipeline layout. */
export class GPUPipelineLayout {
  /** @internal */ readonly _layout: PipelineLayout;

  constructor(layout: PipelineLayout) {
    this._layout = layout;
  }
}

/** A WebGPU compute pipeline. */
export class GPUComputePipeline {
  /** @internal */ readonly _pipeline: Pipeline;
  private readonly _bindGroupLayouts: GPUBindGroupLayout[];

  constructor(pipeline: Pipeline, bindGroupLayouts: GPUBindGroupLayout[]) {
    this._pipeline = pipeline;
    this._bindGroupLayouts = bindGroupLayouts;
  }

  /** Get the bind group layout at a given index. */
  getBindGroupLayout(index: number): GPUBindGroupLayout {
    if (index < this._bindGroupLayouts.length) {
      return this._bindGroupLayouts[index];
    }
    throw new Error(`Bind group layout index ${index} out of range`);
  }
}

/** A WebGPU bind group. */
export class GPUBindGroup {
  /** @internal */ readonly _ds: DescriptorSet;

  constructor(ds: DescriptorSet) {
    this._ds = ds;
  }
}

/** A frozen WebGPU command buffer. */
export class GPUCommandBuffer {
  /** @internal */ readonly _cb: import("@coding-adventures/compute-runtime").CommandBuffer;

  constructor(cb: import("@coding-adventures/compute-runtime").CommandBuffer) {
    this._cb = cb;
  }
}

// =========================================================================
// GPUComputePassEncoder
// =========================================================================

/** A WebGPU compute pass encoder. */
export class GPUComputePassEncoder {
  private readonly _encoder: GPUCommandEncoder;
  private _pipeline: GPUComputePipeline | null = null;
  private readonly _bindGroups: Map<number, GPUBindGroup> = new Map();

  constructor(encoder: GPUCommandEncoder) {
    this._encoder = encoder;
  }

  setPipeline(pipeline: GPUComputePipeline): void {
    this._pipeline = pipeline;
  }

  setBindGroup(index: number, bindGroup: GPUBindGroup): void {
    this._bindGroups.set(index, bindGroup);
  }

  dispatchWorkgroups(x: number, y = 1, z = 1): void {
    if (!this._pipeline) {
      throw new Error("No pipeline set");
    }

    const cb = this._encoder._cb;
    cb.cmdBindPipeline(this._pipeline._pipeline);
    const sortedEntries = [...this._bindGroups.entries()].sort(
      (a, b) => a[0] - b[0],
    );
    for (const [, bg] of sortedEntries) {
      cb.cmdBindDescriptorSet(bg._ds);
    }
    cb.cmdDispatch(x, y, z);
  }

  end(): void {
    // No-op
  }
}

// =========================================================================
// GPUCommandEncoder
// =========================================================================

/** A WebGPU command encoder -- builds a GPUCommandBuffer. */
export class GPUCommandEncoder {
  /** @internal */ readonly _device: GPUDevice;
  /** @internal */ readonly _cb: import("@coding-adventures/compute-runtime").CommandBuffer;

  constructor(device: GPUDevice) {
    this._device = device;
    this._cb = device._logicalDevice.createCommandBuffer();
    this._cb.begin();
  }

  /** Begin a compute pass. */
  beginComputePass(_descriptor?: GPUComputePassDescriptor): GPUComputePassEncoder {
    return new GPUComputePassEncoder(this);
  }

  /** Copy data between buffers. */
  copyBufferToBuffer(
    source: GPUBuffer,
    sourceOffset: number,
    destination: GPUBuffer,
    destinationOffset: number,
    size: number,
  ): void {
    this._cb.cmdCopyBuffer(
      source._buffer,
      destination._buffer,
      size,
      sourceOffset,
      destinationOffset,
    );
  }

  /** Finish recording and produce a frozen command buffer. */
  finish(): GPUCommandBuffer {
    this._cb.end();
    return new GPUCommandBuffer(this._cb);
  }
}

// =========================================================================
// GPUQueue
// =========================================================================

/** A WebGPU queue -- the single submission queue. */
export class GPUQueue {
  private readonly _device: GPUDevice;

  constructor(device: GPUDevice) {
    this._device = device;
  }

  /** Submit command buffers for execution. */
  submit(commandBuffers: GPUCommandBuffer[]): void {
    const queue = this._device._computeQueue;
    for (const gpuCb of commandBuffers) {
      const fence = this._device._logicalDevice.createFence();
      queue.submit([gpuCb._cb], { fence });
      fence.wait();
    }
  }

  /** Write data to a buffer (convenience method). */
  writeBuffer(buffer: GPUBuffer, bufferOffset: number, data: Uint8Array): void {
    const mm = this._device._memoryManager;
    const mapped = mm.map(buffer._buffer);
    mapped.write(bufferOffset, data);
    mm.unmap(buffer._buffer);
  }
}

// =========================================================================
// GPUDevice
// =========================================================================

/**
 * A WebGPU device -- the main entry point for GPU programming.
 *
 * Provides device.queue: the single submission queue.
 */
export class GPUDevice extends BaseVendorSimulator {
  readonly queue: GPUQueue;
  readonly features: Set<string> = new Set(["compute"]);
  readonly limits: GPUDeviceLimits = {
    maxBufferSize: 2 * 1024 * 1024 * 1024,
    maxComputeWorkgroupSizeX: 1024,
  };

  constructor(physicalDevice?: import("@coding-adventures/compute-runtime").PhysicalDevice) {
    if (physicalDevice) {
      super({ vendorHint: physicalDevice.vendor });
    } else {
      super();
    }
    this.queue = new GPUQueue(this);
  }

  createBuffer(descriptor: GPUBufferDescriptor): GPUBuffer {
    const memType =
      MemoryType.DEVICE_LOCAL |
      MemoryType.HOST_VISIBLE |
      MemoryType.HOST_COHERENT;
    const usage =
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST;

    const buf = this._memoryManager.allocate(descriptor.size, memType, usage);
    const gpuBuf = new GPUBuffer(
      buf,
      this._memoryManager,
      descriptor.size,
      descriptor.usage,
    );

    if (descriptor.mappedAtCreation) {
      gpuBuf.mapAsync(GPUMapMode.WRITE);
    }

    return gpuBuf;
  }

  createShaderModule(descriptor: GPUShaderModuleDescriptor): GPUShaderModule {
    const code = Array.isArray(descriptor.code) ? descriptor.code : null;
    const shader = this._logicalDevice.createShaderModule({ code });
    return new GPUShaderModule(shader);
  }

  createComputePipeline(descriptor: GPUComputePipelineDescriptor): GPUComputePipeline {
    const shader =
      descriptor.compute?.module?._shader ??
      this._logicalDevice.createShaderModule();

    /**
     * When layout is "auto", we create a placeholder layout that will be
     * replaced when createBindGroup is called. This mirrors how real WebGPU
     * implementations infer layouts from shader reflection.
     */
    const dsLayout = this._logicalDevice.createDescriptorSetLayout([]);
    const plLayout = this._logicalDevice.createPipelineLayout([dsLayout]);
    const pipeline = this._logicalDevice.createComputePipeline(shader, plLayout);

    const bgLayout = new GPUBindGroupLayout(dsLayout, true);
    return new GPUComputePipeline(pipeline, [bgLayout]);
  }

  createBindGroupLayout(descriptor: GPUBindGroupLayoutDescriptor): GPUBindGroupLayout {
    const bindings = descriptor.entries.map((e) =>
      makeDescriptorBinding({
        binding: e.binding,
        type: e.buffer?.type ?? "storage",
      }),
    );
    const layout = this._logicalDevice.createDescriptorSetLayout(bindings);
    return new GPUBindGroupLayout(layout);
  }

  createPipelineLayout(descriptor: GPUPipelineLayoutDescriptor): GPUPipelineLayout {
    const layouts = descriptor.bindGroupLayouts.map((bg) => bg._layout);
    const pl = this._logicalDevice.createPipelineLayout(layouts);
    return new GPUPipelineLayout(pl);
  }

  createBindGroup(descriptor: GPUBindGroupDescriptor): GPUBindGroup {
    /**
     * If the layout was auto-generated (from createComputePipeline with
     * layout: "auto"), we need to build a real layout from the entries.
     * This mirrors how WebGPU implementations infer layouts from shader
     * reflection — the "auto" layout adapts to whatever bindings are used.
     */
    let layout: DescriptorSetLayout;
    if (descriptor.layout && !descriptor.layout._isAuto) {
      layout = descriptor.layout._layout;
    } else {
      const bindings = descriptor.entries.map((e) =>
        makeDescriptorBinding({ binding: e.binding, type: "storage" }),
      );
      layout = this._logicalDevice.createDescriptorSetLayout(bindings);
      // Update the auto layout so subsequent calls are consistent
      if (descriptor.layout && descriptor.layout._isAuto) {
        descriptor.layout._layout = layout;
        descriptor.layout._isAuto = false;
      }
    }
    const ds = this._logicalDevice.createDescriptorSet(layout);
    for (const entry of descriptor.entries) {
      if (entry.resource) {
        ds.write(entry.binding, entry.resource._buffer);
      }
    }
    return new GPUBindGroup(ds);
  }

  createCommandEncoder(_descriptor?: GPUCommandEncoderDescriptor): GPUCommandEncoder {
    return new GPUCommandEncoder(this);
  }

  destroy(): void {
    this._logicalDevice.waitIdle();
  }
}

// =========================================================================
// GPUAdapter
// =========================================================================

/** A WebGPU adapter -- represents a physical GPU. */
export class GPUAdapter {
  /** @internal */ readonly _physical: import("@coding-adventures/compute-runtime").PhysicalDevice;
  readonly features: Set<string> = new Set(["compute"]);
  readonly limits: GPUAdapterLimits = {
    maxBufferSize: 2 * 1024 * 1024 * 1024,
    maxComputeWorkgroupSizeX: 1024,
  };

  constructor(physicalDevice: import("@coding-adventures/compute-runtime").PhysicalDevice) {
    this._physical = physicalDevice;
  }

  get name(): string {
    return this._physical.name;
  }

  /** Request a device from this adapter. */
  requestDevice(_descriptor?: GPUDeviceDescriptor): GPUDevice {
    return new GPUDevice(this._physical);
  }
}

// =========================================================================
// GPU -- the top-level entry point
// =========================================================================

/** The WebGPU entry point -- like navigator.gpu in browsers. */
export class GPU {
  private readonly _runtimeInstance: RuntimeInstance;
  private readonly _physicalDevices: import("@coding-adventures/compute-runtime").PhysicalDevice[];

  constructor() {
    this._runtimeInstance = new RuntimeInstance();
    this._physicalDevices = this._runtimeInstance.enumeratePhysicalDevices();
  }

  /** Request a GPU adapter. */
  requestAdapter(options?: GPURequestAdapterOptions): GPUAdapter {
    if (this._physicalDevices.length === 0) {
      throw new Error("No GPU adapters available");
    }

    if (options?.powerPreference === "low-power") {
      for (const pd of this._physicalDevices) {
        if (pd.memoryProperties.isUnified) {
          return new GPUAdapter(pd);
        }
      }
    }

    return new GPUAdapter(this._physicalDevices[0]);
  }
}
