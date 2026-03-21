# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Memory management -- typed allocations, mapping, staging.
# ---------------------------------------------------------------------------
#
# === Memory Types on a GPU ===
#
# Unlike a CPU where all RAM is equally accessible, GPUs have distinct memory
# pools with different performance characteristics:
#
#     +-------------------------------------------------------------+
#     |                    Discrete GPU (NVIDIA, AMD)                 |
#     |                                                              |
#     |   CPU side (system RAM)              GPU side (VRAM)         |
#     |   +------------------+               +------------------+   |
#     |   |   HOST_VISIBLE   |<---- PCIe --->|   DEVICE_LOCAL   |   |
#     |   |   HOST_COHERENT  |   ~32 GB/s    |   (HBM / GDDR6)  |   |
#     |   |   (staging pool) |               |   1-3 TB/s        |   |
#     |   +------------------+               +------------------+   |
#     +-------------------------------------------------------------+
#
#     +-------------------------------------------------------------+
#     |                  Unified Memory (Apple M-series)              |
#     |                                                              |
#     |   +------------------------------------------------------+  |
#     |   |        DEVICE_LOCAL + HOST_VISIBLE + HOST_COHERENT     |  |
#     |   |        (shared physical RAM)                           |  |
#     |   |        Both CPU and GPU see the same bytes             |  |
#     |   |        No copy needed!                                 |  |
#     |   +------------------------------------------------------+  |
#     +-------------------------------------------------------------+
#
# === The Staging Buffer Pattern ===
#
# On discrete GPUs, the standard way to get data onto the GPU is:
#
#     1. Allocate a HOST_VISIBLE staging buffer (CPU can write to it)
#     2. Map it, write your data, unmap it
#     3. Record a cmd_copy_buffer from staging -> DEVICE_LOCAL
#     4. Submit and wait
#
# On unified memory (Apple), you skip all of this -- allocate DEVICE_LOCAL +
# HOST_VISIBLE, write directly, and the GPU sees it immediately.

