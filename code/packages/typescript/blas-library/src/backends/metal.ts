/**
 * MetalBlas -- Apple Metal BLAS backend.
 *
 * === How MetalBlas Works ===
 *
 * This backend wraps `MTLDevice` from Layer 4. Metal's key advantage is
 * **unified memory** -- on Apple Silicon, CPU and GPU share the same RAM.
 * This means no host-to-device copies:
 *
 *     CUDA:   cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
 *     Metal:  makeBuffer -> writeBytes       -> compute -> contents()
 *
 * The buffer is always accessible from both CPU and GPU, so writes are
 * immediate and reads require no copy.
 *
 * === Real Accelerate/MPS ===
 *
 * On real Apple hardware, Metal Performance Shaders (MPS) provides optimized
 * BLAS operations that leverage the Apple GPU's unified memory architecture.
 * PyTorch MPS backend uses this.
 */

import {
  MTLDevice,
  type MTLBuffer,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * Metal BLAS backend -- wraps MTLDevice from Layer 4.
 *
 * ================================================================
 * METAL BLAS -- APPLE SILICON UNIFIED MEMORY
 * ================================================================
 *
 * Metal's unified memory model eliminates host-device copies:
 * - makeBuffer() allocates memory visible to both CPU and GPU
 * - writeBytes() writes directly (no staging buffer needed)
 * - contents() reads directly (no download needed)
 *
 * This is the biggest ergonomic advantage of Apple Silicon for GPU
 * computing.
 *
 * Usage:
 *     const blas = new MetalBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class MetalBlas extends GpuBlasBase {
  private _device: MTLDevice;

  constructor() {
    super();
    this._device = new MTLDevice();
  }

  get name(): string {
    return "metal";
  }

  get deviceName(): string {
    return this._device.name;
  }

  /**
   * Create a Metal buffer with unified memory and write data.
   *
   * Metal's unified memory means the buffer is immediately accessible
   * from both CPU and GPU -- no separate upload step needed.
   */
  protected _upload(data: Uint8Array): MTLBuffer {
    const buf = this._device.makeBuffer(data.length);
    buf.writeBytes(data);
    return buf;
  }

  /**
   * Read directly from the Metal buffer (unified memory).
   *
   * contents() returns a CPU-accessible view of GPU memory.
   * On real Apple Silicon, this is literally the same memory --
   * no copy happens.
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const buf = handle as MTLBuffer;
    const contents = buf.contents();
    return new Uint8Array(contents.slice(0, size));
  }

  /**
   * Metal buffers are freed when they go out of scope.
   *
   * In our simulator, Metal uses ARC-style memory management.
   * The buffer will be deallocated when no references remain.
   */
  protected _free(_handle: unknown): void {
    // Metal uses automatic reference counting
  }
}
