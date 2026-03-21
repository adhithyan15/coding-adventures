/**
 * OpenClBlas -- portable OpenCL BLAS backend.
 *
 * === How OpenClBlas Works ===
 *
 * This backend wraps `CLContext` and `CLCommandQueue` from Layer 4.
 * OpenCL's distinctive feature is event-based dependencies -- every enqueue
 * operation returns a CLEvent that subsequent operations can wait on.
 *
 * For each BLAS operation:
 *     1. ctx.createBuffer()            -- allocate device memory
 *     2. queue.enqueueWriteBuffer()    -- upload data (returns event)
 *     3. (compute)                     -- perform the operation
 *     4. queue.enqueueReadBuffer()     -- download results
 *     5. queue.finish()                -- wait for all operations
 *
 * OpenCL is the most portable GPU API -- it runs on NVIDIA, AMD, Intel GPUs,
 * and even CPUs and FPGAs.
 */

import {
  CLContext,
  CLMemFlags,
  type CLBuffer,
  type CLCommandQueue,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * OpenCL BLAS backend -- wraps CLContext from Layer 4.
 *
 * ================================================================
 * OPENCL BLAS -- PORTABLE GPU ACCELERATION
 * ================================================================
 *
 * OpenCL (Open Computing Language) is the Khronos Group's cross-platform
 * compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's
 * GPU and even on CPUs.
 *
 * Our simulator exercises the OpenCL memory pipeline:
 * createBuffer -> enqueueWrite -> compute -> enqueueRead -> finish
 *
 * Usage:
 *     const blas = new OpenClBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class OpenClBlas extends GpuBlasBase {
  private _ctx: CLContext;
  private _queue: CLCommandQueue;

  constructor() {
    super();
    this._ctx = new CLContext();
    this._queue = this._ctx.createCommandQueue();
  }

  get name(): string {
    return "opencl";
  }

  get deviceName(): string {
    return this._ctx._devices[0].name;
  }

  /**
   * Create a CLBuffer and upload data via enqueueWriteBuffer.
   *
   * OpenCL separates buffer creation from data upload:
   *     1. createBuffer(READ_WRITE, size)   -- allocate
   *     2. enqueueWriteBuffer(buf, data)    -- upload (async, returns event)
   */
  protected _upload(data: Uint8Array): CLBuffer {
    const buf = this._ctx.createBuffer(CLMemFlags.READ_WRITE, data.length);
    this._queue.enqueueWriteBuffer(buf, 0, data.length, data);
    return buf;
  }

  /**
   * Download data via enqueueReadBuffer.
   *
   * OpenCL's read is also asynchronous with event tracking:
   *     enqueueReadBuffer(buf, offset, size, hostPtr)
   * We call finish() to wait for completion.
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const buf = handle as CLBuffer;
    const hostBuf = new Uint8Array(size);
    this._queue.enqueueReadBuffer(buf, 0, size, hostBuf);
    this._queue.finish();
    return hostBuf;
  }

  /**
   * OpenCL buffers are freed when the context is destroyed.
   *
   * In our simulator, there's no explicit free for CLBuffer.
   * The buffer will be garbage collected with the context.
   */
  protected _free(_handle: unknown): void {
    // CLBuffer doesn't have an explicit free
  }
}
