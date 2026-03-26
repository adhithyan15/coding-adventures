# frozen_string_literal: true

# ---------------------------------------------------------------------------
# OpenGlBlas -- legacy OpenGL compute BLAS backend.
# ---------------------------------------------------------------------------
#
# === How OpenGlBlas Works ===
#
# This backend wraps GLContext from Layer 4. OpenGL uses a global state
# machine model -- you bind things to "current" state and then issue commands
# that operate on whatever is currently bound.
#
# For each BLAS operation:
#     1. gl.gen_buffers()        -- generate buffer IDs
#     2. gl.buffer_data()        -- allocate and upload data
#     3. (compute)               -- perform operation
#     4. gl.map_buffer_range()   -- map buffer for reading
#     5. gl.delete_buffers()     -- free buffers
#
# === Real OpenGL BLAS ===
#
# OpenGL compute shaders (4.3+) use Shader Storage Buffer Objects (SSBOs)
# for GPU-accessible storage. While OpenGL compute is mostly replaced by
# Vulkan/Metal for new projects, it remains widely available on older hardware.

module CodingAdventures
  module BlasLibrary
    module Backends
      class OpenGlBlas < GpuBlasBase
        # ================================================================
        # OPENGL BLAS -- LEGACY STATE MACHINE GPU ACCELERATION
        # ================================================================
        #
        # OpenGL is the oldest surviving GPU API (1992). Compute shaders
        # were added in OpenGL 4.3 (2012), bolted onto the existing state
        # machine model.
        #
        # The state machine means:
        # - glBindBuffer(target, id)  sets "current buffer" globally
        # - glBufferData(target, ...) operates on WHATEVER is currently bound
        # - You must remember what's bound at all times
        #
        # Simple for small programs, error-prone for large ones.
        #
        # Usage:
        #     blas = OpenGlBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          @gl = VendorApiSimulators::GLContext.new
        end

        # Backend identifier.
        def name
          "opengl"
        end

        # Human-readable device name.
        def device_name
          "OpenGL Device"
        end

        def _upload(data)
          buf_id = @gl.gen_buffers(1)[0]
          @gl.bind_buffer(VendorApiSimulators::GL_SHADER_STORAGE_BUFFER, buf_id)
          @gl.buffer_data(
            VendorApiSimulators::GL_SHADER_STORAGE_BUFFER,
            data.bytesize,
            data,
            VendorApiSimulators::GL_STATIC_DRAW
          )
          buf_id
        end

        def _download(handle, size)
          @gl.bind_buffer(VendorApiSimulators::GL_SHADER_STORAGE_BUFFER, handle)
          mapped = @gl.map_buffer_range(
            VendorApiSimulators::GL_SHADER_STORAGE_BUFFER,
            0,
            size,
            VendorApiSimulators::GL_MAP_READ_BIT
          )
          data = mapped.byteslice(0, size)
          @gl.unmap_buffer(VendorApiSimulators::GL_SHADER_STORAGE_BUFFER)
          data
        end

        def _free(handle)
          @gl.delete_buffers([handle])
        end
      end
    end
  end
end
