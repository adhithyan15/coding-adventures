/**
 * WebGpuBlas -- browser-friendly WebGPU BLAS backend.
 *
 * === How WebGpuBlas Works ===
 *
 * This backend wraps `GPUDevice` from Layer 4. WebGPU is designed for
 * safe, browser-based GPU compute with automatic synchronization.
 *
 * For each BLAS operation:
 *     1. device.createBuffer(STORAGE | COPY_DST)  -- allocate with usage flags
 *     2. device.queue.writeBuffer()                -- upload data
 *     3. (compute)                                 -- perform operation
 *     4. Create a MAP_READ staging buffer, copy, map, read
 *     5. Buffer.destroy() (explicit cleanup)
 *
 * WebGPU's key simplification: a single queue (`device.queue`) handles
 * everything. No queue families, no multiple queues.
 */

import {
  GPU,
  GPUBufferUsage,
  GPUMapMode,
  type GPUDevice,
  type GPUBuffer,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * WebGPU BLAS backend -- wraps GPUDevice from Layer 4.
 *
 * ================================================================
 * WEBGPU BLAS -- SAFE BROWSER-FIRST GPU ACCELERATION
 * ================================================================
 *
 * WebGPU provides a safe, validated GPU API designed for browsers:
 * - Single queue (device.queue)
 * - Automatic barriers (no manual synchronization)
 * - Usage-based buffer creation (STORAGE, COPY_SRC, COPY_DST, MAP_READ)
 *
 * Usage:
 *     const blas = new WebGpuBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class WebGpuBlas extends GpuBlasBase {
  private _device: GPUDevice;

  constructor() {
    super();
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    this._device = adapter.requestDevice();
  }

  get name(): string {
    return "webgpu";
  }

  get deviceName(): string {
    return "WebGPU Device";
  }

  /**
   * Create a WebGPU buffer with STORAGE usage and write data.
   *
   * WebGPU buffers need explicit usage flags at creation time:
   * - STORAGE: can be used in compute shaders
   * - COPY_DST: can be written to (for upload)
   * - COPY_SRC: can be copied from (for readback)
   */
  protected _upload(data: Uint8Array): GPUBuffer {
    const buf = this._device.createBuffer({
      size: data.length,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
    });
    this._device.queue.writeBuffer(buf, 0, data);
    return buf;
  }

  /**
   * Create a MAP_READ staging buffer, copy, and read.
   *
   * WebGPU cannot map a STORAGE buffer directly. You must:
   *     1. Create a staging buffer with MAP_READ | COPY_DST
   *     2. Copy from the source buffer to the staging buffer
   *     3. Map the staging buffer
   *     4. Read the mapped data
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const srcBuf = handle as GPUBuffer;

    // Create a staging buffer for readback
    const staging = this._device.createBuffer({
      size,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    // Copy from source to staging
    const encoder = this._device.createCommandEncoder();
    encoder.copyBufferToBuffer(srcBuf, 0, staging, 0, size);
    const cmdBuf = encoder.finish();
    this._device.queue.submit([cmdBuf]);

    // Map and read
    staging.mapAsync(GPUMapMode.READ);
    const data = staging.getMappedRange(0, size);
    staging.unmap();

    return new Uint8Array(data);
  }

  /**
   * WebGPU buffers are freed via destroy() or garbage collection.
   */
  protected _free(handle: unknown): void {
    const buf = handle as GPUBuffer;
    buf.destroy();
  }
}
