# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Execution traces -- making every instruction's journey visible.
# ---------------------------------------------------------------------------
#
# === Why Traces? ===
#
# A key principle of this project is educational transparency: every operation
# should be observable. When a GPU core executes an instruction, the trace
# records exactly what happened:
#
#     Cycle 3 | PC=2 | FFMA R3, R0, R1, R2
#     -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
#     -> Registers changed: {R3: 7.0}
#     -> Next PC: 3
#
# This lets a student (or debugger) follow the execution step by step,
# understanding not just *what* the GPU did but *why* -- which registers were
# read, what computation was performed, and what state changed.
#
# === Trace vs Log ===
#
# A trace is more structured than a log message. Each field is typed and
# accessible programmatically, which enables:
# - Automated testing (assert trace.registers_changed == {"R3" => 7.0})
# - Visualization tools (render execution as a timeline)
# - Performance analysis (count cycles, track register usage)

module CodingAdventures
  module GpuCore
    # A record of one instruction's execution on a GPU core.
    #
    # Every call to GPUCore#step returns one of these, providing full
    # visibility into what the instruction did.
    #
    # Fields:
    #     cycle:             The clock cycle number (1-indexed).
    #     pc:                The program counter BEFORE this instruction executed.
    #     instruction:       The instruction that was executed.
    #     description:       Human-readable description of what happened.
    #     next_pc:           The program counter AFTER this instruction.
    #     halted:            True if this instruction stopped execution.
    #     registers_changed: Which registers changed and their new values.
    #     memory_changed:    Which memory addresses changed and their new values.
    GPUCoreTrace = Data.define(
      :cycle,
      :pc,
      :instruction,
      :description,
      :next_pc,
      :halted,
      :registers_changed,
      :memory_changed
    ) do
      def initialize(
        cycle:,
        pc:,
        instruction:,
        description:,
        next_pc:,
        halted: false,
        registers_changed: {},
        memory_changed: {}
      )
        super
      end

      # Pretty-print this trace record for educational display.
      #
      # Returns a multi-line string like:
      #
      #     [Cycle 3] PC=2: FFMA R3, R0, R1, R2
      #       -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
      #       -> Registers: {R3: 7.0}
      #       -> Next PC: 3
      def format
        lines = ["[Cycle #{cycle}] PC=#{pc}: #{instruction}"]
        lines << "  -> #{description}"

        unless registers_changed.empty?
          regs = registers_changed.map { |k, v| "#{k}=#{v}" }.join(", ")
          lines << "  -> Registers: {#{regs}}"
        end

        unless memory_changed.empty?
          mems = memory_changed.map { |k, v| "0x#{k.to_s(16).rjust(4, "0").upcase}=#{v}" }.join(", ")
          lines << "  -> Memory: {#{mems}}"
        end

        if halted
          lines << "  -> HALTED"
        else
          lines << "  -> Next PC: #{next_pc}"
        end

        lines.join("\n")
      end
    end
  end
end
