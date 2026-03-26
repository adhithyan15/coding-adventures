# frozen_string_literal: true

# === Sparse Memory: Simulating a 32-bit Address Space ===
#
# A real 32-bit CPU can address 4 GB of memory (2^32 bytes). But most of
# that address space is empty -- a typical embedded system might have:
#
#   0x00000000 - 0x000FFFFF: 1 MB of RAM (for code and data)
#   0xFFFB0000 - 0xFFFFFFFF: 320 KB of I/O registers (for peripherals)
#
# Everything in between is unmapped -- accessing it would trigger a bus fault
# on real hardware. Allocating a contiguous 4 GB byte array to simulate this
# would be wasteful and impractical.
#
# SparseMemory solves this by mapping only the regions that actually exist.
# Each region is a named byte array at a specific base address. Reads and
# writes are dispatched to the correct region by checking address ranges.
#
# === How it works ===
#
# Think of SparseMemory as a building with multiple floors:
#
#   Floor 0 (0x00000000): RAM      -- read/write, for code and data
#   Floor N (0xFFFB0000): I/O Regs -- some read-only, some read/write
#
# === Read-only regions ===
#
# Some regions should never be written to (ROM, read-only status registers).
# When a region is marked read_only, writes are silently ignored, matching
# real hardware behavior.

module CodingAdventures
  module CpuSimulator
    # A contiguous block of addressable memory.
    #
    # Each region has a base address, a size, and a backing byte array.
    # The region occupies addresses [base, base + size). Any access within
    # this range is translated to an offset into the data array:
    #
    #   offset = address - base
    #   value  = data[offset]
    class MemoryRegion
      attr_reader :base, :size, :data, :name, :read_only

      def initialize(base:, size:, name: "", read_only: false, data: nil)
        @base = base
        @size = size
        @name = name
        @read_only = read_only
        @data = data ? data.dup : Array.new(size, 0)
      end
    end

    # Maps address ranges to backing byte arrays, enabling a full 32-bit
    # address space without allocating 4 GB.
    #
    # === Region lookup ===
    #
    # On every access, SparseMemory searches through its regions to find one
    # that contains the target address. This is a linear scan -- O(N) where N
    # is the number of regions.
    #
    # === Unmapped addresses ===
    #
    # If no region contains the target address, the access raises a
    # RuntimeError. On real hardware this would be a bus fault.
    #
    # Example:
    #
    #   regions = [
    #     MemoryRegion.new(base: 0x00000000, size: 0x100000, name: "RAM"),
    #     MemoryRegion.new(base: 0xFFFB0000, size: 0x50000, name: "I/O", read_only: true),
    #   ]
    #   mem = SparseMemory.new(regions)
    class SparseMemory
      attr_reader :regions

      def initialize(regions)
        @regions = regions.map do |r|
          MemoryRegion.new(
            base: r.base,
            size: r.size,
            name: r.name,
            read_only: r.read_only,
            data: r.data
          )
        end
      end

      # Read a single byte from the sparse address space.
      def read_byte(address)
        region, offset = find_region(address, 1)
        region.data[offset]
      end

      # Write a single byte. If the target region is read-only, the write
      # is silently ignored (matches real hardware behavior).
      def write_byte(address, value)
        region, offset = find_region(address, 1)
        return if region.read_only

        region.data[offset] = value & 0xFF
      end

      # Read a 32-bit word (4 bytes) in little-endian byte order.
      #
      # Little-endian means the least significant byte is stored at the lowest
      # address. For 0xDEADBEEF at address 0x1000:
      #
      #   0x1000: 0xEF  (least significant)
      #   0x1001: 0xBE
      #   0x1002: 0xAD
      #   0x1003: 0xDE  (most significant)
      def read_word(address)
        region, offset = find_region(address, 4)
        region.data[offset] |
          (region.data[offset + 1] << 8) |
          (region.data[offset + 2] << 16) |
          (region.data[offset + 3] << 24)
      end

      # Write a 32-bit word in little-endian byte order.
      # If the target region is read-only, the write is silently ignored.
      def write_word(address, value)
        region, offset = find_region(address, 4)
        return if region.read_only

        value = value & 0xFFFFFFFF
        region.data[offset] = value & 0xFF
        region.data[offset + 1] = (value >> 8) & 0xFF
        region.data[offset + 2] = (value >> 16) & 0xFF
        region.data[offset + 3] = (value >> 24) & 0xFF
      end

      # Copy bytes into the sparse address space starting at `address`.
      #
      # Typically used to load a program binary into simulated RAM or to
      # initialize ROM contents. The entire range must fall within a single
      # region.
      #
      # Note: load_bytes bypasses the read_only check. This allows pre-loading
      # ROM contents during system initialization before the CPU starts.
      def load_bytes(address, data)
        bytes = data.is_a?(String) ? data.bytes : data
        region, offset = find_region(address, bytes.size)
        bytes.each_with_index do |byte, i|
          region.data[offset + i] = byte
        end
      end

      # Return a copy of bytes from the sparse address space as an Array.
      # The returned array is a copy -- modifying it does not affect memory.
      def dump(start, length)
        region, offset = find_region(start, length)
        region.data[offset, length].dup
      end

      # Return the number of mapped regions.
      def region_count
        @regions.size
      end

      private

      # Locate the MemoryRegion containing [address, address + num_bytes).
      # Returns [region, offset_within_region].
      # Raises RuntimeError if unmapped (models a bus fault).
      def find_region(address, num_bytes)
        range_end = address + num_bytes
        @regions.each do |r|
          region_end = r.base + r.size
          if address >= r.base && range_end <= region_end
            offset = address - r.base
            return [r, offset]
          end
        end
        raise "SparseMemory: unmapped address 0x#{format("%08X", address)} (accessing #{num_bytes} bytes)"
      end
    end
  end
end
