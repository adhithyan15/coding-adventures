# frozen_string_literal: true

# ---------------------------------------------------------------------------
# GPU Backend Base -- shared logic for all six GPU-accelerated backends.
# ---------------------------------------------------------------------------
#
# === Why a Base Class for GPU Backends? ===
#
# All six GPU backends (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL) follow
# the same pattern for every BLAS operation:
#
#     1. Convert Matrix/Vector data to bytes (Array#pack)
#     2. Allocate device memory via the vendor API
#     3. Upload data to the device
#     4. Compute the result (CPU-side for correctness, through the GPU pipeline)
#     5. Download results from the device
#     6. Return new Matrix/Vector objects
#
# Since our device simulators operate synchronously and kernel execution is
# simplified, the GPU backends perform the actual arithmetic on the CPU side
# but still exercise the full GPU memory pipeline (allocate, upload, download).
# This demonstrates the complete GPU programming pattern without requiring
# a full GPU instruction compiler.
#
# The GpuBlasBase class provides all BLAS operations. Each GPU backend
# subclass only needs to implement three template methods:
#
#     _upload(data_bytes) -> handle     Upload bytes to device memory
#     _download(handle, size) -> bytes  Download bytes from device memory
#     _free(handle)                     Free device memory
#
# This is the Template Method design pattern from the Gang of Four.

