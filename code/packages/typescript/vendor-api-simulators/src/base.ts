/**
 * BaseVendorSimulator -- the shared foundation for all six vendor API simulators.
 *
 * === Why a Base Class? ===
 *
 * Every GPU API, no matter how different its surface looks, needs to do the same
 * things underneath:
 *
 *     1. Find a GPU                  --> RuntimeInstance
 *     2. Create a usable handle      --> LogicalDevice
 *     3. Get a queue for submission   --> CommandQueue
 *     4. Manage memory                --> MemoryManager
 *
 * This base class sets all that up. Each simulator subclass then adds its
 * vendor-specific vocabulary on top.
 *
 * Think of it like building six different restaurant fronts (CUDA Grill, Metal
 * Bistro, Vulkan Steakhouse...) that all share the same kitchen in the back.
 * The kitchen is our compute runtime (Layer 5). The restaurant menus look
 * completely different, but the same chefs cook the same food.
 *
 * === Device Selection ===
 *
 * Different APIs have different preferences for which device to use:
 *
 *     - CUDA always wants an NVIDIA GPU (vendorHint="nvidia")
 *     - Metal always wants an Apple device (vendorHint="apple")
 *     - OpenCL, Vulkan, WebGPU, OpenGL are cross-vendor
 *
 * The _selectDevice() method handles this: it picks the best matching device
 * from the runtime's enumerated physical devices, preferring the vendor hint
 * if given, then falling back to any GPU.
 *
 * === The _createAndSubmitCb() Helper ===
 *
 * CUDA and OpenGL hide command buffers from the user. When you call
 * cudaMemcpy() or glDispatchCompute(), those APIs internally:
 *
 *     1. Create a command buffer
 *     2. Begin recording
 *     3. Record the command(s) via a callback
 *     4. End recording
 *     5. Submit to the compute queue with a fence
 *     6. Wait for the fence
 *
 * This helper encapsulates that pattern. Pass a callback that records
 * commands into a CB, and this method handles the rest.
 */

import {
  type CommandBuffer,
  type CommandQueue,
  type LogicalDevice,
  type MemoryManager,
  type PhysicalDevice,
  DeviceType,
  RuntimeInstance,
} from "@coding-adventures/compute-runtime";

/**
 * Common foundation for all vendor API simulators.
 *
 * === What This Provides ===
 *
 * Every subclass gets:
 * - _instance:         RuntimeInstance (Layer 5 entry point)
 * - _physicalDevices:  All available physical devices
 * - _physicalDevice:   The selected physical device
 * - _logicalDevice:    The usable device handle
 * - _computeQueue:     A compute queue for submitting work
 * - _memoryManager:    For allocating and managing memory
 *
 * === Usage ===
 *
 * Subclasses call the constructor with optional deviceType and
 * vendorHint to control which device is selected:
 *
 *     class CUDARuntime extends BaseVendorSimulator {
 *       constructor() {
 *         super({ vendorHint: "nvidia" });
 *       }
 *     }
 *
 *     class MTLDevice extends BaseVendorSimulator {
 *       constructor() {
 *         super({ vendorHint: "apple" });
 *       }
 *     }
 */
export class BaseVendorSimulator {
  /** @internal */ readonly _instance: RuntimeInstance;
  /** @internal */ readonly _physicalDevices: PhysicalDevice[];
  /** @internal */ readonly _physicalDevice: PhysicalDevice;
  /** @internal */ readonly _logicalDevice: LogicalDevice;
  /** @internal */ readonly _computeQueue: CommandQueue;
  /** @internal */ readonly _memoryManager: MemoryManager;

