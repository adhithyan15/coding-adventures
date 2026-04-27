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

    # ====================================================================
    # TypedVMValue -- A Value Tagged with Its Type
    # ====================================================================
    #
    # Some execution environments (like WebAssembly) need every stack value
    # to carry a type tag alongside its payload. An i32(42) and an f64(42.0)
    # are *different* values even though the numeric payload looks similar.
    #
    # TypedVMValue is a simple struct with two fields:
    #
    #   +-------+---------------------------------------------------------+
    #   | Field | Description                                             |
    #   +-------+---------------------------------------------------------+
    #   | type  | Integer type code (e.g. 0x7F for i32, 0x7C for f64)    |
    #   | value | The raw Ruby payload (Integer, Float, String, etc.)     |
    #   +-------+---------------------------------------------------------+
    #
    # The GenericVM's typed_stack stores these. The push_typed/pop_typed
    # methods work with TypedVMValues instead of raw Ruby values.
    #
    TypedVMValue = Struct.new(:type, :value)

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
    # == Context-Aware Execution
    #
    # Some execution environments (like WASM) need to pass extra context
    # (memory, tables, globals, labels) to instruction handlers. Use
    # register_context_opcode and execute_with_context for this:
    #
    #   vm.register_context_opcode(0x41, ->(vm, instr, code, ctx) {
    #     vm.push_typed(TypedVMValue.new(0x7F, instr.operand))
    #     vm.advance_pc
    #   })
    #
    #   vm.execute_with_context(code_object, wasm_context)
    #
    class GenericVM
      attr_accessor :stack, :variables, :locals, :pc, :halted, :output, :call_stack,
                    :typed_stack, :execution_context

      # Pre- and post-instruction hooks. Set these to callables (Procs/lambdas)
      # that will be invoked before and after every instruction dispatch.
      #
      # The pre-hook receives (vm, instruction, code) and can modify state
      # before the handler runs (e.g., decoding variable-length bytecodes).
      #
      # The post-hook receives (vm, instruction, code) and can perform
      # cleanup or bookkeeping after the handler runs.
      attr_accessor :pre_instruction_hook, :post_instruction_hook

      def initialize
        # -- Execution state ------------------------------------------------
        @stack = []
        @variables = {}
        @locals = []
        @pc = 0
        @halted = false
        @output = []
        @call_stack = []

        # -- Typed execution state (for WASM-style typed values) -----------
        @typed_stack = []
        @execution_context = nil

        # -- Plugin registries ----------------------------------------------
        @handlers = {}
        @context_handlers = {}
        @builtins = {}

        # -- Hooks -----------------------------------------------------------
        @pre_instruction_hook = nil
        @post_instruction_hook = nil

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

      # Register a context-aware handler for an opcode.
      #
      # Context handlers receive four arguments: (vm, instruction, code, context).
      # The extra +context+ argument carries environment-specific state (e.g.,
      # WASM's linear memory, tables, globals, and labels).
      #
      # Context handlers are only invoked during execute_with_context; during
      # regular execute, the normal handler is used instead.
      #
      #   vm.register_context_opcode(0x41, ->(vm, instr, code, ctx) {
      #     vm.push_typed(TypedVMValue.new(0x7F, instr.operand))
      #     vm.advance_pc
      #   })
      #
      def register_context_opcode(opcode, handler)
        @context_handlers[opcode] = handler
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
      # Typed Stack Operations
      # ====================================================================
      #
      # The typed stack stores TypedVMValue instances -- values paired with
      # their type tag. This is used by execution environments (like WASM)
      # where every value on the stack must carry type information.
      #
      # The typed stack is separate from the untyped stack so that existing
      # language plugins (Starlark, Brainfuck) continue to work unchanged.

      # Push a TypedVMValue onto the typed stack.
      def push_typed(typed_value)
        @typed_stack.push(typed_value)
      end

      # Pop and return the top TypedVMValue from the typed stack.
      def pop_typed
        raise StackUnderflowError, "Cannot pop from an empty typed stack" if @typed_stack.empty?

        @typed_stack.pop
      end

      # Return the top TypedVMValue without removing it.
      def peek_typed
        raise StackUnderflowError, "Cannot peek at an empty typed stack" if @typed_stack.empty?

        @typed_stack.last
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
      #
      # When a context is active (set via execute_with_context), context
      # handlers are preferred over regular handlers, and hooks are called.
      def step(code)
        instruction = code.instructions[@pc]
        pc_before = @pc
        stack_before = @stack.dup

        # -- Pre-instruction hook --
        @pre_instruction_hook&.call(self, instruction, code)

        # -- Decode & Execute --
        # During context execution, prefer context handlers; fall back to
        # regular handlers for opcodes that don't need context awareness.
        handler = if @execution_context
                    @context_handlers[instruction.opcode] || @handlers[instruction.opcode]
                  else
                    @handlers[instruction.opcode]
                  end

        if handler.nil?
          raise InvalidOpcodeError,
                "Unknown opcode: 0x#{instruction.opcode.to_s(16).rjust(2, "0")}. " \
                "No handler registered."
        end

        # Context handlers receive the execution context as a fourth argument.
        output_value = if @execution_context && @context_handlers.key?(instruction.opcode)
                         handler.call(self, instruction, code, @execution_context)
                       else
                         handler.call(self, instruction, code)
                       end

        # -- Post-instruction hook --
        @post_instruction_hook&.call(self, instruction, code)

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

      # Execute a CodeObject with an execution context.
      #
      # This is the entry point for context-aware execution. The context
      # is stored on the VM and passed to every context handler as the
      # fourth argument.
      #
      # Use this for WASM execution where handlers need access to memory,
      # tables, globals, and labels.
      #
      #   vm.execute_with_context(code_object, wasm_context)
      #
      def execute_with_context(code, context)
        @execution_context = context

        while !@halted && @pc < code.instructions.length
          step(code)
        end
      ensure
        @execution_context = nil
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
        @typed_stack = []
        @execution_context = nil
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
