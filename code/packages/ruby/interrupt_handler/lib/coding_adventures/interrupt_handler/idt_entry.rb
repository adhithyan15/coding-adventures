# frozen_string_literal: true

module CodingAdventures
  module InterruptHandler
    # One row in the Interrupt Descriptor Table. Maps an interrupt number
    # to the memory address of its handler function (ISR).
    #
    # Attributes:
    #   isr_address:     Where the CPU jumps when this interrupt fires.
    #   present:         true = valid entry. false = unused (triggers double fault).
    #   privilege_level: 0 = kernel only.
    class IDTEntry
      attr_accessor :isr_address, :present, :privilege_level

      # Create a new IDT entry.
      #
      # @param isr_address [Integer] Address of the interrupt service routine
      # @param present [Boolean] True if this entry is valid
      # @param privilege_level [Integer] 0 = kernel only
      def initialize(isr_address: 0, present: false, privilege_level: 0)
        @isr_address = isr_address
        @present = present
        @privilege_level = privilege_level
      end

      # Compare two IDT entries for equality.
      def ==(other)
        other.is_a?(IDTEntry) &&
          isr_address == other.isr_address &&
          present == other.present &&
          privilege_level == other.privilege_level
      end
    end
  end
end
