# frozen_string_literal: true

module CodingAdventures
  module InterruptHandler
    # Manages the full interrupt lifecycle: pending queue, masking,
    # enable/disable, and dispatching to ISRs.
    #
    # The lifecycle:
    #   1. Device calls raise_interrupt(number)
    #   2. Pipeline checks has_pending? between instructions
    #   3. If pending: next_pending returns highest-priority interrupt
    #   4. CPU saves context, looks up IDT, dispatches ISR
    #   5. After ISR: acknowledge removes from pending
    #   6. CPU restores context and resumes
    #
    # Mask Register (32 bits):
    #   Bit N = 1 means interrupt N is masked (blocked), for N in 0..31.
    #   Interrupts 32+ are always unmasked (unless globally disabled).
    #
    # Priority: Lower interrupt number = higher priority.
    class InterruptController
      attr_accessor :idt, :registry, :mask_register, :enabled

      def initialize
        @idt = IDT.new
        @registry = ISRRegistry.new
        @pending = []
        @mask_register = 0 # 32-bit mask, all unmasked
        @enabled = true     # global enable flag
      end

      # Add an interrupt to the pending queue.
      # No duplicates; queue stays sorted ascending.
      #
      # @param number [Integer] Interrupt number to raise
      def raise_interrupt(number)
        return if @pending.include?(number)

        # Insert in sorted order (binary search via bsearch_index)
        index = @pending.bsearch_index { |n| n >= number } || @pending.length
        @pending.insert(index, number)
      end

      # Return true if any unmasked pending interrupts exist and enabled.
      #
      # @return [Boolean]
      def has_pending?
        return false unless @enabled
        @pending.any? { |n| !masked?(n) }
      end

      # Return highest-priority (lowest-numbered) unmasked pending interrupt.
      # Returns -1 if none available or globally disabled.
      #
      # @return [Integer]
      def next_pending
        return -1 unless @enabled
        @pending.each { |n| return n unless masked?(n) }
        -1
      end

      # Remove the given interrupt from the pending queue (EOI).
      #
      # @param number [Integer]
      def acknowledge(number)
        @pending.delete(number)
      end

      # Set or clear the mask for interrupt number (0-31 only).
      # masked=true blocks; masked=false allows.
      #
      # @param number [Integer]
      # @param masked [Boolean]
      def set_mask(number, masked)
        return if number < 0 || number > 31
        if masked
          @mask_register |= (1 << number)
        else
          @mask_register &= ~(1 << number)
        end
      end

      # Return true if the interrupt is currently masked (blocked).
      # Interrupts 32+ are never masked by the mask register.
      #
      # @param number [Integer]
      # @return [Boolean]
      def masked?(number)
        return false if number < 0 || number > 31
        (@mask_register & (1 << number)) != 0
      end

      # Set the global interrupt enable flag.
      def enable
        @enabled = true
      end

      # Clear the global interrupt enable flag.
      def disable
        @enabled = false
      end

      # Return the number of pending interrupts (masked and unmasked).
      #
      # @return [Integer]
      def pending_count
        @pending.length
      end

      # Remove all pending interrupts.
      def clear_all
        @pending.clear
      end
    end
  end
end
