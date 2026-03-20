# frozen_string_literal: true

# ---------------------------------------------------------------------------
# MetalBlas -- Apple Metal BLAS backend.
# ---------------------------------------------------------------------------
#
# === How MetalBlas Works ===
#
# This backend wraps MTLDevice from Layer 4. Metal's key advantage is
# **unified memory** -- on Apple Silicon, CPU and GPU share the same RAM.
# This means no host-to-device copies:
#
#     CUDA:   cuda.malloc -> cuda.memcpy(H2D) -> compute -> cuda.memcpy(D2H) -> cuda.free
#     Metal:  make_buffer -> write_bytes       -> compute -> contents
#
# The buffer is always accessible from both CPU and GPU, so writes are
# immediate and reads require no copy.
#
# === Real Accelerate/MPS ===
#
# On real Apple hardware, Metal Performance Shaders (MPS) provides optimized
# BLAS operations that leverage the Apple GPU's unified memory architecture.
# PyTorch MPS backend uses this.

module CodingAdventures
  module BlasLibrary
    module Backends
      class MetalBlas < GpuBlasBase
        # ================================================================
        # METAL BLAS -- APPLE SILICON UNIFIED MEMORY
        # ================================================================
        #
        # Metal's unified memory model eliminates host-device copies:
        # - make_buffer allocates memory visible to both CPU and GPU
        # - write_bytes writes directly (no staging buffer needed)
        # - contents reads directly (no download needed)
        #
        # This is the biggest ergonomic advantage of Apple Silicon for GPU
        # computing.
        #
        # Usage:
        #     blas = MetalBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          @device = VendorApiSimulators::MTLDevice.new
        end

        # Backend identifier.
        def name
          "metal"
        end

        # Human-readable device name.
        def device_name
          @device.name
        end

        def _upload(data)
          buf = @device.make_buffer(data.bytesize)
          buf.write_bytes(data)
          buf
        end

        def _download(handle, size)
          contents = handle.contents
          contents.byteslice(0, size)
        end

        def _free(_handle)
          # Metal uses automatic reference counting
        end
      end
    end
  end
end
