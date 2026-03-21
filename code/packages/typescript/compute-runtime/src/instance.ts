/**
 * Instance -- device discovery, physical/logical device management.
 *
 * === The Entry Point ===
 *
 * The RuntimeInstance is how everything starts. It's the first object you
 * create, and it gives you access to all available hardware:
 *
 *     const instance = new RuntimeInstance();
 *     const devices = instance.enumeratePhysicalDevices();
 *     // -> [PhysicalDevice("NVIDIA H100"), PhysicalDevice("Apple M3 Max ANE"), ...]
 *
 * === Physical vs Logical Device ===
 *
 * A PhysicalDevice is a read-only description of hardware. You can query
 * its name, type, memory, and capabilities, but you can't use it directly.
 *
 * A LogicalDevice is a usable handle. It wraps a PhysicalDevice and provides:
 * - Command queues for submitting work
 * - Memory manager for allocating buffers
 * - Factory methods for pipelines, sync objects, etc.
 */

import type { AcceleratorDevice } from "@coding-adventures/device-simulator";
import {
  NvidiaGPU,
  AmdGPU,
  GoogleTPU,
  IntelGPU,
  AppleANE,
} from "@coding-adventures/device-simulator";

import { CommandBuffer } from "./command-buffer.js";
import { CommandQueue } from "./command-queue.js";
import { MemoryManager } from "./memory.js";
import {
  DescriptorSet,
  DescriptorSetLayout,
  Pipeline,
  PipelineLayout,
  ShaderModule,
} from "./pipeline.js";
import {
  DeviceType,
  MemoryType,
  QueueType,
  type DescriptorBinding,
  type DeviceLimits,
  type MemoryHeap,
  type MemoryProperties,
  type QueueFamily,
  type RuntimeStats,
  makeDeviceLimits,
  makeRuntimeStats,
} from "./protocols.js";
import { Event, Fence, Semaphore } from "./sync.js";

// =========================================================================
// PhysicalDevice -- read-only hardware description
// =========================================================================

/**
 * Read-only description of a physical accelerator.
 *
 * You can't execute anything on a PhysicalDevice. Create a LogicalDevice
 * for that.
 */
export class PhysicalDevice {
  private readonly _deviceId: number;
  private readonly _name: string;
  private readonly _deviceType: DeviceType;
  private readonly _vendor: string;
  /** @internal */ readonly _accelerator: AcceleratorDevice;
  private readonly _memoryProperties: MemoryProperties;
  private readonly _queueFamilies: QueueFamily[];
  private readonly _limits: DeviceLimits;

  constructor(
    deviceId: number,
    name: string,
    deviceType: DeviceType,
    vendor: string,
    accelerator: AcceleratorDevice,
    memoryProperties: MemoryProperties,
    queueFamilies: QueueFamily[],
    limits: DeviceLimits,
  ) {
    this._deviceId = deviceId;
    this._name = name;
    this._deviceType = deviceType;
    this._vendor = vendor;
    this._accelerator = accelerator;
    this._memoryProperties = memoryProperties;
    this._queueFamilies = [...queueFamilies];
    this._limits = limits;
  }

  /** Unique device identifier. */
  get deviceId(): number {
    return this._deviceId;
  }

  /** Human-readable name. */
  get name(): string {
    return this._name;
  }

  /** GPU, TPU, or NPU. */
  get deviceType(): DeviceType {
    return this._deviceType;
  }

  /** Vendor identifier. */
  get vendor(): string {
    return this._vendor;
  }

  /** Available memory types and heaps. */
  get memoryProperties(): MemoryProperties {
    return this._memoryProperties;
  }

  /** Available queue families. */
  get queueFamilies(): QueueFamily[] {
    return [...this._queueFamilies];
  }

  /** Hardware limits. */
  get limits(): DeviceLimits {
    return this._limits;
  }

  /**
   * Check if a feature is supported.
   *
   * Supported features: "fp32", "fp16", "unified_memory", "transfer_queue"
   */
  supportsFeature(feature: string): boolean {
    const features: Record<string, boolean> = {
      fp32: true,
      fp16: true,
      unified_memory: this._memoryProperties.isUnified,
      transfer_queue: this._queueFamilies.some(
        (qf) => qf.queueType === QueueType.TRANSFER,
      ),
    };
    return features[feature] ?? false;
  }
}

