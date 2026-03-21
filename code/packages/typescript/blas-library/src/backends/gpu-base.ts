/**
 * GPU Backend Base -- shared logic for all six GPU-accelerated backends.
 *
 * === Why a Base Class for GPU Backends? ===
 *
 * All six GPU backends (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL) follow
 * the same pattern for every BLAS operation:
 *
 *     1. Convert Matrix/Vector data to bytes (Float32Array)
 *     2. Allocate device memory via the vendor API
 *     3. Upload data to the device
 *     4. Compute the result (CPU-side for correctness, through the GPU pipeline)
 *     5. Download results from the device
 *     6. Return new Matrix/Vector objects
 *
 * Since our device simulators operate synchronously and kernel execution is
 * simplified, the GPU backends perform the actual arithmetic on the CPU side
 * but still exercise the full GPU memory pipeline (allocate, upload, download).
 * This demonstrates the complete GPU programming pattern without requiring
 * a full GPU instruction compiler.
 *
 * The `GpuBlasBase` class provides all BLAS operations. Each GPU backend
 * subclass only needs to implement three template methods:
 *
 *     _upload(data: Uint8Array): unknown     Upload bytes to device memory
 *     _download(handle: unknown, size: number): Uint8Array  Download bytes
 *     _free(handle: unknown): void           Free device memory
 *
 * This is the Template Method design pattern from the Gang of Four.
 */

import { Matrix, Side, Transpose, Vector } from "../types.js";
import type { BlasBackend } from "../protocol.js";
import { CpuBlas } from "./cpu.js";

/**
 * Base class for GPU BLAS backends.
 *
 * ================================================================
 * GPU BLAS BASE -- TEMPLATE FOR ALL GPU BACKENDS
 * ================================================================
 *
 * This base class provides the full BLAS interface by:
 *
 * 1. Delegating the actual arithmetic to CpuBlas (the reference)
 * 2. Wrapping every call with GPU memory operations:
 *    - Upload input data to device memory
 *    - (Compute on CPU -- correct by construction)
 *    - Download results from device memory
 *
 * Each GPU backend subclass provides the vendor-specific memory
 * operations via _upload(), _download(), and _free().
 *
 * Why this approach?
 * - All 7 backends produce IDENTICAL results (correctness guarantee)
 * - The GPU memory pipeline is fully exercised (malloc, memcpy, free)
 * - We avoid the complexity of compiling BLAS kernels to GPU instructions
 * ================================================================
 */
export abstract class GpuBlasBase implements BlasBackend {
  /** The CPU reference implementation used for actual computation. */
  protected _cpu: CpuBlas;

  constructor() {
    this._cpu = new CpuBlas();
  }

  // =================================================================
  // Abstract properties -- subclasses must provide these
  // =================================================================

  abstract get name(): string;
  abstract get deviceName(): string;

  // =================================================================
  // Template methods -- subclasses override these
  // =================================================================

  /** Upload bytes to device memory. Returns a handle. */
  protected abstract _upload(data: Uint8Array): unknown;

  /** Download bytes from device memory. */
  protected abstract _download(handle: unknown, size: number): Uint8Array;

  /** Free device memory. */
  protected abstract _free(handle: unknown): void;

  // =================================================================
  // Helpers: serialize/deserialize Matrix and Vector
  // =================================================================

  /** Pack matrix data as little-endian floats. */
  protected _matrixToBytes(m: Matrix): Uint8Array {
    const buf = new Float32Array(m.data);
    return new Uint8Array(buf.buffer);
  }

  /** Pack vector data as little-endian floats. */
  protected _vectorToBytes(v: Vector): Uint8Array {
    const buf = new Float32Array(v.data);
    return new Uint8Array(buf.buffer);
  }

  /** Unpack little-endian floats from bytes. */
  protected _bytesToFloats(data: Uint8Array, count: number): number[] {
    const floatView = new Float32Array(
      data.buffer, data.byteOffset, count
    );
    return Array.from(floatView);
  }

  // =================================================================
  // GPU round-trip helper
  // =================================================================

  /** Upload a vector to GPU, download it back. Exercises the pipeline. */
  protected _gpuRoundTripVector(v: Vector): Vector {
    const dataBytes = this._vectorToBytes(v);
    const handle = this._upload(dataBytes);
    const resultBytes = this._download(handle, dataBytes.length);
    this._free(handle);
    const floats = this._bytesToFloats(resultBytes, v.size);
    return new Vector(floats, v.size);
  }

  /** Upload a matrix to GPU, download it back. Exercises the pipeline. */
  protected _gpuRoundTripMatrix(m: Matrix): Matrix {
    const dataBytes = this._matrixToBytes(m);
    const handle = this._upload(dataBytes);
    const resultBytes = this._download(handle, dataBytes.length);
    this._free(handle);
    const floats = this._bytesToFloats(resultBytes, m.rows * m.cols);
    return new Matrix(floats, m.rows, m.cols, m.order);
  }

