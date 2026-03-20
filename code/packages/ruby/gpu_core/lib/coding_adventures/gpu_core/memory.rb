# frozen_string_literal: true

# ---------------------------------------------------------------------------
# LocalMemory -- byte-addressable scratchpad with floating-point load/store.
# ---------------------------------------------------------------------------
#
# === What is Local Memory? ===
#
# Every GPU thread has a small, private memory area called "local memory" or
# "scratchpad." It's used for temporary storage that doesn't fit in registers:
# spilled variables, array elements, intermediate results.
#
#     +----------------------------------------------+
#     |              Local Memory (4 KB)              |
#     +----------------------------------------------+
#     |  0x000: [42] [00] [48] [42]  <- 3.14 as FP32 |
#     |  0x004: [EC] [51] [2D] [40]  <- 2.71 as FP32 |
#     |  0x008: [00] [00] [00] [00]  <- 0.0           |
#     |  ...                                          |
#     |  0xFFC: [00] [00] [00] [00]                    |
#     +----------------------------------------------+
#
# === How Floats Live in Memory ===
#
# A FloatBits value (sign + exponent + mantissa) must be converted to raw bytes
# before it can be stored in memory. This is the same process that happens in
# real hardware when a GPU core executes a STORE instruction:
#
#     1. Take the FloatBits fields: sign=0, exponent=[01111111], mantissa=[10010...]
#     2. Concatenate into a bit string: 0_01111111_10010001000011111101101
#     3. Group into bytes: [3F] [C9] [0F] [DB]  (that's 3.14159 in FP32)
#     4. Write bytes to memory in little-endian order: [DB] [0F] [C9] [3F]
#
# Loading reverses this: read bytes, reassemble bits, create FloatBits.
#
# === Memory Sizes Across Vendors ===
#
#     NVIDIA: 512 KB local memory per thread (rarely used, slow)
#     AMD:    Scratch memory, up to 4 MB per wavefront
#     ARM:    Stack memory region per thread
#     TPU:    No per-PE memory (data flows through systolic array)
#
# Our default of 4 KB is small but sufficient for educational programs.

