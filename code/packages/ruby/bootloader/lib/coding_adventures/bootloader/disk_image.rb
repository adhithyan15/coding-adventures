# frozen_string_literal: true

module CodingAdventures
  module Bootloader
    DISK_KERNEL_OFFSET    = 0x00080000
    DISK_USER_PROGRAM_BASE = 0x00100000
    DEFAULT_DISK_SIZE     = 2 * 1024 * 1024

    class DiskImage
      def initialize(size_bytes = DEFAULT_DISK_SIZE)
        @data = Array.new(size_bytes, 0)
      end

      def load_kernel(binary) = load_at(DISK_KERNEL_OFFSET, binary)

      def load_user_program(binary, offset) = load_at(offset, binary)

      def load_at(offset, data)
        bytes = data.is_a?(String) ? data.bytes : data
        raise "DiskImage: data exceeds disk size" if offset + bytes.length > @data.length
        bytes.each_with_index { |b, i| @data[offset + i] = b }
      end

      def read_word(offset)
        return 0 if offset < 0 || offset + 4 > @data.length
        @data[offset] | (@data[offset + 1] << 8) | (@data[offset + 2] << 16) | (@data[offset + 3] << 24)
      end

      def read_byte_at(offset)
        return 0 if offset < 0 || offset >= @data.length
        @data[offset]
      end

      def data = @data
      def size = @data.length
    end
  end
end