  /**
   * Initialize the simulator with device discovery and setup.
   *
   * @param options.deviceType - Preferred device type (GPU, TPU, NPU). If
   *   undefined, any type is acceptable.
   * @param options.vendorHint - Preferred vendor string (e.g., "nvidia",
   *   "apple"). If the preferred vendor isn't found, falls back to any
   *   available device.
   */
  constructor(options: {
    deviceType?: DeviceType;
    vendorHint?: string;
  } = {}) {
    // Step 1: Create the runtime instance (discovers all hardware)
    this._instance = new RuntimeInstance();

    // Step 2: Enumerate all physical devices
    this._physicalDevices = this._instance.enumeratePhysicalDevices();

    // Step 3: Select the best matching device
    this._physicalDevice = this._selectDevice(
      options.deviceType,
      options.vendorHint,
    );

    // Step 4: Create a logical device (the usable handle)
    this._logicalDevice = this._instance.createLogicalDevice(
      this._physicalDevice,
    );

    // Step 5: Get a compute queue for submitting work
    this._computeQueue = this._logicalDevice.queues["compute"][0];

    // Step 6: Get the memory manager for allocations
    this._memoryManager = this._logicalDevice.memoryManager;
  }

  /**
   * Pick the best matching device from enumerated physical devices.
   *
   * === Selection Strategy ===
   *
   * The strategy is a two-pass filter:
   *
   * Pass 1: Try to match both vendorHint AND deviceType (if given).
   * Pass 2: Try vendorHint only.
   * Pass 3: Try deviceType only.
   * Pass 4: Take the first device (any will do).
   *
   * This ensures that:
   * - CUDARuntime(vendorHint="nvidia") gets an NVIDIA GPU
   * - MTLDevice(vendorHint="apple") gets an Apple device
   * - VulkanRuntime() gets whatever is available
   */
  protected _selectDevice(
    deviceType?: DeviceType,
    vendorHint?: string,
  ): PhysicalDevice {
    if (this._physicalDevices.length === 0) {
      throw new Error("No physical devices available");
    }

    // Pass 1: Match both vendor and type
    if (vendorHint && deviceType) {
      for (const dev of this._physicalDevices) {
        if (dev.vendor === vendorHint && dev.deviceType === deviceType) {
          return dev;
        }
      }
    }

    // Pass 2: Match vendor only
    if (vendorHint) {
      for (const dev of this._physicalDevices) {
        if (dev.vendor === vendorHint) {
          return dev;
        }
      }
    }

    // Pass 3: Match device type only
    if (deviceType) {
      for (const dev of this._physicalDevices) {
        if (dev.deviceType === deviceType) {
          return dev;
        }
      }
    }

    // Pass 4: Take whatever is available
    return this._physicalDevices[0];
  }

  /**
   * Create a command buffer, record commands, submit, and wait.
   *
   * === The "Immediate Execution" Pattern ===
   *
   * APIs like CUDA and OpenGL present an "immediate" execution model
   * where each API call appears to execute right away. Under the hood,
   * they still use command buffers -- they just hide them from you.
   *
   * This method implements that pattern:
   *
   *     1. Create a new command buffer
   *     2. Begin recording
   *     3. Call recordFn(cb) to record whatever commands the caller wants
   *     4. End recording
   *     5. Submit to the queue with a fence
   *     6. Wait for the fence to signal (synchronous completion)
   *     7. Return the command buffer (for inspection/debugging)
   *
   * @param recordFn - A callback that receives a CommandBuffer in RECORDING
   *   state and records commands into it.
   * @param queue - Which queue to submit to. Defaults to _computeQueue.
   * @returns The completed CommandBuffer.
   */
  protected _createAndSubmitCb(
    recordFn: (cb: CommandBuffer) => void,
    queue?: CommandQueue,
  ): CommandBuffer {
    const targetQueue = queue ?? this._computeQueue;

    // Create and begin recording
    const cb = this._logicalDevice.createCommandBuffer();
    cb.begin();

    // Let the caller record whatever commands they need
    recordFn(cb);

    // End recording and submit
    cb.end();
    const fence = this._logicalDevice.createFence();
    targetQueue.submit([cb], { fence });
    fence.wait();

    return cb;
  }
}
