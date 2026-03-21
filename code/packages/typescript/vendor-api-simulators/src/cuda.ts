/**
 * CUDA Runtime Simulator -- NVIDIA's "just launch it" GPU programming model.
 *
 * === What is CUDA? ===
 *
 * CUDA (Compute Unified Device Architecture) is NVIDIA's proprietary GPU
 * computing platform. It's the most popular GPU programming API, used by
 * PyTorch, TensorFlow, and virtually all ML research.
 *
 * CUDA's design philosophy is "make the common case easy." The common
 * case for GPU programming is:
 *
 *     1. Allocate memory on the GPU          --> cudaMalloc()
 *     2. Copy data from CPU to GPU           --> cudaMemcpy(HostToDevice)
 *     3. Launch a kernel                     --> kernel<<<grid, block>>>(args)
 *     4. Copy results back                   --> cudaMemcpy(DeviceToHost)
 *     5. Free memory                         --> cudaFree()
 *
 * Each of these is a single function call. Compare this to Vulkan, where
 * launching a kernel requires creating a pipeline, descriptor set, command
 * buffer, recording commands, submitting, and waiting.
 *
 * === How CUDA Hides Complexity ===
 *
 * When you write kernel<<<grid, block>>>(args) in CUDA, here's what
 * happens internally (and what our simulator does):
 *
 *     1. Create a Pipeline from the kernel's code
 *     2. Create a DescriptorSet and bind the argument buffers
 *     3. Create a CommandBuffer
 *     4. Record: bind_pipeline, bind_descriptor_set, dispatch
 *     5. Submit the CommandBuffer to the default stream's queue
 *     6. Wait for completion (synchronous in default stream)
 *
 * === Streams ===
 *
 * CUDA streams are independent execution queues. The default stream (stream 0)
 * is synchronous -- every operation completes before the next starts. Additional
 * streams can overlap:
 *
 *     Stream 0 (default):  [kernel A]--[kernel B]--[kernel C]
 *     Stream 1:            --[upload]--[kernel D]--[download]
 *
 * === Memory Model ===
 *
 * CUDA simplifies memory into two main types:
 *
 *     cudaMalloc():        GPU-only memory (DEVICE_LOCAL in Layer 5)
 *     cudaMallocManaged(): Unified memory accessible from both CPU and GPU
 */

