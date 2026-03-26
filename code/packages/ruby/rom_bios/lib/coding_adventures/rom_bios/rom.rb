# frozen_string_literal: true

# === ROM: Read-Only Memory ===
#
# ROM is a memory region where writes are silently ignored. Real computers
# have a ROM chip soldered to the motherboard containing firmware that
# the CPU executes on power-on. The program counter starts at the ROM's
# base address (0xFFFF0000).
#
# Analogy: ROM is like a recipe card laminated in plastic -- you can read
# it any number of times, but you cannot write on it.
#
# Memory map:
#   0xFFFF_FFFF +------------------+
#               |    ROM (64 KB)   |  <- CPU starts here
#   0xFFFF_0000 +------------------+
#               |   Framebuffer    |
#   0xFFFB_0000 +------------------+
#               |       ...        |
#   0x0001_0000 +------------------+
#               |   Bootloader     |  <- BIOS jumps here
#   0x0000_0000 +------------------+
#               |       IDT        |
#               +------------------+

module CodingAdventures
  module RomBios
    # Default base address: top of 32-bit address space minus 64 KB.
    DEFAULT_ROM_BASE = 0xFFFF0000

    # Default ROM size: 64 KB (65536 bytes).
    DEFAULT_ROM_SIZE = 65536

    # Configuration for the ROM memory region.
    ROMConfig = Data.define(:base_address, :size) do
      def initialize(base_address: DEFAULT_ROM_BASE, size: DEFAULT_ROM_SIZE)
        super
      end
    end

    # Read-only memory region. Writes are silently ignored.
    #
    # Example:
    #   bios = BIOSFirmware.new(BIOSConfig.new)
    #   rom = ROM.new(ROMConfig.new, bios.generate)
    #   first_word = rom.read_word(0xFFFF0000)
    #   rom.write(0xFFFF0000, 0xFF)  # silently ignored
    class ROM
      attr_reader :config

      def initialize(config, firmware = nil)
        firmware ||= []
        raise ArgumentError, "firmware larger than ROM size" if firmware.length > config.size

        @config = config
        @data = Array.new(config.size, 0)
        firmware.each_with_index { |byte, i| @data[i] = byte }
      end

      # Read a single byte from the given absolute address.
      # Out-of-range addresses return 0.
      def read(address)
        offset = address_to_offset(address)
        return 0 if offset < 0

        @data[offset]
      end

      # Read a 32-bit little-endian word at the given absolute address.
      def read_word(address)
        offset = address_to_offset(address)
        return 0 if offset < 0 || offset + 3 >= @data.length

        @data[offset] |
          (@data[offset + 1] << 8) |
          (@data[offset + 2] << 16) |
          (@data[offset + 3] << 24)
      end

      # Attempt to write a byte to ROM (silently ignored).
      def write(address, value)
        # ROM is read-only. Silently ignored.
      end

      # Total size of ROM in bytes.
      def size
        @config.size
      end

      # Base address of ROM.
      def base_address
        @config.base_address
      end

      # True if address falls within the ROM region.
      def contains?(address)
        address_to_offset(address) >= 0
      end

      private

      def address_to_offset(address)
        return -1 if address < @config.base_address

        offset = address - @config.base_address
        return -1 if offset >= @config.size

        offset
      end
    end
  end
end
