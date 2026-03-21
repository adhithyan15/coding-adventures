# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Instance -- device discovery, physical/logical device management.
# ---------------------------------------------------------------------------
#
# === The Entry Point ===
#
# The RuntimeInstance is how everything starts. It's the first object you
# create, and it gives you access to all available hardware:
#
#     instance = RuntimeInstance.new
#     devices = instance.enumerate_physical_devices
#     # -> [PhysicalDevice("NVIDIA H100"), PhysicalDevice("Apple M3 Max ANE"), ...]
#
# === Physical vs Logical Device ===
#
# A PhysicalDevice is a read-only description of hardware. You can query
# its name, type, memory, and capabilities, but you can't use it directly.
#
# A LogicalDevice is a usable handle. It wraps a PhysicalDevice and provides:
# - Command queues for submitting work
# - Memory manager for allocating buffers
# - Factory methods for pipelines, sync objects, etc.
#
# Why the separation?
# - A system may have multiple GPUs. You query all of them, compare, and pick.
# - Multiple logical devices can share one physical device.
# - The physical device never changes. The logical device owns mutable state.
#
# This pattern comes directly from Vulkan (VkPhysicalDevice vs VkDevice).

module CodingAdventures
  module ComputeRuntime
    # =====================================================================
    # PhysicalDevice -- read-only hardware description
    # =====================================================================
    #
    # === What You Can Learn ===
    #
    # - name: "NVIDIA H100", "Apple M3 Max ANE", etc.
    # - device_type: :gpu, :tpu, or :npu
    # - vendor: "nvidia", "amd", "google", "intel", "apple"
    # - memory_properties: what memory types are available, how much
    # - queue_families: what kinds of queues the device supports
    # - limits: hardware constraints
    #
    # You can't execute anything on a PhysicalDevice. Create a LogicalDevice
    # for that.
    class PhysicalDevice
      attr_reader :device_id, :name, :device_type, :vendor,
        :memory_properties, :queue_families, :limits

      def initialize(device_id:, name:, device_type:, vendor:,
        accelerator:, memory_properties:, queue_families:, limits:)
        @device_id = device_id
        @name = name
        @device_type = device_type
        @vendor = vendor
        @accelerator = accelerator
        @memory_properties = memory_properties
        @queue_families = queue_families.dup
        @limits = limits
      end

      # Check if a feature is supported.
      #
      # Currently supported features:
      # - "fp32": 32-bit float (always true)
      # - "fp16": 16-bit float
      # - "unified_memory": CPU/GPU shared memory
      # - "transfer_queue": dedicated DMA engine
      #
      # @param feature [String] Feature name to query.
      # @return [Boolean] True if supported.
      def supports_feature(feature)
        features = {
          "fp32" => true,
          "fp16" => true,
          "unified_memory" => @memory_properties.is_unified,
          "transfer_queue" => @queue_families.any? { |qf| qf.queue_type == :transfer }
        }
        features.fetch(feature, false)
      end

      # Internal: get underlying accelerator device.
      def _accelerator = @accelerator
    end

    # =====================================================================
    # LogicalDevice -- usable handle with queues and factories
    # =====================================================================
    #
    # === What You Can Do ===
    #
    # - Submit work via command queues
    # - Allocate memory via memory_manager
    # - Create command buffers, pipelines, sync objects
    # - Wait for all work to complete
    class LogicalDevice
      attr_reader :physical_device, :queues, :memory_manager, :stats

      def initialize(physical_device:, accelerator:, queues:, memory_manager:, stats:)
        @physical_device = physical_device
        @accelerator = accelerator
        @queues = queues
        @memory_manager = memory_manager
        @stats = stats
      end

      # --- Factory methods ---

      # Create a new command buffer.
      def create_command_buffer
        CommandBuffer.new
      end

      # Create a shader module from code or operation descriptor.
      #
      # For GPU-style devices, pass code (list of Instructions).
      # For dataflow devices, pass operation name.
      #
      # @param code [Array, nil] GPU-style instruction list.
      # @param operation [String] Dataflow-style operation name.
      # @param entry_point [String] Entry point name (default "main").
      # @param local_size [Array<Integer>] Workgroup dimensions.
      # @return [ShaderModule] A new ShaderModule.
      def create_shader_module(code: nil, operation: "", entry_point: "main", local_size: [32, 1, 1])
        ShaderModule.new(
          code: code,
          operation: operation,
          entry_point: entry_point,
          local_size: local_size
        )
      end

      # Create a descriptor set layout.
      #
      # @param bindings [Array<DescriptorBinding>] List of binding slots.
      # @return [DescriptorSetLayout]
      def create_descriptor_set_layout(bindings)
        DescriptorSetLayout.new(bindings)
      end

      # Create a pipeline layout.
      #
      # @param set_layouts [Array<DescriptorSetLayout>] Descriptor set layouts.
      # @param push_constant_size [Integer] Max push constant bytes.
      # @return [PipelineLayout]
      def create_pipeline_layout(set_layouts, push_constant_size: 0)
        PipelineLayout.new(set_layouts, push_constant_size: push_constant_size)
      end

      # Create a compute pipeline.
      #
      # @param shader [ShaderModule] Compiled shader module.
      # @param layout [PipelineLayout] Pipeline layout.
      # @return [Pipeline]
      def create_compute_pipeline(shader, layout)
        Pipeline.new(shader, layout)
      end

      # Create a descriptor set from a layout.
      #
      # @param layout [DescriptorSetLayout] The layout to create from.
      # @return [DescriptorSet]
      def create_descriptor_set(layout)
        DescriptorSet.new(layout)
      end

      # Create a fence for CPU<->GPU synchronization.
      #
      # @param signaled [Boolean] If true, fence starts already signaled.
      # @return [Fence]
      def create_fence(signaled: false)
        Fence.new(signaled: signaled)
      end

      # Create a semaphore for GPU queue<->queue synchronization.
      def create_semaphore
        Semaphore.new
      end

      # Create an event for fine-grained GPU-side signaling.
      def create_event
        Event.new
      end

      # Block until all queues finish all pending work.
      def wait_idle
        @queues.each_value do |queue_list|
          queue_list.each(&:wait_idle)
        end
      end

      # Reset all device state.
      def reset
        @accelerator.reset
      end
    end

    # =====================================================================
    # Helper: create a PhysicalDevice from an AcceleratorDevice
    # =====================================================================

    # @private
    def self._make_physical_device(device_id, accelerator, device_type, vendor)
      config = accelerator.config
      is_unified = config.unified_memory

      # Build memory heaps based on device type
      heaps = if is_unified
        [
          MemoryHeap.new(
            size: config.global_memory_size,
            flags: MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT
          )
        ]
      else
        [
          # VRAM heap (GPU-only, fast)
          MemoryHeap.new(
            size: config.global_memory_size,
            flags: MemoryType::DEVICE_LOCAL
          ),
          # Staging heap (CPU-visible, slower)
          MemoryHeap.new(
            size: [config.global_memory_size / 4, 256 * 1024 * 1024].min,
            flags: MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT
          )
        ]
      end

      memory_properties = MemoryProperties.new(heaps: heaps, is_unified: is_unified)

      # Build queue families
      queue_families = [
        QueueFamily.new(queue_type: :compute, count: 4)
      ]
      # Discrete GPUs have a separate transfer queue (DMA engine)
      unless is_unified
        queue_families << QueueFamily.new(queue_type: :transfer, count: 2)
      end

      limits = DeviceLimits.new

      PhysicalDevice.new(
        device_id: device_id,
        name: accelerator.name,
        device_type: device_type,
        vendor: vendor,
        accelerator: accelerator,
        memory_properties: memory_properties,
        queue_families: queue_families,
        limits: limits
      )
    end

    # =====================================================================
    # RuntimeInstance -- the entry point
    # =====================================================================
    #
    # === Usage ===
    #
    #     instance = RuntimeInstance.new
    #
    #     # Enumerate all available devices
    #     devices = instance.enumerate_physical_devices
    #
    #     # Pick one and create a logical device
    #     device = instance.create_logical_device(
    #         physical_device: devices[0],
    #         queue_requests: [{ type: "compute", count: 1 }],
    #     )
    #
    # === Default Devices ===
    #
    # By default, the instance creates one of each device type with small
    # configurations for testing.
    class RuntimeInstance
      attr_reader :version

      def initialize(devices: nil)
        @version = "0.1.0"

        if devices
          @physical_devices = devices.each_with_index.map do |(dev, dtype, vendor), i|
            ComputeRuntime._make_physical_device(i, dev, dtype, vendor)
          end
        else
          @physical_devices = _create_default_devices
        end
      end

      # Return all available physical devices.
      def enumerate_physical_devices
        @physical_devices.dup
      end

      # Create a logical device from a physical device.
      #
      # @param physical_device [PhysicalDevice] The hardware to use.
      # @param queue_requests [Array<Hash>, nil] Optional queue configuration.
      #     Each hash has "type" (String) and "count" (Integer).
      #     Default: one compute queue.
      # @return [LogicalDevice] A LogicalDevice ready for use.
      def create_logical_device(physical_device, queue_requests: nil)
        queue_requests ||= [{"type" => "compute", "count" => 1}]

        stats = RuntimeStats.new
        accelerator = physical_device._accelerator

        memory_manager = MemoryManager.new(
          device: accelerator,
          memory_properties: physical_device.memory_properties,
          stats: stats
        )

        # Create requested queues
        queues = {}
        queue_requests.each do |req|
          qt_str = req["type"] || req[:type]
          count = req["count"] || req[:count] || 1
          qt = case qt_str
          when "compute" then :compute
          when "transfer" then :transfer
          else :compute_transfer
          end
          queue_list = count.times.map do |i|
            CommandQueue.new(
              queue_type: qt,
              queue_index: i,
              device: accelerator,
              memory_manager: memory_manager,
              stats: stats
            )
          end
          queues[qt_str] = queue_list
        end

        LogicalDevice.new(
          physical_device: physical_device,
          accelerator: accelerator,
          queues: queues,
          memory_manager: memory_manager,
          stats: stats
        )
      end

      private

      # Create small default devices for testing.
      def _create_default_devices
        defaults = [
          [DeviceSimulator::NvidiaGPU.new(num_sms: 2), :gpu, "nvidia"],
          [DeviceSimulator::AmdGPU.new(num_cus: 2), :gpu, "amd"],
          [DeviceSimulator::GoogleTPU.new(mxu_size: 2), :tpu, "google"],
          [DeviceSimulator::IntelGPU.new(num_cores: 2), :gpu, "intel"],
          [DeviceSimulator::AppleANE.new(num_cores: 2), :npu, "apple"]
        ]
        defaults.each_with_index.map do |(dev, dtype, vendor), i|
          ComputeRuntime._make_physical_device(i, dev, dtype, vendor)
        end
      end
    end
  end
end
