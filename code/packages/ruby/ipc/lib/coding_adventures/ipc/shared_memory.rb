# frozen_string_literal: true

# Shared Memory -- zero-copy communication via a shared data region.
#
# Pipes and message queues both **copy** data: the sender writes bytes into
# a kernel buffer, and the receiver copies them out. For large data transfers,
# this double-copy is expensive.
#
# Shared memory eliminates copying entirely. Two (or more) processes map the
# **same physical pages** into their virtual address spaces. A write by one
# process is immediately visible to the others -- no system call, no copy,
# no kernel involvement after the initial setup.
#
# === How It Works (Real OS) ===
#
#   Process A's address space       Process B's address space
#   +-----------------------+       +-----------------------+
#   | ...                   |       | ...                   |
#   | 0x8000: Shared Region |       | 0xC000: Shared Region |
#   |   "Hello from A"     |  <--  |   "Hello from A"      |
#   | ...                   |  -->  | ...                   |
#   +-----------------------+       +-----------------------+
#            |                                |
#            +----------+--------------------+
#                       |
#                Physical Page Frame #42
#
# Both virtual addresses (0x8000 in A, 0xC000 in B) map to the same physical
# page. When A writes "Hello from A", B can read it instantly without any
# copying -- they're literally reading the same bytes in RAM.
#
# === Our Simulation ===
#
# We can't manipulate real page tables, so we simulate shared memory with a
# named region backed by a Ruby Array of bytes. "Attaching" a process means
# recording its PID in the attached_pids set; reading and writing access the
# shared array directly.
#
# === WARNING: No Synchronization ===
#
# Shared memory has NO built-in synchronization. If process A writes while
# process B reads, B may see partially-updated data (a "torn read"). Real
# programs use semaphores, mutexes, or atomic operations to coordinate access.
# We omit synchronization here for simplicity, but this is a critical concern
# in production systems.

module CodingAdventures
  module Ipc
    # Error raised when a read or write exceeds the region's bounds.
    class SharedMemoryBoundsError < StandardError; end

    class SharedMemoryRegion
      attr_reader :name, :size, :owner_pid, :attached_pids

      # Create a new shared memory region.
      #
      # Parameters:
      #   name      - a string identifier (like a file path) that unrelated
      #               processes use to find this region. Think of it like a
      #               phone number: if two processes agree on the name, they
      #               can share memory.
      #   size      - the region size in bytes.
      #   owner_pid - the process ID of the creator. The owner has special
      #               privileges (e.g., deleting the region).
      def initialize(name:, size:, owner_pid:)
        @name = name
        @size = size
        @owner_pid = owner_pid

        # The shared data -- a flat array of bytes, zero-initialized.
        # This represents the physical page(s) that back the region.
        @data = Array.new(size, 0)

        # Set of process IDs currently attached to this region.
        # Used for cleanup: when the last process detaches and the region
        # is marked for deletion, the kernel can free the memory.
        @attached_pids = Set.new
      end

      # Attach a process to this shared memory region.
      #
      # In a real OS, this modifies the process's page table to map the
      # shared physical pages into the process's virtual address space.
      # In our simulation, we just record the PID.
      def attach(pid)
        @attached_pids.add(pid)
      end

      # Detach a process from this shared memory region.
      #
      # In a real OS, this unmaps the shared pages from the process's
      # virtual address space. The physical pages remain as long as at
      # least one process is still attached (or the region hasn't been
      # marked for deletion).
      def detach(pid)
        @attached_pids.delete(pid)
      end

      # Read `count` bytes starting at `offset`.
      #
      # Returns an array of byte values. Raises SharedMemoryBoundsError
      # if the read would go past the end of the region.
      #
      # Unlike pipes (which are sequential), shared memory supports
      # **random access** -- you can read from any offset at any time.
      def read(offset, count)
        validate_bounds!(offset, count)
        @data[offset, count].dup
      end

      # Write `data` (array of byte values) starting at `offset`.
      #
      # Raises SharedMemoryBoundsError if the write would go past the end
      # of the region.
      #
      # WARNING: This method has no synchronization. If two processes call
      # write() concurrently with overlapping ranges, the result is
      # undefined (a "data race"). Real programs must use external
      # synchronization (semaphores, mutexes) to prevent this.
      def write(offset, data)
        validate_bounds!(offset, data.length)
        data.each_with_index do |byte, i|
          @data[offset + i] = byte & 0xFF
        end
      end

      private

      # Check that offset + count stays within the region's bounds.
      #
      # This is the equivalent of the kernel's bounds checking when a
      # process accesses shared memory. A real OS would send SIGSEGV
      # (segmentation fault) for out-of-bounds access; we raise a Ruby
      # exception instead.
      def validate_bounds!(offset, count)
        if offset < 0 || count < 0 || offset + count > @size
          raise SharedMemoryBoundsError,
            "access at offset #{offset} with count #{count} exceeds region size #{@size}"
        end
      end
    end
  end
end
