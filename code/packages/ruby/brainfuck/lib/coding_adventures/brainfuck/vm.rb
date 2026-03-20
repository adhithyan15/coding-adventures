# frozen_string_literal: true

# ==========================================================================
# Brainfuck VM Factory — Plugging Brainfuck Into the GenericVM
# ==========================================================================
#
# The Factory Pattern
# ==========================================================================
#
# This module provides create_brainfuck_vm — a factory function that
# creates a GenericVM fully configured for Brainfuck. It:
#
# 1. Creates a fresh GenericVM instance.
# 2. Attaches Brainfuck-specific state (tape, data pointer, input buffer).
# 3. Registers all 9 opcode handlers.
#
# The result is a GenericVM that speaks Brainfuck — same execution engine
# as Starlark, different language semantics.
#
# ==========================================================================
# Convenience Executor
# ==========================================================================
#
# For simple use cases, execute_brainfuck wraps the full pipeline:
#
#   result = execute_brainfuck("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.")
#
# This translates the source, creates a VM, and executes in one call.
# ==========================================================================

module CodingAdventures
  module Brainfuck
    # The result of executing a Brainfuck program.
    #
    # @!attribute output [String] the program's output (all . commands concatenated)
    # @!attribute tape [Array<Integer>] the final state of the tape
    # @!attribute dp [Integer] the final data pointer position
    # @!attribute traces [Array<VMTrace>] step-by-step execution traces
    # @!attribute steps [Integer] total number of instructions executed
    BrainfuckResult = Data.define(:output, :tape, :dp, :traces, :steps)

    # Create a GenericVM configured for Brainfuck execution.
    #
    # This is the factory function that wires up Brainfuck's handlers and
    # state. The returned VM is ready to execute any Brainfuck CodeObject.
    #
    # @param input_data [String] input to feed to , commands (default: "")
    # @return [CodingAdventures::VirtualMachine::GenericVM]
    #
    # @example
    #   code = CodingAdventures::Brainfuck.translate("+++.")
    #   vm = CodingAdventures::Brainfuck.create_brainfuck_vm
    #   traces = vm.execute(code)
    #   vm.output.join  #=> "\x03"
    #
    def self.create_brainfuck_vm(input_data: "")
      vm = VirtualMachine::GenericVM.new

      # -- Attach Brainfuck-specific state ----------------------------------
      # Ruby's open classes let us define singleton methods and instance
      # variables on any object. The handlers read and write these to
      # implement Brainfuck semantics.
      vm.define_singleton_method(:tape) { @tape }
      vm.define_singleton_method(:tape=) { |v| @tape = v }
      vm.define_singleton_method(:dp) { @dp }
      vm.define_singleton_method(:dp=) { |v| @dp = v }
      vm.define_singleton_method(:input_buffer) { @input_buffer }
      vm.define_singleton_method(:input_buffer=) { |v| @input_buffer = v }
      vm.define_singleton_method(:input_pos) { @input_pos }
      vm.define_singleton_method(:input_pos=) { |v| @input_pos = v }

      vm.tape = Array.new(TAPE_SIZE, 0)
      vm.dp = 0
      vm.input_buffer = input_data
      vm.input_pos = 0

      # -- Register all opcode handlers -------------------------------------
      HANDLERS.each do |opcode, handler|
        vm.register_opcode(opcode, handler)
      end

      vm
    end

    # Translate and execute a Brainfuck program in one call.
    #
    # This is the convenience function for quick execution. It handles
    # the full pipeline: source → translate → create VM → execute → result.
    #
    # @param source [String] the Brainfuck source code
    # @param input_data [String] input bytes for , commands
    # @return [BrainfuckResult]
    #
    # @example Addition (2 + 5 = 7)
    #   result = CodingAdventures::Brainfuck.execute_brainfuck("++>+++++[<+>-]")
    #   result.tape[0]  #=> 7
    #
    # @example Hello character (ASCII 72 = 'H')
    #   result = CodingAdventures::Brainfuck.execute_brainfuck("+++++++++[>++++++++<-]>.")
    #   result.output  #=> "H"
    #
    def self.execute_brainfuck(source, input_data: "")
      code = translate(source)
      vm = create_brainfuck_vm(input_data: input_data)
      traces = vm.execute(code)

      BrainfuckResult.new(
        output: vm.output.join,
        tape: vm.tape.dup,
        dp: vm.dp,
        traces: traces,
        steps: traces.length
      )
    end
  end
end
