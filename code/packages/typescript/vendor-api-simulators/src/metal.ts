/**
 * Metal Runtime Simulator -- Apple's unified memory GPU programming model.
 *
 * === What is Metal? ===
 *
 * Metal is Apple's GPU API, designed exclusively for Apple hardware. Its key
 * innovation is **unified memory** -- on Apple Silicon (M1/M2/M3/M4), the
 * CPU and GPU share the same physical RAM. This eliminates host-to-device copies.
 *
 * === The Command Encoder Model ===
 *
 * Metal uses a distinctive pattern for recording GPU commands:
 *
 *     1. Get a command buffer from the command queue
 *     2. Create a **command encoder** (compute, blit, render)
 *     3. Record commands into the encoder
 *     4. End the encoder
 *     5. Commit the command buffer
 *
 * === Unified Memory ===
 *
 *     CUDA:   cudaMalloc -> device-only, need cudaMemcpy
 *     Metal:  makeBuffer -> unified, buffer.contents() gives CPU access directly
 */

import {
  type Buffer as RuntimeBuffer,
  type Fence,
  type MemoryManager,
  BufferUsage,
  MemoryType,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// Metal-specific types
// =========================================================================

/**
 * MTLSize -- grid/threadgroup dimensions in Metal.
 *
 * Metal uses (width, height, depth) instead of (x, y, z).
 */
export interface MTLSize {
  readonly width: number;
  readonly height: number;
  readonly depth: number;
}

/** Create an MTLSize. */
export function makeMTLSize(width: number, height: number, depth: number): MTLSize {
  return { width, height, depth };
}

/** Metal storage mode options for buffers. */
export enum MTLResourceOptions {
  storageModeShared = "shared",
  storageModePrivate = "private",
  storageModeManaged = "managed",
}

/** Status of a Metal command buffer. */
export enum MTLCommandBufferStatus {
  notEnqueued = "notEnqueued",
  enqueued = "enqueued",
  committed = "committed",
  scheduled = "scheduled",
  completed = "completed",
  error = "error",
}

// =========================================================================
// MTLBuffer -- unified memory buffer
// =========================================================================

/**
 * A Metal buffer -- always accessible from both CPU and GPU.
 *
 * Because Apple Silicon uses unified memory, you can:
 *
 *     const buf = device.makeBuffer(1024);
 *     buf.writeBytes(data);             // CPU writes directly
 *     // ... GPU computes on buf ...
 *     const result = buf.contents();    // CPU reads directly
 */
export class MTLBuffer {
  /** @internal */ readonly _buffer: RuntimeBuffer;
  private readonly _mm: MemoryManager;
  private readonly _length: number;

  constructor(buffer: RuntimeBuffer, memoryManager: MemoryManager, length: number) {
    this._buffer = buffer;
    this._mm = memoryManager;
    this._length = length;
  }

  /** Buffer size in bytes. */
  get length(): number {
    return this._length;
  }

  /**
   * Get CPU-accessible view of the buffer contents.
   *
   * In real Metal, this returns a raw pointer to shared memory.
   */
  contents(): Uint8Array {
    this._mm.invalidate(this._buffer);
    return this._mm.getBufferData(this._buffer.bufferId);
  }

  /**
   * Write bytes to the buffer from CPU side.
   *
   * Convenience method: maps, writes, and unmaps in one call.
   */
  writeBytes(data: Uint8Array, offset = 0): void {
    const mapped = this._mm.map(this._buffer);
    mapped.write(offset, data);
    this._mm.unmap(this._buffer);
  }
}

// =========================================================================
// MTLFunction and MTLLibrary -- shader management
// =========================================================================

/** A Metal shader function extracted from a library. */
export class MTLFunction {
  private readonly _name: string;
  /** @internal */ readonly _code: unknown[] | null;

  constructor(name: string, code?: unknown[] | null) {
    this._name = name;
    this._code = code ?? null;
  }

  get name(): string {
    return this._name;
  }
}

/** A Metal shader library -- a collection of compiled functions. */
export class MTLLibrary {
  private readonly _source: string;
  private readonly _functions: Map<string, unknown[] | null>;

  constructor(source: string, functions?: Map<string, unknown[] | null>) {
    this._source = source;
    this._functions = functions ?? new Map();
  }

  /** Extract a function from the library by name. */
  makeFunction(name: string): MTLFunction {
    const code = this._functions.get(name) ?? null;
    return new MTLFunction(name, code);
  }
}

// =========================================================================
// MTLComputePipelineState
// =========================================================================

/** A compiled Metal compute pipeline state. */
export class MTLComputePipelineState {
  /** @internal */ readonly _function: MTLFunction;
  /** @internal */ readonly _pipeline: unknown;

  constructor(func: MTLFunction, device: unknown) {
    this._function = func;
    const dev = device as import("@coding-adventures/compute-runtime").LogicalDevice;
    const shader = dev.createShaderModule({ code: func._code });
    const dsLayout = dev.createDescriptorSetLayout([]);
    const plLayout = dev.createPipelineLayout([dsLayout]);
    this._pipeline = dev.createComputePipeline(shader, plLayout);
  }

  get maxTotalThreadsPerThreadgroup(): number {
    return 1024;
  }
}

// =========================================================================
// MTLComputeCommandEncoder
// =========================================================================

/**
 * A Metal compute command encoder -- records compute commands.
 *
 * Instead of recording commands directly into a command buffer (Vulkan
 * style), Metal uses typed encoders that scope commands by type.
 */
export class MTLComputeCommandEncoder {
  private readonly _commandBuffer: MTLCommandBuffer;
  private _pipelineState: MTLComputePipelineState | null = null;
  /** @internal */ readonly _buffers: Map<number, MTLBuffer> = new Map();
  /** @internal */ readonly _pushData: Map<number, Uint8Array> = new Map();
  private _ended = false;

  constructor(commandBuffer: MTLCommandBuffer) {
    this._commandBuffer = commandBuffer;
  }

  /** Set which compute pipeline to use for dispatches. */
  setComputePipelineState(pso: MTLComputePipelineState): void {
    this._pipelineState = pso;
  }

  /** Bind a buffer to an argument index. */
  setBuffer(buffer: MTLBuffer, offset: number, index: number): void {
    this._buffers.set(index, buffer);
  }

  /** Set inline bytes as a kernel argument (push constants). */
  setBytes(data: Uint8Array, index: number): void {
    this._pushData.set(index, data);
  }

  /** Dispatch with explicit threadgroup count. */
  dispatchThreadgroups(
    threadgroupsPerGrid: MTLSize,
    threadsPerThreadgroup: MTLSize,
  ): void {
    if (!this._pipelineState) {
      throw new Error("No compute pipeline state set");
    }

    const cb = this._commandBuffer._cb;
    const device = this._commandBuffer._device;

    // Create a fresh pipeline with the correct local size
    const pso = this._pipelineState;
    const shader = device.createShaderModule({
      code: pso._function._code,
      localSize: [
        threadsPerThreadgroup.width,
        threadsPerThreadgroup.height,
        threadsPerThreadgroup.depth,
      ],
    });

    // Build descriptor set from bound buffers
    const sortedKeys = [...this._buffers.keys()].sort((a, b) => a - b);
    const bindings = sortedKeys.map((i) =>
      makeDescriptorBinding({ binding: i, type: "storage" }),
    );
    const dsLayout = device.createDescriptorSetLayout(bindings);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const ds = device.createDescriptorSet(dsLayout);
    for (const i of sortedKeys) {
      ds.write(i, this._buffers.get(i)!._buffer);
    }

    // Record into the command buffer
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindDescriptorSet(ds);
    cb.cmdDispatch(
      threadgroupsPerGrid.width,
      threadgroupsPerGrid.height,
      threadgroupsPerGrid.depth,
    );
  }

  /** Dispatch with total thread count (Metal calculates grid). */
  dispatchThreads(
    threadsPerGrid: MTLSize,
    threadsPerThreadgroup: MTLSize,
  ): void {
    const groups: MTLSize = {
      width: Math.max(
        1,
        Math.ceil(threadsPerGrid.width / threadsPerThreadgroup.width),
      ),
      height: Math.max(
        1,
        Math.ceil(threadsPerGrid.height / threadsPerThreadgroup.height),
      ),
      depth: Math.max(
        1,
        Math.ceil(threadsPerGrid.depth / threadsPerThreadgroup.depth),
      ),
    };
    this.dispatchThreadgroups(groups, threadsPerThreadgroup);
  }

  /** End recording into this encoder. */
  endEncoding(): void {
    this._ended = true;
  }

  /** Whether this encoder has been ended. */
  get ended(): boolean {
    return this._ended;
  }
}

// =========================================================================
// MTLBlitCommandEncoder
// =========================================================================

/**
 * A Metal blit command encoder -- records copy/fill operations.
 */
export class MTLBlitCommandEncoder {
  private readonly _commandBuffer: MTLCommandBuffer;
  private _ended = false;

  constructor(commandBuffer: MTLCommandBuffer) {
    this._commandBuffer = commandBuffer;
  }

  /** Copy data between buffers. */
  copyFromBuffer(
    src: MTLBuffer,
    srcOffset: number,
    toBuffer: MTLBuffer,
    dstOffset: number,
    size: number,
  ): void {
    const cb = this._commandBuffer._cb;
    cb.cmdCopyBuffer(src._buffer, toBuffer._buffer, size, srcOffset, dstOffset);
  }

  /** Fill a buffer region with a byte value. */
  fillBuffer(buffer: MTLBuffer, range: { start: number; end: number }, value: number): void {
    const cb = this._commandBuffer._cb;
    cb.cmdFillBuffer(buffer._buffer, value, range.start, range.end - range.start);
  }

  /** End recording into this blit encoder. */
  endEncoding(): void {
    this._ended = true;
  }

  get ended(): boolean {
    return this._ended;
  }
}

// =========================================================================
// MTLCommandBuffer
// =========================================================================

/**
 * A Metal command buffer -- records and submits GPU work.
 *
 * Uses the encoder model: create encoders, record commands, commit.
 */
export class MTLCommandBuffer {
  private readonly _queue: MTLCommandQueue;
  /** @internal */ readonly _device: import("@coding-adventures/compute-runtime").LogicalDevice;
  /** @internal */ readonly _cb: import("@coding-adventures/compute-runtime").CommandBuffer;
  private readonly _fence: Fence;
  private _status: MTLCommandBufferStatus = MTLCommandBufferStatus.notEnqueued;
  private readonly _completedHandlers: Array<() => void> = [];

  constructor(queue: MTLCommandQueue) {
    this._queue = queue;
    this._device = queue._device._logicalDevice;
    this._cb = this._device.createCommandBuffer();
    this._cb.begin();
    this._fence = this._device.createFence();
  }

  get status(): MTLCommandBufferStatus {
    return this._status;
  }

  /** Create a compute command encoder. */
  makeComputeCommandEncoder(): MTLComputeCommandEncoder {
    return new MTLComputeCommandEncoder(this);
  }

  /** Create a blit (copy/fill) command encoder. */
  makeBlitCommandEncoder(): MTLBlitCommandEncoder {
    return new MTLBlitCommandEncoder(this);
  }

  /** Submit this command buffer for execution. */
  commit(): void {
    this._cb.end();
    this._status = MTLCommandBufferStatus.committed;
    this._queue._queue.submit([this._cb], { fence: this._fence });
    this._status = MTLCommandBufferStatus.completed;
    for (const handler of this._completedHandlers) {
      handler();
    }
  }

  /** Block until the command buffer finishes execution. */
  waitUntilCompleted(): void {
    this._fence.wait();
  }

  /** Register a callback for when execution completes. */
  addCompletedHandler(handler: () => void): void {
    this._completedHandlers.push(handler);
  }
}

// =========================================================================
// MTLCommandQueue
// =========================================================================

/** A Metal command queue -- creates command buffers for submission. */
export class MTLCommandQueue {
  /** @internal */ readonly _device: MTLDevice;
  /** @internal */ readonly _queue: import("@coding-adventures/compute-runtime").CommandQueue;

  constructor(device: MTLDevice) {
    this._device = device;
    this._queue = device._computeQueue;
  }

  /** Create a new command buffer for this queue. */
  makeCommandBuffer(): MTLCommandBuffer {
    return new MTLCommandBuffer(this);
  }
}

// =========================================================================
// MTLDevice -- the main Metal device object
// =========================================================================

/**
 * A Metal device -- the main entry point for Metal programming.
 *
 * Metal always uses unified memory. All buffers are CPU-accessible
 * (storageModeShared by default).
 *
 * === Usage ===
 *
 *     const device = new MTLDevice();
 *     const queue = device.makeCommandQueue();
 *     const buf = device.makeBuffer(1024);
 *     buf.writeBytes(data);
 *     const result = buf.contents();
 */
export class MTLDevice extends BaseVendorSimulator {
  constructor() {
    super({ vendorHint: "apple" });
  }

  get name(): string {
    return this._physicalDevice.name;
  }

  /** Create a command queue for this device. */
  makeCommandQueue(): MTLCommandQueue {
    return new MTLCommandQueue(this);
  }

  /**
   * Allocate a buffer on the device.
   *
   * All Metal buffers use unified memory by default.
   */
  makeBuffer(
    length: number,
    _options: MTLResourceOptions = MTLResourceOptions.storageModeShared,
  ): MTLBuffer {
    const memType =
      MemoryType.DEVICE_LOCAL |
      MemoryType.HOST_VISIBLE |
      MemoryType.HOST_COHERENT;
    const usage =
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST;

    const buf = this._memoryManager.allocate(length, memType, usage);
    return new MTLBuffer(buf, this._memoryManager, length);
  }

  /** Create a shader library from source code. */
  makeLibrary(source: string): MTLLibrary {
    return new MTLLibrary(source);
  }

  /** Create a compute pipeline state from a shader function. */
  makeComputePipelineState(func: MTLFunction): MTLComputePipelineState {
    return new MTLComputePipelineState(func, this._logicalDevice);
  }
}
