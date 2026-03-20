/**
 * VulkanBlas -- explicit Vulkan BLAS backend.
 *
 * === How VulkanBlas Works ===
 *
 * This backend wraps the Vulkan API from Layer 4. Vulkan is the most verbose
 * GPU API -- you explicitly manage everything: buffer creation, memory
 * allocation, binding, mapping, and unmapping.
 *
 * For each BLAS operation, we allocate VkDeviceMemory, write data via the
 * underlying memory manager's map/write/unmap cycle, and read it back the
 * same way.
 *
 * === Why Vulkan? ===
 *
 * Vulkan gives the programmer maximum control over the GPU. There are no
 * hidden allocations, no implicit synchronization. The driver does exactly
 * what you say -- nothing more. The reward is predictable performance and
 * the ability to squeeze every last FLOP from the hardware.
 */

import {
  VkInstance,
  type VkDevice,
  type VkDeviceMemory,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * Vulkan BLAS backend -- wraps VkDevice from Layer 4.
 *
 * ================================================================
 * VULKAN BLAS -- MAXIMUM CONTROL GPU ACCELERATION
 * ================================================================
 *
 * Vulkan forces you to be explicit about everything:
 * - Buffer creation with usage flags
 * - Memory allocation with property flags
 * - Explicit map/unmap for data transfer
 *
 * The reward is maximum performance and predictability -- the driver
 * does exactly what you say, nothing more.
 *
 * Usage:
 *     const blas = new VulkanBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class VulkanBlas extends GpuBlasBase {
  private _vkInstance: VkInstance;
  private _vkDevice: VkDevice;

  constructor() {
    super();
    this._vkInstance = new VkInstance();
    const physicalDevices = this._vkInstance.vkEnumeratePhysicalDevices();
    this._vkDevice = this._vkInstance.vkCreateDevice(physicalDevices[0]);
  }

  get name(): string {
    return "vulkan";
  }

  get deviceName(): string {
    return "Vulkan Device";
  }

  /**
   * Allocate Vulkan device memory and write data.
   *
   * We use VkDeviceMemory which wraps a Layer 5 Buffer. The write
   * goes through the memory manager's map/write/unmap cycle to
   * actually persist data to the device buffer.
   */
  protected _upload(data: Uint8Array): VkDeviceMemory {
    const memory = this._vkDevice.vkAllocateMemory({
      size: data.length,
      memoryTypeIndex: 0,
    });

    // Write through the underlying memory manager (Layer 5)
    const mm = memory._mm;
    const mapped = mm.map(memory._buffer);
    mapped.write(0, data);
    mm.unmap(memory._buffer);

    return memory;
  }

  /**
   * Read data from Vulkan device memory.
   *
   * Map the memory, read the bytes, unmap. This is the Vulkan way --
   * explicit control over every memory access.
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const memory = handle as VkDeviceMemory;
    const mm = memory._mm;
    mm.invalidate(memory._buffer);
    const mapped = mm.map(memory._buffer);
    const data = mapped.read(0, size);
    mm.unmap(memory._buffer);
    return new Uint8Array(data);
  }

  /**
   * In our simulator, memory is freed by garbage collection.
   */
  protected _free(_handle: unknown): void {
    // Vulkan memory freed when VkDeviceMemory goes out of scope
  }
}