module CodingAdventures
  module ComputeRuntime
    # =====================================================================
    # Buffer -- a typed allocation on the device
    # =====================================================================
    #
    # === Buffer Lifecycle ===
    #
    #     allocate() -> Buffer (with device_address)
    #     map()      -> MappedMemory (CPU can read/write)
    #     unmap()    -> buffer is GPU-only again
    #     free()     -> memory returned to pool
    #
    # Fields:
    #   buffer_id:       Unique identifier for this buffer.
    #   size:            Size in bytes.
    #   memory_type:     Integer (bitwise OR of MemoryType flags).
    #   usage:           Integer (bitwise OR of BufferUsage flags).
    #   device_address:  Address on the device (from Layer 6 malloc).
    #   mapped:          Whether this buffer is currently CPU-mapped.
    #   freed:           Whether this buffer has been freed.
    class Buffer
      attr_reader :buffer_id, :size, :memory_type, :usage, :device_address
      attr_accessor :mapped, :freed

      def initialize(buffer_id:, size:, memory_type:, usage:, device_address: 0)
        @buffer_id = buffer_id
        @size = size
        @memory_type = memory_type
        @usage = usage
        @device_address = device_address
        @mapped = false
        @freed = false
      end
    end

    # =====================================================================
    # MappedMemory -- CPU-accessible view of a buffer
    # =====================================================================
    #
    # === What is Memory Mapping? ===
    #
    # Mapping makes device memory accessible to the CPU. On discrete GPUs,
    # this only works for HOST_VISIBLE memory (system RAM accessible via PCIe).
    # On unified memory, any buffer can be mapped.
    #
    # After mapping, you can read() and write() bytes. After unmap(), the
    # CPU can no longer access this memory.
    #
    # === Implementation ===
    #
    # Under the hood, we store the data in a Ruby String (binary). When the
    # buffer is unmapped, we sync it back to the device's memory via Layer 6.
    class MappedMemory
      attr_reader :buffer

      def initialize(buffer, data)
        @buffer = buffer
        @data = data
        @dirty = false
      end

      # Size of the mapped region.
      def size
        @data.bytesize
      end

      # Whether any writes have been made since mapping.
      def dirty? = @dirty

      # Alias for compatibility.
      def dirty = @dirty

      # Read bytes from the mapped buffer.
      #
      # @param offset [Integer] Byte offset from start of buffer.
      # @param read_size [Integer] Number of bytes to read.
      # @return [String] The requested bytes (binary string).
      # @raise [ArgumentError] If offset + read_size exceeds buffer size.
      def read(offset, read_size)
        if offset + read_size > @data.bytesize
          raise ArgumentError,
            "Read out of bounds: offset=#{offset}, size=#{read_size}, " \
            "buffer_size=#{@data.bytesize}"
        end
        @data.byteslice(offset, read_size)
      end

      # Write bytes to the mapped buffer.
      #
      # @param offset [Integer] Byte offset from start of buffer.
      # @param bytes [String] Bytes to write (binary string).
      # @raise [ArgumentError] If offset + bytes.length exceeds buffer size.
      def write(offset, bytes)
        if offset + bytes.bytesize > @data.bytesize
          raise ArgumentError,
            "Write out of bounds: offset=#{offset}, data_size=#{bytes.bytesize}, " \
            "buffer_size=#{@data.bytesize}"
        end
        @data[offset, bytes.bytesize] = bytes
        @dirty = true
      end

      # Get the full contents of the mapped buffer.
      def get_data
        @data.dup
      end
    end

    # =====================================================================
    # MemoryManager -- allocates, maps, frees device memory
    # =====================================================================
    #
    # === How It Works ===
    #
    # The MemoryManager wraps Layer 6's raw malloc/free with type information.
    # Each allocation is tagged with a MemoryType and BufferUsage, which the
    # runtime uses for validation and optimization.
    #
    # For HOST_VISIBLE allocations, the manager supports mapping -- making the
    # buffer accessible to the CPU. For DEVICE_LOCAL-only allocations, mapping
    # is not allowed (you must use a staging buffer + copy).
    #
    # === Memory Tracking ===
    #
    # The manager tracks all allocations, peak usage, and produces traces
    # for every operation.
    class MemoryManager
      attr_reader :memory_properties

      def initialize(device:, memory_properties:, stats:)
        @device = device
        @memory_properties = memory_properties
        @stats = stats
        @buffers = {}
        @buffer_data = {}
        @next_id = 0
        @current_bytes = 0
      end

      # Allocate a buffer on the device.
      #
      # === The Allocation Flow ===
      #
      #     MemoryManager.allocate(1024, DEVICE_LOCAL)
      #         |
      #         +---> Validate: size > 0, memory type supported
      #         +---> Layer 6: device.malloc(1024) -> device_address
      #         +---> Create Buffer object with metadata
      #         +---> Track in @buffers hash
      #         +---> Log RuntimeTrace event
      #
      # @param size [Integer] Number of bytes to allocate.
      # @param memory_type [Integer] Bitwise OR of MemoryType flags.
      # @param usage [Integer] Bitwise OR of BufferUsage flags.
      # @return [Buffer] A Buffer with a valid device_address.
      # @raise [ArgumentError] If size <= 0.
      def allocate(size, memory_type, usage: BufferUsage::STORAGE)
        raise ArgumentError, "Allocation size must be positive, got #{size}" if size <= 0

        device_address = @device.malloc(size)

        buf_id = @next_id
        @next_id += 1

        buf = Buffer.new(
          buffer_id: buf_id,
          size: size,
          memory_type: memory_type,
          usage: usage,
          device_address: device_address
        )
        @buffers[buf_id] = buf
        @buffer_data[buf_id] = ("\x00" * size).b

        # Track stats
        @current_bytes += size
        @stats.total_allocated_bytes += size
        @stats.total_allocations += 1
        if @current_bytes > @stats.peak_allocated_bytes
          @stats.peak_allocated_bytes = @current_bytes
        end

        @stats.traces << RuntimeTrace.new(
          event_type: :memory_alloc,
          description: "Allocated #{size} bytes (buf##{buf_id}, type=#{memory_type})"
        )

        buf
      end

      # Free a device memory allocation.
      #
      # @param buffer [Buffer] The buffer to free.
      # @raise [ArgumentError] If buffer is already freed, not found, or still mapped.
      def free(buffer)
        raise ArgumentError, "Buffer #{buffer.buffer_id} already freed" if buffer.freed
        raise ArgumentError, "Buffer #{buffer.buffer_id} not found" unless @buffers.key?(buffer.buffer_id)
        if buffer.mapped
          raise ArgumentError,
            "Buffer #{buffer.buffer_id} is still mapped -- unmap before freeing"
        end

        @device.free(buffer.device_address)
        buffer.freed = true
        @current_bytes -= buffer.size
        @buffers.delete(buffer.buffer_id)
        @buffer_data.delete(buffer.buffer_id)

        @stats.total_frees += 1
        @stats.traces << RuntimeTrace.new(
          event_type: :memory_free,
          description: "Freed buf##{buffer.buffer_id} (#{buffer.size} bytes)"
        )
      end

      # Map a buffer for CPU access.
      #
      # === When Can You Map? ===
      #
      # Only HOST_VISIBLE buffers can be mapped. On discrete GPUs, this
      # means staging buffers (in system RAM). On unified memory devices,
      # all buffers are HOST_VISIBLE so everything can be mapped.
      #
      # @param buffer [Buffer] The buffer to map.
      # @return [MappedMemory] A MappedMemory object for reading/writing.
      # @raise [ArgumentError] If buffer is not HOST_VISIBLE, already mapped, or freed.
      def map(buffer)
        raise ArgumentError, "Cannot map freed buffer #{buffer.buffer_id}" if buffer.freed
        raise ArgumentError, "Buffer #{buffer.buffer_id} is already mapped" if buffer.mapped
        unless (buffer.memory_type & MemoryType::HOST_VISIBLE) != 0
          raise ArgumentError,
            "Cannot map buffer #{buffer.buffer_id}: not HOST_VISIBLE " \
            "(type=#{buffer.memory_type})"
        end

        buffer.mapped = true
        @stats.total_maps += 1

        @stats.traces << RuntimeTrace.new(
          event_type: :memory_map,
          description: "Mapped buf##{buffer.buffer_id}"
        )

        MappedMemory.new(buffer, @buffer_data[buffer.buffer_id])
      end

      # Unmap a buffer, ending CPU access.
      #
      # If the mapped memory was written to (dirty), and the buffer has
      # HOST_COHERENT, the data is automatically synced to the device.
      #
      # @param buffer [Buffer] The buffer to unmap.
      # @raise [ArgumentError] If buffer is not currently mapped.
      def unmap(buffer)
        raise ArgumentError, "Buffer #{buffer.buffer_id} is not mapped" unless buffer.mapped

        # If HOST_COHERENT, automatically sync to device
        if (buffer.memory_type & MemoryType::HOST_COHERENT) != 0
          data = @buffer_data[buffer.buffer_id]
          @device.memcpy_host_to_device(buffer.device_address, data)
        end

        buffer.mapped = false
      end

      # Flush CPU writes to make them visible to GPU.
      #
      # Only needed for HOST_VISIBLE buffers without HOST_COHERENT.
      # For HOST_COHERENT buffers, writes are automatically visible.
      #
      # @param buffer [Buffer] The buffer to flush.
      # @param offset [Integer] Start of the region to flush.
      # @param size [Integer] Size of the region (0 = whole buffer).
      def flush(buffer, offset: 0, size: 0)
        raise ArgumentError, "Cannot flush freed buffer #{buffer.buffer_id}" if buffer.freed
        actual_size = size > 0 ? size : buffer.size
        data = @buffer_data[buffer.buffer_id].byteslice(offset, actual_size)
        @device.memcpy_host_to_device(buffer.device_address + offset, data)
      end

      # Invalidate CPU cache so GPU writes become visible to CPU.
      #
      # After the GPU writes to a buffer and you want to read it back
      # from the CPU, call invalidate first to pull the latest data.
      #
      # @param buffer [Buffer] The buffer to invalidate.
      # @param offset [Integer] Start of the region to invalidate.
      # @param size [Integer] Size of the region (0 = whole buffer).
      def invalidate(buffer, offset: 0, size: 0)
        raise ArgumentError, "Cannot invalidate freed buffer #{buffer.buffer_id}" if buffer.freed
        actual_size = size > 0 ? size : buffer.size
        data, _cycles = @device.memcpy_device_to_host(
          buffer.device_address + offset, actual_size
        )
        @buffer_data[buffer.buffer_id][offset, actual_size] = data
      end

      # Look up a buffer by ID.
      #
      # @param buffer_id [Integer] The buffer ID.
      # @return [Buffer] The buffer.
      # @raise [ArgumentError] If buffer not found.
      def get_buffer(buffer_id)
        raise ArgumentError, "Buffer #{buffer_id} not found" unless @buffers.key?(buffer_id)
        @buffers[buffer_id]
      end

      # Number of currently allocated buffers.
      def allocated_buffer_count
        @buffers.size
      end

      # Current total bytes allocated.
      def current_allocated_bytes
        @current_bytes
      end

      # Internal: get raw data for a buffer.
      def _get_buffer_data(buffer_id)
        @buffer_data[buffer_id]
      end

      # Internal: push buffer data to device. Returns cycles consumed.
      def _sync_buffer_to_device(buffer)
        data = @buffer_data[buffer.buffer_id]
        @device.memcpy_host_to_device(buffer.device_address, data)
      end

      # Internal: pull buffer data from device. Returns cycles consumed.
      def _sync_buffer_from_device(buffer)
        data, cycles = @device.memcpy_device_to_host(
          buffer.device_address, buffer.size
        )
        @buffer_data[buffer.buffer_id][0, buffer.size] = data
        cycles
      end
    end
  end
end