// =========================================================================
// LogicalDevice -- usable handle with queues and factories
// =========================================================================

/**
 * A usable device handle with command queues and resource factories.
 */
export class LogicalDevice {
  private readonly _physical: PhysicalDevice;
  private readonly _accelerator: AcceleratorDevice;
  private readonly _queues: Record<string, CommandQueue[]>;
  private readonly _memoryManager: MemoryManager;
  private readonly _stats: RuntimeStats;

  constructor(
    physicalDevice: PhysicalDevice,
    accelerator: AcceleratorDevice,
    queues: Record<string, CommandQueue[]>,
    memoryManager: MemoryManager,
    stats: RuntimeStats,
  ) {
    this._physical = physicalDevice;
    this._accelerator = accelerator;
    this._queues = queues;
    this._memoryManager = memoryManager;
    this._stats = stats;
  }

  /** The underlying physical device. */
  get physicalDevice(): PhysicalDevice {
    return this._physical;
  }

  /** Command queues by type name ('compute', 'transfer'). */
  get queues(): Record<string, CommandQueue[]> {
    return this._queues;
  }

  /** Memory allocation manager. */
  get memoryManager(): MemoryManager {
    return this._memoryManager;
  }

  /** Runtime statistics. */
  get stats(): RuntimeStats {
    return this._stats;
  }

  // --- Factory methods ---

  /** Create a new command buffer. */
  createCommandBuffer(): CommandBuffer {
    return new CommandBuffer();
  }

  /**
   * Create a shader module from code or operation descriptor.
   *
   * For GPU-style devices, pass code (list of Instructions).
   * For dataflow devices, pass operation name.
   */
  createShaderModule(options: {
    code?: unknown[] | null;
    operation?: string;
    entryPoint?: string;
    localSize?: readonly [number, number, number];
  } = {}): ShaderModule {
    return new ShaderModule(options);
  }

  /** Create a descriptor set layout. */
  createDescriptorSetLayout(bindings: DescriptorBinding[]): DescriptorSetLayout {
    return new DescriptorSetLayout(bindings);
  }

  /** Create a pipeline layout. */
  createPipelineLayout(
    setLayouts: DescriptorSetLayout[],
    pushConstantSize = 0,
  ): PipelineLayout {
    return new PipelineLayout(setLayouts, pushConstantSize);
  }

  /** Create a compute pipeline. */
  createComputePipeline(shader: ShaderModule, layout: PipelineLayout): Pipeline {
    return new Pipeline(shader, layout);
  }

  /** Create a descriptor set from a layout. */
  createDescriptorSet(layout: DescriptorSetLayout): DescriptorSet {
    return new DescriptorSet(layout);
  }

  /** Create a fence for CPU<->GPU synchronization. */
  createFence(signaled = false): Fence {
    return new Fence(signaled);
  }

  /** Create a semaphore for GPU queue<->queue synchronization. */
  createSemaphore(): Semaphore {
    return new Semaphore();
  }

  /** Create an event for fine-grained GPU-side signaling. */
  createEvent(): Event {
    return new Event();
  }

  /** Block until all queues finish all pending work. */
  waitIdle(): void {
    for (const queueList of Object.values(this._queues)) {
      for (const queue of queueList) {
        queue.waitIdle();
      }
    }
  }

  /** Reset all device state. */
  reset(): void {
    this._accelerator.reset();
  }
}

// =========================================================================
// Helper: create PhysicalDevice from AcceleratorDevice
// =========================================================================

