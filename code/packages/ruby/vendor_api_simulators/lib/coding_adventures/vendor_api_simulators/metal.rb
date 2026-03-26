# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Metal Runtime Simulator -- Apple's unified memory GPU programming model.
# ---------------------------------------------------------------------------
#
# === What is Metal? ===
#
# Metal is Apple's GPU API, designed exclusively for Apple hardware (macOS,
# iOS, iPadOS, tvOS). Its key innovation is *unified memory* -- on Apple
# Silicon (M1/M2/M3/M4), the CPU and GPU share the same physical RAM. This
# eliminates the host-to-device copies that CUDA and OpenCL require.
#
# === The Command Encoder Model ===
#
# Metal uses a distinctive pattern for recording GPU commands:
#
#     1. Get a command buffer from the command queue
#     2. Create a *command encoder* (compute, blit, render)
#     3. Record commands into the encoder
#     4. End the encoder
#     5. Commit the command buffer
#
# The encoder adds a layer of scoping that Vulkan doesn't have:
#
#     Vulkan:   cb.begin -> cmd_bind_pipeline -> cmd_dispatch -> cb.end
#     Metal:    cb -> encoder = cb.make_compute_command_encoder
#                   encoder.set_compute_pipeline_state(pso)
#                   encoder.dispatch_threadgroups(...)
#                   encoder.end_encoding
#               cb.commit
#
# === Unified Memory ===
#
# On Apple Silicon, all memory is both CPU-accessible and GPU-accessible:
#
#     CUDA:   cudaMalloc -> device-only, need cudaMemcpy to access from CPU
#     Metal:  make_buffer -> unified, buffer.contents gives CPU access directly

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # Metal-specific types
    # =====================================================================

    # MTLSize -- grid/threadgroup dimensions in Metal.
    #
    # Metal uses (width, height, depth) instead of (x, y, z). Same concept,
    # different naming -- Apple convention for consistency with their graphics API.
    MTLSize = Data.define(:width, :height, :depth) do
      def initialize(width:, height: 1, depth: 1)
        super
      end
    end

    # Metal storage mode options for buffers.
    module MTLResourceOptions
      STORAGE_MODE_SHARED  = "shared"
      STORAGE_MODE_PRIVATE = "private"
      STORAGE_MODE_MANAGED = "managed"
    end

    # Status of a Metal command buffer in its lifecycle.
    module MTLCommandBufferStatus
      NOT_ENQUEUED = "not_enqueued"
      ENQUEUED     = "enqueued"
      COMMITTED    = "committed"
      SCHEDULED    = "scheduled"
      COMPLETED    = "completed"
      ERROR        = "error"
    end

    # =====================================================================
    # MTLBuffer -- unified memory buffer
    # =====================================================================

    # A Metal buffer -- always accessible from both CPU and GPU.
    #
    # Because Apple Silicon uses unified memory, you can:
    #
    #     buf = device.make_buffer(1024)
    #     buf.write_bytes(data)           # CPU writes directly
    #     # ... GPU computes on buf ...
    #     result = buf.contents           # CPU reads directly
    #
    # No staging buffers, no memcpy, no map/unmap ceremony.
    class MTLBuffer
      attr_reader :_buffer, :length

      def initialize(buffer, memory_manager, length)
        @_buffer = buffer
        @_mm = memory_manager
        @length = length
      end

      # Get CPU-accessible view of the buffer contents.
      #
      # In real Metal, this returns a raw pointer to the shared memory.
      # In our simulator, we invalidate (pull from device), then return
      # the buffer's data as a binary String.
      #
      # @return [String] A binary string with the buffer's current contents.
      def contents
        @_mm.invalidate(@_buffer)
        @_mm._get_buffer_data(@_buffer.buffer_id).dup
      end

      # Write bytes to the buffer from CPU side.
      #
      # @param data [String] Bytes to write.
      # @param offset [Integer] Byte offset into the buffer.
      def write_bytes(data, offset: 0)
        mapped = @_mm.map(@_buffer)
        mapped.write(offset, data)
        @_mm.unmap(@_buffer)
      end
    end

    # =====================================================================
    # MTLFunction and MTLLibrary -- shader management
    # =====================================================================

    # A Metal shader function extracted from a library.
    class MTLFunction
      attr_reader :name, :_code

      def initialize(name, code: nil)
        @name = name
        @_code = code
      end
    end

    # A Metal shader library -- a collection of compiled functions.
    class MTLLibrary
      def initialize(source, functions: {})
        @source = source
        @functions = functions
      end

      # Extract a function from the library by name.
      #
      # @param name [String] Function name.
      # @return [MTLFunction]
      def make_function(name)
        code = @functions[name]
        MTLFunction.new(name, code: code)
      end
    end

    # =====================================================================
    # MTLComputePipelineState -- compiled compute pipeline
    # =====================================================================

    # A compiled Metal compute pipeline state.
    class MTLComputePipelineState
      attr_reader :_function, :_pipeline

      def initialize(function, device)
        @_function = function
        @_device = device

        # Create Layer 5 pipeline from the function
        shader = device.create_shader_module(code: function._code)
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        @_pipeline = device.create_compute_pipeline(shader, pl_layout)
      end

      # Maximum threads per threadgroup for this pipeline.
      def max_total_threads_per_threadgroup
        1024
      end
    end

    # =====================================================================
    # MTLComputeCommandEncoder -- records compute commands
    # =====================================================================

    # A Metal compute command encoder -- records compute commands.
    #
    # === The Encoder Pattern ===
    #
    # Instead of recording commands directly into a command buffer (Vulkan
    # style), Metal uses typed encoders that scope commands by type:
    #
    #     encoder = command_buffer.make_compute_command_encoder
    #     encoder.set_compute_pipeline_state(pso)
    #     encoder.set_buffer(buf_x, offset: 0, index: 0)
    #     encoder.set_buffer(buf_y, offset: 0, index: 1)
    #     encoder.dispatch_threadgroups(groups, threads_per_group)
    #     encoder.end_encoding
    class MTLComputeCommandEncoder
      def initialize(command_buffer)
        @command_buffer = command_buffer
        @pipeline_state = nil
        @buffers = {}
        @push_data = {}
        @ended = false
      end

      # Set which compute pipeline to use for dispatches.
      def set_compute_pipeline_state(pso)
        @pipeline_state = pso
      end

      # Bind a buffer to an argument index.
      #
      # @param buffer [MTLBuffer] The buffer to bind.
      # @param offset [Integer] Byte offset into the buffer.
      # @param index [Integer] Argument index.
      def set_buffer(buffer, offset:, index:)
        @buffers[index] = buffer
      end

      # Set inline bytes as a kernel argument (push constants).
      #
      # @param data [String] Raw bytes.
      # @param index [Integer] Argument index.
      def set_bytes(data, index:)
        @push_data[index] = data
      end

      # Dispatch a compute kernel with explicit threadgroup count.
      #
      # @param threadgroups_per_grid [MTLSize] Number of threadgroups.
      # @param threads_per_threadgroup [MTLSize] Threads per threadgroup.
      def dispatch_threadgroups(threadgroups_per_grid, threads_per_threadgroup)
        raise RuntimeError, "No compute pipeline state set" if @pipeline_state.nil?

        cb = @command_buffer._cb
        device = @command_buffer._device

        # Create a fresh pipeline with the correct local size
        pso = @pipeline_state
        shader = device.create_shader_module(
          code: pso._function._code,
          local_size: [
            threads_per_threadgroup.width,
            threads_per_threadgroup.height,
            threads_per_threadgroup.depth
          ]
        )

        # Build descriptor set from bound buffers
        bindings = @buffers.keys.sort.map do |i|
          ComputeRuntime::DescriptorBinding.new(binding: i, type: "storage")
        end
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        @buffers.keys.sort.each do |i|
          ds.write(i, @buffers[i]._buffer)
        end

        # Record into the command buffer
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(ds)
        cb.cmd_dispatch(
          threadgroups_per_grid.width,
          threadgroups_per_grid.height,
          threadgroups_per_grid.depth
        )
      end

      # Dispatch with total thread count (Metal calculates grid).
      #
      # @param threads_per_grid [MTLSize] Total threads in each dimension.
      # @param threads_per_threadgroup [MTLSize] Threads per threadgroup.
      def dispatch_threads(threads_per_grid, threads_per_threadgroup)
        groups = MTLSize.new(
          width: [1, (threads_per_grid.width + threads_per_threadgroup.width - 1) / threads_per_threadgroup.width].max,
          height: [1, (threads_per_grid.height + threads_per_threadgroup.height - 1) / threads_per_threadgroup.height].max,
          depth: [1, (threads_per_grid.depth + threads_per_threadgroup.depth - 1) / threads_per_threadgroup.depth].max
        )
        dispatch_threadgroups(groups, threads_per_threadgroup)
      end

      # End recording into this encoder.
      def end_encoding
        @ended = true
      end

      # Whether this encoder has been ended.
      def ended?
        @ended
      end
    end

    # =====================================================================
    # MTLBlitCommandEncoder -- records data transfer commands
    # =====================================================================

    # A Metal blit command encoder -- records copy/fill operations.
    class MTLBlitCommandEncoder
      def initialize(command_buffer)
        @command_buffer = command_buffer
        @ended = false
      end

      # Copy data between buffers.
      #
      # @param src [MTLBuffer] Source buffer.
      # @param src_offset [Integer] Byte offset in source.
      # @param to_buffer [MTLBuffer] Destination buffer.
      # @param dst_offset [Integer] Byte offset in destination.
      # @param size [Integer] Bytes to copy.
      def copy_from_buffer(src, src_offset, to_buffer:, dst_offset:, size:)
        cb = @command_buffer._cb
        cb.cmd_copy_buffer(src._buffer, to_buffer._buffer, size,
          src_offset: src_offset, dst_offset: dst_offset)
      end

      # Fill a buffer region with a byte value.
      #
      # @param buffer [MTLBuffer] Buffer to fill.
      # @param fill_range [Range] Range of bytes to fill.
      # @param value [Integer] Byte value (0-255).
      def fill_buffer(buffer, fill_range, value)
        cb = @command_buffer._cb
        cb.cmd_fill_buffer(buffer._buffer, value,
          offset: fill_range.begin,
          size: fill_range.size)
      end

      # End recording into this blit encoder.
      def end_encoding
        @ended = true
      end

      def ended?
        @ended
      end
    end

    # =====================================================================
    # MTLCommandBuffer -- wraps Layer 5 CommandBuffer with encoder model
    # =====================================================================

    # A Metal command buffer -- records and submits GPU work.
    class MTLCommandBuffer
      attr_reader :_cb, :_device, :status

      def initialize(queue)
        @queue = queue
        @_device = queue._device._logical_device
        @_cb = @_device.create_command_buffer
        @_cb.begin
        @_fence = @_device.create_fence
        @status = MTLCommandBufferStatus::NOT_ENQUEUED
        @completed_handlers = []
      end

      # Create a compute command encoder for this command buffer.
      #
      # @return [MTLComputeCommandEncoder]
      def make_compute_command_encoder
        MTLComputeCommandEncoder.new(self)
      end

      # Create a blit (copy/fill) command encoder.
      #
      # @return [MTLBlitCommandEncoder]
      def make_blit_command_encoder
        MTLBlitCommandEncoder.new(self)
      end

      # Submit this command buffer for execution (commit).
      def commit
        @_cb.end_recording
        @status = MTLCommandBufferStatus::COMMITTED
        @queue._queue.submit([@_cb], fence: @_fence)
        @status = MTLCommandBufferStatus::COMPLETED
        @completed_handlers.each(&:call)
      end

      # Block until the command buffer finishes execution.
      def wait_until_completed
        @_fence.wait
      end

      # Register a callback to be called when execution completes.
      #
      # @param handler [Proc] A callable with no arguments.
      def add_completed_handler(handler)
        @completed_handlers << handler
      end
    end

    # =====================================================================
    # MTLCommandQueue -- creates command buffers
    # =====================================================================

    # A Metal command queue -- creates command buffers for submission.
    class MTLCommandQueue
      attr_reader :_device, :_queue

      def initialize(device)
        @_device = device
        @_queue = device._compute_queue
      end

      # Create a new command buffer for this queue.
      #
      # @return [MTLCommandBuffer]
      def make_command_buffer
        MTLCommandBuffer.new(self)
      end
    end

    # =====================================================================
    # MTLDevice -- the main Metal device object
    # =====================================================================

    # A Metal device -- the main entry point for Metal programming.
    #
    # === Apple's Simplified Model ===
    #
    # In Vulkan, you have PhysicalDevice (read-only) and LogicalDevice (usable).
    # In Metal, there's just MTLDevice -- it's both.
    #
    # Metal always uses unified memory. All buffers are CPU-accessible
    # (storageModeShared by default), so there's no need for staging buffers.
    #
    # === Usage ===
    #
    #     device = MTLDevice.new
    #     queue = device.make_command_queue
    #
    #     buf = device.make_buffer(1024)
    #     buf.write_bytes(data)  # Direct CPU write!
    #
    #     library = device.make_library(source: "my_shader")
    #     function = library.make_function("compute_fn")
    #     pso = device.make_compute_pipeline_state(function)
    #
    #     cb = queue.make_command_buffer
    #     encoder = cb.make_compute_command_encoder
    #     encoder.set_compute_pipeline_state(pso)
    #     encoder.set_buffer(buf, offset: 0, index: 0)
    #     encoder.dispatch_threadgroups(MTLSize.new(width: 4), MTLSize.new(width: 64))
    #     encoder.end_encoding
    #     cb.commit
    #     cb.wait_until_completed
    #
    #     result = buf.contents  # Direct CPU read!
    class MTLDevice < BaseVendorSimulator
      def initialize
        super(vendor_hint: "apple")
      end

      # Device name.
      def name
        @_physical_device.name
      end

      # Create a command queue for this device.
      #
      # @return [MTLCommandQueue]
      def make_command_queue
        MTLCommandQueue.new(self)
      end

      # Allocate a buffer on the device.
      #
      # @param length [Integer] Buffer size in bytes.
      # @param options [String] Storage mode (default: shared).
      # @return [MTLBuffer]
      def make_buffer(length, options: MTLResourceOptions::STORAGE_MODE_SHARED)
        mem_type = ComputeRuntime::MemoryType::DEVICE_LOCAL |
          ComputeRuntime::MemoryType::HOST_VISIBLE |
          ComputeRuntime::MemoryType::HOST_COHERENT
        usage = ComputeRuntime::BufferUsage::STORAGE |
          ComputeRuntime::BufferUsage::TRANSFER_SRC |
          ComputeRuntime::BufferUsage::TRANSFER_DST

        buf = @_memory_manager.allocate(length, mem_type, usage: usage)
        MTLBuffer.new(buf, @_memory_manager, length)
      end

      # Create a shader library from source code.
      #
      # @param source [String] Shader source code (label in simulator).
      # @return [MTLLibrary]
      def make_library(source:)
        MTLLibrary.new(source)
      end

      # Create a compute pipeline state from a shader function.
      #
      # @param function [MTLFunction] The compiled shader function.
      # @return [MTLComputePipelineState]
      def make_compute_pipeline_state(function)
        MTLComputePipelineState.new(function, @_logical_device)
      end
    end
  end
end
