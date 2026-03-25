# frozen_string_literal: true

# ==========================================================================
# Generic Virtual Machine -- A Pluggable Stack-Based Bytecode Interpreter
# ==========================================================================
#
# The original VM class (in vm.rb) has a hardcoded dispatch table. Every
# opcode is baked into a big case statement. Want to add a new opcode?
# Edit the VM source code.
#
# GenericVM solves this: languages register their opcodes via
# register_opcode(number, handler). The eval loop dispatches to the
# registered handler. If no handler exists, it raises an error.
#
# Think of it like a car:
#   - The chassis (frame, wheels, brakes) = GenericVM
#   - The engine (gas, electric, hybrid)  = language-specific plugin
#
# Starlark plugs in 50+ opcodes. Brainfuck plugs in 8. Both run on
# the same chassis.
# ==========================================================================

module CodingAdventures
  module VirtualMachine
    # A built-in function callable from bytecode.
    BuiltinFunction = Data.define(:name, :implementation)

    # A pluggable stack-based bytecode interpreter.
    #
    # Languages register their opcodes via register_opcode() and their
    # built-in functions via register_builtin(). The GenericVM provides
    # universal execution primitives: stack, variables, locals, call stack,
    # program counter, and the fetch-decode-execute loop.
    #
    # == Usage
    #
    #   vm = GenericVM.new
    #   vm.register_opcode(0x01, ->(vm, instr, code) { ... })
    #   vm.register_opcode(0xFF, ->(vm, instr, code) { vm.halted = true })
    #   traces = vm.execute(code_object)
    #
    class GenericVM
      attr_accessor :stack, :variables, :locals, :pc, :halted, :output, :call_stack

      def initialize
        # -- Execution state ------------------------------------------------
        @stack = []
        @variables = {}
        @locals = []
        @pc = 0
        @halted = false
        @output = []
        @call_stack = []

        # -- Plugin registries ----------------------------------------------
        @handlers = {}
        @builtins = {}

        # -- Configuration --------------------------------------------------
        @max_recursion_depth = nil
        @frozen = false
      end

      # ====================================================================
      # Plugin Registration
      # ====================================================================

      # Register a handler for an opcode number.
      #
      # The handler must be a callable (Proc, lambda, or object with #call)
      # that accepts (vm, instruction, code) and returns a String or nil.
      #
      #   vm.register_opcode(0x20, method(:handle_add))
      #   vm.register_opcode(0xFF, ->(vm, instr, code) { vm.halted = true })
      #
      def register_opcode(opcode, handler)
        @handlers[opcode] = handler
      end

      # Register a built-in function by name.
      def register_builtin(name, implementation)
        @builtins[name] = BuiltinFunction.new(name: name, implementation: implementation)
      end

      # Look up a built-in function by name. Returns nil if not found.
      def get_builtin(name)
        @builtins[name]
      end

      # ====================================================================
      # Global Injection
      # ====================================================================

      # Pre-seed named variables into the VM's global scope.
      #
      # These variables are available to the program as regular variables
      # but are set before execution begins. Useful for build context,
      # environment info, etc.
      #
      # Injected globals are merged into +variables+ — they don't replace
      # the hash. If a key already exists, the injected value overwrites it.
      #
      # @param globals [Hash] a mapping of variable names to values
      #
      # @example
      #   vm.inject_globals("_ctx" => {"os" => "darwin", "arch" => "arm64"})
      def inject_globals(globals)
        globals.each { |key, value| @variables[key] = value }
      end

      # ====================================================================
      # Configuration
      # ====================================================================

      # Set the maximum call stack depth. nil means unlimited.
      # 0 means no function calls at all (Starlark's recursion restriction).
      def set_max_recursion_depth(depth)
        @max_recursion_depth = depth
      end

      # Set whether the VM is in frozen mode.
      def set_frozen(frozen)
        @frozen = frozen
      end

      def frozen?
        @frozen
      end

      def max_recursion_depth
        @max_recursion_depth
      end

      # ====================================================================
      # Stack Operations
      # ====================================================================

      # Push a value onto the operand stack.
      def push(value)
        @stack.push(value)
      end

      # Pop and return the top value from the operand stack.
      def pop
        raise StackUnderflowError, "Cannot pop from an empty stack" if @stack.empty?

        @stack.pop
      end

      # Return the top value without removing it.
      def peek
        raise StackUnderflowError, "Cannot peek at an empty stack" if @stack.empty?

        @stack.last
      end

      # ====================================================================
      # Call Stack Operations
      # ====================================================================

      # Push a call frame onto the call stack.
      def push_frame(frame)
        if @max_recursion_depth && @call_stack.length >= @max_recursion_depth
          raise MaxRecursionError,
                "Maximum recursion depth exceeded (limit: #{@max_recursion_depth})"
        end

        @call_stack.push(frame)
      end

      # Pop and return the top call frame.
      def pop_frame
        raise VMError, "Cannot return -- call stack is empty" if @call_stack.empty?

        @call_stack.pop
      end

      # ====================================================================
      # Program Counter Operations
      # ====================================================================

      # Advance the program counter by one instruction.
      def advance_pc
        @pc += 1
      end

      # Set the program counter to a specific instruction index.
      def jump_to(target)
        @pc = target
      end

      # ====================================================================
      # Execution Engine
      # ====================================================================

      # Execute a CodeObject using the registered opcode handlers.
      #
      # Returns an array of VMTrace entries, one per instruction executed.
      def execute(code)
        traces = []

        while !@halted && @pc < code.instructions.length
          traces << step(code)
        end

        traces
      end

      # Execute one instruction and return a VMTrace.
      def step(code)
        instruction = code.instructions[@pc]
        pc_before = @pc
        stack_before = @stack.dup

        # -- Decode & Execute --
        handler = @handlers[instruction.opcode]
        if handler.nil?
          raise InvalidOpcodeError,
                "Unknown opcode: 0x#{instruction.opcode.to_s(16).rjust(2, "0")}. " \
                "No handler registered."
        end

        output_value = handler.call(self, instruction, code)

        # -- Build trace --
        VMTrace.new(
          pc: pc_before,
          instruction: instruction,
          stack_before: stack_before,
          stack_after: @stack.dup,
          variables: @variables.dup,
          output: output_value,
          description: describe_step(instruction, code, stack_before)
        )
      end

      # Reset the VM to its initial state, preserving registered handlers.
      def reset
        @stack = []
        @variables = {}
        @locals = []
        @pc = 0
        @halted = false
        @output = []
        @call_stack = []
        @frozen = false
      end

      private

      def describe_step(instruction, _code, _stack_before)
        op_name = "0x#{instruction.opcode.to_s(16).rjust(2, "0")}"
        if instruction.operand
          "Execute #{op_name} with operand #{instruction.operand}"
        else
          "Execute #{op_name}"
        end
      end
    end
  end
end
