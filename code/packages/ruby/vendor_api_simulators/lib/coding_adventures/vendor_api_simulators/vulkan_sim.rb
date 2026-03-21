# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Vulkan Runtime Simulator -- the thinnest wrapper over Layer 5.
# ---------------------------------------------------------------------------
#
# === What is Vulkan? ===
#
# Vulkan is the Khronos Group's low-level, cross-platform GPU API. It's the
# most explicit GPU API -- you manage everything: memory types, command buffer
# recording, queue submission, synchronization barriers, descriptor set layouts.
#
# Because our Layer 5 compute runtime is already Vulkan-inspired, this
# simulator is the *thinnest wrapper* of all six. It mainly adds:
#
#     1. Vulkan naming conventions (the vk_ prefix on all methods)
#     2. Vulkan-specific structures (VkBufferCreateInfo, VkSubmitInfo, etc.)
#     3. VkResult return codes instead of Ruby exceptions
#     4. VkCommandPool for grouping command buffers
#
# === Why Vulkan is So Verbose ===
#
# Vulkan forces you to be explicit about everything because:
#
#     1. No hidden allocations -- you control every byte of memory
#     2. No implicit sync -- you insert every barrier yourself
#     3. No automatic resource tracking -- you free what you allocate
#     4. No driver guessing -- you tell the driver exactly what you need

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # Vulkan enums
    # =====================================================================

    # Vulkan function return codes.
    module VkResult
      SUCCESS                     = 0
      NOT_READY                   = 1
      TIMEOUT                     = 2
      ERROR_OUT_OF_DEVICE_MEMORY  = -3
      ERROR_DEVICE_LOST           = -4
      ERROR_INITIALIZATION_FAILED = -5
    end

    module VkPipelineBindPoint
      COMPUTE = "compute"
    end

    # Vulkan buffer usage flags (bit flags, combinable with |).
    module VkBufferUsageFlagBits
      STORAGE_BUFFER = 1
      UNIFORM_BUFFER = 2
      TRANSFER_SRC   = 4
      TRANSFER_DST   = 8
    end

    # Vulkan memory property flags (bit flags, combinable with |).
    module VkMemoryPropertyFlagBits
      DEVICE_LOCAL  = 1
      HOST_VISIBLE  = 2
      HOST_COHERENT = 4
      HOST_CACHED   = 8
    end

    module VkSharingMode
      EXCLUSIVE  = "exclusive"
      CONCURRENT = "concurrent"
    end

    # =====================================================================
    # Vulkan create-info structures
    # =====================================================================

    VkBufferCreateInfo = Data.define(:size, :usage, :sharing_mode) do
      def initialize(
        size: 0,
        usage: VkBufferUsageFlagBits::STORAGE_BUFFER,
        sharing_mode: VkSharingMode::EXCLUSIVE
      )
        super
      end
    end

    VkMemoryAllocateInfo = Data.define(:size, :memory_type_index) do
      def initialize(size: 0, memory_type_index: 0)
        super
      end
    end

    VkShaderModuleCreateInfo = Data.define(:code) do
      def initialize(code: nil)
        super
      end
    end

    VkComputePipelineCreateInfo = Data.define(:shader_stage, :layout) do
      def initialize(shader_stage: nil, layout: nil)
        super
      end
    end

    VkPipelineShaderStageCreateInfo = Data.define(:stage, :mod, :entry_point) do
      def initialize(stage: "compute", mod: nil, entry_point: "main")
        super
      end
    end

    VkSubmitInfo = Data.define(:command_buffers, :wait_semaphores, :signal_semaphores) do
      def initialize(command_buffers: [], wait_semaphores: [], signal_semaphores: [])
        super
      end
    end

    VkBufferCopy = Data.define(:src_offset, :dst_offset, :size) do
      def initialize(src_offset: 0, dst_offset: 0, size: 0)
        super
      end
    end

    VkWriteDescriptorSet = Data.define(:dst_set, :dst_binding, :descriptor_type, :buffer_info) do
      def initialize(dst_set: nil, dst_binding: 0, descriptor_type: "storage", buffer_info: nil)
        super
      end
    end

    VkDescriptorBufferInfo = Data.define(:buffer, :offset, :range) do
      def initialize(buffer: nil, offset: 0, range: 0)
        super
      end
    end

    VkCommandPoolCreateInfo = Data.define(:queue_family_index) do
      def initialize(queue_family_index: 0)
        super
      end
    end

    VkDescriptorSetLayoutCreateInfo = Data.define(:bindings) do
      def initialize(bindings: [])
        super
      end
    end

    VkDescriptorSetLayoutBinding = Data.define(:binding, :descriptor_type, :descriptor_count) do
      def initialize(binding: 0, descriptor_type: "storage", descriptor_count: 1)
        super
      end
    end

    VkPipelineLayoutCreateInfo = Data.define(:set_layouts, :push_constant_size) do
      def initialize(set_layouts: [], push_constant_size: 0)
        super
      end
    end

    VkDescriptorSetAllocateInfo = Data.define(:set_layouts) do
      def initialize(set_layouts: [])
        super
      end
    end

    # =====================================================================
    # Vulkan wrapper objects -- thin wrappers over Layer 5
    # =====================================================================

    # Vulkan physical device -- wraps Layer 5 PhysicalDevice.
    class VkPhysicalDevice
      attr_reader :_physical

      def initialize(physical)
        @_physical = physical
      end

      def vk_get_physical_device_properties
        {
          "device_name" => @_physical.name,
          "device_type" => @_physical.device_type.to_s,
          "vendor" => @_physical.vendor
        }
      end

      def vk_get_physical_device_memory_properties
        mp = @_physical.memory_properties
        {
          "heap_count" => mp.heaps.length,
          "heaps" => mp.heaps.map { |h| {"size" => h.size, "flags" => h.flags.to_s} },
          "is_unified" => mp.is_unified
        }
      end

      def vk_get_physical_device_queue_family_properties
        @_physical.queue_families.map do |qf|
          {"queue_type" => qf.queue_type.to_s, "queue_count" => qf.count}
        end
      end
    end

    # Vulkan buffer -- wraps Layer 5 Buffer.
    class VkBuffer
      attr_reader :_buffer

      def initialize(buffer)
        @_buffer = buffer
      end

      def size
        @_buffer.size
      end
    end

    # Vulkan device memory -- wraps Layer 5 Buffer's memory.
    class VkDeviceMemory
      attr_reader :_buffer, :_mm

      def initialize(buffer, memory_manager)
        @_buffer = buffer
        @_mm = memory_manager
      end
    end

    # Vulkan shader module -- wraps Layer 5 ShaderModule.
    class VkShaderModule
      attr_reader :_shader

      def initialize(shader)
        @_shader = shader
      end
    end

    # Vulkan pipeline -- wraps Layer 5 Pipeline.
    class VkPipeline
      attr_reader :_pipeline

      def initialize(pipeline)
        @_pipeline = pipeline
      end
    end

    # Vulkan descriptor set layout.
    class VkDescriptorSetLayout
      attr_reader :_layout

      def initialize(layout)
        @_layout = layout
      end
    end

    # Vulkan pipeline layout.
    class VkPipelineLayout
      attr_reader :_layout

      def initialize(layout)
        @_layout = layout
      end
    end

    # Vulkan descriptor set.
    class VkDescriptorSet
      attr_reader :_ds

      def initialize(descriptor_set)
        @_ds = descriptor_set
      end
    end

    # Vulkan fence.
    class VkFence
      attr_reader :_fence

      def initialize(fence)
        @_fence = fence
      end

      def signaled
        @_fence.signaled
      end
    end

    # Vulkan semaphore.
    class VkSemaphore
      attr_reader :_semaphore

      def initialize(semaphore)
        @_semaphore = semaphore
      end
    end

    # Vulkan command pool -- groups command buffers.
    class VkCommandPool
      def initialize(device, queue_family_index)
        @device = device
        @queue_family_index = queue_family_index
        @command_buffers = []
      end

      def vk_allocate_command_buffers(count)
        cbs = []
        count.times do
          inner_cb = @device._logical.create_command_buffer
          vk_cb = VkCommandBuffer.new(inner_cb)
          cbs << vk_cb
          @command_buffers << vk_cb
        end
        cbs
      end

      def vk_reset_command_pool
        @command_buffers.each { |vk_cb| vk_cb._cb.reset }
      end

      def vk_free_command_buffers(buffers)
        buffers.each { |buf| @command_buffers.delete(buf) }
      end
    end

    # Vulkan command buffer -- wraps Layer 5 CommandBuffer with vk_ prefix.
    class VkCommandBuffer
      attr_reader :_cb

      def initialize(cb)
        @_cb = cb
      end

      def vk_begin_command_buffer(flags: 0)
        @_cb.begin
      end

      def vk_end_command_buffer
        @_cb.end_recording
      end

      def vk_cmd_bind_pipeline(bind_point, pipeline)
        @_cb.cmd_bind_pipeline(pipeline._pipeline)
      end

      def vk_cmd_bind_descriptor_sets(bind_point, layout, descriptor_sets)
        descriptor_sets.each do |ds|
          @_cb.cmd_bind_descriptor_set(ds._ds)
        end
      end

      def vk_cmd_push_constants(layout, offset, data)
        @_cb.cmd_push_constants(offset, data)
      end

      def vk_cmd_dispatch(x, y = 1, z = 1)
        @_cb.cmd_dispatch(x, y, z)
      end

      def vk_cmd_copy_buffer(src, dst, regions)
        regions.each do |region|
          @_cb.cmd_copy_buffer(
            src._buffer, dst._buffer, region.size,
            src_offset: region.src_offset, dst_offset: region.dst_offset
          )
        end
      end

      def vk_cmd_fill_buffer(buffer, offset, size, data)
        @_cb.cmd_fill_buffer(buffer._buffer, data, offset: offset, size: size)
      end

      def vk_cmd_pipeline_barrier(src_stage, dst_stage, buffer_barriers: nil)
        barrier = ComputeRuntime::PipelineBarrier.new(
          src_stage: src_stage.to_sym,
          dst_stage: dst_stage.to_sym
        )
        @_cb.cmd_pipeline_barrier(barrier)
      end
    end

    # Vulkan queue -- wraps Layer 5 CommandQueue.
    class VkQueue
      attr_reader :_queue

      def initialize(queue)
        @_queue = queue
      end

      # Submit work to the queue (vkQueueSubmit).
      #
      # @param submits [Array<VkSubmitInfo>] Submission info.
      # @param fence [VkFence, nil] Optional fence to signal.
      # @return [Integer] VkResult::SUCCESS
      def vk_queue_submit(submits, fence: nil)
        submits.each do |submit_info|
          cbs = submit_info.command_buffers.map(&:_cb)
          wait_sems = submit_info.wait_semaphores.map(&:_semaphore)
          signal_sems = submit_info.signal_semaphores.map(&:_semaphore)

          @_queue.submit(
            cbs,
            wait_semaphores: wait_sems.empty? ? nil : wait_sems,
            signal_semaphores: signal_sems.empty? ? nil : signal_sems,
            fence: fence ? fence._fence : nil
          )
        end
        VkResult::SUCCESS
      end

      def vk_queue_wait_idle
        @_queue.wait_idle
      end
    end

    # =====================================================================
    # VkDevice -- wraps LogicalDevice
    # =====================================================================

    # Vulkan logical device -- wraps Layer 5 LogicalDevice with vk_ API.
    class VkDevice
      attr_reader :_logical

      def initialize(logical)
        @_logical = logical
      end

      def vk_get_device_queue(family_index, queue_index)
        family_name = family_index == 0 ? "compute" : "transfer"
        if @_logical.queues.key?(family_name)
          queues = @_logical.queues[family_name]
          return VkQueue.new(queues[queue_index]) if queue_index < queues.length
        end
        VkQueue.new(@_logical.queues["compute"][0])
      end

      def vk_create_command_pool(create_info)
        VkCommandPool.new(self, create_info.queue_family_index)
      end

      def vk_allocate_memory(alloc_info)
        mem_type = if alloc_info.memory_type_index == 0
          ComputeRuntime::MemoryType::DEVICE_LOCAL |
            ComputeRuntime::MemoryType::HOST_VISIBLE |
            ComputeRuntime::MemoryType::HOST_COHERENT
        else
          ComputeRuntime::MemoryType::HOST_VISIBLE |
            ComputeRuntime::MemoryType::HOST_COHERENT
        end

        buf = @_logical.memory_manager.allocate(
          alloc_info.size, mem_type,
          usage: ComputeRuntime::BufferUsage::STORAGE |
            ComputeRuntime::BufferUsage::TRANSFER_SRC |
            ComputeRuntime::BufferUsage::TRANSFER_DST
        )
        VkDeviceMemory.new(buf, @_logical.memory_manager)
      end

      def vk_create_buffer(create_info)
        mem_type = ComputeRuntime::MemoryType::DEVICE_LOCAL |
          ComputeRuntime::MemoryType::HOST_VISIBLE |
          ComputeRuntime::MemoryType::HOST_COHERENT
        buf = @_logical.memory_manager.allocate(
          create_info.size, mem_type,
          usage: ComputeRuntime::BufferUsage::STORAGE |
            ComputeRuntime::BufferUsage::TRANSFER_SRC |
            ComputeRuntime::BufferUsage::TRANSFER_DST
        )
        VkBuffer.new(buf)
      end

      def vk_bind_buffer_memory(buffer, memory, offset)
        # No-op in our simulator -- buffers already have backing memory
      end

      def vk_map_memory(memory, offset, size)
        mapped = memory._mm.map(memory._buffer)
        mapped.get_data.dup
      end

      def vk_unmap_memory(memory)
        memory._mm.unmap(memory._buffer) if memory._buffer.mapped
      end

      def vk_create_shader_module(create_info)
        shader = @_logical.create_shader_module(code: create_info.code)
        VkShaderModule.new(shader)
      end

      def vk_create_descriptor_set_layout(create_info)
        bindings = create_info.bindings.map do |b|
          ComputeRuntime::DescriptorBinding.new(
            binding: b.binding,
            type: b.descriptor_type,
            count: b.descriptor_count
          )
        end
        layout = @_logical.create_descriptor_set_layout(bindings)
        VkDescriptorSetLayout.new(layout)
      end

      def vk_create_pipeline_layout(create_info)
        layouts = create_info.set_layouts.map(&:_layout)
        pl = @_logical.create_pipeline_layout(layouts,
          push_constant_size: create_info.push_constant_size)
        VkPipelineLayout.new(pl)
      end

      def vk_create_compute_pipelines(create_infos)
        create_infos.filter_map do |ci|
          shader = ci.shader_stage&.mod&._shader
          layout = ci.layout&._layout
          if shader && layout
            p = @_logical.create_compute_pipeline(shader, layout)
            VkPipeline.new(p)
          end
        end
      end

      def vk_allocate_descriptor_sets(alloc_info)
        alloc_info.set_layouts.map do |sl|
          ds = @_logical.create_descriptor_set(sl._layout)
          VkDescriptorSet.new(ds)
        end
      end

      def vk_update_descriptor_sets(writes)
        writes.each do |write|
          if write.dst_set && write.buffer_info&.buffer
            write.dst_set._ds.write(
              write.dst_binding,
              write.buffer_info.buffer._buffer
            )
          end
        end
      end

      def vk_create_fence(flags: 0)
        signaled = (flags & 1) != 0
        fence = @_logical.create_fence(signaled: signaled)
        VkFence.new(fence)
      end

      def vk_create_semaphore
        sem = @_logical.create_semaphore
        VkSemaphore.new(sem)
      end

      def vk_wait_for_fences(fences, wait_all, timeout)
        fences.each do |f|
          if f._fence.signaled
            return VkResult::SUCCESS unless wait_all
          elsif wait_all
            return VkResult::NOT_READY
          end
        end
        VkResult::SUCCESS
      end

      def vk_reset_fences(fences)
        fences.each { |f| f._fence.reset }
      end

      def vk_device_wait_idle
        @_logical.wait_idle
      end
    end

    # =====================================================================
    # VkInstance -- the Vulkan entry point
    # =====================================================================

    # Vulkan instance -- the entry point for device discovery.
    class VkInstance < BaseVendorSimulator
      def initialize
        super
      end

      # Enumerate all physical devices.
      #
      # @return [Array<VkPhysicalDevice>]
      def vk_enumerate_physical_devices
        @_physical_devices.map { |pd| VkPhysicalDevice.new(pd) }
      end

      # Create a logical device.
      #
      # @param physical_device [VkPhysicalDevice]
      # @return [VkDevice]
      def vk_create_device(physical_device)
        logical = @_instance.create_logical_device(physical_device._physical)
        VkDevice.new(logical)
      end
    end
  end
end
