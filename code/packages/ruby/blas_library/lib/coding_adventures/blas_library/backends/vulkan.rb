# frozen_string_literal: true

# ---------------------------------------------------------------------------
# VulkanBlas -- explicit Vulkan BLAS backend.
# ---------------------------------------------------------------------------
#
# === How VulkanBlas Works ===
#
# This backend wraps the Vulkan API from Layer 4. Vulkan is the most verbose
# GPU API -- you explicitly manage everything: buffer creation, memory
# allocation, binding, mapping, and unmapping.
#
# For each BLAS operation, we allocate VkDeviceMemory, write data via the
# underlying memory manager's map/write/unmap cycle, and read it back the
# same way.
#
# === Real Vulkan BLAS ===
#
# Vulkan forces you to be explicit about everything because:
#     1. No hidden allocations -- you control every byte of memory
#     2. No implicit sync -- you insert every barrier yourself
#     3. No automatic resource tracking -- you free what you allocate
#     4. No driver guessing -- you tell the driver exactly what you need
#
# The reward is maximum performance and predictability.

module CodingAdventures
  module BlasLibrary
    module Backends
      class VulkanBlas < GpuBlasBase
        # ================================================================
        # VULKAN BLAS -- MAXIMUM CONTROL GPU ACCELERATION
        # ================================================================
        #
        # Vulkan forces you to be explicit about everything:
        # - Buffer creation with usage flags
        # - Memory allocation with property flags
        # - Explicit map/unmap for data transfer
        #
        # The reward is maximum performance and predictability -- the driver
        # does exactly what you say, nothing more.
        #
        # Usage:
        #     blas = VulkanBlas.new
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        def initialize
          super
          @vk_instance = VendorApiSimulators::VkInstance.new
          physical_devices = @vk_instance.vk_enumerate_physical_devices
          @vk_device = @vk_instance.vk_create_device(physical_devices[0])
        end

        # Backend identifier.
        def name
          "vulkan"
        end

        # Human-readable device name.
        def device_name
          "Vulkan Device"
        end

        def _upload(data)
          alloc_info = VendorApiSimulators::VkMemoryAllocateInfo.new(
            size: data.bytesize,
            memory_type_index: 0
          )
          memory = @vk_device.vk_allocate_memory(alloc_info)

          # Write through the underlying memory manager (Layer 5)
          mm = memory._mm
          mapped = mm.map(memory._buffer)
          mapped.write(0, data)
          mm.unmap(memory._buffer)

          memory
        end

        def _download(handle, size)
          mm = handle._mm
          mm.invalidate(handle._buffer)
          mapped = mm.map(handle._buffer)
          data = mapped.read(0, size)
          mm.unmap(handle._buffer)
          data
        end

        def _free(_handle)
          # In our simulator, memory is freed by garbage collection.
        end
      end
    end
  end
end