function makePhysicalDevice(
  deviceId: number,
  accelerator: AcceleratorDevice,
  deviceType: DeviceType,
  vendor: string,
): PhysicalDevice {
  const config = accelerator.config;
  const isUnified = config.unifiedMemory;

  // Build memory heaps based on device type
  let heaps: MemoryHeap[];
  if (isUnified) {
    heaps = [
      {
        size: config.globalMemorySize,
        flags:
          MemoryType.DEVICE_LOCAL |
          MemoryType.HOST_VISIBLE |
          MemoryType.HOST_COHERENT,
      },
    ];
  } else {
    heaps = [
      // VRAM heap (GPU-only, fast)
      {
        size: config.globalMemorySize,
        flags: MemoryType.DEVICE_LOCAL,
      },
      // Staging heap (CPU-visible, slower)
      {
        size: Math.min(
          Math.floor(config.globalMemorySize / 4),
          256 * 1024 * 1024,
        ),
        flags: MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      },
    ];
  }

  const memoryProperties: MemoryProperties = {
    heaps,
    isUnified,
  };

  // Build queue families
  const queueFamilies: QueueFamily[] = [
    { queueType: QueueType.COMPUTE, count: 4 },
  ];
  // Discrete GPUs have a separate transfer queue (DMA engine)
  if (!isUnified) {
    queueFamilies.push({ queueType: QueueType.TRANSFER, count: 2 });
  }

  const limits = makeDeviceLimits();

  return new PhysicalDevice(
    deviceId,
    accelerator.name,
    deviceType,
    vendor,
    accelerator,
    memoryProperties,
    queueFamilies,
    limits,
  );
}

// =========================================================================
// RuntimeInstance -- the entry point
// =========================================================================

/**
 * The runtime entry point -- discovers devices and creates handles.
 *
 *     const instance = new RuntimeInstance();
 *     const devices = instance.enumeratePhysicalDevices();
 *     const device = instance.createLogicalDevice(devices[0]);
 */
export class RuntimeInstance {
  private readonly _version = "0.1.0";
  private readonly _physicalDevices: PhysicalDevice[];

  constructor(
    devices?: Array<[AcceleratorDevice, DeviceType, string]>,
  ) {
    if (devices !== undefined) {
      this._physicalDevices = devices.map(([dev, dtype, vendor], i) =>
        makePhysicalDevice(i, dev, dtype, vendor),
      );
    } else {
      this._physicalDevices = this._createDefaultDevices();
    }
  }

  /** Runtime version string. */
  get version(): string {
    return this._version;
  }

  /** Return all available physical devices. */
  enumeratePhysicalDevices(): PhysicalDevice[] {
    return [...this._physicalDevices];
  }

  /**
   * Create a logical device from a physical device.
   *
   * @param physicalDevice - The hardware to use.
   * @param queueRequests - Optional queue configuration.
   *                        Each has "type" (string) and "count" (number).
   */
  createLogicalDevice(
    physicalDevice: PhysicalDevice,
    queueRequests?: Array<{ type: string; count?: number }>,
  ): LogicalDevice {
    const requests = queueRequests ?? [{ type: "compute", count: 1 }];

    const stats = makeRuntimeStats();
    const accelerator = physicalDevice._accelerator;

    const memoryManager = new MemoryManager(
      accelerator,
      physicalDevice.memoryProperties,
      stats,
    );

    // Create requested queues
    const queues: Record<string, CommandQueue[]> = {};
    for (const req of requests) {
      const qtStr = req.type;
      const count = req.count ?? 1;
      let qt: QueueType;
      if (qtStr === "compute") {
        qt = QueueType.COMPUTE;
      } else if (qtStr === "transfer") {
        qt = QueueType.TRANSFER;
      } else {
        qt = QueueType.COMPUTE_TRANSFER;
      }

      const queueList: CommandQueue[] = [];
      for (let i = 0; i < count; i++) {
        queueList.push(
          new CommandQueue(qt, i, accelerator, memoryManager, stats),
        );
      }
      queues[qtStr] = queueList;
    }

    return new LogicalDevice(
      physicalDevice,
      accelerator,
      queues,
      memoryManager,
      stats,
    );
  }

  private _createDefaultDevices(): PhysicalDevice[] {
    const defaults: Array<[AcceleratorDevice, DeviceType, string]> = [
      [new NvidiaGPU({ numSMs: 2 }), DeviceType.GPU, "nvidia"],
      [new AmdGPU({ numCUs: 2 }), DeviceType.GPU, "amd"],
      [new GoogleTPU({ mxuSize: 2 }), DeviceType.TPU, "google"],
      [new IntelGPU({ numCores: 2 }), DeviceType.GPU, "intel"],
      [new AppleANE({ numCores: 2 }), DeviceType.NPU, "apple"],
    ];
    return defaults.map(([dev, dtype, vendor], i) =>
      makePhysicalDevice(i, dev, dtype, vendor),
    );
  }
}