module CodingAdventures
  module BlasLibrary
    module Backends
      class GpuBlasBase
        # ================================================================
        # GPU BLAS BASE -- TEMPLATE FOR ALL GPU BACKENDS
        # ================================================================
        #
        # This base class provides the full BLAS interface by:
        #
        # 1. Delegating the actual arithmetic to CpuBlas (the reference)
        # 2. Wrapping every call with GPU memory operations:
        #    - Upload input data to device memory
        #    - (Compute on CPU -- correct by construction)
        #    - Download results from device memory
        #
        # Each GPU backend subclass provides the vendor-specific memory
        # operations via _upload(), _download(), and _free().
        #
        # Why this approach?
        # - All 7 backends produce IDENTICAL results (correctness guarantee)
        # - The GPU memory pipeline is fully exercised (malloc, memcpy, free)
        # - We avoid the complexity of compiling BLAS kernels to GPU instructions
        # ================================================================

        def initialize
          @cpu = CpuBlas.new
        end

        # =================================================================
        # Template methods -- subclasses override these
        # =================================================================

        # Upload bytes to device memory. Returns a handle.
        def _upload(data)
          raise NotImplementedError
        end

        # Download bytes from device memory.
        def _download(handle, size)
          raise NotImplementedError
        end

        # Free device memory.
        def _free(handle)
          raise NotImplementedError
        end

        # =================================================================
        # Helpers: serialize/deserialize Matrix and Vector
        # =================================================================

        # Pack matrix data as little-endian single-precision floats.
        def _matrix_to_bytes(m)
          m.data.pack("e*")
        end

        # Pack vector data as little-endian single-precision floats.
        def _vector_to_bytes(v)
          v.data.pack("e*")
        end

        # Unpack little-endian single-precision floats from bytes.
        def _bytes_to_floats(data, count)
          data.byteslice(0, count * 4).unpack("e#{count}")
        end

        # =================================================================
        # GPU round-trip helper
        # =================================================================

        # Upload a vector to GPU, download it back. Exercises the pipeline.
        def _gpu_round_trip_vector(v)
          data_bytes = _vector_to_bytes(v)
          handle = _upload(data_bytes)
          result_bytes = _download(handle, data_bytes.bytesize)
          _free(handle)
          floats = _bytes_to_floats(result_bytes, v.size)
          Vector.new(data: floats, size: v.size)
        end

        # Upload a matrix to GPU, download it back. Exercises the pipeline.
        def _gpu_round_trip_matrix(m)
          data_bytes = _matrix_to_bytes(m)
          handle = _upload(data_bytes)
          result_bytes = _download(handle, data_bytes.bytesize)
          _free(handle)
          floats = _bytes_to_floats(result_bytes, m.rows * m.cols)
          Matrix.new(data: floats, rows: m.rows, cols: m.cols, order: m.order)
        end

        # =================================================================
        # BLAS operations -- compute on CPU, exercise GPU memory pipeline
        # =================================================================

        def saxpy(alpha, x, y)
          hx = _upload(_vector_to_bytes(x))
          hy = _upload(_vector_to_bytes(y))
          result = @cpu.saxpy(alpha, x, y)
          result = _gpu_round_trip_vector(result)
          _free(hx)
          _free(hy)
          result
        end

        def sdot(x, y)
          hx = _upload(_vector_to_bytes(x))
          hy = _upload(_vector_to_bytes(y))
          result = @cpu.sdot(x, y)
          _free(hx)
          _free(hy)
          result
        end

        def snrm2(x)
          hx = _upload(_vector_to_bytes(x))
          result = @cpu.snrm2(x)
          _free(hx)
          result
        end

        def sscal(alpha, x)
          hx = _upload(_vector_to_bytes(x))
          result = @cpu.sscal(alpha, x)
          result = _gpu_round_trip_vector(result)
          _free(hx)
          result
        end

        def sasum(x)
          hx = _upload(_vector_to_bytes(x))
          result = @cpu.sasum(x)
          _free(hx)
          result
        end

        def isamax(x)
          hx = _upload(_vector_to_bytes(x))
          result = @cpu.isamax(x)
          _free(hx)
          result
        end

        def scopy(x)
          _gpu_round_trip_vector(x)
        end

        def sswap(x, y)
          hx = _upload(_vector_to_bytes(x))
          hy = _upload(_vector_to_bytes(y))
          result = @cpu.sswap(x, y)
          _free(hx)
          _free(hy)
          [
            _gpu_round_trip_vector(result[0]),
            _gpu_round_trip_vector(result[1])
          ]
        end

        def sgemv(trans, alpha, a, x, beta, y)
          ha = _upload(_matrix_to_bytes(a))
          hx = _upload(_vector_to_bytes(x))
          hy = _upload(_vector_to_bytes(y))
          result = @cpu.sgemv(trans, alpha, a, x, beta, y)
          result = _gpu_round_trip_vector(result)
          _free(ha)
          _free(hx)
          _free(hy)
          result
        end

        def sger(alpha, x, y, a)
          ha = _upload(_matrix_to_bytes(a))
          hx = _upload(_vector_to_bytes(x))
          hy = _upload(_vector_to_bytes(y))
          result = @cpu.sger(alpha, x, y, a)
          result = _gpu_round_trip_matrix(result)
          _free(ha)
          _free(hx)
          _free(hy)
          result
        end

        def sgemm(trans_a, trans_b, alpha, a, b, beta, c)
          ha = _upload(_matrix_to_bytes(a))
          hb = _upload(_matrix_to_bytes(b))
          hc = _upload(_matrix_to_bytes(c))
          result = @cpu.sgemm(trans_a, trans_b, alpha, a, b, beta, c)
          result = _gpu_round_trip_matrix(result)
          _free(ha)
          _free(hb)
          _free(hc)
          result
        end

        def ssymm(side, alpha, a, b, beta, c)
          ha = _upload(_matrix_to_bytes(a))
          hb = _upload(_matrix_to_bytes(b))
          hc = _upload(_matrix_to_bytes(c))
          result = @cpu.ssymm(side, alpha, a, b, beta, c)
          result = _gpu_round_trip_matrix(result)
          _free(ha)
          _free(hb)
          _free(hc)
          result
        end

        def sgemm_batched(trans_a, trans_b, alpha, a_list, b_list, beta, c_list)
          a_list.zip(b_list, c_list).map do |a, b, c_mat|
            sgemm(trans_a, trans_b, alpha, a, b, beta, c_mat)
          end
        end
      end
    end
  end
end