import {
  type Buffer as RuntimeBuffer,
  type CommandBuffer,
  type CommandQueue,
  type Fence,
  BufferUsage,
  MemoryType,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// CUDA-specific types
// =========================================================================

/**
 * dim3 -- the classic CUDA grid/block dimension type.
 *
 * In real CUDA, dim3 is a struct with x, y, z fields. When you write
 * kernel<<<dim3(4, 1, 1), dim3(64, 1, 1)>>>, you're saying:
 *   "Launch 4 blocks of 64 threads each, in 1D."
 */
export interface dim3 {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

/** Create a dim3 value. */
export function makeDim3(x: number, y: number, z: number): dim3 {
  return { x, y, z };
}

/**
 * Direction of a CUDA memory copy.
 *
 * === The Four Copy Directions ===
 *
 *     HostToDevice:    CPU RAM -> GPU VRAM (upload)
 *     DeviceToHost:    GPU VRAM -> CPU RAM (download)
 *     DeviceToDevice:  GPU VRAM -> GPU VRAM (on-device copy)
 *     HostToHost:      CPU RAM -> CPU RAM (plain memcpy)
 */
export enum CUDAMemcpyKind {
  HostToDevice = "host_to_device",
  DeviceToHost = "device_to_host",
  DeviceToDevice = "device_to_device",
  HostToHost = "host_to_host",
}

/**
 * Properties of a CUDA device, similar to cudaDeviceProp.
 */
export interface CUDADeviceProperties {
  readonly name: string;
  readonly totalGlobalMem: number;
  readonly sharedMemPerBlock: number;
  readonly maxThreadsPerBlock: number;
  readonly maxGridSize: readonly [number, number, number];
  readonly warpSize: number;
  readonly computeCapability: readonly [number, number];
}

/**
 * A CUDA kernel -- compiled GPU code ready to launch.
 *
 * In real CUDA, kernels are C++ functions decorated with __global__.
 * In our simulator, a kernel wraps a list of GPU instructions.
 */
export interface CUDAKernel {
  readonly code: unknown[];
  readonly name: string;
}

/** Create a CUDAKernel. */
export function makeCUDAKernel(
  code: unknown[],
  name = "unnamed_kernel",
): CUDAKernel {
  return { code, name };
}

/**
 * A CUDA device pointer -- a handle to GPU memory.
 *
 * In real CUDA, cudaMalloc() returns a void* pointer to device memory.
 * You can't dereference it on the CPU -- it's only valid on the GPU.
 */
export interface CUDADevicePtr {
  /** @internal */ readonly _buffer: RuntimeBuffer;
  readonly deviceAddress: number;
  readonly size: number;
}

/**
 * A CUDA stream -- an independent execution queue.
 *
 * A stream is a sequence of GPU operations that execute in order.
 * Operations in the same stream are guaranteed to execute sequentially.
 * Operations in different streams MAY execute concurrently.
 */
export class CUDAStream {
  /** @internal */ readonly _queue: CommandQueue;
  /** @internal */ _pendingFence: Fence | null;

  constructor(queue: CommandQueue) {
    this._queue = queue;
    this._pendingFence = null;
  }
}

/**
 * A CUDA event -- a timestamp marker in a stream.
 *
 * Events are used for two things in CUDA:
 * 1. GPU timing -- record event before and after a kernel, measure elapsed
 * 2. Stream synchronization -- one stream can wait for another's event
 */
export class CUDAEvent {
  /** @internal */ readonly _fence: Fence;
  /** @internal */ _timestamp = 0;
  /** @internal */ _recorded = false;

  constructor(fence: Fence) {
    this._fence = fence;
  }
}

// =========================================================================
// CUDARuntime -- the main simulator class
// =========================================================================

/**
 * CUDA runtime simulator -- wraps Layer 5 with CUDA semantics.
 *
 * === Usage ===
 *
 *     const cuda = new CUDARuntime();
 *     const d_x = cuda.malloc(1024);
 *     cuda.memcpy(d_x, hostData, 1024, CUDAMemcpyKind.HostToDevice);
 *     cuda.launchKernel(kernel, makeDim3(4,1,1), makeDim3(64,1,1), [d_x]);
 *     cuda.deviceSynchronize();
 *     cuda.free(d_x);
 */
export class CUDARuntime extends BaseVendorSimulator {
  private _deviceId = 0;
  /** @internal */ readonly _streams: CUDAStream[] = [];
  /** @internal */ readonly _events: CUDAEvent[] = [];

  constructor() {
    super({ vendorHint: "nvidia" });
  }

  // =================================================================
  // Device management
  // =================================================================

  /**
   * Select which GPU to use (cudaSetDevice).
   *
   * @throws Error if deviceId is out of range.
   */
  setDevice(deviceId: number): void {
    if (deviceId < 0 || deviceId >= this._physicalDevices.length) {
      throw new Error(
        `Invalid device ID ${deviceId}. ` +
        `Available: 0-${this._physicalDevices.length - 1}`,
      );
    }
    this._deviceId = deviceId;
  }

  /** Get the current device ID (cudaGetDevice). */
  getDevice(): number {
    return this._deviceId;
  }

  /** Query device properties (cudaGetDeviceProperties). */
  getDeviceProperties(): CUDADeviceProperties {
    const pd = this._physicalDevice;
    const memSize = pd.memoryProperties.heaps.reduce(
      (sum, h) => sum + h.size,
      0,
    );
    return {
      name: pd.name,
      totalGlobalMem: memSize,
      sharedMemPerBlock: 49152, // 48 KB
      maxThreadsPerBlock: pd.limits.maxWorkgroupSize[0],
      maxGridSize: pd.limits.maxWorkgroupCount,
      warpSize: 32,
      computeCapability: [8, 0],
    };
  }

  /**
   * Wait for all GPU work to complete (cudaDeviceSynchronize).
   *
   * Maps to: LogicalDevice.waitIdle()
   */
  deviceSynchronize(): void {
    this._logicalDevice.waitIdle();
  }

  /**
   * Reset the device (cudaDeviceReset).
   *
   * Destroys all allocations, streams, and state.
   */
  deviceReset(): void {
    this._logicalDevice.reset();
    this._streams.length = 0;
    this._events.length = 0;
  }

  // =================================================================
  // Memory management
  // =================================================================

  /**
   * Allocate device memory (cudaMalloc).
   *
   * Allocates GPU-only memory. We use HOST_VISIBLE | HOST_COHERENT for
   * simulation convenience so we can actually read/write data.
   */
  malloc(size: number): CUDADevicePtr {
    const buf = this._memoryManager.allocate(
      size,
      MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
    );
    return {
      _buffer: buf,
      deviceAddress: buf.deviceAddress,
      size,
    };
  }

  /**
   * Allocate unified/managed memory (cudaMallocManaged).
   *
   * Managed memory is accessible from both CPU and GPU.
   */
  mallocManaged(size: number): CUDADevicePtr {
    const buf = this._memoryManager.allocate(
      size,
      MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
    );
    return {
      _buffer: buf,
      deviceAddress: buf.deviceAddress,
      size,
    };
  }

  /**
   * Free device memory (cudaFree).
   *
   * @throws Error if the pointer has already been freed.
   */
  free(ptr: CUDADevicePtr): void {
    this._memoryManager.free(ptr._buffer);
  }

  /**
   * Copy memory between host and device (cudaMemcpy).
   *
   * === The Four Copy Directions ===
   *
   * HostToDevice:    src is Uint8Array (CPU), dst is CUDADevicePtr (GPU)
   * DeviceToHost:    src is CUDADevicePtr (GPU), dst is Uint8Array (CPU)
   * DeviceToDevice:  both are CUDADevicePtr
   * HostToHost:      both are Uint8Array
   *
   * @throws TypeError if src/dst types don't match the specified kind.
   */
  memcpy(
    dst: CUDADevicePtr | Uint8Array,
    src: CUDADevicePtr | Uint8Array,
    size: number,
    kind: CUDAMemcpyKind,
  ): void {
    if (kind === CUDAMemcpyKind.HostToDevice) {
      // CPU -> GPU: map the device buffer, write host data, unmap
      if (!isCUDADevicePtr(dst)) {
        throw new TypeError("dst must be CUDADevicePtr for HostToDevice");
      }
      if (isCUDADevicePtr(src)) {
        throw new TypeError("src must be Uint8Array for HostToDevice");
      }
      const mapped = this._memoryManager.map(dst._buffer);
      mapped.write(0, new Uint8Array(src.buffer, src.byteOffset, size));
      this._memoryManager.unmap(dst._buffer);
    } else if (kind === CUDAMemcpyKind.DeviceToHost) {
      // GPU -> CPU: map the device buffer, read data, unmap
      if (!isCUDADevicePtr(src)) {
        throw new TypeError("src must be CUDADevicePtr for DeviceToHost");
      }
      if (isCUDADevicePtr(dst)) {
        throw new TypeError("dst must be Uint8Array for DeviceToHost");
      }
      // Sync from device first
      this._memoryManager.invalidate(src._buffer);
      const mapped = this._memoryManager.map(src._buffer);
      const data = mapped.read(0, size);
      this._memoryManager.unmap(src._buffer);
      dst.set(data.subarray(0, size));
    } else if (kind === CUDAMemcpyKind.DeviceToDevice) {
      // GPU -> GPU: use a command buffer with cmdCopyBuffer
      if (!isCUDADevicePtr(dst)) {
        throw new TypeError("dst must be CUDADevicePtr for DeviceToDevice");
      }
      if (!isCUDADevicePtr(src)) {
        throw new TypeError("src must be CUDADevicePtr for DeviceToDevice");
      }
      const srcBuf = src._buffer;
      const dstBuf = dst._buffer;
      this._createAndSubmitCb((cb: CommandBuffer) => {
        cb.cmdCopyBuffer(srcBuf, dstBuf, size);
      });
    } else if (kind === CUDAMemcpyKind.HostToHost) {
      // CPU -> CPU: plain memory copy
      if (isCUDADevicePtr(dst)) {
        throw new TypeError("dst must be Uint8Array for HostToHost");
      }
      if (isCUDADevicePtr(src)) {
        throw new TypeError("src must be Uint8Array for HostToHost");
      }
      dst.set(src.subarray(0, size));
    }
  }

  /**
   * Set device memory to a value (cudaMemset).
   *
   * Fills the first `size` bytes with the byte value `value`.
   */
  memset(ptr: CUDADevicePtr, value: number, size: number): void {
    this._createAndSubmitCb((cb: CommandBuffer) => {
      cb.cmdFillBuffer(ptr._buffer, value, 0, size);
    });
  }

  // =================================================================
  // Kernel launch -- the heart of CUDA
  // =================================================================

  /**
   * Launch a CUDA kernel (the <<<grid, block>>> operator).
   *
   * This single call hides the entire Vulkan-style pipeline:
   *
   *     1. Create a ShaderModule from the kernel's code
   *     2. Create descriptor bindings for arguments
   *     3. Create the compute pipeline
   *     4. Bind and dispatch
   */
  launchKernel(
    kernel: CUDAKernel,
    grid: dim3,
    block: dim3,
    args?: CUDADevicePtr[],
    sharedMem?: number,
    stream?: CUDAStream,
  ): void {
    const device = this._logicalDevice;
    const argList = args ?? [];

    // Step 1: Create shader module with the kernel's code
    const shader = device.createShaderModule({
      code: kernel.code,
      localSize: [block.x, block.y, block.z],
    });

    // Step 2: Create descriptor set layout with one binding per argument
    const bindings = argList.map((_, i) =>
      makeDescriptorBinding({ binding: i, type: "storage" }),
    );
    const dsLayout = device.createDescriptorSetLayout(bindings);
    const plLayout = device.createPipelineLayout([dsLayout]);

    // Step 3: Create the compute pipeline
    const pipeline = device.createComputePipeline(shader, plLayout);

    // Step 4: Create and populate descriptor set
    const ds = device.createDescriptorSet(dsLayout);
    argList.forEach((arg, i) => {
      ds.write(i, arg._buffer);
    });

    // Step 5-8: Record and submit via helper
    const targetQueue = stream ? stream._queue : undefined;
    this._createAndSubmitCb(
      (cb: CommandBuffer) => {
        cb.cmdBindPipeline(pipeline);
        cb.cmdBindDescriptorSet(ds);
        cb.cmdDispatch(grid.x, grid.y, grid.z);
      },
      targetQueue,
    );
  }

  // =================================================================
  // Streams
  // =================================================================

  /** Create a new CUDA stream (cudaStreamCreate). */
  createStream(): CUDAStream {
    const stream = new CUDAStream(this._computeQueue);
    this._streams.push(stream);
    return stream;
  }

  /**
   * Destroy a CUDA stream (cudaStreamDestroy).
   *
   * @throws Error if the stream is not found.
   */
  destroyStream(stream: CUDAStream): void {
    const idx = this._streams.indexOf(stream);
    if (idx === -1) {
      throw new Error("Stream not found or already destroyed");
    }
    this._streams.splice(idx, 1);
  }

  /** Wait for all operations in a stream (cudaStreamSynchronize). */
  streamSynchronize(stream: CUDAStream): void {
    if (stream._pendingFence !== null) {
      stream._pendingFence.wait();
    }
  }

  // =================================================================
  // Events (for GPU timing)
  // =================================================================

  /** Create a CUDA event (cudaEventCreate). */
  createEvent(): CUDAEvent {
    const fence = this._logicalDevice.createFence();
    const event = new CUDAEvent(fence);
    this._events.push(event);
    return event;
  }

  /** Record an event in a stream (cudaEventRecord). */
  recordEvent(event: CUDAEvent, stream?: CUDAStream): void {
    const queue = stream ? stream._queue : this._computeQueue;
    event._timestamp = queue.totalCycles;
    event._fence.signal();
    event._recorded = true;
  }

  /**
   * Wait for an event to complete (cudaEventSynchronize).
   *
   * @throws Error if the event was never recorded.
   */
  synchronizeEvent(event: CUDAEvent): void {
    if (!event._recorded) {
      throw new Error("Event was never recorded");
    }
    event._fence.wait();
  }

  /**
   * Measure elapsed GPU time between two events (cudaEventElapsedTime).
   *
   * @returns Elapsed time in milliseconds.
   * @throws Error if either event was not recorded.
   */
  elapsedTime(start: CUDAEvent, end: CUDAEvent): number {
    if (!start._recorded) {
      throw new Error("Start event was never recorded");
    }
    if (!end._recorded) {
      throw new Error("End event was never recorded");
    }
    const cycles = end._timestamp - start._timestamp;
    return cycles / 1_000_000.0; // 1 GHz -> 1 cycle = 1 ns = 0.000001 ms
  }
}

// =========================================================================
// Type guard for CUDADevicePtr
// =========================================================================

/** Check if a value is a CUDADevicePtr (has _buffer property). */
function isCUDADevicePtr(
  value: CUDADevicePtr | Uint8Array,
): value is CUDADevicePtr {
  return "_buffer" in value;
}
