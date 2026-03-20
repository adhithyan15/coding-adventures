# frozen_string_literal: true

# ---------------------------------------------------------------------------
# CudaBlas -- NVIDIA CUDA BLAS backend.
# ---------------------------------------------------------------------------
#
# === How CudaBlas Works ===
#
# This backend wraps the CUDARuntime from Layer 4 (vendor_api_simulators).
# For each BLAS operation, it follows the classic CUDA pattern:
#
#     1. cuda.malloc           -- allocate device memory for inputs and output
#     2. cuda.memcpy(:host_to_device)  -- upload input data from host to device
#     3. (compute)             -- perform the operation
#     4. cuda.memcpy(:device_to_host)  -- download results from device to host
#     5. cuda.free             -- release device memory
#
# Since our simulator's kernel execution is simplified, the actual arithmetic
# is performed by the CPU reference (CpuBlas). The GPU memory pipeline is
# fully exercised to demonstrate the CUDA programming pattern.
#
# === Real cuBLAS ===
#
# In the real world, cublasSgemm() launches highly optimized CUDA kernels
# that tile the computation across thousands of GPU threads, using shared
# memory, warp-level primitives, and tensor cores. Our simulator demonstrates
# the memory management pattern without that complexity.

module CodingAdventures
  module BlasLibrary
    module Backends
      class CudaBlas < GpuBlasBase
        # ================================================================
        # CUDA BLAS -- NVIDIA GPU ACCELERATION
        # ================================================================
        #
        # The most widely used GPU BLAS backend in ML. Real cuBLAS achieves
        # near-peak FLOPS on NVIDIA GPUs through:
        # - Tiled GEMM with shared memory
        # - Tensor Core acceleration (FP16/TF32)
        # - Warp-level matrix multiply (WMMA)
        #
        # Our simulator demonstrates the memory management pattern:
        # cuda.malloc -> cuda.memcpy(H2D) -> compute -> cuda.memcpy(D2H) -> cuda.free
        #
        # Usage:
        #     blas = CudaBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          @cuda = VendorApiSimulators::CUDARuntime.new
        end

        # Backend identifier.
        def name
          "cuda"
        end

        # Human-readable device name from CUDA properties.
        def device_name
          props = @cuda.get_device_properties
          props.name
        end

        def _upload(data)
          ptr = @cuda.malloc(data.bytesize)
          @cuda.memcpy(ptr, data, data.bytesize, :host_to_device)
          ptr
        end

        def _download(handle, size)
          host_buf = (+("\x00" * size)).force_encoding(Encoding::BINARY)
          @cuda.memcpy(host_buf, handle, size, :device_to_host)
          host_buf
        end

        def _free(handle)
          @cuda.free(handle)
        end
      end
    end
  end
end
