# frozen_string_literal: true

# ---------------------------------------------------------------------------
# WebGPU Runtime Simulator -- safe, browser-first GPU programming.
# ---------------------------------------------------------------------------
#
# === What is WebGPU? ===
#
# WebGPU is the modern web GPU API, designed to run safely in browsers.
# It sits on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
# or D3D12 (Windows), providing a safe, portable abstraction.
#
# === Key Simplifications Over Vulkan ===
#
#     1. Single queue -- device.queue is all you get
#     2. Automatic barriers -- no manual pipeline barriers
#     3. No memory types -- just usage flags
#     4. Always validated -- every operation is checked
#     5. Immutable command buffers -- once finish is called, frozen
#
# === The WebGPU Object Hierarchy ===
#
#     GPU (navigator.gpu in browsers)
#     +-- GPUAdapter (represents a physical device)
#         +-- GPUDevice (the usable handle)
#             +-- device.queue (GPUQueue -- single queue!)
#             +-- create_buffer -> GPUBuffer
#             +-- create_shader_module -> GPUShaderModule
#             +-- create_compute_pipeline -> GPUComputePipeline
#             +-- create_bind_group -> GPUBindGroup
#             +-- create_command_encoder -> GPUCommandEncoder
#                 +-- begin_compute_pass -> GPUComputePassEncoder
#                 +-- finish -> GPUCommandBuffer (frozen!)

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # WebGPU flags
    # =====================================================================

    # WebGPU buffer usage flags (bit flags, combinable with |).
    module GPUBufferUsage
      MAP_READ  = 1
      MAP_WRITE = 2
      COPY_SRC  = 4
      COPY_DST  = 8
      STORAGE   = 16
      UNIFORM   = 32
    end

    # WebGPU buffer map modes.
    module GPUMapMode
      READ  = 1
      WRITE = 2
    end

    # =====================================================================
    # WebGPU descriptor types
    # =====================================================================

    GPUBufferDescriptor = Data.define(:size, :usage, :mapped_at_creation) do
      def initialize(size: 0, usage: GPUBufferUsage::STORAGE, mapped_at_creation: false)
        super
      end
    end

    GPUShaderModuleDescriptor = Data.define(:code) do
      def initialize(code: nil)
        super
      end
    end

    GPUProgrammableStage = Data.define(:mod, :entry_point) do
      def initialize(mod: nil, entry_point: "main")
        super
      end
    end

    GPUComputePipelineDescriptor = Data.define(:layout, :compute) do
      def initialize(layout: "auto", compute: nil)
        super
      end
    end

    GPUBufferBindingLayout = Data.define(:type) do
      def initialize(type: "storage")
        super
      end
    end

    GPUBindGroupLayoutEntry = Data.define(:binding, :visibility, :buffer) do
      def initialize(binding: 0, visibility: 0x04, buffer: GPUBufferBindingLayout.new)
        super
      end
    end

    GPUBindGroupLayoutDescriptor = Data.define(:entries) do
      def initialize(entries: [])
        super
      end
    end

    GPUBindGroupEntry = Data.define(:binding, :resource) do
      def initialize(binding: 0, resource: nil)
        super
      end
    end

    GPUBindGroupDescriptor = Data.define(:layout, :entries) do
      def initialize(layout: nil, entries: [])
        super
      end
    end

    GPUPipelineLayoutDescriptor = Data.define(:bind_group_layouts) do
      def initialize(bind_group_layouts: [])
        super
      end
    end

    GPURequestAdapterOptions = Data.define(:power_preference) do
      def initialize(power_preference: "high-performance")
        super
      end
    end

    GPUDeviceDescriptor = Data.define(:required_features) do
      def initialize(required_features: [])
        super
      end
    end

    GPUAdapterLimits = Data.define(:max_buffer_size, :max_compute_workgroup_size_x) do
      def initialize(max_buffer_size: 2 * 1024 * 1024 * 1024, max_compute_workgroup_size_x: 1024)
        super
      end
    end

    GPUDeviceLimits = Data.define(:max_buffer_size, :max_compute_workgroup_size_x) do
      def initialize(max_buffer_size: 2 * 1024 * 1024 * 1024, max_compute_workgroup_size_x: 1024)
        super
      end
    end

    GPUComputePassDescriptor = Data.define(:label) do
      def initialize(label: "")
        super
      end
    end

    GPUCommandEncoderDescriptor = Data.define(:label) do
      def initialize(label: "")
        super
      end
    end

    # =====================================================================
    # WebGPU wrapper objects
    # =====================================================================

    # A WebGPU buffer -- memory on the device.
    class GPUBuffer
      attr_reader :_buffer, :size, :usage

      def initialize(buffer, memory_manager, size, usage)
        @_buffer = buffer
        @_mm = memory_manager
        @size = size
        @usage = usage
        @mapped = false
        @mapped_data = nil
        @destroyed = false
      end

      # Map the buffer for CPU access (simulated as synchronous).
      #
      # @param mode [Integer] READ or WRITE.
      # @param offset [Integer] Byte offset to map from.
      # @param map_size [Integer, nil] Bytes to map. nil = entire buffer.
      def map_async(mode, offset: 0, map_size: nil)
        raise RuntimeError, "Cannot map a destroyed buffer" if @destroyed
        actual_size = map_size || @size
        @_mm.invalidate(@_buffer)
        data = @_mm._get_buffer_data(@_buffer.buffer_id)
        @mapped_data = data.byteslice(offset, actual_size).dup
        # Ensure it's a mutable binary string
        @mapped_data = (+@mapped_data).force_encoding(Encoding::BINARY)
        @mapped = true
      end

      # Get a view of the mapped buffer data.
      #
      # @param offset [Integer] Byte offset within the mapped range.
      # @param range_size [Integer, nil] Bytes to return. nil = entire mapped range.
      # @return [String] Binary string of the buffer contents.
      # @raise [RuntimeError] If buffer is not mapped.
      def get_mapped_range(offset: 0, range_size: nil)
        raise RuntimeError, "Buffer is not mapped. Call map_async first." unless @mapped || @mapped_data
        actual_size = range_size || @mapped_data.bytesize
        @mapped_data.byteslice(offset, actual_size)
      end

      # Unmap the buffer, making it usable by the GPU again.
      def unmap
        raise RuntimeError, "Buffer is not mapped" unless @mapped
        if @mapped_data
          mapped = @_mm.map(@_buffer)
          mapped.write(0, @mapped_data)
          @_mm.unmap(@_buffer)
        end
        @mapped = false
        @mapped_data = nil
      end

      # Destroy this buffer, releasing its memory.
      def destroy
        unless @destroyed
          @_mm.free(@_buffer)
          @destroyed = true
        end
      end

      def destroyed?
        @destroyed
      end

      def mapped?
        @mapped
      end
    end

    # A WebGPU shader module.
    class GPUShaderModule
      attr_reader :_shader

      def initialize(shader)
        @_shader = shader
      end
    end

    # A WebGPU bind group layout.
    class GPUBindGroupLayout
      attr_reader :_layout

      def initialize(layout)
        @_layout = layout
      end
    end

    # A WebGPU pipeline layout.
    class GPUPipelineLayout
      attr_reader :_layout

      def initialize(layout)
        @_layout = layout
      end
    end

    # A WebGPU compute pipeline.
    class GPUComputePipeline
      attr_reader :_pipeline

      def initialize(pipeline, bind_group_layouts)
        @_pipeline = pipeline
        @_bind_group_layouts = bind_group_layouts
      end

      # Get the bind group layout at a given index.
      #
      # @param index [Integer] Bind group index.
      # @return [GPUBindGroupLayout]
      def get_bind_group_layout(index)
        raise IndexError, "Bind group layout index #{index} out of range" if index >= @_bind_group_layouts.length
        @_bind_group_layouts[index]
      end
    end

    # A WebGPU bind group.
    class GPUBindGroup
      attr_reader :_ds

      def initialize(ds)
        @_ds = ds
      end
    end

    # A frozen WebGPU command buffer -- immutable after finish.
    class GPUCommandBuffer
      attr_reader :_cb

      def initialize(cb)
        @_cb = cb
      end
    end

    # =====================================================================
    # GPUComputePassEncoder -- records compute commands
    # =====================================================================

    class GPUComputePassEncoder
      def initialize(encoder)
        @encoder = encoder
        @pipeline = nil
        @bind_groups = {}
      end

      def set_pipeline(pipeline)
        @pipeline = pipeline
      end

      def set_bind_group(index, bind_group)
        @bind_groups[index] = bind_group
      end

      # Dispatch compute workgroups.
      #
      # @param x [Integer] Workgroups in X dimension.
      # @param y [Integer] Workgroups in Y dimension.
      # @param z [Integer] Workgroups in Z dimension.
      def dispatch_workgroups(x, y = 1, z = 1)
        raise RuntimeError, "No pipeline set" if @pipeline.nil?

        cb = @encoder._cb
        cb.cmd_bind_pipeline(@pipeline._pipeline)
        @bind_groups.sort.each do |_idx, bg|
          cb.cmd_bind_descriptor_set(bg._ds)
        end
        cb.cmd_dispatch(x, y, z)
      end

      # End this compute pass.
      def end_pass
        # No-op
      end
    end

    # =====================================================================
    # GPUCommandEncoder -- records commands into a command buffer
    # =====================================================================

    class GPUCommandEncoder
      attr_reader :_cb

      def initialize(device)
        @device = device
        @_cb = device._logical_device.create_command_buffer
        @_cb.begin
      end

      # Begin a compute pass.
      #
      # @return [GPUComputePassEncoder]
      def begin_compute_pass(descriptor: nil)
        GPUComputePassEncoder.new(self)
      end

      # Copy data between buffers.
      def copy_buffer_to_buffer(source, source_offset, destination, destination_offset, size)
        @_cb.cmd_copy_buffer(
          source._buffer, destination._buffer, size,
          src_offset: source_offset, dst_offset: destination_offset
        )
      end

      # Finish recording and produce a frozen command buffer.
      #
      # @return [GPUCommandBuffer]
      def finish
        @_cb.end_recording
        GPUCommandBuffer.new(@_cb)
      end
    end

    # =====================================================================
    # GPUQueue -- the single submission queue
    # =====================================================================

    class GPUQueue
      def initialize(device)
        @device = device
      end

      # Submit command buffers for execution.
      #
      # @param command_buffers [Array<GPUCommandBuffer>]
      def submit(command_buffers)
        queue = @device._compute_queue
        command_buffers.each do |gpu_cb|
          fence = @device._logical_device.create_fence
          queue.submit([gpu_cb._cb], fence: fence)
          fence.wait
        end
      end

      # Write data to a buffer (convenience method).
      #
      # @param buffer [GPUBuffer] Destination buffer.
      # @param buffer_offset [Integer] Byte offset in the buffer.
      # @param data [String] Data to write.
      def write_buffer(buffer, buffer_offset, data)
        mm = @device._memory_manager
        mapped = mm.map(buffer._buffer)
        mapped.write(buffer_offset, data)
        mm.unmap(buffer._buffer)
      end
    end

    # =====================================================================
    # GPUDevice -- the main WebGPU device
    # =====================================================================

    class GPUDevice < BaseVendorSimulator
      attr_reader :queue, :features, :limits

      def initialize(physical_device: nil)
        if physical_device
          super(vendor_hint: physical_device.vendor)
        else
          super()
        end
        @queue = GPUQueue.new(self)
        @features = Set.new(["compute"])
        @limits = GPUDeviceLimits.new
      end

      # Create a buffer.
      #
      # @param descriptor [GPUBufferDescriptor]
      # @return [GPUBuffer]
      def create_buffer(descriptor)
        mem_type = ComputeRuntime::MemoryType::DEVICE_LOCAL |
          ComputeRuntime::MemoryType::HOST_VISIBLE |
          ComputeRuntime::MemoryType::HOST_COHERENT
        usage = ComputeRuntime::BufferUsage::STORAGE |
          ComputeRuntime::BufferUsage::TRANSFER_SRC |
          ComputeRuntime::BufferUsage::TRANSFER_DST

        buf = @_memory_manager.allocate(descriptor.size, mem_type, usage: usage)
        gpu_buf = GPUBuffer.new(buf, @_memory_manager, descriptor.size, descriptor.usage)

        gpu_buf.map_async(GPUMapMode::WRITE) if descriptor.mapped_at_creation

        gpu_buf
      end

      # Create a shader module.
      #
      # @param descriptor [GPUShaderModuleDescriptor]
      # @return [GPUShaderModule]
      def create_shader_module(descriptor)
        code = descriptor.code.is_a?(Array) ? descriptor.code : nil
        shader = @_logical_device.create_shader_module(code: code)
        GPUShaderModule.new(shader)
      end

      # Create a compute pipeline.
      #
      # @param descriptor [GPUComputePipelineDescriptor]
      # @return [GPUComputePipeline]
      def create_compute_pipeline(descriptor)
        shader = if descriptor.compute&.mod
          descriptor.compute.mod._shader
        else
          @_logical_device.create_shader_module
        end

        ds_layout = @_logical_device.create_descriptor_set_layout([])
        pl_layout = @_logical_device.create_pipeline_layout([ds_layout])
        pipeline = @_logical_device.create_compute_pipeline(shader, pl_layout)

        bg_layout = GPUBindGroupLayout.new(ds_layout)
        GPUComputePipeline.new(pipeline, [bg_layout])
      end

      # Create a bind group layout.
      #
      # @param descriptor [GPUBindGroupLayoutDescriptor]
      # @return [GPUBindGroupLayout]
      def create_bind_group_layout(descriptor)
        bindings = descriptor.entries.map do |e|
          ComputeRuntime::DescriptorBinding.new(
            binding: e.binding,
            type: e.buffer ? e.buffer.type : "storage"
          )
        end
        layout = @_logical_device.create_descriptor_set_layout(bindings)
        GPUBindGroupLayout.new(layout)
      end

      # Create a pipeline layout.
      #
      # @param descriptor [GPUPipelineLayoutDescriptor]
      # @return [GPUPipelineLayout]
      def create_pipeline_layout(descriptor)
        layouts = descriptor.bind_group_layouts.map(&:_layout)
        pl = @_logical_device.create_pipeline_layout(layouts)
        GPUPipelineLayout.new(pl)
      end

      # Create a bind group (WebGPU's descriptor set).
      #
      # @param descriptor [GPUBindGroupDescriptor]
      # @return [GPUBindGroup]
      def create_bind_group(descriptor)
        layout = if descriptor.layout
          descriptor.layout._layout
        else
          @_logical_device.create_descriptor_set_layout([])
        end
        ds = @_logical_device.create_descriptor_set(layout)
        descriptor.entries.each do |entry|
          ds.write(entry.binding, entry.resource._buffer) if entry.resource
        end
        GPUBindGroup.new(ds)
      end

      # Create a command encoder.
      #
      # @return [GPUCommandEncoder]
      def create_command_encoder(descriptor: nil)
        GPUCommandEncoder.new(self)
      end

      # Destroy this device and release all resources.
      def destroy
        @_logical_device.wait_idle
      end
    end

    # =====================================================================
    # GPUAdapter -- physical device wrapper
    # =====================================================================

    class GPUAdapter
      attr_reader :features, :limits

      def initialize(physical_device)
        @_physical = physical_device
        @features = Set.new(["compute"])
        @limits = GPUAdapterLimits.new
      end

      def name
        @_physical.name
      end

      # Request a device from this adapter.
      #
      # @param descriptor [GPUDeviceDescriptor, nil]
      # @return [GPUDevice]
      def request_device(descriptor: nil)
        GPUDevice.new(physical_device: @_physical)
      end
    end

    # =====================================================================
    # GPU -- the top-level WebGPU entry point
    # =====================================================================

    class GPU
      def initialize
        @_instance = ComputeRuntime::RuntimeInstance.new
        @_physical_devices = @_instance.enumerate_physical_devices
      end

      # Request a GPU adapter.
      #
      # @param options [GPURequestAdapterOptions, nil]
      # @return [GPUAdapter]
      def request_adapter(options: nil)
        raise RuntimeError, "No GPU adapters available" if @_physical_devices.empty?

        if options&.power_preference == "low-power"
          @_physical_devices.each do |pd|
            return GPUAdapter.new(pd) if pd.memory_properties.is_unified
          end
        end

        GPUAdapter.new(@_physical_devices[0])
      end
    end
  end
end
