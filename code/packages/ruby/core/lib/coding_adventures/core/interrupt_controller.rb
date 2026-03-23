# frozen_string_literal: true

# InterruptController -- routes interrupts to cores.
#
# = What are Interrupts?
#
# An interrupt is a signal that temporarily diverts the CPU from its current
# work to handle an urgent event. Examples:
#
#   - Timer interrupt: "100ms have passed, let the OS scheduler run"
#   - I/O interrupt: "keyboard key was pressed" or "network packet arrived"
#   - Inter-processor interrupt (IPI): "Core 0 needs Core 1 to flush its TLB"
#   - Software interrupt: "this program wants to make a system call"
#
# = How the Controller Works
#
# The interrupt controller is the traffic cop for interrupts:
#
#  1. An external device (or another core) raises an interrupt.
#  2. The controller queues it and decides which core should handle it.
#  3. On the next cycle, the controller signals the target core.
#  4. The core acknowledges the interrupt and begins handling it.

module CodingAdventures
  module Core
    # Represents an interrupt waiting to be delivered.
    PendingInterrupt = Data.define(:interrupt_id, :target_core)

    # Records a core acknowledging an interrupt.
    AcknowledgedInterrupt = Data.define(:core_id, :interrupt_id)

    class InterruptController
      # Creates an interrupt controller for the given number of cores.
      #
      # @param num_cores [Integer] total number of cores in the system.
      def initialize(num_cores)
        @num_cores = num_cores
        @pending = []
        @acknowledged = []
      end

      # Queues an interrupt for delivery.
      #
      # If target_core is -1, the interrupt will be routed to core 0 (simplest
      # routing policy).
      #
      # @param interrupt_id [Integer] identifies the interrupt source.
      # @param target_core [Integer] which core should handle it (-1 = any).
      def raise_interrupt(interrupt_id, target_core)
        target_core = 0 if target_core == -1
        target_core = 0 if target_core >= @num_cores
        @pending << PendingInterrupt.new(
          interrupt_id: interrupt_id,
          target_core: target_core
        )
      end

      # Records that a core has begun handling an interrupt.
      #
      # @param core_id [Integer] which core acknowledged.
      # @param interrupt_id [Integer] which interrupt was acknowledged.
      def acknowledge(core_id, interrupt_id)
        @acknowledged << AcknowledgedInterrupt.new(
          core_id: core_id,
          interrupt_id: interrupt_id
        )

        # Remove from pending.
        removed = false
        @pending = @pending.reject do |p|
          if !removed && p.interrupt_id == interrupt_id && p.target_core == core_id
            removed = true
            true
          else
            false
          end
        end
      end

      # Returns all pending interrupts targeted at a specific core.
      #
      # @param core_id [Integer] which core to check.
      # @return [Array<PendingInterrupt>] pending interrupts for this core.
      def pending_for_core(core_id)
        @pending.select { |p| p.target_core == core_id }
      end

      # Returns the total number of pending (unacknowledged) interrupts.
      #
      # @return [Integer] pending count.
      def pending_count
        @pending.length
      end

      # Returns the total number of acknowledged interrupts.
      #
      # @return [Integer] acknowledged count.
      def acknowledged_count
        @acknowledged.length
      end

      # Clears all pending and acknowledged interrupts.
      def reset
        @pending = []
        @acknowledged = []
      end
    end
  end
end
