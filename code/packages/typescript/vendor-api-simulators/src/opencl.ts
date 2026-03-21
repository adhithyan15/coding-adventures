/**
 * OpenCL Runtime Simulator -- cross-platform "portable compute" model.
 *
 * === What is OpenCL? ===
 *
 * OpenCL (Open Computing Language) is the Khronos Group's cross-platform
 * compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's GPU,
 * and even on CPUs and FPGAs.
 *
 * === The OpenCL Object Hierarchy ===
 *
 *     CLPlatform          "Which vendor's implementation?"
 *         +-- CLDevice    "Which specific GPU/CPU?"
 *     CLContext            "A group of devices I want to use together"
 *         +-- CLBuffer     "Memory on one of the context's devices"
 *         +-- CLProgram    "Source code, not yet compiled"
 *         |   +-- CLKernel "Compiled function, ready to dispatch"
 *         +-- CLCommandQueue "Where I enqueue operations"
 *                 +-- CLEvent "Dependency token for operation ordering"
 *
 * === Event-Based Dependencies ===
 *
 * OpenCL's most distinctive feature is its event model. Every enqueue
 * operation returns a CLEvent. You can pass event lists to subsequent
 * operations to create dependency chains (arbitrary DAGs, not just linear).
 */

import {
  type Buffer as RuntimeBuffer,
  type Fence,
  type PhysicalDevice,
  BufferUsage,
  MemoryType,
  RuntimeInstance,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// OpenCL enums and flags
// =========================================================================

/** OpenCL device types for filtering during discovery. */
export enum CLDeviceType {
  GPU = "gpu",
  CPU = "cpu",
  ACCELERATOR = "accelerator",
  ALL = "all",
}

/** OpenCL memory flags -- simpler than Vulkan's memory types. */
export enum CLMemFlags {
  READ_WRITE = 1,
  READ_ONLY = 2,
  WRITE_ONLY = 4,
  COPY_HOST_PTR = 8,
  USE_HOST_PTR = 16,
  ALLOC_HOST_PTR = 32,
}

/** Build status of a CLProgram. */
export enum CLBuildStatus {
  SUCCESS = "success",
  ERROR = "error",
  IN_PROGRESS = "in_progress",
  NONE = "none",
}

/** Status of an OpenCL event. */
export enum CLEventStatus {
  QUEUED = "queued",
  SUBMITTED = "submitted",
  RUNNING = "running",
  COMPLETE = "complete",
}

/** Device info parameter IDs. */
export enum CLDeviceInfo {
  NAME = "name",
  TYPE = "type",
  MAX_COMPUTE_UNITS = "max_compute_units",
  MAX_WORK_GROUP_SIZE = "max_work_group_size",
  GLOBAL_MEM_SIZE = "global_mem_size",
}

// =========================================================================
// CLEvent -- dependency token
// =========================================================================

/**
 * An OpenCL event -- a dependency token for operation ordering.
 *
 * Every enqueue operation returns a CLEvent. You can:
 * - Wait on it (blocking the CPU)
 * - Pass it in waitList to another operation (GPU-side dependency)
 * - Query its status
 */
export class CLEvent {
  /** @internal */ readonly _fence: Fence;

  constructor(fence: Fence) {
    this._fence = fence;
  }

  /** Block until this event completes. */
  wait(): void {
    this._fence.wait();
  }

  /** Query the current status of this event. */
  get status(): CLEventStatus {
    if (this._fence.signaled) {
      return CLEventStatus.COMPLETE;
    }
    return CLEventStatus.QUEUED;
  }
}

// =========================================================================
// CLDevice -- wraps PhysicalDevice
// =========================================================================

/**
 * An OpenCL device -- a specific piece of hardware.
 *
 * Wraps a Layer 5 PhysicalDevice with OpenCL-style property queries.
 */
export class CLDevice {
  /** @internal */ readonly _physical: PhysicalDevice;

  constructor(physicalDevice: PhysicalDevice) {
    this._physical = physicalDevice;
  }

  get name(): string {
    return this._physical.name;
  }

  get deviceType(): CLDeviceType {
    const dt = this._physical.deviceType;
    if (dt === "gpu") return CLDeviceType.GPU;
    if (dt === "tpu") return CLDeviceType.ACCELERATOR;
    if (dt === "npu") return CLDeviceType.ACCELERATOR;
    return CLDeviceType.GPU;
  }

  get maxComputeUnits(): number {
    return 4;
  }

  get maxWorkGroupSize(): number {
    return this._physical.limits.maxWorkgroupSize[0];
  }

  get globalMemSize(): number {
    return this._physical.memoryProperties.heaps.reduce(
      (sum, h) => sum + h.size,
      0,
    );
  }

  /** Query device information by parameter ID. */
  getInfo(param: CLDeviceInfo): unknown {
    const infoMap: Record<string, unknown> = {
      [CLDeviceInfo.NAME]: this.name,
      [CLDeviceInfo.TYPE]: this.deviceType,
      [CLDeviceInfo.MAX_COMPUTE_UNITS]: this.maxComputeUnits,
      [CLDeviceInfo.MAX_WORK_GROUP_SIZE]: this.maxWorkGroupSize,
      [CLDeviceInfo.GLOBAL_MEM_SIZE]: this.globalMemSize,
    };
    return infoMap[param];
  }
}

// =========================================================================
// CLBuffer -- wraps Buffer
// =========================================================================

/** An OpenCL buffer -- memory allocated on a device. */
export class CLBuffer {
  /** @internal */ readonly _buffer: RuntimeBuffer;
  private readonly _size: number;
  private readonly _flags: CLMemFlags;

  constructor(buffer: RuntimeBuffer, size: number, flags: CLMemFlags) {
    this._buffer = buffer;
    this._size = size;
    this._flags = flags;
  }

  get size(): number {
    return this._size;
  }

  get flags(): CLMemFlags {
    return this._flags;
  }
}

// =========================================================================
// CLKernel -- a compiled kernel function
// =========================================================================

/**
 * An OpenCL kernel -- arguments are set one at a time with setArg().
 */
export class CLKernel {
  private readonly _name: string;
  /** @internal */ readonly _code: unknown[] | null;
  /** @internal */ readonly _args: Map<number, CLBuffer | number | Uint8Array> = new Map();

  constructor(name: string, code?: unknown[] | null) {
    this._name = name;
    this._code = code ?? null;
  }

  get name(): string {
    return this._name;
  }

  /** Set a kernel argument at the given index. */
  setArg(index: number, value: CLBuffer | number | Uint8Array): void {
    this._args.set(index, value);
  }
}

// =========================================================================
// CLProgram -- source code + compilation
// =========================================================================

/**
 * An OpenCL program -- source code that can be compiled for a device.
 *
 * OpenCL uses runtime compilation: you provide kernel source as a string,
 * call build(), and the OpenCL implementation compiles it.
 */
export class CLProgram {
  private readonly _source: string;
  /** @internal */ readonly _context: CLContext;
  private _buildStatus: CLBuildStatus = CLBuildStatus.NONE;
  private readonly _kernels: Map<string, unknown[] | null> = new Map();

  constructor(source: string, context: CLContext) {
    this._source = source;
    this._context = context;
  }

  get buildStatus(): CLBuildStatus {
    return this._buildStatus;
  }

  /** Compile the program for the target device(s). */
  build(_devices?: CLDevice[], _options = ""): void {
    this._buildStatus = CLBuildStatus.SUCCESS;
  }

  /**
   * Extract a kernel function from the compiled program.
   *
   * @throws Error if the program hasn't been built.
   */
  createKernel(name: string): CLKernel {
    if (this._buildStatus !== CLBuildStatus.SUCCESS) {
      throw new Error(
        `Program not built (status: ${this._buildStatus}). ` +
        "Call program.build() first.",
      );
    }
    return new CLKernel(name, this._kernels.get(name) ?? null);
  }
}

// =========================================================================
// CLCommandQueue -- enqueue operations with event dependencies
// =========================================================================

/**
 * An OpenCL command queue -- where operations are enqueued.
 *
 * Every operation returns a CLEvent for dependency tracking.
 */
export class CLCommandQueue {
  /** @internal */ readonly _context: CLContext;
  private readonly _device: CLDevice;

  constructor(context: CLContext, device: CLDevice) {
    this._context = context;
    this._device = device;
  }

  /**
   * Enqueue a kernel for execution (clEnqueueNDRangeKernel).
   *
   * The globalSize specifies total work items. If localSize is undefined,
   * the runtime picks an optimal workgroup size.
   */
  enqueueNDRangeKernel(
    kernel: CLKernel,
    globalSize: number[],
    localSize?: number[],
    waitList?: CLEvent[],
  ): CLEvent {
    // Wait for dependency events
    for (const event of waitList ?? []) {
      event.wait();
    }

    const device = this._context._logicalDevice;

    // Determine local size
    let local: [number, number, number];
    if (!localSize) {
      local = [32, 1, 1];
    } else {
      local = [
        localSize[0],
        localSize.length > 1 ? localSize[1] : 1,
        localSize.length > 2 ? localSize[2] : 1,
      ];
    }

    // Calculate grid dimensions
    const gridX = Math.max(1, Math.ceil(globalSize[0] / local[0]));
    const gridY =
      globalSize.length > 1
        ? Math.max(1, Math.ceil(globalSize[1] / local[1]))
        : 1;
    const gridZ =
      globalSize.length > 2
        ? Math.max(1, Math.ceil(globalSize[2] / local[2]))
        : 1;

    // Create shader module from kernel code
    const shader = device.createShaderModule({
      code: kernel._code,
      localSize: local,
    });

    // Build descriptor set from kernel arguments
    const bufferArgs = new Map<number, CLBuffer>();
    for (const [i, arg] of kernel._args.entries()) {
      if (arg instanceof CLBuffer) {
        bufferArgs.set(i, arg);
      }
    }

    const sortedKeys = [...bufferArgs.keys()].sort((a, b) => a - b);
    const bindings = sortedKeys.map((i) =>
      makeDescriptorBinding({ binding: i, type: "storage" }),
    );
    const dsLayout = device.createDescriptorSetLayout(bindings);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const ds = device.createDescriptorSet(dsLayout);
    for (const i of sortedKeys) {
      ds.write(i, bufferArgs.get(i)!._buffer);
    }

    // Record and submit
    const fence = device.createFence();
    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindDescriptorSet(ds);
    cb.cmdDispatch(gridX, gridY, gridZ);
    cb.end();

    this._context._computeQueue.submit([cb], { fence });
    fence.wait();

    return new CLEvent(fence);
  }

  /** Write host data to a device buffer. */
  enqueueWriteBuffer(
    buffer: CLBuffer,
    offset: number,
    size: number,
    hostPtr: Uint8Array,
    waitList?: CLEvent[],
  ): CLEvent {
    for (const event of waitList ?? []) {
      event.wait();
    }

    const mm = this._context._memoryManager;
    const mapped = mm.map(buffer._buffer);
    mapped.write(offset, new Uint8Array(hostPtr.buffer, hostPtr.byteOffset, size));
    mm.unmap(buffer._buffer);

    const fence = this._context._logicalDevice.createFence(true);
    return new CLEvent(fence);
  }

  /** Read device buffer data to host memory. */
  enqueueReadBuffer(
    buffer: CLBuffer,
    offset: number,
    size: number,
    hostPtr: Uint8Array,
    waitList?: CLEvent[],
  ): CLEvent {
    for (const event of waitList ?? []) {
      event.wait();
    }

    const mm = this._context._memoryManager;
    mm.invalidate(buffer._buffer);
    const mapped = mm.map(buffer._buffer);
    const data = mapped.read(offset, size);
    mm.unmap(buffer._buffer);
    hostPtr.set(data.subarray(0, size));

    const fence = this._context._logicalDevice.createFence(true);
    return new CLEvent(fence);
  }

  /** Copy between two device buffers. */
  enqueueCopyBuffer(
    src: CLBuffer,
    dst: CLBuffer,
    size: number,
    waitList?: CLEvent[],
  ): CLEvent {
    for (const event of waitList ?? []) {
      event.wait();
    }

    const device = this._context._logicalDevice;
    const fence = device.createFence();
    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdCopyBuffer(src._buffer, dst._buffer, size);
    cb.end();
    this._context._computeQueue.submit([cb], { fence });
    fence.wait();

    return new CLEvent(fence);
  }

  /** Fill a buffer with a pattern. */
  enqueueFillBuffer(
    buffer: CLBuffer,
    pattern: Uint8Array,
    offset: number,
    size: number,
  ): CLEvent {
    const device = this._context._logicalDevice;
    const fence = device.createFence();
    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdFillBuffer(buffer._buffer, pattern.length > 0 ? pattern[0] : 0, offset, size);
    cb.end();
    this._context._computeQueue.submit([cb], { fence });
    fence.wait();

    return new CLEvent(fence);
  }

  /**
   * Block until all enqueued operations complete (clFinish).
   */
  finish(): void {
    this._context._logicalDevice.waitIdle();
  }

  /** Ensure all enqueued operations are submitted (clFlush). No-op in simulator. */
  flush(): void {
    // No-op in synchronous simulator
  }
}

// =========================================================================
// CLContext -- the OpenCL execution context
// =========================================================================

/**
 * An OpenCL context -- groups devices and manages shared resources.
 *
 * Our simulator creates a context with a single device, wrapping the
 * Layer 5 LogicalDevice.
 */
export class CLContext extends BaseVendorSimulator {
  /** @internal */ readonly _devices: CLDevice[];

  constructor(devices?: CLDevice[]) {
    if (devices && devices.length > 0) {
      const vendor = devices[0]._physical.vendor;
      super({ vendorHint: vendor });
      this._devices = devices;
    } else {
      super();
      this._devices = this._physicalDevices.map(
        (pd) => new CLDevice(pd),
      );
    }
  }

  /**
   * Create a device buffer (clCreateBuffer).
   *
   * Maps OpenCL memory flags to Layer 5 memory types.
   */
  createBuffer(
    flags: CLMemFlags,
    size: number,
    hostPtr?: Uint8Array,
  ): CLBuffer {
    const memType =
      MemoryType.DEVICE_LOCAL |
      MemoryType.HOST_VISIBLE |
      MemoryType.HOST_COHERENT;
    const usage =
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST;

    const buf = this._memoryManager.allocate(size, memType, usage);
    const clBuf = new CLBuffer(buf, size, flags);

    // If COPY_HOST_PTR, write the initial data
    if (hostPtr && (flags & CLMemFlags.COPY_HOST_PTR) !== 0) {
      const mapped = this._memoryManager.map(buf);
      mapped.write(0, new Uint8Array(hostPtr.buffer, hostPtr.byteOffset, Math.min(hostPtr.length, size)));
      this._memoryManager.unmap(buf);
    }

    return clBuf;
  }

  /** Create a program from source code. */
  createProgramWithSource(source: string): CLProgram {
    return new CLProgram(source, this);
  }

  /** Create a command queue for a device. */
  createCommandQueue(device?: CLDevice, _properties = 0): CLCommandQueue {
    const dev = device ?? this._devices[0];
    return new CLCommandQueue(this, dev);
  }
}

// =========================================================================
// CLPlatform -- the top-level discovery object
// =========================================================================

/**
 * An OpenCL platform -- represents a vendor's OpenCL implementation.
 *
 * In our simulator, there's one platform wrapping our Layer 5 runtime.
 */
export class CLPlatform {
  private readonly _runtimeInstance: RuntimeInstance;
  private readonly _physicalDevices: PhysicalDevice[];
  private readonly _name = "Coding Adventures Compute Platform";
  private readonly _vendor = "Coding Adventures";
  private readonly _version = "OpenCL 3.0";

  constructor() {
    this._runtimeInstance = new RuntimeInstance();
    this._physicalDevices = this._runtimeInstance.enumeratePhysicalDevices();
  }

  /** Enumerate available OpenCL platforms. */
  static getPlatforms(): CLPlatform[] {
    return [new CLPlatform()];
  }

  get name(): string {
    return this._name;
  }

  get vendor(): string {
    return this._vendor;
  }

  get version(): string {
    return this._version;
  }

  /** Get devices of a specific type on this platform. */
  getDevices(deviceType: CLDeviceType = CLDeviceType.ALL): CLDevice[] {
    const devices = this._physicalDevices.map((pd) => new CLDevice(pd));
    if (deviceType === CLDeviceType.ALL) {
      return devices;
    }
    return devices.filter((d) => d.deviceType === deviceType);
  }
}
