# frozen_string_literal: true

# ---------------------------------------------------------------------------
# OpenCL Runtime Simulator -- cross-platform "portable compute" model.
# ---------------------------------------------------------------------------
#
# === What is OpenCL? ===
#
# OpenCL (Open Computing Language) is the Khronos Group's cross-platform
# compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's GPU,
# and even on CPUs and FPGAs. The tradeoff is more boilerplate -- you must
# explicitly manage platforms, devices, contexts, and command queues.
#
# === The OpenCL Object Hierarchy ===
#
#     CLPlatform          "Which vendor's implementation?"
#         +-- CLDevice    "Which specific GPU/CPU?"
#     CLContext            "A group of devices I want to use together"
#         +-- CLBuffer     "Memory on one of the context's devices"
#         +-- CLProgram    "Source code, not yet compiled"
#         |   +-- CLKernel "Compiled function, ready to dispatch"
#         +-- CLCommandQueue "Where I enqueue operations"
#                 +-- CLEvent "Dependency token for operation ordering"
#
# === Event-Based Dependencies ===
#
# OpenCL's most distinctive feature is its event model. Every enqueue
# operation returns a CLEvent. You can pass event lists to subsequent
# operations to create dependency chains:
#
#     ev1 = queue.enqueue_write_buffer(buf_x, 0, 4, data_x)
#     ev2 = queue.enqueue_write_buffer(buf_y, 0, 4, data_y)
#     ev3 = queue.enqueue_nd_range_kernel(kernel, [128], wait_list: [ev1, ev2])
#     ev4 = queue.enqueue_read_buffer(buf_y, 0, 4, output, wait_list: [ev3])
#
# This is more flexible than CUDA's stream model because dependencies
# can form arbitrary DAGs, not just linear sequences.

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # OpenCL enums and flags
    # =====================================================================

    # OpenCL device types for filtering during discovery.
    module CLDeviceType
      GPU = "gpu"
      CPU = "cpu"
      ACCELERATOR = "accelerator"
      ALL = "all"
    end

    # OpenCL memory flags -- simpler than Vulkan's memory types.
    #
    # READ_WRITE:    Default. GPU can read and write this buffer.
    # READ_ONLY:     GPU can only read. Allows compiler optimization.
    # WRITE_ONLY:    GPU can only write. Allows compiler optimization.
    # COPY_HOST_PTR: Initialize buffer contents from provided host data.
    # USE_HOST_PTR:  Use the host pointer directly (zero-copy if possible).
    # ALLOC_HOST_PTR: Allocate in host-visible memory for CPU access.
    module CLMemFlags
      READ_WRITE    = 1
      READ_ONLY     = 2
      WRITE_ONLY    = 4
      COPY_HOST_PTR = 8
      USE_HOST_PTR  = 16
      ALLOC_HOST_PTR = 32
    end

    # Build status of a CLProgram.
    module CLBuildStatus
      SUCCESS     = "success"
      ERROR       = "error"
      IN_PROGRESS = "in_progress"
      NONE        = "none"
    end

    # Status of an OpenCL event.
    module CLEventStatus
      QUEUED    = "queued"
      SUBMITTED = "submitted"
      RUNNING   = "running"
      COMPLETE  = "complete"
    end

    # Device info parameter IDs for CLDevice#get_info.
    module CLDeviceInfo
      NAME               = "name"
      TYPE               = "type"
      MAX_COMPUTE_UNITS  = "max_compute_units"
      MAX_WORK_GROUP_SIZE = "max_work_group_size"
      GLOBAL_MEM_SIZE    = "global_mem_size"
    end

    # =====================================================================
    # CLEvent -- dependency token
    # =====================================================================

    # An OpenCL event -- a dependency token for operation ordering.
    #
    # Every enqueue operation returns a CLEvent. You can:
    #     - Wait on it (blocking the CPU)
    #     - Pass it in wait_list to another operation (GPU-side dependency)
    #     - Query its status
    class CLEvent
      attr_reader :_fence

      def initialize(fence)
        @_fence = fence
      end

      # Block until this event completes.
      def wait
        @_fence.wait
      end

      # Query the current status of this event.
      def status
        @_fence.signaled ? CLEventStatus::COMPLETE : CLEventStatus::QUEUED
      end
    end

    # =====================================================================
    # CLDevice -- wraps PhysicalDevice
    # =====================================================================

    # An OpenCL device -- a specific piece of hardware.
    class CLDevice
      attr_reader :_physical

      def initialize(physical_device)
        @_physical = physical_device
      end

      def name
        @_physical.name
      end

      def device_type
        dt = @_physical.device_type
        case dt
        when :gpu then CLDeviceType::GPU
        when :tpu, :npu then CLDeviceType::ACCELERATOR
        else CLDeviceType::GPU
        end
      end

      def max_compute_units
        4
      end

      def max_work_group_size
        @_physical.limits.max_workgroup_size[0]
      end

      def global_mem_size
        @_physical.memory_properties.heaps.sum(&:size)
      end

      # Query device information by parameter ID.
      #
      # @param param [String] Which property to query.
      # @return [Object] The requested value.
      def get_info(param)
        case param
        when CLDeviceInfo::NAME then name
        when CLDeviceInfo::TYPE then device_type
        when CLDeviceInfo::MAX_COMPUTE_UNITS then max_compute_units
        when CLDeviceInfo::MAX_WORK_GROUP_SIZE then max_work_group_size
        when CLDeviceInfo::GLOBAL_MEM_SIZE then global_mem_size
        end
      end
    end

    # =====================================================================
    # CLBuffer -- wraps Buffer
    # =====================================================================

    # An OpenCL buffer -- memory allocated on a device.
    class CLBuffer
      attr_reader :_buffer, :size, :flags

      def initialize(buffer, size, flags)
        @_buffer = buffer
        @size = size
        @flags = flags
      end
    end

    # =====================================================================
    # CLKernel -- a compiled kernel function
    # =====================================================================

    # An OpenCL kernel -- a compiled function extracted from a CLProgram.
    #
    # In OpenCL, kernel arguments are set one at a time with set_arg.
    class CLKernel
      attr_reader :name, :_code, :_args

      def initialize(name, code: nil)
        @name = name
        @_code = code
        @_args = {}
      end

      # Set a kernel argument at the given index.
      #
      # @param index [Integer] Argument index (0-based).
      # @param value [CLBuffer, Integer, Float, String] The argument value.
      def set_arg(index, value)
        @_args[index] = value
      end
    end

    # =====================================================================
    # CLProgram -- source code + compilation
    # =====================================================================

    # An OpenCL program -- source code that can be compiled for a device.
    class CLProgram
      attr_reader :build_status

      def initialize(source, context)
        @source = source
        @context = context
        @build_status = CLBuildStatus::NONE
        @kernels = {}
      end

      # Compile the program for the target device(s).
      #
      # @param devices [Array<CLDevice>, nil] Target devices. nil = all.
      # @param options [String] Compiler options (ignored).
      def build(devices: nil, options: "")
        @build_status = CLBuildStatus::SUCCESS
      end

      # Extract a kernel function from the compiled program.
      #
      # @param name [String] The kernel function name.
      # @return [CLKernel]
      # @raise [RuntimeError] If the program hasn't been built.
      def create_kernel(name)
        unless @build_status == CLBuildStatus::SUCCESS
          raise RuntimeError,
            "Program not built (status: #{@build_status}). " \
            "Call program.build first."
        end
        CLKernel.new(name, code: @kernels[name])
      end
    end

    # =====================================================================
    # CLCommandQueue -- enqueue operations with event dependencies
    # =====================================================================

    # An OpenCL command queue -- where operations are enqueued.
    #
    # Every operation returns a CLEvent for dependency tracking.
    class CLCommandQueue
      attr_reader :_context, :_device

      def initialize(context, device)
        @_context = context
        @_device = device
      end

      # Enqueue a kernel for execution (clEnqueueNDRangeKernel).
      #
      # @param kernel [CLKernel] The kernel to execute.
      # @param global_size [Array<Integer>] Total work items per dimension.
      # @param local_size [Array<Integer>, nil] Work items per workgroup.
      # @param wait_list [Array<CLEvent>, nil] Events to wait for.
      # @return [CLEvent]
      def enqueue_nd_range_kernel(kernel, global_size, local_size: nil, wait_list: nil)
        (wait_list || []).each(&:wait)

        device = @_context._logical_device

        # Determine local size (workgroup size)
        local = if local_size.nil?
          [32, 1, 1]
        else
          [
            local_size[0],
            local_size.length > 1 ? local_size[1] : 1,
            local_size.length > 2 ? local_size[2] : 1
          ]
        end

        # Calculate grid dimensions (number of workgroups)
        grid_x = [1, (global_size[0] + local[0] - 1) / local[0]].max
        grid_y = global_size.length > 1 ? [1, (global_size[1] + local[1] - 1) / local[1]].max : 1
        grid_z = global_size.length > 2 ? [1, (global_size[2] + local[2] - 1) / local[2]].max : 1

        # Create shader module from kernel code
        shader = device.create_shader_module(code: kernel._code, local_size: local)

        # Build descriptor set from kernel arguments
        buffer_args = kernel._args.select { |_i, arg| arg.is_a?(CLBuffer) }
        bindings = buffer_args.keys.sort.map do |i|
          ComputeRuntime::DescriptorBinding.new(binding: i, type: "storage")
        end
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        buffer_args.keys.sort.each do |i|
          ds.write(i, buffer_args[i]._buffer)
        end

        # Record and submit
        fence = device.create_fence
        cb = device.create_command_buffer
        cb.begin
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(ds)
        cb.cmd_dispatch(grid_x, grid_y, grid_z)
        cb.end_recording

        queue = @_context._compute_queue
        queue.submit([cb], fence: fence)
        fence.wait

        CLEvent.new(fence)
      end

      # Write host data to a device buffer (clEnqueueWriteBuffer).
      #
      # @param buffer [CLBuffer] Destination device buffer.
      # @param offset [Integer] Byte offset in the buffer.
      # @param size [Integer] Bytes to write.
      # @param host_ptr [String] Source host data.
      # @param wait_list [Array<CLEvent>, nil] Events to wait for first.
      # @return [CLEvent]
      def enqueue_write_buffer(buffer, offset, size, host_ptr, wait_list: nil)
        (wait_list || []).each(&:wait)

        mm = @_context._memory_manager
        mapped = mm.map(buffer._buffer)
        mapped.write(offset, host_ptr.byteslice(0, size))
        mm.unmap(buffer._buffer)

        fence = @_context._logical_device.create_fence(signaled: true)
        CLEvent.new(fence)
      end

      # Read device buffer data to host memory (clEnqueueReadBuffer).
      #
      # @param buffer [CLBuffer] Source device buffer.
      # @param offset [Integer] Byte offset in the buffer.
      # @param size [Integer] Bytes to read.
      # @param host_ptr [String] Destination host buffer (will be replaced).
      # @param wait_list [Array<CLEvent>, nil] Events to wait for first.
      # @return [CLEvent]
      def enqueue_read_buffer(buffer, offset, size, host_ptr, wait_list: nil)
        (wait_list || []).each(&:wait)

        mm = @_context._memory_manager
        mm.invalidate(buffer._buffer)
        mapped = mm.map(buffer._buffer)
        data = mapped.read(offset, size)
        mm.unmap(buffer._buffer)
        host_ptr.replace(data)

        fence = @_context._logical_device.create_fence(signaled: true)
        CLEvent.new(fence)
      end

      # Copy between two device buffers (clEnqueueCopyBuffer).
      #
      # @param src [CLBuffer] Source buffer.
      # @param dst [CLBuffer] Destination buffer.
      # @param size [Integer] Bytes to copy.
      # @param wait_list [Array<CLEvent>, nil] Events to wait for first.
      # @return [CLEvent]
      def enqueue_copy_buffer(src, dst, size, wait_list: nil)
        (wait_list || []).each(&:wait)

        device = @_context._logical_device
        fence = device.create_fence
        cb = device.create_command_buffer
        cb.begin
        cb.cmd_copy_buffer(src._buffer, dst._buffer, size)
        cb.end_recording
        @_context._compute_queue.submit([cb], fence: fence)
        fence.wait

        CLEvent.new(fence)
      end

      # Fill a buffer with a pattern (clEnqueueFillBuffer).
      #
      # @param buffer [CLBuffer] Buffer to fill.
      # @param pattern [String] Byte pattern to repeat.
      # @param offset [Integer] Start offset.
      # @param size [Integer] Bytes to fill.
      # @return [CLEvent]
      def enqueue_fill_buffer(buffer, pattern, offset, size)
        device = @_context._logical_device
        fence = device.create_fence
        cb = device.create_command_buffer
        cb.begin
        value = pattern.empty? ? 0 : pattern.getbyte(0)
        cb.cmd_fill_buffer(buffer._buffer, value, offset: offset, size: size)
        cb.end_recording
        @_context._compute_queue.submit([cb], fence: fence)
        fence.wait

        CLEvent.new(fence)
      end

      # Block until all enqueued operations complete (clFinish).
      def finish
        @_context._logical_device.wait_idle
      end

      # Ensure all enqueued operations are submitted (clFlush).
      # No-op in our synchronous simulator.
      def flush
        # No-op
      end
    end

    # =====================================================================
    # CLContext -- the OpenCL execution context
    # =====================================================================

    # An OpenCL context -- groups devices and manages shared resources.
    class CLContext < BaseVendorSimulator
      attr_reader :_devices

      def initialize(devices: nil)
        if devices
          vendor = devices[0]._physical.vendor
          super(vendor_hint: vendor)
          @_devices = devices
        else
          super()
          @_devices = @_physical_devices.map { |pd| CLDevice.new(pd) }
        end
      end

      # Create a device buffer (clCreateBuffer).
      #
      # @param flags [Integer] Memory flags (CLMemFlags constants, combinable with |).
      # @param size [Integer] Buffer size in bytes.
      # @param host_ptr [String, nil] Optional initial data.
      # @return [CLBuffer]
      def create_buffer(flags, size, host_ptr: nil)
        mem_type = ComputeRuntime::MemoryType::DEVICE_LOCAL |
          ComputeRuntime::MemoryType::HOST_VISIBLE |
          ComputeRuntime::MemoryType::HOST_COHERENT
        usage = ComputeRuntime::BufferUsage::STORAGE |
          ComputeRuntime::BufferUsage::TRANSFER_SRC |
          ComputeRuntime::BufferUsage::TRANSFER_DST

        buf = @_memory_manager.allocate(size, mem_type, usage: usage)
        cl_buf = CLBuffer.new(buf, size, flags)

        # If COPY_HOST_PTR, write the initial data
        if host_ptr && (flags & CLMemFlags::COPY_HOST_PTR) != 0
          mapped = @_memory_manager.map(buf)
          mapped.write(0, host_ptr.byteslice(0, size))
          @_memory_manager.unmap(buf)
        end

        cl_buf
      end

      # Create a program from source code (clCreateProgramWithSource).
      #
      # @param source [String] Kernel source code (label in our simulator).
      # @return [CLProgram]
      def create_program_with_source(source)
        CLProgram.new(source, self)
      end

      # Create a command queue for a device (clCreateCommandQueue).
      #
      # @param device [CLDevice, nil] Target device. nil = first device.
      # @param properties [Integer] Queue properties (ignored).
      # @return [CLCommandQueue]
      def create_command_queue(device: nil, properties: 0)
        dev = device || @_devices[0]
        CLCommandQueue.new(self, dev)
      end
    end

    # =====================================================================
    # CLPlatform -- the top-level discovery object
    # =====================================================================

    # An OpenCL platform -- represents a vendor's OpenCL implementation.
    class CLPlatform
      attr_reader :name, :vendor, :version

      def initialize
        @_instance = ComputeRuntime::RuntimeInstance.new
        @_physical_devices = @_instance.enumerate_physical_devices
        @name = "Coding Adventures Compute Platform"
        @vendor = "Coding Adventures"
        @version = "OpenCL 3.0"
      end

      # Enumerate available OpenCL platforms.
      #
      # @return [Array<CLPlatform>] List containing one CLPlatform.
      def self.get_platforms
        [new]
      end

      # Get devices of a specific type on this platform.
      #
      # @param device_type [String] Filter by device type. "all" = everything.
      # @return [Array<CLDevice>]
      def get_devices(device_type = CLDeviceType::ALL)
        devices = @_physical_devices.map { |pd| CLDevice.new(pd) }
        return devices if device_type == CLDeviceType::ALL
        devices.select { |d| d.device_type == device_type }
      end
    end
  end
end
