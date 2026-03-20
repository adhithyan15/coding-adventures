# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Global Memory -- device-wide VRAM / HBM simulator.
# ---------------------------------------------------------------------------
#
# === What is Global Memory? ===
#
# Global memory is the large, high-bandwidth memory that serves the entire
# accelerator device. Every compute unit can read from and write to global
# memory, making it the shared data store for all parallel computation.
#
#     NVIDIA: HBM3 (High Bandwidth Memory) -- 80 GB on H100
#     AMD:    GDDR6 -- 24 GB on RX 7900 XTX
#     Google: HBM2e -- 32 GB per TPU v4 chip
#     Intel:  GDDR6 -- 16 GB on Arc A770
#     Apple:  Unified LPDDR5 -- shared with CPU/GPU, up to 192 GB
#
# === Key Properties ===
#
# 1. **High bandwidth**: 1-3 TB/s. Much faster than CPU memory (~50 GB/s).
# 2. **High latency**: ~400-800 cycles to service a request.
# 3. **Shared**: ALL compute units on the device share global memory.
# 4. **Coalescing**: The memory controller can merge multiple thread
#    requests into fewer wide transactions if the addresses are contiguous.
# 5. **Partitioned**: Memory is physically split across channels/stacks.
#
# === Sparse Memory Representation ===
#
# Real devices have 16-80 GB of VRAM. We obviously can't allocate that in
# a simulator. Instead, we use a sparse Hash: only addresses that have
# been written to consume actual memory. A read to an uninitialized address
# returns zeros (matching real hardware behavior after cudaMemset).

