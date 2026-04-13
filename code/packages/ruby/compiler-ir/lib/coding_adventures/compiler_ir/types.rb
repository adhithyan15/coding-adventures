# frozen_string_literal: true

# ==========================================================================
# IR Types — Operands, Instructions, Data, Program, ID Generator
# ==========================================================================
#
# This file defines the core data types of the intermediate representation:
#
#   IrRegister   — a virtual register (v0, v1, v2, ...)
#   IrImmediate  — a literal integer value (42, -1, 255)
#   IrLabel      — a named jump target or data reference ("_start", "tape")
#   IrInstruction — a single IR instruction (opcode + operands + ID)
#   IrDataDecl   — a data segment declaration (.data tape 30000 0)
#   IrProgram    — a complete IR program (instructions + data + entry)
#   IDGenerator  — produces unique monotonic instruction IDs
#
# ── Operand design ──────────────────────────────────────────────────────────
#
# All three operand types are frozen value objects (Data.define). They are
# immutable — once created, they never change. This is the Ruby equivalent of
# Go's struct-by-value semantics.
#
# Ruby does not have interfaces, so "operand duck-typing" is used: all three
# types respond to to_s. Code that needs to distinguish types uses is_a?.
#
# ── IrInstruction and IrProgram ─────────────────────────────────────────────
#
# IrInstruction uses a Struct (mutable) so that downstream passes can update
# fields. IrProgram is a class (not a struct) to support add_instruction and
# add_data mutation methods, matching the Go API.
# ==========================================================================

module CodingAdventures
  module CompilerIr
    # ──────────────────────────────────────────────────────────────────────────
    # IrRegister — a virtual register
    #
    # Virtual registers are named v0, v1, v2, ... (the index field).
    # There are infinitely many — the backend's register allocator maps
    # them to physical registers.
    #
    # Example:
    #   IrRegister.new(0).to_s  #=> "v0"
    #   IrRegister.new(5).to_s  #=> "v5"
    # ──────────────────────────────────────────────────────────────────────────
    IrRegister = Data.define(:index) do
      # Returns the canonical text representation of this virtual register.
      # The format is always "v" followed by the index number.
      def to_s
        "v#{index}"
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IrImmediate — a literal integer value
    #
    # Immediates are signed integers that appear directly in instructions.
    # They are printed as plain decimal numbers.
    #
    # Example:
    #   IrImmediate.new(42).to_s   #=> "42"
    #   IrImmediate.new(-1).to_s   #=> "-1"
    #   IrImmediate.new(255).to_s  #=> "255"
    # ──────────────────────────────────────────────────────────────────────────
    IrImmediate = Data.define(:value) do
      # Returns the decimal string representation of the immediate value.
      def to_s
        value.to_s
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IrLabel — a named target for jumps, branches, calls, or data references
    #
    # Labels are strings like "loop_0_start", "_start", "tape", "__trap_oob".
    # They resolve to addresses during code generation.
    #
    # Example:
    #   IrLabel.new("_start").to_s      #=> "_start"
    #   IrLabel.new("loop_0_end").to_s  #=> "loop_0_end"
    # ──────────────────────────────────────────────────────────────────────────
    IrLabel = Data.define(:name) do
      # Returns the label name as-is. Labels are their own string representation.
      def to_s
        name
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IrInstruction — a single IR instruction
    #
    # Every instruction has:
    #   opcode   — what operation to perform (ADD_IMM, BRANCH_Z, etc.)
    #   operands — the arguments (registers, immediates, labels)
    #   id       — a unique monotonic integer for source mapping
    #
    # The id field is the key that connects this instruction to the source
    # map chain. Each instruction gets a unique ID assigned by the IDGenerator,
    # and that ID flows through all pipeline stages.
    #
    # Label instructions have id = -1 because labels produce no machine code
    # and therefore have no meaningful position in the output.
    #
    # Example:
    #   IrInstruction.new(IrOp::ADD_IMM, [IrRegister.new(1), IrRegister.new(1), IrImmediate.new(1)], 3)
    #   # prints as: ADD_IMM v1, v1, 1  ; #3
    # ──────────────────────────────────────────────────────────────────────────
    IrInstruction = Struct.new(:opcode, :operands, :id)

    # ──────────────────────────────────────────────────────────────────────────
    # IrDataDecl — a data segment declaration
    #
    # Declares a named region of memory with a given size and initial byte
    # value. For Brainfuck, this is the tape:
    #
    #   IrDataDecl.new("tape", 30000, 0)
    #     →  .data tape 30000 0
    #
    # The init value is repeated for every byte in the region. init=0 means
    # zero-initialized (equivalent to .bss in most assembly formats).
    # ──────────────────────────────────────────────────────────────────────────
    IrDataDecl = Struct.new(:label, :size, :init)

    # ──────────────────────────────────────────────────────────────────────────
    # IrProgram — a complete IR program
    #
    # An IrProgram contains:
    #   instructions — the linear sequence of IR instructions
    #   data         — data segment declarations (.bss, .data)
    #   entry_label  — the label where execution begins
    #   version      — IR version number (1 = Brainfuck subset)
    #
    # The instructions array is ordered — execution flows from index 0
    # to len-1, with jumps/branches altering the flow.
    #
    # Usage:
    #   program = IrProgram.new("_start")
    #   program.add_instruction(IrInstruction.new(IrOp::HALT, [], 0))
    #   program.instructions  #=> [IrInstruction(...)]
    # ──────────────────────────────────────────────────────────────────────────
    class IrProgram
      attr_accessor :instructions, :data, :entry_label, :version

      # Creates a new IR program with the given entry label and version 1.
      #
      # @param entry_label [String] the label where execution begins
      def initialize(entry_label)
        @entry_label = entry_label
        @version = 1
        @instructions = []
        @data = []
      end

      # Appends an instruction to the program.
      #
      # @param instr [IrInstruction] the instruction to append
      def add_instruction(instr)
        @instructions << instr
      end

      # Appends a data declaration to the program.
      #
      # @param decl [IrDataDecl] the data declaration to append
      def add_data(decl)
        @data << decl
      end
    end

    # ──────────────────────────────────────────────────────────────────────────
    # IDGenerator — produces unique monotonic instruction IDs
    #
    # Every IR instruction in the pipeline needs a unique ID for source
    # mapping. The IDGenerator ensures no two instructions ever share an ID,
    # even across multiple compiler invocations within the same process.
    #
    # The generator starts at 0 by default, but can be initialised to any
    # starting value so that multiple compilers can contribute instructions
    # to the same program without ID collisions.
    #
    # Usage:
    #   gen = IDGenerator.new
    #   id1 = gen.next  # 0
    #   id2 = gen.next  # 1
    #   id3 = gen.next  # 2
    #   gen.current     # 3  (next value to be returned)
    # ──────────────────────────────────────────────────────────────────────────
    class IDGenerator
      # Creates a new ID generator starting at zero.
      def initialize(start = 0)
        @next = start
      end

      # Returns the next unique ID and increments the counter.
      #
      # @return [Integer] the next unique ID
      def next
        id = @next
        @next += 1
        id
      end

      # Returns the current counter value without incrementing.
      # This is the ID that will be returned by the next call to next().
      #
      # @return [Integer] the next ID to be assigned
      def current
        @next
      end
    end
  end
end
