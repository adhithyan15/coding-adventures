# frozen_string_literal: true

# ---------------------------------------------------------------------------
# ExecuteResult -- the outcome of executing a single instruction.
# ---------------------------------------------------------------------------
#
# === Why ExecuteResult? ===
#
# When the ISA executes an instruction, it needs to communicate back to the
# GPUCore what happened. The ExecuteResult carries all of this information:
#
#     1. A human-readable description (for tracing and debugging)
#     2. How to update the program counter (PC)
#     3. Which registers and memory locations changed
#     4. Whether execution should stop (HALT)
#
# This is the bridge between the ISA layer (which knows about opcodes) and
# the core layer (which knows about the fetch-execute loop).
#
# === Ruby Implementation ===
#
# We use Data.define (Ruby 3.2+) to create an immutable value object. This
# mirrors Python's frozen dataclass. The result is created once by the ISA
# and never modified -- it's a snapshot of what happened.

module CodingAdventures
  module GpuCore
    # ExecuteResult -- what an instruction execution produces.
    #
    # Fields:
    #     description:       Human-readable summary, e.g. "R3 = R1 * R2 = 6.0"
    #     next_pc_offset:    How to advance the program counter.
    #                        +1 for most instructions (next instruction).
    #                        Other values for branches/jumps.
    #     absolute_jump:     If true, next_pc_offset is an absolute address,
    #                        not a relative offset.
    #     registers_changed: Hash of register name => new float value.
    #     memory_changed:    Hash of memory address => new float value.
    #     halted:            True if this instruction stops execution.
    ExecuteResult = Data.define(
      :description,
      :next_pc_offset,
      :absolute_jump,
      :registers_changed,
      :memory_changed,
      :halted
    ) do
      # Provide sensible defaults for optional fields.
      #
      # Data.define in Ruby requires all fields to be passed, so we override
      # initialize to provide defaults for the common case (most instructions
      # just advance PC by 1, don't jump, and don't halt).
      def initialize(
        description:,
        next_pc_offset: 1,
        absolute_jump: false,
        registers_changed: nil,
        memory_changed: nil,
        halted: false
      )
        super
      end
    end
  end
end
