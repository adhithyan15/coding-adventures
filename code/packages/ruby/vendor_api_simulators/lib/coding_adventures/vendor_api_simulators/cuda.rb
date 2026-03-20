# frozen_string_literal: true

# ---------------------------------------------------------------------------
# CUDA Runtime Simulator -- NVIDIA's "just launch it" GPU programming model.
# ---------------------------------------------------------------------------
#
# === What is CUDA? ===
#
# CUDA (Compute Unified Device Architecture) is NVIDIA's proprietary GPU
# computing platform. It's the most popular GPU programming API, used by
# PyTorch, TensorFlow, and virtually all ML research.
#
# CUDA's design philosophy is "make the common case easy." The common case
# for GPU programming is:
#
#     1. Allocate memory on the GPU          --> cuda.malloc(size)
#     2. Copy data from CPU to GPU           --> cuda.memcpy(dst, src, size, :host_to_device)
#     3. Launch a kernel                     --> cuda.launch_kernel(kernel, grid, block, args)
#     4. Copy results back                   --> cuda.memcpy(dst, src, size, :device_to_host)
#     5. Free memory                         --> cuda.free(ptr)
#
# Each of these is a single method call. Compare this to Vulkan, where
# launching a kernel requires creating a pipeline, descriptor set, command
# buffer, recording commands, submitting, and waiting.
#
# === How CUDA Hides Complexity ===
#
# When you call cuda.launch_kernel, here's what happens internally (and what
# our simulator does):
#
#     1. Create a Pipeline from the kernel's code
#     2. Create a DescriptorSet and bind the argument buffers
#     3. Create a CommandBuffer
#     4. Record: bind_pipeline, bind_descriptor_set, dispatch
#     5. Submit the CommandBuffer to the default stream's queue
#     6. Wait for completion (synchronous in default stream)
#
# You never see steps 1-6. That's the magic of CUDA -- it feels like calling
# a function, but underneath it's the full Vulkan-style pipeline.
#
# === Streams ===
#
# CUDA streams are independent execution queues. The default stream (stream 0)
# is synchronous -- every operation completes before the next starts. Additional
# streams can overlap:
#
#     Stream 0 (default):  [kernel A]--[kernel B]--[kernel C]
#     Stream 1:            --[upload]--[kernel D]--[download]
#
# Operations in the same stream are sequential. Operations in different
# streams can overlap. This maps directly to Layer 5's CommandQueue concept.
#
# === Memory Model ===
#
# CUDA simplifies memory into two main types:
#
#     cuda.malloc:          GPU-only memory (DEVICE_LOCAL in Layer 5)
#     cuda.malloc_managed:  Unified memory accessible from both CPU and GPU
#                           (DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT)
#
# The memcpy method handles transfers between these memory types.

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # CUDA-specific types
    # =====================================================================

    # Dim3 -- the classic CUDA grid/block dimension type.
    #
    # In real CUDA, dim3 is a struct with x, y, z fields. When you write
    # kernel<<<dim3(4, 1, 1), dim3(64, 1, 1)>>>, you're saying:
    #   "Launch 4 blocks of 64 threads each, in 1D."
    #
    # We use Ruby's Data.define for a lightweight immutable struct.
    Dim3 = Data.define(:x, :y, :z) do
      def initialize(x:, y: 1, z: 1)
        super
      end
    end

    # The four copy directions for CUDA memcpy.
    #
    # === The Four Copy Directions ===
    #
    #     :host_to_device    CPU RAM -> GPU VRAM (upload)
    #     :device_to_host    GPU VRAM -> CPU RAM (download)
    #     :device_to_device  GPU VRAM -> GPU VRAM (on-device copy)
    #     :host_to_host      CPU RAM -> CPU RAM (plain memcpy)
    #
    # In real CUDA, these map to different DMA engine configurations:
    #     - HostToDevice uses the PCIe DMA engine (CPU->GPU direction)
    #     - DeviceToHost uses the PCIe DMA engine (GPU->CPU direction)
    #     - DeviceToDevice uses the internal GPU copy engine
    #     - HostToHost uses plain CPU memcpy (no GPU involvement)
    CUDA_MEMCPY_KINDS = %i[host_to_device device_to_host device_to_device host_to_host].freeze

    # CUDADeviceProperties -- properties of a CUDA device, similar to cudaDeviceProp.
    #
    # In real CUDA, you query these with cudaGetDeviceProperties(). They tell you
    # what the GPU can do -- how much memory, how many threads, what compute
    # capability.
    CUDADeviceProperties = Data.define(
      :name,
      :total_global_mem,
      :shared_mem_per_block,
      :max_threads_per_block,
      :max_grid_size,
      :warp_size,
      :compute_capability
    ) do
      def initialize(
        name: "",
        total_global_mem: 0,
        shared_mem_per_block: 49152,
        max_threads_per_block: 1024,
        max_grid_size: [65535, 65535, 65535],
        warp_size: 32,
        compute_capability: [8, 0]
      )
        super
      end
    end

    # CUDAKernel -- a compiled GPU kernel ready to launch.
    #
    # In real CUDA, kernels are C++ functions decorated with __global__.
    # In our simulator, a kernel wraps a list of GPU instructions from
    # the gpu-core package (Layer 9).
    CUDAKernel = Data.define(:code, :name) do
      def initialize(code:, name: "unnamed_kernel")
        super
      end
    end

    # CUDADevicePtr -- a handle to GPU memory.
    #
    # In real CUDA, cudaMalloc() returns a void* pointer to device memory.
    # You can't dereference it on the CPU -- it's only valid on the GPU.
    #
    # In our simulator, CUDADevicePtr wraps a Layer 5 Buffer object and
    # exposes its device_address and size.
    class CUDADevicePtr
      attr_reader :_buffer, :device_address, :size

      def initialize(buffer:, device_address: 0, size: 0)
        @_buffer = buffer
        @device_address = device_address
        @size = size
      end
    end

    # CUDAStream -- an independent execution queue.
    #
    # === What is a Stream? ===
    #
    # A stream is a sequence of GPU operations that execute in order.
    # Operations in the same stream are guaranteed to execute sequentially.
    # Operations in different streams MAY execute concurrently.
    #
    # The default stream (stream 0) has special semantics -- it synchronizes
    # with all other streams. Our simulator models each stream as a separate
    # Layer 5 CommandQueue.
    class CUDAStream
      attr_reader :_queue
      attr_accessor :_pending_fence

      def initialize(queue)
        @_queue = queue
        @_pending_fence = nil
      end
    end

    # CUDAEvent -- a timestamp marker in a stream.
    #
    # === What is an Event? ===
    #
    # Events are used for two things in CUDA:
    #     1. GPU timing -- record event before and after a kernel, measure elapsed
    #     2. Stream synchronization -- one stream can wait for another's event
    #
    # In our simulator, an event wraps a Layer 5 Fence with a timestamp.
    class CUDAEvent
      attr_reader :_fence
      attr_accessor :_timestamp, :_recorded

      def initialize(fence)
        @_fence = fence
        @_timestamp = 0
        @_recorded = false
      end
    end

    # =====================================================================
    # CUDARuntime -- the main simulator class
    # =====================================================================
    #
    # This is the main entry point for CUDA-style programming:
    #
    #     cuda = CUDARuntime.new
    #
    #     # Allocate, copy, launch, synchronize -- just like real CUDA
    #     d_x = cuda.malloc(1024)
    #     cuda.memcpy(d_x, host_data, 1024, :host_to_device)
    #     cuda.launch_kernel(kernel, grid: Dim3.new(x: 4), block: Dim3.new(x: 64), args: [d_x])
    #     cuda.device_synchronize
    #     cuda.free(d_x)
    class CUDARuntime < BaseVendorSimulator
      def initialize
        super(vendor_hint: "nvidia")
        @device_id = 0
        @streams = []
        @events = []
      end

      # ===============================================================
      # Device management
      # ===============================================================

      # Select which GPU to use (cudaSetDevice).
      #
      # In multi-GPU systems, this switches the "current" device. In our
      # simulator, we only model one device, so this validates the ID.
      #
      # @param device_id [Integer] Device index (0-based).
      # @raise [ArgumentError] If device_id is out of range.
      def set_device(device_id)
        if device_id < 0 || device_id >= @_physical_devices.length
          raise ArgumentError,
            "Invalid device ID #{device_id}. " \
            "Available: 0-#{@_physical_devices.length - 1}"
        end
        @device_id = device_id
      end

      # Get the current device ID (cudaGetDevice).
      #
      # @return [Integer] The current device index.
      def get_device
        @device_id
      end

      # Query device properties (cudaGetDeviceProperties).
      #
      # Returns a CUDADeviceProperties with information about the current
      # device -- name, memory size, limits, etc.
      #
      # @return [CUDADeviceProperties] Device properties for the current GPU.
      def get_device_properties
        pd = @_physical_device
        mem_size = pd.memory_properties.heaps.sum(&:size)
        CUDADeviceProperties.new(
          name: pd.name,
          total_global_mem: mem_size,
          max_threads_per_block: pd.limits.max_workgroup_size[0],
          max_grid_size: pd.limits.max_workgroup_count
        )
      end

      # Wait for all GPU work to complete (cudaDeviceSynchronize).
      #
      # This is the bluntest synchronization tool -- it blocks the CPU
      # until every kernel, every copy, every operation on every stream
      # has finished.
      #
      # Maps to: LogicalDevice#wait_idle
      def device_synchronize
        @_logical_device.wait_idle
      end

      # Reset the device (cudaDeviceReset).
      #
      # Destroys all allocations, streams, and state. In real CUDA,
      # this is used for cleanup at program exit.
      #
      # Maps to: LogicalDevice#reset
      def device_reset
        @_logical_device.reset
        @streams.clear
        @events.clear
      end

      # ===============================================================
      # Memory management
      # ===============================================================

      # Allocate device memory (cudaMalloc).
      #
      # Allocates GPU-only memory (DEVICE_LOCAL). The CPU cannot read or
      # write this memory directly -- you must use memcpy to transfer
      # data to/from it.
      #
      # Note: We use HOST_VISIBLE | HOST_COHERENT for simulation convenience
      # so we can actually read/write data.
      #
      # @param size [Integer] Number of bytes to allocate.
      # @return [CUDADevicePtr] A handle to the allocated memory.
      def malloc(size)
        buf = @_memory_manager.allocate(
          size,
          ComputeRuntime::MemoryType::DEVICE_LOCAL |
            ComputeRuntime::MemoryType::HOST_VISIBLE |
            ComputeRuntime::MemoryType::HOST_COHERENT,
          usage: ComputeRuntime::BufferUsage::STORAGE |
            ComputeRuntime::BufferUsage::TRANSFER_SRC |
            ComputeRuntime::BufferUsage::TRANSFER_DST
        )
        CUDADevicePtr.new(
          buffer: buf,
          device_address: buf.device_address,
          size: size
        )
      end

      # Allocate unified/managed memory (cudaMallocManaged).
      #
      # Managed memory is accessible from both CPU and GPU. The CUDA
      # runtime handles page migration automatically.
      #
      # @param size [Integer] Number of bytes to allocate.
      # @return [CUDADevicePtr] A handle to the unified memory allocation.
      def malloc_managed(size)
        buf = @_memory_manager.allocate(
          size,
          ComputeRuntime::MemoryType::DEVICE_LOCAL |
            ComputeRuntime::MemoryType::HOST_VISIBLE |
            ComputeRuntime::MemoryType::HOST_COHERENT,
          usage: ComputeRuntime::BufferUsage::STORAGE |
            ComputeRuntime::BufferUsage::TRANSFER_SRC |
            ComputeRuntime::BufferUsage::TRANSFER_DST
        )
        CUDADevicePtr.new(
          buffer: buf,
          device_address: buf.device_address,
          size: size
        )
      end

      # Free device memory (cudaFree).
      #
      # @param ptr [CUDADevicePtr] The device pointer to free.
      def free(ptr)
        @_memory_manager.free(ptr._buffer)
      end

      # Copy memory between host and device (cudaMemcpy).
      #
      # === The Four Copy Directions ===
      #
      #     :host_to_device    src is String (CPU), dst is CUDADevicePtr (GPU)
      #     :device_to_host    src is CUDADevicePtr (GPU), dst is String (CPU)
      #     :device_to_device  both src and dst are CUDADevicePtr
      #     :host_to_host      both are Strings (no GPU involvement)
      #
      # @param dst [CUDADevicePtr, String] Destination.
      # @param src [CUDADevicePtr, String] Source.
      # @param size [Integer] Number of bytes to copy.
      # @param kind [Symbol] Copy direction (one of CUDA_MEMCPY_KINDS).
      # @raise [TypeError] If src/dst types don't match the specified kind.
      def memcpy(dst, src, size, kind)
        case kind
        when :host_to_device
          raise TypeError, "dst must be CUDADevicePtr for host_to_device" unless dst.is_a?(CUDADevicePtr)
          raise TypeError, "src must be String for host_to_device" unless src.is_a?(String)
          mapped = @_memory_manager.map(dst._buffer)
          mapped.write(0, src.byteslice(0, size))
          @_memory_manager.unmap(dst._buffer)

        when :device_to_host
          raise TypeError, "src must be CUDADevicePtr for device_to_host" unless src.is_a?(CUDADevicePtr)
          raise TypeError, "dst must be String for device_to_host" unless dst.is_a?(String)
          @_memory_manager.invalidate(src._buffer)
          mapped = @_memory_manager.map(src._buffer)
          data = mapped.read(0, size)
          @_memory_manager.unmap(src._buffer)
          dst.replace(data)

        when :device_to_device
          raise TypeError, "dst must be CUDADevicePtr for device_to_device" unless dst.is_a?(CUDADevicePtr)
          raise TypeError, "src must be CUDADevicePtr for device_to_device" unless src.is_a?(CUDADevicePtr)
          _create_and_submit_cb do |cb|
            cb.cmd_copy_buffer(src._buffer, dst._buffer, size)
          end

        when :host_to_host
          raise TypeError, "dst must be String for host_to_host" unless dst.is_a?(String)
          raise TypeError, "src must be String for host_to_host" unless src.is_a?(String)
          dst.replace(src.byteslice(0, size))
        end
      end

      # Set device memory to a value (cudaMemset).
      #
      # Fills the first +size+ bytes of device memory with the byte value.
      #
      # @param ptr [CUDADevicePtr] Device pointer to fill.
      # @param value [Integer] Byte value (0-255).
      # @param size [Integer] Number of bytes to fill.
      def memset(ptr, value, size)
        _create_and_submit_cb do |cb|
          cb.cmd_fill_buffer(ptr._buffer, value, offset: 0, size: size)
        end
      end

      # ===============================================================
      # Kernel launch -- the heart of CUDA
      # ===============================================================

      # Launch a CUDA kernel (the <<<grid, block>>> operator).
      #
      # === What Happens Internally ===
      #
      # This single call hides the entire Vulkan-style pipeline:
      #
      #     1. Create a ShaderModule from the kernel's code
      #     2. Create a DescriptorSetLayout and PipelineLayout
      #     3. Create a Pipeline binding the shader to the layout
      #     4. Create a DescriptorSet and bind the argument buffers
      #     5. Create a CommandBuffer
      #     6. Record: bind_pipeline -> bind_descriptor_set -> dispatch
      #     7. Submit to the queue (default or specified stream)
      #     8. Wait for completion
      #
      # @param kernel [CUDAKernel] The kernel containing GPU instructions.
      # @param grid [Dim3] Grid dimensions (number of thread blocks).
      # @param block [Dim3] Block dimensions (threads per block).
      # @param args [Array<CUDADevicePtr>] Arguments to the kernel.
      # @param shared_mem [Integer] Dynamic shared memory per block (bytes).
      # @param stream [CUDAStream, nil] Optional stream. nil = default.
      def launch_kernel(kernel, grid:, block:, args: [], shared_mem: 0, stream: nil)
        device = @_logical_device

        # Step 1: Create shader module with the kernel's code
        shader = device.create_shader_module(
          code: kernel.code,
          local_size: [block.x, block.y, block.z]
        )

        # Step 2: Create descriptor set layout with one binding per argument
        bindings = args.each_with_index.map do |_arg, i|
          ComputeRuntime::DescriptorBinding.new(binding: i, type: "storage")
        end
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])

        # Step 3: Create the compute pipeline
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        # Step 4: Create and populate descriptor set
        ds = device.create_descriptor_set(ds_layout)
        args.each_with_index do |arg, i|
          ds.write(i, arg._buffer)
        end

        # Step 5-8: Record and submit
        queue = stream ? stream._queue : nil
        _create_and_submit_cb(queue: queue) do |cb|
          cb.cmd_bind_pipeline(pipeline)
          cb.cmd_bind_descriptor_set(ds)
          cb.cmd_dispatch(grid.x, grid.y, grid.z)
        end
      end

      # ===============================================================
      # Streams
      # ===============================================================

      # Create a new CUDA stream (cudaStreamCreate).
      #
      # A stream is an independent execution queue. Operations enqueued
      # to different streams can overlap (execute concurrently on the GPU).
      #
      # @return [CUDAStream] A new stream.
      def create_stream
        stream = CUDAStream.new(@_compute_queue)
        @streams << stream
        stream
      end

      # Destroy a CUDA stream (cudaStreamDestroy).
      #
      # @param stream [CUDAStream] The stream to destroy.
      # @raise [ArgumentError] If the stream is not found.
      def destroy_stream(stream)
        unless @streams.include?(stream)
          raise ArgumentError, "Stream not found or already destroyed"
        end
        @streams.delete(stream)
      end

      # Wait for all operations in a stream (cudaStreamSynchronize).
      #
      # @param stream [CUDAStream] The stream to synchronize.
      def stream_synchronize(stream)
        stream._pending_fence&.wait
      end

      # ===============================================================
      # Events (for GPU timing)
      # ===============================================================

      # Create a CUDA event (cudaEventCreate).
      #
      # @return [CUDAEvent] A new event.
      def create_event
        fence = @_logical_device.create_fence
        event = CUDAEvent.new(fence)
        @events << event
        event
      end

      # Record an event in a stream (cudaEventRecord).
      #
      # @param event [CUDAEvent] The event to record.
      # @param stream [CUDAStream, nil] Which stream. nil = default.
      def record_event(event, stream: nil)
        queue = stream ? stream._queue : @_compute_queue
        event._timestamp = queue.total_cycles
        event._fence.signal
        event._recorded = true
      end

      # Wait for an event to complete (cudaEventSynchronize).
      #
      # @param event [CUDAEvent] The event to wait for.
      # @raise [RuntimeError] If the event was never recorded.
      def synchronize_event(event)
        raise RuntimeError, "Event was never recorded" unless event._recorded
        event._fence.wait
      end

      # Measure elapsed GPU time between two events (cudaEventElapsedTime).
      #
      # @param start_event [CUDAEvent] The start event.
      # @param end_event [CUDAEvent] The end event.
      # @return [Float] Elapsed time in milliseconds.
      # @raise [RuntimeError] If either event was not recorded.
      def elapsed_time(start_event, end_event)
        raise RuntimeError, "Start event was never recorded" unless start_event._recorded
        raise RuntimeError, "End event was never recorded" unless end_event._recorded
        cycles = end_event._timestamp - start_event._timestamp
        cycles / 1_000_000.0
      end
    end
  end
end