  // =================================================================
  // BLAS operations -- compute on CPU, exercise GPU memory pipeline
  // =================================================================

  /** SAXPY via GPU pipeline. */
  saxpy(alpha: number, x: Vector, y: Vector): Vector {
    const hx = this._upload(this._vectorToBytes(x));
    const hy = this._upload(this._vectorToBytes(y));
    let result = this._cpu.saxpy(alpha, x, y);
    result = this._gpuRoundTripVector(result);
    this._free(hx);
    this._free(hy);
    return result;
  }

  /** DOT via GPU pipeline. */
  sdot(x: Vector, y: Vector): number {
    const hx = this._upload(this._vectorToBytes(x));
    const hy = this._upload(this._vectorToBytes(y));
    const result = this._cpu.sdot(x, y);
    this._free(hx);
    this._free(hy);
    return result;
  }

  /** NRM2 via GPU pipeline. */
  snrm2(x: Vector): number {
    const hx = this._upload(this._vectorToBytes(x));
    const result = this._cpu.snrm2(x);
    this._free(hx);
    return result;
  }

  /** SCAL via GPU pipeline. */
  sscal(alpha: number, x: Vector): Vector {
    const hx = this._upload(this._vectorToBytes(x));
    let result = this._cpu.sscal(alpha, x);
    result = this._gpuRoundTripVector(result);
    this._free(hx);
    return result;
  }

  /** ASUM via GPU pipeline. */
  sasum(x: Vector): number {
    const hx = this._upload(this._vectorToBytes(x));
    const result = this._cpu.sasum(x);
    this._free(hx);
    return result;
  }

  /** IAMAX via GPU pipeline. */
  isamax(x: Vector): number {
    const hx = this._upload(this._vectorToBytes(x));
    const result = this._cpu.isamax(x);
    this._free(hx);
    return result;
  }

  /** COPY via GPU pipeline. */
  scopy(x: Vector): Vector {
    return this._gpuRoundTripVector(x);
  }

  /** SWAP via GPU pipeline. */
  sswap(x: Vector, y: Vector): [Vector, Vector] {
    const hx = this._upload(this._vectorToBytes(x));
    const hy = this._upload(this._vectorToBytes(y));
    const result = this._cpu.sswap(x, y);
    this._free(hx);
    this._free(hy);
    return [
      this._gpuRoundTripVector(result[0]),
      this._gpuRoundTripVector(result[1]),
    ];
  }

  /** GEMV via GPU pipeline. */
  sgemv(
    trans: Transpose,
    alpha: number,
    a: Matrix,
    x: Vector,
    beta: number,
    y: Vector,
  ): Vector {
    const ha = this._upload(this._matrixToBytes(a));
    const hx = this._upload(this._vectorToBytes(x));
    const hy = this._upload(this._vectorToBytes(y));
    let result = this._cpu.sgemv(trans, alpha, a, x, beta, y);
    result = this._gpuRoundTripVector(result);
    this._free(ha);
    this._free(hx);
    this._free(hy);
    return result;
  }

  /** GER via GPU pipeline. */
  sger(alpha: number, x: Vector, y: Vector, a: Matrix): Matrix {
    const ha = this._upload(this._matrixToBytes(a));
    const hx = this._upload(this._vectorToBytes(x));
    const hy = this._upload(this._vectorToBytes(y));
    let result = this._cpu.sger(alpha, x, y, a);
    result = this._gpuRoundTripMatrix(result);
    this._free(ha);
    this._free(hx);
    this._free(hy);
    return result;
  }

  /** GEMM via GPU pipeline. */
  sgemm(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix {
    const ha = this._upload(this._matrixToBytes(a));
    const hb = this._upload(this._matrixToBytes(b));
    const hc = this._upload(this._matrixToBytes(c));
    let result = this._cpu.sgemm(transA, transB, alpha, a, b, beta, c);
    result = this._gpuRoundTripMatrix(result);
    this._free(ha);
    this._free(hb);
    this._free(hc);
    return result;
  }

  /** SYMM via GPU pipeline. */
  ssymm(
    side: Side,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix {
    const ha = this._upload(this._matrixToBytes(a));
    const hb = this._upload(this._matrixToBytes(b));
    const hc = this._upload(this._matrixToBytes(c));
    let result = this._cpu.ssymm(side, alpha, a, b, beta, c);
    result = this._gpuRoundTripMatrix(result);
    this._free(ha);
    this._free(hb);
    this._free(hc);
    return result;
  }

  /** Batched GEMM via GPU pipeline. */
  sgemmBatched(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    aList: Matrix[],
    bList: Matrix[],
    beta: number,
    cList: Matrix[],
  ): Matrix[] {
    return aList.map((a, i) =>
      this.sgemm(transA, transB, alpha, a, bList[i], beta, cList[i])
    );
  }
}
