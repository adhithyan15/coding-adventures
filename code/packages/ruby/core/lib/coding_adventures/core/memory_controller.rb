# frozen_string_literal: true

# MemoryController -- serializes memory requests from multiple cores.
#
# = Why a Memory Controller?
#
# In a multi-core system, multiple cores may request memory access in the
# same clock cycle. Real memory (DRAM) can only handle a limited number of
# concurrent requests, so the memory controller queues and serializes them.
#
# The memory controller is like a librarian at a busy library: patrons
# (cores) line up with their requests, and the librarian processes them
# one at a time, delivering books (data) after a delay (latency).
#
# = Latency Simulation
#
# Each memory request takes `latency` cycles to complete. The controller
# counts down the remaining cycles on each tick. When a request reaches
# zero remaining cycles, its data is delivered to the requester.
#
# = Memory Model
#
# The underlying memory is a flat byte array. Word reads/writes use
# little-endian byte ordering, matching modern ARM and x86 architectures.

module CodingAdventures
  module Core
    # An in-flight read request.
    MemoryRequest = Data.define(:address, :num_bytes, :requester_id, :cycles_left)

    # An in-flight write request.
    MemoryWriteRequest = Data.define(:address, :data, :requester_id, :cycles_left)

    # A completed read -- data delivered to a requester.
    MemoryReadResult = Data.define(:requester_id, :address, :data)

    class MemoryController
      # @return [Integer] total size of memory in bytes.
      attr_reader :memory_size

      # Creates a memory controller with the given backing memory and access
      # latency.
      #
      # The memory array is shared (not copied) -- multiple cores access the
      # same underlying bytes. This models shared physical memory.
      #
      # @param memory [Array<Integer>] backing byte array.
      # @param latency [Integer] number of cycles for a memory access.
      def initialize(memory, latency)
        @memory = memory
        @latency = latency
        @pending_reads = []
        @pending_writes = []
        @completed_reads = []
      end

      # Submits a read request.
      #
      # The read will complete after `latency` cycles. Call tick each cycle
      # and check the returned results for completed reads.
      #
      # @param address [Integer] starting byte address.
      # @param num_bytes [Integer] number of bytes to read.
      # @param requester_id [Integer] which core submitted the request.
      def request_read(address, num_bytes, requester_id)
        @pending_reads << {address: address, num_bytes: num_bytes,
                           requester_id: requester_id, cycles_left: @latency}
      end

      # Submits a write request.
      #
      # The write completes after `latency` cycles. The data is committed to
      # memory when the request finishes (not immediately).
      #
      # @param address [Integer] starting byte address.
      # @param data [Array<Integer>] bytes to write.
      # @param requester_id [Integer] which core submitted the request.
      def request_write(address, data, requester_id)
        data_copy = data.dup
        @pending_writes << {address: address, data: data_copy,
                            requester_id: requester_id, cycles_left: @latency}
      end

      # Advances the memory controller by one cycle.
      #
      # Decrements all pending request counters. When a request reaches zero
      # remaining cycles, it is completed:
      #   - Reads: data is copied from memory and returned in the result list
      #   - Writes: data is committed to memory
      #
      # @return [Array<MemoryReadResult>] completed read results.
      def tick
        @completed_reads = []

        # Process pending reads.
        remaining = []
        @pending_reads.each do |req|
          req[:cycles_left] -= 1
          if req[:cycles_left] <= 0
            data = read_memory(req[:address], req[:num_bytes])
            @completed_reads << MemoryReadResult.new(
              requester_id: req[:requester_id],
              address: req[:address],
              data: data
            )
          else
            remaining << req
          end
        end
        @pending_reads = remaining

        # Process pending writes.
        remaining_writes = []
        @pending_writes.each do |req|
          req[:cycles_left] -= 1
          if req[:cycles_left] <= 0
            write_memory(req[:address], req[:data])
          else
            remaining_writes << req
          end
        end
        @pending_writes = remaining_writes

        @completed_reads
      end

      # Reads a 32-bit word from memory at the given address.
      # Little-endian byte order.
      #
      # @param address [Integer] byte address.
      # @return [Integer] 32-bit word value.
      def read_word(address)
        return 0 if address < 0 || address + 4 > @memory.length
        @memory[address] |
          (@memory[address + 1] << 8) |
          (@memory[address + 2] << 16) |
          (@memory[address + 3] << 24)
      end

      # Writes a 32-bit word to memory at the given address.
      # Little-endian byte order.
      #
      # @param address [Integer] byte address.
      # @param value [Integer] 32-bit word value.
      def write_word(address, value)
        return if address < 0 || address + 4 > @memory.length
        @memory[address] = value & 0xFF
        @memory[address + 1] = (value >> 8) & 0xFF
        @memory[address + 2] = (value >> 16) & 0xFF
        @memory[address + 3] = (value >> 24) & 0xFF
      end

      # Copies program bytes into memory starting at the given address.
      #
      # @param program [Array<Integer>] byte array to load.
      # @param start_address [Integer] memory address to write to.
      def load_program(program, start_address)
        return if start_address < 0 || start_address + program.length > @memory.length
        program.each_with_index do |byte, i|
          @memory[start_address + i] = byte
        end
      end

      # Returns the total size of memory in bytes.
      #
      # @return [Integer] memory size.
      def memory_size
        @memory.length
      end

      # Returns the number of in-flight requests.
      #
      # @return [Integer] pending count.
      def pending_count
        @pending_reads.length + @pending_writes.length
      end

      private

      # Reads bytes from the backing memory array.
      def read_memory(address, num_bytes)
        if address < 0 || address + num_bytes > @memory.length
          return Array.new(num_bytes, 0)
        end
        @memory[address, num_bytes].dup
      end

      # Writes bytes to the backing memory array.
      def write_memory(address, data)
        return if address < 0 || address + data.length > @memory.length
        data.each_with_index do |byte, i|
          @memory[address + i] = byte
        end
      end
    end
  end
end
