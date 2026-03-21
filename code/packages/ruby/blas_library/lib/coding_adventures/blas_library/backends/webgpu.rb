# frozen_string_literal: true

# ---------------------------------------------------------------------------
# WebGpuBlas -- browser-friendly WebGPU BLAS backend.
# ---------------------------------------------------------------------------
#
# === How WebGpuBlas Works ===
#
# This backend wraps GPUDevice from Layer 4. WebGPU is designed for safe,
# browser-based GPU compute with automatic synchronization.
#
# For each BLAS operation:
#     1. device.create_buffer(STORAGE | COPY_DST) -- allocate with usage flags
#     2. device.queue.write_buffer()              -- upload data
#     3. (compute)                                 -- perform operation
#     4. Create a MAP_READ staging buffer, copy, map, read
#     5. Buffer goes out of scope (auto-freed)
#
# === Real WebGPU BLAS ===
#
# WebGPU's key simplification: a single queue (device.queue) handles
# everything. No queue families, no multiple queues. It provides a safe,
# validated GPU API designed for browsers with automatic barriers and
# usage-based buffer creation.

module CodingAdventures
  module BlasLibrary
    module Backends
      class WebGpuBlas < GpuBlasBase
        # ================================================================
        # WEBGPU BLAS -- SAFE BROWSER-FIRST GPU ACCELERATION
        # ================================================================
        #
        # WebGPU provides a safe, validated GPU API designed for browsers:
        # - Single queue (device.queue)
        # - Automatic barriers (no manual synchronization)
        # - Usage-based buffer creation (STORAGE, COPY_SRC, COPY_DST, MAP_READ)
        #
        # Usage:
        #     blas = WebGpuBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          gpu = VendorApiSimulators::GPU.new
          adapter = gpu.request_adapter
          @device = adapter.request_device
        end

        # Backend identifier.
        def name
          "webgpu"
        end

        # Human-readable device name.
        def device_name
          "WebGPU Device"
        end

        def _upload(data)
          desc = VendorApiSimulators::GPUBufferDescriptor.new(
            size: data.bytesize,
            usage: VendorApiSimulators::GPUBufferUsage::STORAGE |
              VendorApiSimulators::GPUBufferUsage::COPY_DST |
              VendorApiSimulators::GPUBufferUsage::COPY_SRC
          )
          buf = @device.create_buffer(desc)
          @device.queue.write_buffer(buf, 0, data)
          buf
        end

        def _download(handle, size)
          # Create a staging buffer for readback
          staging_desc = VendorApiSimulators::GPUBufferDescriptor.new(
            size: size,
            usage: VendorApiSimulators::GPUBufferUsage::MAP_READ |
              VendorApiSimulators::GPUBufferUsage::COPY_DST
          )
          staging = @device.create_buffer(staging_desc)

          # Copy from source to staging
          encoder = @device.create_command_encoder
          encoder.copy_buffer_to_buffer(handle, 0, staging, 0, size)
          cmd_buf = encoder.finish
          @device.queue.submit([cmd_buf])

          # Map and read
          staging.map_async("read")
          data = staging.get_mapped_range(offset: 0, range_size: size)
          staging.unmap
          data
        end

        def _free(handle)
          handle.destroy
        end
      end
    end
  end
end
