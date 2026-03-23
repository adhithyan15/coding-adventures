# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    PROCESS_READY      = 0
    PROCESS_RUNNING    = 1
    PROCESS_BLOCKED    = 2
    PROCESS_TERMINATED = 3

    PROCESS_STATE_NAMES = {0 => "Ready", 1 => "Running", 2 => "Blocked", 3 => "Terminated"}.freeze

    class ProcessControlBlock
      attr_accessor :pid, :state, :saved_registers, :saved_pc, :stack_pointer,
        :memory_base, :memory_size, :name, :exit_code

      def initialize(pid: 0, state: PROCESS_READY, name: "", memory_base: 0, memory_size: 0)
        @pid = pid
        @state = state
        @saved_registers = Array.new(32, 0)
        @saved_pc = memory_base
        @stack_pointer = memory_base + memory_size - 16
        @memory_base = memory_base
        @memory_size = memory_size
        @name = name
        @exit_code = 0
      end
    end

    ProcessInfo = Data.define(:pid, :name, :state, :pc) do
      def initialize(pid: 0, name: "", state: PROCESS_READY, pc: 0) = super
    end
  end
end