module CodingAdventures
  module DeviceSimulator
    class SimpleGlobalMemory
      attr_reader :capacity, :bandwidth

      # Create a new global memory simulator.
      #
      # @param capacity [Integer] Total memory in bytes.
      # @param bandwidth [Float] Peak bandwidth in bytes per cycle.
      # @param latency [Integer] Access latency in cycles.
      # @param channels [Integer] Number of memory partitions/channels.
      # @param transaction_size [Integer] Width of a single memory transaction (bytes).
      # @param host_bandwidth [Float] PCIe/NVLink bandwidth in bytes per cycle.
      # @param host_latency [Integer] Initial latency for host transfers in cycles.
      # @param unified [Boolean] If true, host transfers are zero-cost (Apple).
      def initialize(
        capacity: 16 * 1024 * 1024,
        bandwidth: 1000.0,
        latency: 400,
        channels: 8,
        transaction_size: 128,
        host_bandwidth: 64.0,
        host_latency: 1000,
        unified: false
      )
        @capacity = capacity
        @bandwidth = bandwidth
        @latency = latency
        @channels = channels
        @transaction_size = transaction_size
        @host_bandwidth = host_bandwidth
        @host_latency = host_latency
        @unified = unified

        # Sparse storage -- only written addresses consume memory
        @data = {}

        # Simple bump allocator
        @next_free = 0
        @allocations = {} # start_addr -> size

        # Statistics
        @stats = GlobalMemoryStats.new
      end

      # Access statistics.
      #
      # @return [GlobalMemoryStats]
      def stats
        @stats.update_efficiency
        @stats
      end

      # --- Allocation ---

      # Allocate memory. Returns the start address.
      #
      # Uses a simple bump allocator with alignment. Like cudaMalloc,
      # this returns a device pointer that can be passed to kernels.
      #
      # @param size [Integer] Number of bytes to allocate.
      # @param alignment [Integer] Alignment in bytes (default 256 for cache lines).
      # @return [Integer] Start address of the allocated region.
      # @raise [MemoryError] If not enough memory remains.
      def allocate(size, alignment: 256)
        # Align the next free pointer
        aligned = (@next_free + alignment - 1) & ~(alignment - 1)

        if aligned + size > @capacity
          raise MemoryError,
            "Out of device memory: requested #{size} bytes " \
            "at #{aligned}, capacity #{@capacity}"
        end

        @allocations[aligned] = size
        @next_free = aligned + size
        aligned
      end

      # Free a previous allocation.
      #
      # Note: our simple bump allocator doesn't reclaim memory. In a real
      # implementation you'd use a more sophisticated allocator. But for
      # simulation purposes, this tracks that the free was called.
      #
      # @param address [Integer] The address to free.
      def free(address)
        @allocations.delete(address)
      end

      # --- Read / Write ---

      # Read bytes from global memory.
      #
      # Uninitialized addresses return zeros (like cudaMemset(0)).
      #
      # @param address [Integer] Start address to read from.
      # @param size [Integer] Number of bytes to read.
      # @return [String] The data as a binary string.
      # @raise [IndexError] If address is out of range.
      def read(address, size)
        if address < 0 || address + size > @capacity
          raise IndexError,
            "Address #{address}+#{size} out of range [0, #{@capacity})"
        end

        @stats.total_reads += 1
        @stats.bytes_transferred += size

        result = "\x00".b * size
        size.times do |i|
          result.setbyte(i, @data.fetch(address + i, 0))
        end
        result
      end

      # Write bytes to global memory.
      #
      # @param address [Integer] Start address to write to.
      # @param data [String] The data to write (binary string).
      # @raise [IndexError] If address is out of range.
      def write(address, data)
        size = data.bytesize
        if address < 0 || address + size > @capacity
          raise IndexError,
            "Address #{address}+#{size} out of range [0, #{@capacity})"
        end

        @stats.total_writes += 1
        @stats.bytes_transferred += size

        size.times do |i|
          @data[address + i] = data.getbyte(i)
        end
      end

      # --- Host transfers ---

      # Copy from host (CPU) to device memory.
      #
      # Like cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice).
      #
      # For unified memory (Apple), this is zero-cost -- no actual data
      # movement, just a page table remap.
      #
      # @param dst_addr [Integer] Destination address in device memory.
      # @param data [String] The data to copy (binary string).
      # @param host_bandwidth [Float, nil] Override for host bandwidth.
      # @return [Integer] Number of cycles consumed by the transfer.
      def copy_from_host(dst_addr, data, host_bandwidth: nil)
        write(dst_addr, data)

        bw = host_bandwidth || @host_bandwidth
        size = data.bytesize
        @stats.host_to_device_bytes += size

        if @unified
          # Unified memory: zero-copy
          return 0
        end

        # Transfer time = latency + size / bandwidth
        cycles = bw > 0 ? @host_latency + (size / bw).to_i : 0
        @stats.host_transfer_cycles += cycles
        cycles
      end

      # Copy from device memory to host (CPU).
      #
      # Like cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost).
      #
      # @param src_addr [Integer] Source address in device memory.
      # @param size [Integer] Number of bytes to copy.
      # @param host_bandwidth [Float, nil] Override for host bandwidth.
      # @return [Array(String, Integer)] Tuple of [data, cycles_consumed].
      def copy_to_host(src_addr, size, host_bandwidth: nil)
        data = read(src_addr, size)

        bw = host_bandwidth || @host_bandwidth
        @stats.device_to_host_bytes += size

        if @unified
          return [data, 0]
        end

        cycles = bw > 0 ? @host_latency + (size / bw).to_i : 0
        @stats.host_transfer_cycles += cycles
        [data, cycles]
      end

      # --- Coalescing ---

      # Given per-thread addresses, merge into coalesced transactions.
      #
      # === Coalescing Algorithm ===
      #
      # 1. For each thread's address, compute which transaction-sized
      #    aligned region it falls in.
      # 2. Group threads by aligned region.
      # 3. Each group becomes one transaction.
      #
      # The fewer transactions, the better -- ideal is 1 transaction
      # for 32 threads (128 bytes of contiguous access).
      #
      # @param addresses [Array<Integer>] List of addresses, one per thread.
      # @param size [Integer] Size of each thread's access in bytes.
      # @return [Array<MemoryTransaction>] Transactions after coalescing.
      def coalesce(addresses, size: 4)
        ts = @transaction_size

        # Group threads by aligned transaction address
        groups = {} # aligned_addr -> thread_mask
        addresses.each_with_index do |addr, thread_idx|
          aligned = (addr / ts) * ts
          groups[aligned] ||= 0
          groups[aligned] |= 1 << thread_idx
        end

        transactions = groups.sort.map do |aligned, mask|
          MemoryTransaction.new(address: aligned, size: ts, thread_mask: mask)
        end

        # Track stats
        @stats.total_requests += addresses.length
        @stats.total_transactions += transactions.length

        # Check partition conflicts
        channels_hit = {}
        transactions.each do |txn|
          channel = (txn.address / ts) % @channels
          channels_hit[channel] = (channels_hit[channel] || 0) + 1
        end
        channels_hit.each_value do |count|
          @stats.partition_conflicts += count - 1 if count > 1
        end

        transactions
      end

      # --- Reset ---

      # Clear all data, allocations, and statistics.
      def reset
        @data.clear
        @next_free = 0
        @allocations.clear
        @stats = GlobalMemoryStats.new
      end
    end

    # Ruby's MemoryError doesn't exist by default -- define a simple alias.
    # In MRI Ruby, there's no built-in MemoryError, so we use NoMemoryError
    # which is a subclass of Exception. For our simulation, we use a custom
    # StandardError-based error for easier catching.
    class MemoryError < StandardError; end
  end
end
