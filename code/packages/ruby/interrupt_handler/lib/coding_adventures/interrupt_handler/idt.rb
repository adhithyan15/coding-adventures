# frozen_string_literal: true

module CodingAdventures
  module InterruptHandler
    # Interrupt Descriptor Table — an array of 256 entries stored at address
    # 0x00000000 in memory. Each entry maps an interrupt number to the address
    # of its handler (ISR).
    #
    # Why 256 entries? Matches x86 convention:
    #   0-31:   CPU exceptions
    #   32-47:  Hardware device interrupts
    #   128:    System call (ecall)
    #   Most entries are unused (present = false).
    #
    # Each IDT Entry (8 bytes) in memory:
    #   Bytes 0-3: ISR address (little-endian uint32)
    #   Byte 4:    Present (0x00 or 0x01)
    #   Byte 5:    Privilege level (uint8)
    #   Bytes 6-7: Reserved (0x00, 0x00)
    class IDT
      attr_reader :entries

      # Create a new IDT with all 256 entries marked as not present.
      def initialize
        @entries = Array.new(256) { IDTEntry.new }
      end

      # Install a handler at the given interrupt number (0-255).
      #
      # @param number [Integer] Interrupt number (0-255)
      # @param entry [IDTEntry] The entry to install
      # @raise [ArgumentError] If number is out of range
      def set_entry(number, entry)
        raise ArgumentError, "IDT entry number must be 0-255" if number < 0 || number > 255
        @entries[number] = entry
      end

      # Return the entry for the given interrupt number (0-255).
      #
      # @param number [Integer] Interrupt number (0-255)
      # @return [IDTEntry]
      # @raise [ArgumentError] If number is out of range
      def get_entry(number)
        raise ArgumentError, "IDT entry number must be 0-255" if number < 0 || number > 255
        @entries[number]
      end

      # Serialize the IDT into a byte array at the given base address.
      # Uses little-endian format (RISC-V convention).
      #
      # @param memory [Array<Integer>] Byte array to write into
      # @param base_address [Integer] Starting offset in memory
      def write_to_memory(memory, base_address)
        256.times do |i|
          offset = base_address + i * IDT_ENTRY_SIZE
          entry = @entries[i]

          # Bytes 0-3: ISR address (little-endian)
          addr = entry.isr_address & 0xFFFFFFFF
          memory[offset] = addr & 0xFF
          memory[offset + 1] = (addr >> 8) & 0xFF
          memory[offset + 2] = (addr >> 16) & 0xFF
          memory[offset + 3] = (addr >> 24) & 0xFF

          # Byte 4: Present bit
          memory[offset + 4] = entry.present ? 0x01 : 0x00

          # Byte 5: Privilege level
          memory[offset + 5] = entry.privilege_level & 0xFF

          # Bytes 6-7: Reserved
          memory[offset + 6] = 0x00
          memory[offset + 7] = 0x00
        end
      end

      # Deserialize the IDT from a byte array at the given base address.
      #
      # @param memory [Array<Integer>] Byte array to read from
      # @param base_address [Integer] Starting offset in memory
      def load_from_memory(memory, base_address)
        256.times do |i|
          offset = base_address + i * IDT_ENTRY_SIZE

          # Bytes 0-3: ISR address (little-endian)
          isr_address = memory[offset] |
            (memory[offset + 1] << 8) |
            (memory[offset + 2] << 16) |
            (memory[offset + 3] << 24)

          # Byte 4: Present bit
          present = memory[offset + 4] != 0x00

          # Byte 5: Privilege level
          privilege_level = memory[offset + 5]

          @entries[i] = IDTEntry.new(
            isr_address: isr_address,
            present: present,
            privilege_level: privilege_level
          )
        end
      end
    end
  end
end
