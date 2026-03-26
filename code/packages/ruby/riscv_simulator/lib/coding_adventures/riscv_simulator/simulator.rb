# frozen_string_literal: true

# === RISC-V RV32I Simulator with M-mode privileged extensions ===
#
# Implements all 37 RV32I instructions plus M-mode CSR access, trap handling,
# and mret. Uses the cpu-simulator package for the generic fetch-decode-execute
# pipeline.
#
# Architecture:
#   opcodes.rb   -- opcode and funct3/funct7 constants
#   decode.rb    -- instruction decoder for all six formats (R/I/S/B/U/J)
#   execute.rb   -- instruction executor for all operations
#   csr.rb       -- Control and Status Register file for M-mode
#   encoding.rb  -- helpers to construct machine code for testing
#   simulator.rb -- top-level simulator struct and factory

require "coding_adventures_cpu_simulator"

module CodingAdventures
  module RiscvSimulator
    class RiscVSimulator
      attr_reader :cpu, :csr

      def initialize(memory_size: 65536, memory: nil)
        @decoder = RiscVDecoder.new
        @csr = CSRFile.new
        @executor = RiscVExecutor.new(csr: @csr)
        @cpu = CpuSimulator::CPU.new(
          decoder: @decoder, executor: @executor,
          num_registers: 32, bit_width: 32, memory_size: memory_size,
          memory: memory
        )
      end

      def run(program)
        @cpu.load_program(program)
        @cpu.run
      end

      def step
        @cpu.step
      end
    end
  end
end
