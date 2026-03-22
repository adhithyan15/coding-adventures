# frozen_string_literal: true

module CodingAdventures
  module InterruptHandler
    # Holds all CPU state needed to resume after an interrupt.
    #
    # When an interrupt fires, the CPU pushes this frame onto the kernel
    # stack before jumping to the ISR. When the ISR returns, the CPU pops
    # the frame and resumes the interrupted code.
    #
    # Layout (136 bytes):
    #   PC (return address)         4 bytes
    #   MStatus register            4 bytes
    #   MCause register             4 bytes
    #   x1-x31 (31 registers)      124 bytes
    #   Total: 34 words = 136 bytes
    #
    # Why save ALL 32 registers? The ISR is arbitrary code -- it might use
    # any register. Saving everything is safe and simple.
    class InterruptFrame
      attr_accessor :pc, :registers, :mstatus, :mcause

      # @param pc [Integer] Saved program counter
      # @param registers [Array<Integer>] All 32 RISC-V registers (x0-x31)
      # @param mstatus [Integer] Machine status register
      # @param mcause [Integer] Interrupt number that caused this
      def initialize(pc: 0, registers: nil, mstatus: 0, mcause: 0)
        @pc = pc
        @registers = registers || Array.new(32, 0)
        @mstatus = mstatus
        @mcause = mcause
      end
    end

    # Create an InterruptFrame from the current CPU state.
    # Called at the beginning of interrupt handling, before the ISR runs.
    #
    # @param registers [Array<Integer>] All 32 general-purpose registers
    # @param pc [Integer] Program counter (next instruction after interrupt)
    # @param mstatus [Integer] Machine status register
    # @param mcause [Integer] Interrupt number
    # @return [InterruptFrame]
    def self.save_context(registers, pc, mstatus, mcause)
      InterruptFrame.new(
        pc: pc,
        registers: registers.dup, # defensive copy
        mstatus: mstatus,
        mcause: mcause
      )
    end

    # Extract CPU state from an InterruptFrame.
    # Called after the ISR completes, to resume the interrupted code.
    #
    # @param frame [InterruptFrame]
    # @return [Array] [registers, pc, mstatus]
    def self.restore_context(frame)
      [frame.registers.dup, frame.pc, frame.mstatus]
    end
  end
end
