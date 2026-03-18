# frozen_string_literal: true

# ==========================================================================
# VM -- The Stack-Based Bytecode Interpreter
# ==========================================================================
#
# The heart of the computing stack. Takes a CodeObject (compiled bytecode)
# and executes it instruction by instruction, maintaining:
#
#   stack      -- the operand stack where all computation happens
#   variables  -- named variable storage (like global scope)
#   locals     -- indexed local variable slots (like function scope)
#   pc         -- the program counter pointing to the current instruction
#   call_stack -- saved contexts for function calls
#   output     -- captured print output
#
# The fetch-decode-execute cycle:
#   1. Fetch  -- read instruction at pc
#   2. Decode -- look at opcode
#   3. Execute -- perform operation
#   4. Advance -- move pc (unless we jumped)
#   5. Repeat
# ==========================================================================

module CodingAdventures
  module VirtualMachine
    class VM
      attr_reader :stack, :variables, :locals, :pc, :output, :call_stack
      attr_accessor :halted

      def initialize
        reset
      end

      # Reset the VM to its initial state.
      def reset
        @stack = []
        @variables = {}
        @locals = []
        @pc = 0
        @halted = false
        @output = []
        @call_stack = []
      end

      # Execute a complete CodeObject, returning a trace of every step.
      def execute(code)
        traces = []
        while !@halted && @pc < code.instructions.length
          traces << step(code)
        end
        traces
      end

      # Execute one instruction and return a trace of what happened.
      def step(code)
        instruction = code.instructions[@pc]
        pc_before = @pc
        stack_before = @stack.dup

        output_value = dispatch(instruction, code)

        description = describe(instruction, code, stack_before)
        VMTrace.new(
          pc: pc_before,
          instruction: instruction,
          stack_before: stack_before,
          stack_after: @stack.dup,
          variables: @variables.dup,
          output: output_value,
          description: description
        )
      end

      private

      # The big dispatch -- decode and execute a single instruction.
      def dispatch(instruction, code)
        output_value = nil

        case instruction.opcode

        # == Stack Operations ==
        when OpCode::LOAD_CONST
          index = require_operand(instruction)
          validate_index(index, code.constants.length, "LOAD_CONST", "constants pool")
          @stack.push(code.constants[index])
          @pc += 1

        when OpCode::POP
          do_pop
          @pc += 1

        when OpCode::DUP
          raise StackUnderflowError, "DUP requires at least one value on the stack" if @stack.empty?
          @stack.push(@stack.last)
          @pc += 1

        # == Variable Operations ==
        when OpCode::STORE_NAME
          index = require_operand(instruction)
          validate_index(index, code.names.length, "STORE_NAME", "names pool")
          name = code.names[index]
          value = do_pop
          @variables[name] = value
          @pc += 1

        when OpCode::LOAD_NAME
          index = require_operand(instruction)
          validate_index(index, code.names.length, "LOAD_NAME", "names pool")
          name = code.names[index]
          unless @variables.key?(name)
            raise UndefinedNameError, "Variable '#{name}' is not defined"
          end
          @stack.push(@variables[name])
          @pc += 1

        when OpCode::STORE_LOCAL
          index = require_operand(instruction)
          unless index.is_a?(Integer) && index >= 0
            raise InvalidOperandError, "STORE_LOCAL operand must be a non-negative integer, got #{index.inspect}"
          end
          value = do_pop
          @locals.fill(nil, @locals.length..index) if @locals.length <= index
          @locals[index] = value
          @pc += 1

        when OpCode::LOAD_LOCAL
          index = require_operand(instruction)
          unless index.is_a?(Integer) && index >= 0
            raise InvalidOperandError, "LOAD_LOCAL operand must be a non-negative integer, got #{index.inspect}"
          end
          if index >= @locals.length
            raise InvalidOperandError,
                  "LOAD_LOCAL slot #{index} has not been initialized (only #{@locals.length} slots exist)"
          end
          @stack.push(@locals[index])
          @pc += 1

        # == Arithmetic ==
        when OpCode::ADD
          b = do_pop
          a = do_pop
          @stack.push(a + b)
          @pc += 1

        when OpCode::SUB
          b = do_pop
          a = do_pop
          @stack.push(a - b)
          @pc += 1

        when OpCode::MUL
          b = do_pop
          a = do_pop
          @stack.push(a * b)
          @pc += 1

        when OpCode::DIV
          b = do_pop
          a = do_pop
          raise DivisionByZeroError, "Division by zero" if b == 0
          @stack.push(a / b)
          @pc += 1

        # == Comparison ==
        when OpCode::CMP_EQ
          b = do_pop
          a = do_pop
          @stack.push(a == b ? 1 : 0)
          @pc += 1

        when OpCode::CMP_LT
          b = do_pop
          a = do_pop
          @stack.push(a < b ? 1 : 0)
          @pc += 1

        when OpCode::CMP_GT
          b = do_pop
          a = do_pop
          @stack.push(a > b ? 1 : 0)
          @pc += 1

        # == Control Flow ==
        when OpCode::JUMP
          target = require_operand(instruction)
          unless target.is_a?(Integer)
            raise InvalidOperandError, "JUMP operand must be an integer, got #{target.inspect}"
          end
          @pc = target

        when OpCode::JUMP_IF_FALSE
          target = require_operand(instruction)
          unless target.is_a?(Integer)
            raise InvalidOperandError, "JUMP_IF_FALSE operand must be an integer, got #{target.inspect}"
          end
          condition = do_pop
          @pc = falsy?(condition) ? target : @pc + 1

        when OpCode::JUMP_IF_TRUE
          target = require_operand(instruction)
          unless target.is_a?(Integer)
            raise InvalidOperandError, "JUMP_IF_TRUE operand must be an integer, got #{target.inspect}"
          end
          condition = do_pop
          @pc = falsy?(condition) ? @pc + 1 : target

        # == Functions ==
        when OpCode::CALL
          name_index = require_operand(instruction)
          validate_index(name_index, code.names.length, "CALL", "names pool")
          func_name = code.names[name_index]
          unless @variables.key?(func_name)
            raise UndefinedNameError, "Function '#{func_name}' is not defined"
          end
          func_code = @variables[func_name]
          unless func_code.is_a?(CodeObject)
            raise VMError, "'#{func_name}' is not callable (expected CodeObject, got #{func_code.class.name})"
          end

          frame = CallFrame.new(
            return_address: @pc + 1,
            saved_variables: @variables.dup,
            saved_locals: @locals.dup
          )
          @call_stack.push(frame)

          @locals = []
          @pc = 0
          while !@halted && @pc < func_code.instructions.length
            current_instr = func_code.instructions[@pc]
            break if current_instr.opcode == OpCode::RETURN
            dispatch(current_instr, func_code)
          end

          frame = @call_stack.pop
          @pc = frame.return_address
          @locals = frame.saved_locals

        when OpCode::RETURN
          if @call_stack.any?
            frame = @call_stack.pop
            @pc = frame.return_address
            @locals = frame.saved_locals
          else
            @halted = true
          end

        # == I/O ==
        when OpCode::PRINT
          value = do_pop
          output_str = value.to_s
          @output.push(output_str)
          output_value = output_str
          @pc += 1

        # == VM Control ==
        when OpCode::HALT
          @halted = true

        else
          raise InvalidOpcodeError, "Unknown opcode: #{instruction.opcode.inspect}"
        end

        output_value
      end

      # Pop and return the top of stack, raising on underflow.
      def do_pop
        raise StackUnderflowError, "Cannot pop from an empty stack -- possible compiler bug" if @stack.empty?
        @stack.pop
      end

      # Get the operand, raising if missing.
      def require_operand(instruction)
        if instruction.operand.nil?
          name = OpCode::NAMES[instruction.opcode] || instruction.opcode.to_s
          raise InvalidOperandError, "#{name} requires an operand but none was provided"
        end
        instruction.operand
      end

      # Validate that an index is in range.
      def validate_index(index, pool_size, op_name, pool_name)
        unless index.is_a?(Integer) && index >= 0 && index < pool_size
          raise InvalidOperandError,
                "#{op_name} operand #{index.inspect} is out of range (#{pool_name} has #{pool_size} entries)"
        end
      end

      # Check if a value is falsy (0, nil, or empty string).
      def falsy?(value)
        value == 0 || value.nil? || value == ""
      end

      # Generate a human-readable description of what an instruction did.
      def describe(instruction, code, stack_before)
        op = instruction.opcode

        case op
        when OpCode::LOAD_CONST
          idx = instruction.operand
          val = (idx.is_a?(Integer) && idx >= 0 && idx < code.constants.length) ? code.constants[idx] : "?"
          "Push constant #{val.inspect} onto the stack"
        when OpCode::POP
          val = stack_before.empty? ? "?" : stack_before.last
          "Discard top of stack (#{val.inspect})"
        when OpCode::DUP
          val = stack_before.empty? ? "?" : stack_before.last
          "Duplicate top of stack (#{val.inspect})"
        when OpCode::STORE_NAME
          idx = instruction.operand
          name = (idx.is_a?(Integer) && idx >= 0 && idx < code.names.length) ? code.names[idx] : "?"
          val = stack_before.empty? ? "?" : stack_before.last
          "Store #{val.inspect} into variable '#{name}'"
        when OpCode::LOAD_NAME
          idx = instruction.operand
          name = (idx.is_a?(Integer) && idx >= 0 && idx < code.names.length) ? code.names[idx] : "?"
          "Push variable '#{name}' onto the stack"
        when OpCode::STORE_LOCAL
          val = stack_before.empty? ? "?" : stack_before.last
          "Store #{val.inspect} into local slot #{instruction.operand}"
        when OpCode::LOAD_LOCAL
          "Push local slot #{instruction.operand} onto the stack"
        when OpCode::ADD
          if stack_before.length >= 2
            a, b = stack_before[-2], stack_before[-1]
            "Pop #{b.inspect} and #{a.inspect}, push sum #{(a + b).inspect}"
          else
            "Add top two stack values"
          end
        when OpCode::SUB
          if stack_before.length >= 2
            a, b = stack_before[-2], stack_before[-1]
            "Pop #{b.inspect} and #{a.inspect}, push difference #{(a - b).inspect}"
          else
            "Subtract top two stack values"
          end
        when OpCode::MUL
          if stack_before.length >= 2
            a, b = stack_before[-2], stack_before[-1]
            "Pop #{b.inspect} and #{a.inspect}, push product #{(a * b).inspect}"
          else
            "Multiply top two stack values"
          end
        when OpCode::DIV
          "Divide top two stack values"
        when OpCode::CMP_EQ
          "Compare top two stack values for equality"
        when OpCode::CMP_LT
          "Compare top two stack values (less than)"
        when OpCode::CMP_GT
          "Compare top two stack values (greater than)"
        when OpCode::JUMP
          "Jump to instruction #{instruction.operand}"
        when OpCode::JUMP_IF_FALSE
          "Jump to #{instruction.operand} if top of stack is falsy"
        when OpCode::JUMP_IF_TRUE
          "Jump to #{instruction.operand} if top of stack is truthy"
        when OpCode::CALL
          idx = instruction.operand
          name = (idx.is_a?(Integer) && idx >= 0 && idx < code.names.length) ? code.names[idx] : "?"
          "Call function '#{name}'"
        when OpCode::RETURN
          "Return from function"
        when OpCode::PRINT
          val = stack_before.empty? ? "?" : stack_before.last
          "Print #{val.inspect}"
        when OpCode::HALT
          "Halt execution"
        else
          "Unknown operation"
        end
      end
    end
  end
end
