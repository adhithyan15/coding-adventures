# frozen_string_literal: true

# === HardwareInfo: Boot Protocol Structure ===
#
# The BIOS writes this struct at address 0x00001000 after initialization.
# The bootloader and kernel read it to discover hardware configuration.
#
# Memory layout (28 bytes, all little-endian uint32):
#   Offset  Field              Default
#   0x00    memory_size        (probed)
#   0x04    display_columns    80
#   0x08    display_rows       25
#   0x0C    framebuffer_base   0xFFFB0000
#   0x10    idt_base           0x00000000
#   0x14    idt_entries        256
#   0x18    bootloader_entry   0x00010000

module CodingAdventures
  module RomBios
    # Fixed address where BIOS writes HardwareInfo.
    HARDWARE_INFO_ADDRESS = 0x00001000

    # Size of HardwareInfo: 7 fields * 4 bytes = 28 bytes.
    HARDWARE_INFO_SIZE = 28

    # Hardware configuration discovered and set by the BIOS.
    HardwareInfo = Data.define(
      :memory_size,
      :display_columns,
      :display_rows,
      :framebuffer_base,
      :idt_base,
      :idt_entries,
      :bootloader_entry
    ) do
      def initialize(
        memory_size: 0,
        display_columns: 80,
        display_rows: 25,
        framebuffer_base: 0xFFFB0000,
        idt_base: 0x00000000,
        idt_entries: 256,
        bootloader_entry: 0x00010000
      )
        super
      end

      # Serialize to 28-byte little-endian array.
      def to_bytes
        [memory_size, display_columns, display_rows,
          framebuffer_base, idt_base, idt_entries,
          bootloader_entry].pack("V7").bytes
      end

      # Deserialize from a byte array.
      def self.from_bytes(data)
        raise ArgumentError, "data too short" if data.length < HARDWARE_INFO_SIZE

        fields = data[0, HARDWARE_INFO_SIZE].pack("C*").unpack("V7")
        new(
          memory_size: fields[0],
          display_columns: fields[1],
          display_rows: fields[2],
          framebuffer_base: fields[3],
          idt_base: fields[4],
          idt_entries: fields[5],
          bootloader_entry: fields[6]
        )
      end
    end
  end
end
