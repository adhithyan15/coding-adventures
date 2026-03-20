# frozen_string_literal: true

# ---------------------------------------------------------------------------
# OpenClBlas -- portable OpenCL BLAS backend.
# ---------------------------------------------------------------------------
#
# === How OpenClBlas Works ===
#
# This backend wraps CLContext and CLCommandQueue from Layer 4
# (vendor_api_simulators). OpenCL's distinctive feature is event-based
# dependencies -- every enqueue operation returns a CLEvent that subsequent
# operations can wait on.
#
# For each BLAS operation:
#     1. ctx.create_buffer()            -- allocate device memory
#     2. queue.enqueue_write_buffer()   -- upload data (returns event)
#     3. (compute)                      -- perform the operation
#     4. queue.enqueue_read_buffer()    -- download results (waits on compute)
#     5. queue.finish()                 -- wait for all operations
#
# === Real OpenCL BLAS ===
#
# OpenCL is the most portable GPU API -- it runs on NVIDIA, AMD, Intel GPUs,
# and even CPUs and FPGAs. Libraries like clBLAS and CLBlast provide optimized
# BLAS kernels for OpenCL. Our simulator demonstrates the memory management
# pattern without that complexity.

module CodingAdventures
  module BlasLibrary
    module Backends
      class OpenClBlas < GpuBlasBase
        # ================================================================
        # OPENCL BLAS -- PORTABLE GPU ACCELERATION
        # ================================================================
        #
        # OpenCL (Open Computing Language) is the Khronos Group's cross-
        # platform compute API. Unlike CUDA (NVIDIA only), OpenCL runs on
        # any vendor's GPU and even on CPUs.
        #
        # Our simulator exercises the OpenCL memory pipeline:
        # create_buffer -> enqueue_write -> compute -> enqueue_read -> finish
        #
        # Usage:
        #     blas = OpenClBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          @ctx = VendorApiSimulators::CLContext.new
          @queue = @ctx.create_command_queue
        end

        # Backend identifier.
        def name
          "opencl"
        end

        # Human-readable device name from OpenCL device info.
        def device_name
          @ctx._devices[0].name
        end

        def _upload(data)
          buf = @ctx.create_buffer(VendorApiSimulators::CLMemFlags::READ_WRITE, data.bytesize)
          @queue.enqueue_write_buffer(buf, 0, data.bytesize, data)
          buf
        end

        def _download(handle, size)
          host_buf = (+("\x00" * size)).force_encoding(Encoding::BINARY)
          @queue.enqueue_read_buffer(handle, 0, size, host_buf)
          @queue.finish
          host_buf
        end

        def _free(_handle)
          # OpenCL buffers are freed when the context is destroyed.
          # In our simulator, there's no explicit free for CLBuffer.
        end
      end
    end
  end
end