module CodingAdventures
  module GpuCore
    # Byte-addressable local scratchpad memory with FP-aware load/store.
    #
    # Provides both raw byte access and convenient floating-point operations
    # that handle the conversion between FloatBits and byte sequences.
    #
    # @param size [Integer] Memory size in bytes (default 4096 = 4 KB).
    class LocalMemory
      attr_reader :size

      def initialize(size: 4096)
        if size < 1
          raise ArgumentError, "Memory size must be positive, got #{size}"
        end

        @size = size
        # Ruby doesn't have a built-in bytearray like Python, so we use
        # an Array of integers (each 0-255). This is less memory-efficient
        # but perfectly clear for educational purposes.
        @data = Array.new(size, 0)
      end

      # Validate that an access is within bounds.
      #
      # Every memory access goes through this check first. In real hardware,
      # out-of-bounds memory access causes a segfault or GPU error. We raise
      # IndexError to make debugging easier.
      def check_bounds(address, num_bytes)
        if address < 0 || address + num_bytes > @size
          raise IndexError,
            "Memory access at #{address}:#{address + num_bytes} out of bounds [0, #{@size})"
        end
      end

      # --- Raw byte access ---

      # Read a single byte from memory.
      def read_byte(address)
        check_bounds(address, 1)
        @data[address]
      end

      # Write a single byte to memory.
      def write_byte(address, value)
        check_bounds(address, 1)
        @data[address] = value & 0xFF
      end

      # Read multiple bytes from memory.
      #
      # @param address [Integer] Starting byte address.
      # @param count [Integer] Number of bytes to read.
      # @return [Array<Integer>] Array of byte values.
      def read_bytes(address, count)
        check_bounds(address, count)
        @data[address, count].dup
      end

      # Write multiple bytes to memory.
      #
      # @param address [Integer] Starting byte address.
      # @param data [Array<Integer>] Array of byte values to write.
      def write_bytes(address, data)
        check_bounds(address, data.length)
        data.each_with_index do |byte, i|
          @data[address + i] = byte & 0xFF
        end
      end

      # --- Floating-point access ---

      # How many bytes a float format uses: FP32=4, FP16/BF16=2.
      def float_byte_width(fmt)
        fmt.total_bits / 8
      end

      # Convert a FloatBits to raw bytes (little-endian).
      #
      # The process:
      # 1. Concatenate sign + exponent + mantissa into one integer
      # 2. Pack that integer into bytes
      #
      # Example for FP32 value 1.0:
      #     sign=0, exponent=[0,1,1,1,1,1,1,1], mantissa=[0]*23
      #     -> bit string: 0_01111111_00000000000000000000000
      #     -> integer: 0x3F800000
      #     -> bytes (little-endian): [00, 00, 80, 3F]
      def floatbits_to_bytes(value)
        # Reassemble the bit pattern from FloatBits fields
        bits = value.sign
        value.exponent.each do |b|
          bits = (bits << 1) | b
        end
        value.mantissa.each do |b|
          bits = (bits << 1) | b
        end

        # Pack as bytes using little-endian format
        byte_width = float_byte_width(value.fmt)
        if byte_width == 4
          # Pack as 32-bit unsigned little-endian, then convert to byte array
          [bits].pack("V").bytes
        elsif byte_width == 2
          # Pack as 16-bit unsigned little-endian, then convert to byte array
          [bits].pack("v").bytes
        else
          raise ArgumentError, "Unsupported float width: #{byte_width} bytes"
        end
      end

      # Convert raw bytes (little-endian) back to a FloatBits.
      #
      # Reverses floatbits_to_bytes: unpack integer, split into fields.
      def bytes_to_floatbits(data, fmt)
        byte_width = float_byte_width(fmt)
        if byte_width == 4
          bits = data.pack("C*").unpack1("V")
        elsif byte_width == 2
          bits = data.pack("C*").unpack1("v")
        else
          raise ArgumentError, "Unsupported float width: #{byte_width} bytes"
        end

        total_bits = fmt.total_bits
        mantissa_bits = fmt.mantissa_bits
        exponent_bits = fmt.exponent_bits

        # Mantissa is the lowest mantissa_bits bits
        mantissa_mask = (1 << mantissa_bits) - 1
        mantissa_int = bits & mantissa_mask
        mantissa = Array.new(mantissa_bits) { |i| (mantissa_int >> (mantissa_bits - 1 - i)) & 1 }

        # Exponent is the next exponent_bits bits
        exponent_mask = (1 << exponent_bits) - 1
        exponent_int = (bits >> mantissa_bits) & exponent_mask
        exponent = Array.new(exponent_bits) { |i| (exponent_int >> (exponent_bits - 1 - i)) & 1 }

        # Sign is the highest bit
        sign = (bits >> (total_bits - 1)) & 1

        FpArithmetic::FloatBits.new(sign: sign, exponent: exponent, mantissa: mantissa, fmt: fmt)
      end

      # Load a floating-point value from memory.
      #
      # Reads the appropriate number of bytes (4 for FP32, 2 for FP16/BF16)
      # starting at the given address, and converts them to a FloatBits.
      #
      # @param address [Integer] Byte address to read from.
      # @param fmt [FloatFormat] The floating-point format to interpret the bytes as.
      # @return [FloatBits] A FloatBits value decoded from the bytes at that address.
      def load_float(address, fmt = FpArithmetic::FP32)
        byte_width = float_byte_width(fmt)
        data = read_bytes(address, byte_width)
        bytes_to_floatbits(data, fmt)
      end

      # Store a floating-point value to memory.
      #
      # Converts the FloatBits to bytes and writes them starting at the
      # given address.
      #
      # @param address [Integer] Byte address to write to.
      # @param value [FloatBits] The FloatBits value to store.
      def store_float(address, value)
        data = floatbits_to_bytes(value)
        write_bytes(address, data)
      end

      # Convenience: load a float and convert to Ruby float.
      def load_float_as_ruby(address, fmt = FpArithmetic::FP32)
        FpArithmetic.bits_to_float(load_float(address, fmt))
      end

      # Convenience: store a Ruby float to memory.
      def store_ruby_float(address, value, fmt = FpArithmetic::FP32)
        store_float(address, FpArithmetic.float_to_bits(value, fmt))
      end

      # Return a slice of memory as a list of byte values.
      #
      # Useful for debugging. Default shows the first 64 bytes.
      def dump(start = 0, length = 64)
        end_addr = [start + length, @size].min
        @data[start...end_addr].dup
      end

      # Human-readable representation showing size and usage.
      def to_s
        used = @data.count { |b| b != 0 }
        "LocalMemory(#{@size} bytes, #{used} non-zero)"
      end

      def inspect
        to_s
      end

      private :check_bounds, :float_byte_width, :floatbits_to_bytes, :bytes_to_floatbits
    end
  end
end
