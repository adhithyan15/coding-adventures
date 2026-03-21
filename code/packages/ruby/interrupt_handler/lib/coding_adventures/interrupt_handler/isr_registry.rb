# frozen_string_literal: true

module CodingAdventures
  module InterruptHandler
    # Maps interrupt numbers to Ruby handler callables.
    #
    # This is the "software side" of interrupt handling: the IDT maps
    # interrupt numbers to memory addresses (hardware simulation), while
    # the ISR Registry maps them to Ruby procs/lambdas (emulation).
    #
    # Why both? In a real CPU, the IDT entry's ISR address points to machine
    # code in memory. In our emulator, we map the same interrupt number to
    # a Ruby callable.
    class ISRRegistry
      def initialize
        @handlers = {}
      end

      # Install a handler for the given interrupt number.
      # Overwrites any previously registered handler.
      #
      # @param interrupt_number [Integer]
      # @param handler [Proc] Called with (frame, kernel) when dispatched
      def register(interrupt_number, handler = nil, &block)
        @handlers[interrupt_number] = handler || block
      end

      # Call the registered handler for the given interrupt number.
      #
      # @param interrupt_number [Integer]
      # @param frame [InterruptFrame] Saved CPU state
      # @param kernel [Object] Opaque kernel handle
      # @raise [KeyError] If no handler is registered (double fault)
      def dispatch(interrupt_number, frame, kernel)
        handler = @handlers[interrupt_number]
        raise KeyError, "No ISR handler registered for interrupt #{interrupt_number}" unless handler
        handler.call(frame, kernel)
      end

      # Return true if a handler is registered for this interrupt number.
      #
      # @param interrupt_number [Integer]
      # @return [Boolean]
      def has_handler?(interrupt_number)
        @handlers.key?(interrupt_number)
      end
    end
  end
end
