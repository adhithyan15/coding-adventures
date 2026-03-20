/**
 * CudaBlas -- NVIDIA CUDA BLAS backend.
 *
 * === How CudaBlas Works ===
 *
 * This backend wraps the `CUDARuntime` from Layer 4 (vendor-api-simulators).
 * For each BLAS operation, it follows the classic CUDA pattern:
 *
 *     1. cudaMalloc()           -- allocate device memory for inputs and output
 *     2. cudaMemcpy(H2D)       -- upload input data from host to device
 *     3. (compute)             -- perform the operation
 *     4. cudaMemcpy(D2H)       -- download results from device to host
 *     5. cudaFree()            -- release device memory
 *
 * Since our simulator's kernel execution is simplified, the actual arithmetic
 * is performed by the CPU reference (CpuBlas). The GPU memory pipeline is
 * fully exercised to demonstrate the CUDA programming pattern.
 *
 * === Real cuBLAS ===
 *
 * In the real world, `cublasSgemm()` launches highly optimized CUDA kernels
 * that tile the computation across thousands of GPU threads, using shared
 * memory, warp-level primitives, and tensor cores. Our simulator demonstrates
 * the memory management pattern without that complexity.
 */

import {
  CUDARuntime,
  CUDAMemcpyKind,
  type CUDADevicePtr,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * CUDA BLAS backend -- wraps CUDARuntime from Layer 4.
 *
 * ================================================================
 * CUDA BLAS -- NVIDIA GPU ACCELERATION
 * ================================================================
 *
 * The most widely used GPU BLAS backend in ML. Real cuBLAS achieves
 * near-peak FLOPS on NVIDIA GPUs through:
 * - Tiled GEMM with shared memory
 * - Tensor Core acceleration (FP16/TF32)
 * - Warp-level matrix multiply (WMMA)
 *
 * Our simulator demonstrates the memory management pattern:
 * cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
 *
 * Usage:
 *     const blas = new CudaBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class CudaBlas extends GpuBlasBase {
  private _cuda: CUDARuntime;

  constructor() {
    super();
    this._cuda = new CUDARuntime();
  }

  get name(): string {
    return "cuda";
  }

  get deviceName(): string {
    const props = this._cuda.getDeviceProperties();
    return props.name;
  }

  /**
   * Allocate GPU memory and upload data via cudaMemcpy(H2D).
   *
   * The classic CUDA pattern:
   *     1. cudaMalloc(ptr, size)  -- allocate on device
   *     2. cudaMemcpy(ptr, hostData, size, HostToDevice) -- upload
   */
  protected _upload(data: Uint8Array): CUDADevicePtr {
    const ptr = this._cuda.malloc(data.length);
    this._cuda.memcpy(ptr, data, data.length, CUDAMemcpyKind.HostToDevice);
    return ptr;
  }

  /**
   * Download data from GPU via cudaMemcpy(D2H).
   *
   * The reverse of upload:
   *     cudaMemcpy(hostBuf, devicePtr, size, DeviceToHost)
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const ptr = handle as CUDADevicePtr;
    const hostBuf = new Uint8Array(size);
    this._cuda.memcpy(hostBuf, ptr, size, CUDAMemcpyKind.DeviceToHost);
    return hostBuf;
  }

  /**
   * Free GPU memory via cudaFree().
   */
  protected _free(handle: unknown): void {
    this._cuda.free(handle as CUDADevicePtr);
  }
}
