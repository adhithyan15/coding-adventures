# frozen_string_literal: true

# ==========================================================================
# Interpreter -- The Register-Based Bytecode Execution Engine
# ==========================================================================
#
# Architecture overview
# ---------------------
# This interpreter follows the V8 Ignition design:
#
#   1. ACCUMULATOR MODEL
#      Most operations read one operand from the accumulator and one from a
#      named register. The result goes back into the accumulator. This keeps
#      bytecode compact: "ADD r2" is a 2-byte instruction (opcode + register
#      index) instead of the 3-byte "ADD r0, r1, r2" of a three-address ISA.
#
#   2. REGISTER FILE PER CALL FRAME
#      Each function invocation gets its own array of registers. There is no
#      shared global register file — registers are local to the frame, like
#      locals in a stack-based VM but addressable by index, not position.
#
#   3. FEEDBACK VECTORS
#      Every function has a parallel array of "feedback slots". Before each
#      arithmetic or property-access instruction, the interpreter records the
#      runtime types of the operands into the relevant slot. A future JIT
#      tier can read these slots to emit specialized machine code.
#
#   4. CALL DEPTH LIMIT
#      To prevent infinite recursion from crashing the Ruby process, we track
#      call depth and raise VMError when it exceeds MAX_CALL_DEPTH.
#
# Execution model
# ---------------
# Each call to `execute(code)` creates a fresh top-level CallFrame and calls
# `run_frame(frame)`. Instructions are dispatched by a large `case/when`.
# The interpreter is deliberately simple and unoptimized — clarity over speed.
#
# How a function call works:
#
#   Caller bytecode:
#     STAR r0              ; save the function in register 0
#     LDA_CONSTANT 5       ; load argument
#     STAR r1              ; save argument in r1
#     CALL r0, 1, r1       ; call func in r0 with 1 arg starting at r1
#
#   Interpreter:
#     1. Read registers[r0] → must be a VMFunction
#     2. Create new CallFrame with the VMFunction's CodeObject
#     3. Copy argument registers into the new frame
#     4. Recursively call run_frame(new_frame)
#     5. New frame's return value becomes caller's accumulator
#
module CodingAdventures
  module RegisterVM
    class Interpreter
      # Maximum allowed call depth (prevents stack overflow in Ruby itself).
      MAX_CALL_DEPTH = 500

      # The interpreter's global variable store. Populated by STORE_GLOBAL and
      # read by LOAD_GLOBAL. Shared across all frames in one execution.
      attr_reader :globals

      # Lines printed by PRINT instructions, accumulated in order.
      attr_reader :output

      def initialize(max_depth: MAX_CALL_DEPTH)
        @globals     = {}
        @output      = []
        @call_depth  = 0
        @max_depth   = max_depth

        # Seed useful built-in "globals" that programs can call.
        # These are Ruby Procs stored in @globals under a naming convention.
        register_builtins
      end

      # -----------------------------------------------------------------------
      # Public API
      # -----------------------------------------------------------------------

      # Execute a CodeObject and return a VMResult.
      #
      # @param code [CodeObject]
      # @return [VMResult] — return_value, output, and error (nil on success)
      def execute(code)
        frame = new_frame(code, nil)
        return_value =
          begin
            run_frame(frame)
          rescue VMError => e
            return VMResult.new(return_value: nil, output: @output.dup, error: e)
          end
        VMResult.new(return_value: return_value, output: @output.dup, error: nil)
      end

      # Execute a CodeObject and return an Array of TraceStep records,
      # one per instruction executed.
      #
      # @param code [CodeObject]
      # @return [Array<TraceStep>]
      def execute_with_trace(code)
        frame  = new_frame(code, nil)
        steps  = []
        begin
          run_frame_traced(frame, steps)
        rescue VMError
          # partial trace is still useful
        end
        steps
      end

      private

      # -----------------------------------------------------------------------
      # Frame Management
      # -----------------------------------------------------------------------

      # Build a fresh CallFrame for `code`, linked to `caller_frame`.
      def new_frame(code, caller_frame)
        registers       = Array.new([code.register_count, 1].max, UNDEFINED)
        feedback_vector = Feedback.new_vector([code.feedback_slot_count, 1].max)
        CallFrame.new(
          code:            code,
          ip:              0,
          accumulator:     UNDEFINED,
          registers:       registers,
          feedback_vector: feedback_vector,
          context:         nil,
          caller_frame:    caller_frame
        )
      end

      # -----------------------------------------------------------------------
      # Main Execution Loops
      # -----------------------------------------------------------------------

      # Run a frame until it returns or halts.
      # Returns the final accumulator value.
      def run_frame(frame)
        loop do
          check_ip!(frame)
          instr = fetch(frame)

          case instr.opcode

          # =================================================================
          # 0x00–0x06  Accumulator Loads
          # =================================================================

          when Opcodes::LDA_CONSTANT
            # Load a literal from the constants pool.
            # operand[0] = index into code.constants
            frame.accumulator = frame.code.constants[instr.operands[0]]

          when Opcodes::LDA_ZERO
            # Optimized: load the integer 0 without a constants pool lookup.
            frame.accumulator = 0

          when Opcodes::LDA_TRUE
            frame.accumulator = true

          when Opcodes::LDA_FALSE
            frame.accumulator = false

          when Opcodes::LDA_NULL
            frame.accumulator = nil

          when Opcodes::LDA_UNDEFINED
            frame.accumulator = UNDEFINED

          when Opcodes::LDAR
            # Load Accumulator from Register: acc = registers[r]
            frame.accumulator = frame.registers[instr.operands[0]]

          # =================================================================
          # 0x10–0x11  Register Moves
          # =================================================================

          when Opcodes::STAR
            # Store Accumulator to Register: registers[r] = acc
            frame.registers[instr.operands[0]] = frame.accumulator

          when Opcodes::MOV
            # Copy register to register: registers[dst] = registers[src]
            src = instr.operands[0]
            dst = instr.operands[1]
            frame.registers[dst] = frame.registers[src]

          # =================================================================
          # 0x20–0x2F  Arithmetic & Bitwise
          # =================================================================
          # Convention: acc is the *left* operand, register is the *right*.
          # Feedback is recorded before computation so the slot reflects the
          # types of the actual inputs.

          when Opcodes::ADD
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            frame.accumulator = do_add(left, right)

          when Opcodes::SUB
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            frame.accumulator = coerce_num(left) - coerce_num(right)

          when Opcodes::MUL
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            frame.accumulator = coerce_num(left) * coerce_num(right)

          when Opcodes::DIV
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            r = coerce_num(right)
            raise VMError.new("Division by zero", instruction_index: frame.ip - 1, opcode: Opcodes::DIV) if r == 0

            l = coerce_num(left)
            frame.accumulator = (l.is_a?(Integer) && r.is_a?(Integer)) ? l / r : l.to_f / r.to_f

          when Opcodes::MOD
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            r = coerce_num(right)
            raise VMError.new("Modulo by zero", instruction_index: frame.ip - 1, opcode: Opcodes::MOD) if r == 0

            frame.accumulator = coerce_num(left) % r

          when Opcodes::EXP
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, left, right)
            frame.accumulator = coerce_num(left) ** coerce_num(right)

          when Opcodes::BIT_AND
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = coerce_int(left) & coerce_int(right)

          when Opcodes::BIT_OR
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = coerce_int(left) | coerce_int(right)

          when Opcodes::BIT_XOR
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = coerce_int(left) ^ coerce_int(right)

          when Opcodes::BIT_NOT
            # Unary bitwise NOT: acc = ~acc
            frame.accumulator = ~coerce_int(frame.accumulator)

          when Opcodes::SHIFT_LEFT
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = coerce_int(left) << (coerce_int(right) & 31)

          when Opcodes::SHIFT_RIGHT
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = coerce_int(left) >> (coerce_int(right) & 31)

          when Opcodes::SHIFT_RIGHT_U
            # Logical (unsigned) right shift. Ruby integers are arbitrary precision
            # so we mask to 32 bits after the shift to mimic JavaScript's >>> .
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = (coerce_int(left) & 0xFFFF_FFFF) >> (coerce_int(right) & 31)

          when Opcodes::NEG
            frame.accumulator = -coerce_num(frame.accumulator)

          when Opcodes::INC
            frame.accumulator = coerce_num(frame.accumulator) + 1

          when Opcodes::DEC
            frame.accumulator = coerce_num(frame.accumulator) - 1

          # =================================================================
          # 0x30–0x3A  Comparison
          # =================================================================
          # All comparisons leave true / false in the accumulator.

          when Opcodes::CMP_EQ
            right = frame.registers[instr.operands[0]]
            frame.accumulator = (frame.accumulator == right)

          when Opcodes::CMP_NEQ
            right = frame.registers[instr.operands[0]]
            frame.accumulator = (frame.accumulator != right)

          when Opcodes::CMP_LT
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = comparable(left) < comparable(right)

          when Opcodes::CMP_LTE
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = comparable(left) <= comparable(right)

          when Opcodes::CMP_GT
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = comparable(left) > comparable(right)

          when Opcodes::CMP_GTE
            left  = frame.accumulator
            right = frame.registers[instr.operands[0]]
            frame.accumulator = comparable(left) >= comparable(right)

          when Opcodes::TEST_NULL
            frame.accumulator = frame.accumulator.nil?

          when Opcodes::TEST_UNDEFINED
            frame.accumulator = frame.accumulator.equal?(UNDEFINED)

          when Opcodes::TEST_BOOLEAN
            v = frame.accumulator
            frame.accumulator = (v == true || v == false)

          when Opcodes::TEST_NUMBER
            frame.accumulator = frame.accumulator.is_a?(Numeric)

          when Opcodes::TEST_STRING
            frame.accumulator = frame.accumulator.is_a?(String)

          # =================================================================
          # 0x40–0x45  Control Flow
          # =================================================================
          # Jump targets are absolute instruction indices, NOT byte offsets.

          when Opcodes::JUMP
            frame.ip = instr.operands[0]

          when Opcodes::JUMP_IF_TRUE
            frame.ip = instr.operands[0] if truthy?(frame.accumulator)

          when Opcodes::JUMP_IF_FALSE
            frame.ip = instr.operands[0] unless truthy?(frame.accumulator)

          when Opcodes::JUMP_IF_NULL
            frame.ip = instr.operands[0] if frame.accumulator.nil?

          when Opcodes::JUMP_IF_NOT_NULL
            frame.ip = instr.operands[0] unless frame.accumulator.nil?

          when Opcodes::LOOP
            # Back-edge jump. Semantically identical to JUMP but named separately
            # so tooling can identify loop back-edges for profiling / OSR.
            frame.ip = instr.operands[0]

          # =================================================================
          # 0x50–0x53  Function / Call
          # =================================================================

          when Opcodes::CALL
            fn_reg   = instr.operands[0]
            arg_count = instr.operands[1]
            first_arg_reg = instr.operands[2]

            callee = frame.registers[fn_reg]
            Feedback.record_call_site(
              frame.feedback_vector,
              instr.feedback_slot,
              Feedback.value_type(callee)
            )
            frame.accumulator = dispatch_call(callee, frame, arg_count, first_arg_reg)

          when Opcodes::RETURN
            return frame.accumulator

          when Opcodes::CALL_BUILTIN
            name_idx  = instr.operands[0]
            arg_count = instr.operands[1]
            first_arg_reg = instr.operands[2]
            name = frame.code.names[name_idx]

            builtin = @globals[name]
            unless builtin.respond_to?(:call)
              raise VMError.new("Unknown built-in: #{name}", instruction_index: frame.ip - 1, opcode: Opcodes::CALL_BUILTIN)
            end

            args = (0...arg_count).map { |i| frame.registers[first_arg_reg + i] }
            frame.accumulator = builtin.call(*args)

          when Opcodes::CREATE_CLOSURE
            # Wrap a nested CodeObject in a VMFunction, capturing the current context.
            nested_code = frame.code.constants[instr.operands[0]]
            frame.accumulator = VMFunction.new(code: nested_code, context: frame.context)

          # =================================================================
          # 0x60–0x65  Variables / Scope Chain
          # =================================================================

          when Opcodes::LOAD_GLOBAL
            name = frame.code.names[instr.operands[0]]
            val  = @globals[name]
            frame.accumulator = val.nil? ? UNDEFINED : val

          when Opcodes::STORE_GLOBAL
            name = frame.code.names[instr.operands[0]]
            @globals[name] = frame.accumulator

          when Opcodes::LOAD_CONTEXT_SLOT
            depth = instr.operands[0]
            idx   = instr.operands[1]
            frame.accumulator = Scope.get_slot(frame.context, depth, idx)

          when Opcodes::STORE_CONTEXT_SLOT
            depth = instr.operands[0]
            idx   = instr.operands[1]
            Scope.set_slot(frame.context, depth, idx, frame.accumulator)

          when Opcodes::PUSH_CONTEXT
            slot_count   = instr.operands[0]
            frame.context = Scope.new_context(frame.context, slot_count)

          when Opcodes::POP_CONTEXT
            raise VMError.new("POP_CONTEXT: no context to pop", instruction_index: frame.ip - 1) if frame.context.nil?

            frame.context = frame.context.parent

          # =================================================================
          # 0x70–0x74  Object / Property
          # =================================================================

          when Opcodes::CREATE_OBJECT
            hid = Feedback.new_hidden_class_id
            frame.accumulator = VMObject.new(hidden_class_id: hid, properties: {})

          when Opcodes::LOAD_PROPERTY
            obj  = frame.accumulator
            unless obj.is_a?(VMObject)
              raise VMError.new("LOAD_PROPERTY on non-object: #{obj.inspect}", instruction_index: frame.ip - 1, opcode: Opcodes::LOAD_PROPERTY)
            end

            name = frame.code.names[instr.operands[0]]
            Feedback.record_property_load(frame.feedback_vector, instr.feedback_slot, obj.hidden_class_id)
            frame.accumulator = obj.properties.fetch(name, UNDEFINED)

          when Opcodes::STORE_PROPERTY
            obj  = frame.accumulator
            unless obj.is_a?(VMObject)
              raise VMError.new("STORE_PROPERTY on non-object", instruction_index: frame.ip - 1, opcode: Opcodes::STORE_PROPERTY)
            end

            name  = frame.code.names[instr.operands[0]]
            value = frame.registers[instr.operands[1]]
            obj.properties[name] = value

          when Opcodes::DELETE_PROPERTY
            obj  = frame.accumulator
            unless obj.is_a?(VMObject)
              raise VMError.new("DELETE_PROPERTY on non-object", instruction_index: frame.ip - 1, opcode: Opcodes::DELETE_PROPERTY)
            end

            name = frame.code.names[instr.operands[0]]
            obj.properties.delete(name)
            frame.accumulator = true

          when Opcodes::HAS_PROPERTY
            obj  = frame.accumulator
            unless obj.is_a?(VMObject)
              raise VMError.new("HAS_PROPERTY on non-object", instruction_index: frame.ip - 1, opcode: Opcodes::HAS_PROPERTY)
            end

            name = frame.code.names[instr.operands[0]]
            frame.accumulator = obj.properties.key?(name)

          # =================================================================
          # 0x80–0x84  Array
          # =================================================================

          when Opcodes::CREATE_ARRAY
            frame.accumulator = []

          when Opcodes::LOAD_ELEMENT
            arr   = frame.accumulator
            idx   = frame.registers[instr.operands[0]]
            unless arr.is_a?(Array)
              raise VMError.new("LOAD_ELEMENT on non-array: #{arr.inspect}", instruction_index: frame.ip - 1, opcode: Opcodes::LOAD_ELEMENT)
            end

            frame.accumulator = arr[coerce_int(idx)] || UNDEFINED

          when Opcodes::STORE_ELEMENT
            arr   = frame.accumulator
            idx   = frame.registers[instr.operands[0]]
            value = frame.registers[instr.operands[1]]
            unless arr.is_a?(Array)
              raise VMError.new("STORE_ELEMENT on non-array", instruction_index: frame.ip - 1, opcode: Opcodes::STORE_ELEMENT)
            end

            arr[coerce_int(idx)] = value

          when Opcodes::PUSH_ELEMENT
            arr   = frame.accumulator
            value = frame.registers[instr.operands[0]]
            unless arr.is_a?(Array)
              raise VMError.new("PUSH_ELEMENT on non-array", instruction_index: frame.ip - 1, opcode: Opcodes::PUSH_ELEMENT)
            end

            arr << value

          when Opcodes::ARRAY_LENGTH
            arr = frame.accumulator
            unless arr.is_a?(Array)
              raise VMError.new("ARRAY_LENGTH on non-array", instruction_index: frame.ip - 1, opcode: Opcodes::ARRAY_LENGTH)
            end

            frame.accumulator = arr.length

          # =================================================================
          # 0x90–0x93  Type / Coercion
          # =================================================================

          when Opcodes::TYPEOF
            frame.accumulator = typeof_value(frame.accumulator)

          when Opcodes::TO_NUMBER
            frame.accumulator = coerce_num(frame.accumulator)

          when Opcodes::TO_STRING
            frame.accumulator = vm_to_s(frame.accumulator)

          when Opcodes::TO_BOOLEAN
            frame.accumulator = truthy?(frame.accumulator)

          # =================================================================
          # 0xA0–0xA3  Logical
          # =================================================================

          when Opcodes::LOGICAL_OR
            rhs = frame.registers[instr.operands[0]]
            frame.accumulator = truthy?(frame.accumulator) ? frame.accumulator : rhs

          when Opcodes::LOGICAL_AND
            rhs = frame.registers[instr.operands[0]]
            frame.accumulator = truthy?(frame.accumulator) ? rhs : frame.accumulator

          when Opcodes::LOGICAL_NOT
            frame.accumulator = !truthy?(frame.accumulator)

          when Opcodes::NULLISH_COALESCE
            # Replace acc with rhs only if acc is null or undefined.
            rhs = frame.registers[instr.operands[0]]
            if frame.accumulator.nil? || frame.accumulator.equal?(UNDEFINED)
              frame.accumulator = rhs
            end

          # =================================================================
          # 0xB0  I/O
          # =================================================================

          when Opcodes::PRINT
            @output << vm_to_s(frame.accumulator)

          # =================================================================
          # 0xFF  HALT / RETURN
          # =================================================================

          when Opcodes::HALT
            return frame.accumulator

          else
            raise VMError.new(
              "Unknown opcode: 0x#{instr.opcode.to_s(16)} (#{Opcodes.name(instr.opcode)})",
              instruction_index: frame.ip - 1,
              opcode: instr.opcode
            )
          end
        end
      end

      # Same loop as run_frame but records a TraceStep before each instruction.
      def run_frame_traced(frame, steps)
        loop do
          check_ip!(frame)
          instr = frame.code.instructions[frame.ip]

          acc_before = frame.accumulator
          regs_snap  = frame.registers.dup

          # Advance IP before dispatching (run_frame does this via fetch())
          frame.ip += 1

          # Build trace step
          steps << TraceStep.new(
            ip:                  frame.ip - 1,
            opcode_name:         Opcodes.name(instr.opcode),
            operands:            instr.operands.dup,
            accumulator_before:  acc_before,
            accumulator_after:   nil, # filled in after dispatch
            registers_snapshot:  regs_snap
          )

          # Re-dispatch using the non-traced path for brevity.
          # We temporarily rewind ip so run_frame's fetch() sees the same instr.
          frame.ip -= 1
          run_one_instruction(frame, instr)

          # Patch the trace step with the post-instruction accumulator.
          steps.last.accumulator_after = frame.accumulator

          return frame.accumulator if instr.opcode == Opcodes::HALT || instr.opcode == Opcodes::RETURN
        end
      end

      # Execute a single pre-fetched instruction in `frame`, without advancing IP.
      # Used by run_frame_traced.
      def run_one_instruction(frame, instr)
        # Advance IP (mirrors fetch() in run_frame)
        frame.ip += 1

        # Re-use the main dispatch. We do this by temporarily pretending the
        # instruction is at ip-1 (it was already fetched).
        case instr.opcode

        when Opcodes::LDA_CONSTANT   then frame.accumulator = frame.code.constants[instr.operands[0]]
        when Opcodes::LDA_ZERO       then frame.accumulator = 0
        when Opcodes::LDA_TRUE       then frame.accumulator = true
        when Opcodes::LDA_FALSE      then frame.accumulator = false
        when Opcodes::LDA_NULL       then frame.accumulator = nil
        when Opcodes::LDA_UNDEFINED  then frame.accumulator = UNDEFINED
        when Opcodes::LDAR           then frame.accumulator = frame.registers[instr.operands[0]]
        when Opcodes::STAR           then frame.registers[instr.operands[0]] = frame.accumulator
        when Opcodes::MOV            then frame.registers[instr.operands[1]] = frame.registers[instr.operands[0]]

        when Opcodes::ADD
          l, r = frame.accumulator, frame.registers[instr.operands[0]]
          Feedback.record_binary_op(frame.feedback_vector, instr.feedback_slot, l, r)
          frame.accumulator = do_add(l, r)
        when Opcodes::SUB
          l, r = frame.accumulator, frame.registers[instr.operands[0]]
          frame.accumulator = coerce_num(l) - coerce_num(r)
        when Opcodes::MUL
          l, r = frame.accumulator, frame.registers[instr.operands[0]]
          frame.accumulator = coerce_num(l) * coerce_num(r)
        when Opcodes::DIV
          l, r = coerce_num(frame.accumulator), coerce_num(frame.registers[instr.operands[0]])
          raise VMError.new("Division by zero") if r == 0
          frame.accumulator = (l.is_a?(Integer) && r.is_a?(Integer)) ? l / r : l.to_f / r.to_f
        when Opcodes::MOD
          l, r = coerce_num(frame.accumulator), coerce_num(frame.registers[instr.operands[0]])
          raise VMError.new("Modulo by zero") if r == 0
          frame.accumulator = l % r
        when Opcodes::EXP
          frame.accumulator = coerce_num(frame.accumulator) ** coerce_num(frame.registers[instr.operands[0]])
        when Opcodes::BIT_AND        then frame.accumulator = coerce_int(frame.accumulator) & coerce_int(frame.registers[instr.operands[0]])
        when Opcodes::BIT_OR         then frame.accumulator = coerce_int(frame.accumulator) | coerce_int(frame.registers[instr.operands[0]])
        when Opcodes::BIT_XOR        then frame.accumulator = coerce_int(frame.accumulator) ^ coerce_int(frame.registers[instr.operands[0]])
        when Opcodes::BIT_NOT        then frame.accumulator = ~coerce_int(frame.accumulator)
        when Opcodes::SHIFT_LEFT     then frame.accumulator = coerce_int(frame.accumulator) << (coerce_int(frame.registers[instr.operands[0]]) & 31)
        when Opcodes::SHIFT_RIGHT    then frame.accumulator = coerce_int(frame.accumulator) >> (coerce_int(frame.registers[instr.operands[0]]) & 31)
        when Opcodes::SHIFT_RIGHT_U  then frame.accumulator = (coerce_int(frame.accumulator) & 0xFFFF_FFFF) >> (coerce_int(frame.registers[instr.operands[0]]) & 31)
        when Opcodes::NEG            then frame.accumulator = -coerce_num(frame.accumulator)
        when Opcodes::INC            then frame.accumulator = coerce_num(frame.accumulator) + 1
        when Opcodes::DEC            then frame.accumulator = coerce_num(frame.accumulator) - 1

        when Opcodes::CMP_EQ         then frame.accumulator = (frame.accumulator == frame.registers[instr.operands[0]])
        when Opcodes::CMP_NEQ        then frame.accumulator = (frame.accumulator != frame.registers[instr.operands[0]])
        when Opcodes::CMP_LT         then frame.accumulator = comparable(frame.accumulator) < comparable(frame.registers[instr.operands[0]])
        when Opcodes::CMP_LTE        then frame.accumulator = comparable(frame.accumulator) <= comparable(frame.registers[instr.operands[0]])
        when Opcodes::CMP_GT         then frame.accumulator = comparable(frame.accumulator) > comparable(frame.registers[instr.operands[0]])
        when Opcodes::CMP_GTE        then frame.accumulator = comparable(frame.accumulator) >= comparable(frame.registers[instr.operands[0]])
        when Opcodes::TEST_NULL      then frame.accumulator = frame.accumulator.nil?
        when Opcodes::TEST_UNDEFINED then frame.accumulator = frame.accumulator.equal?(UNDEFINED)
        when Opcodes::TEST_BOOLEAN   then frame.accumulator = (frame.accumulator == true || frame.accumulator == false)
        when Opcodes::TEST_NUMBER    then frame.accumulator = frame.accumulator.is_a?(Numeric)
        when Opcodes::TEST_STRING    then frame.accumulator = frame.accumulator.is_a?(String)

        when Opcodes::JUMP           then frame.ip = instr.operands[0]
        when Opcodes::JUMP_IF_TRUE   then frame.ip = instr.operands[0] if truthy?(frame.accumulator)
        when Opcodes::JUMP_IF_FALSE  then frame.ip = instr.operands[0] unless truthy?(frame.accumulator)
        when Opcodes::JUMP_IF_NULL   then frame.ip = instr.operands[0] if frame.accumulator.nil?
        when Opcodes::JUMP_IF_NOT_NULL then frame.ip = instr.operands[0] unless frame.accumulator.nil?
        when Opcodes::LOOP           then frame.ip = instr.operands[0]

        when Opcodes::CALL
          fn_reg, arg_count, first_arg = instr.operands
          callee = frame.registers[fn_reg]
          frame.accumulator = dispatch_call(callee, frame, arg_count, first_arg)
        when Opcodes::RETURN         then return frame.accumulator
        when Opcodes::CALL_BUILTIN
          name_idx, arg_count, first_arg = instr.operands
          name = frame.code.names[name_idx]
          builtin = @globals[name]
          raise VMError.new("Unknown built-in: #{name}") unless builtin.respond_to?(:call)
          args = (0...arg_count).map { |i| frame.registers[first_arg + i] }
          frame.accumulator = builtin.call(*args)
        when Opcodes::CREATE_CLOSURE
          frame.accumulator = VMFunction.new(code: frame.code.constants[instr.operands[0]], context: frame.context)

        when Opcodes::LOAD_GLOBAL
          name = frame.code.names[instr.operands[0]]
          v = @globals[name]
          frame.accumulator = v.nil? ? UNDEFINED : v
        when Opcodes::STORE_GLOBAL
          @globals[frame.code.names[instr.operands[0]]] = frame.accumulator
        when Opcodes::LOAD_CONTEXT_SLOT
          frame.accumulator = Scope.get_slot(frame.context, instr.operands[0], instr.operands[1])
        when Opcodes::STORE_CONTEXT_SLOT
          Scope.set_slot(frame.context, instr.operands[0], instr.operands[1], frame.accumulator)
        when Opcodes::PUSH_CONTEXT
          frame.context = Scope.new_context(frame.context, instr.operands[0])
        when Opcodes::POP_CONTEXT
          frame.context = frame.context.parent

        when Opcodes::CREATE_OBJECT
          frame.accumulator = VMObject.new(hidden_class_id: Feedback.new_hidden_class_id, properties: {})
        when Opcodes::LOAD_PROPERTY
          obj = frame.accumulator
          raise VMError.new("LOAD_PROPERTY on non-object") unless obj.is_a?(VMObject)
          Feedback.record_property_load(frame.feedback_vector, instr.feedback_slot, obj.hidden_class_id)
          frame.accumulator = obj.properties.fetch(frame.code.names[instr.operands[0]], UNDEFINED)
        when Opcodes::STORE_PROPERTY
          obj = frame.accumulator
          raise VMError.new("STORE_PROPERTY on non-object") unless obj.is_a?(VMObject)
          obj.properties[frame.code.names[instr.operands[0]]] = frame.registers[instr.operands[1]]
        when Opcodes::DELETE_PROPERTY
          obj = frame.accumulator
          raise VMError.new("DELETE_PROPERTY on non-object") unless obj.is_a?(VMObject)
          obj.properties.delete(frame.code.names[instr.operands[0]])
          frame.accumulator = true
        when Opcodes::HAS_PROPERTY
          obj = frame.accumulator
          raise VMError.new("HAS_PROPERTY on non-object") unless obj.is_a?(VMObject)
          frame.accumulator = obj.properties.key?(frame.code.names[instr.operands[0]])

        when Opcodes::CREATE_ARRAY     then frame.accumulator = []
        when Opcodes::LOAD_ELEMENT
          arr = frame.accumulator
          raise VMError.new("LOAD_ELEMENT on non-array") unless arr.is_a?(Array)
          frame.accumulator = arr[coerce_int(frame.registers[instr.operands[0]])] || UNDEFINED
        when Opcodes::STORE_ELEMENT
          arr = frame.accumulator
          raise VMError.new("STORE_ELEMENT on non-array") unless arr.is_a?(Array)
          arr[coerce_int(frame.registers[instr.operands[0]])] = frame.registers[instr.operands[1]]
        when Opcodes::PUSH_ELEMENT
          arr = frame.accumulator
          raise VMError.new("PUSH_ELEMENT on non-array") unless arr.is_a?(Array)
          arr << frame.registers[instr.operands[0]]
        when Opcodes::ARRAY_LENGTH
          arr = frame.accumulator
          raise VMError.new("ARRAY_LENGTH on non-array") unless arr.is_a?(Array)
          frame.accumulator = arr.length

        when Opcodes::TYPEOF     then frame.accumulator = typeof_value(frame.accumulator)
        when Opcodes::TO_NUMBER  then frame.accumulator = coerce_num(frame.accumulator)
        when Opcodes::TO_STRING  then frame.accumulator = vm_to_s(frame.accumulator)
        when Opcodes::TO_BOOLEAN then frame.accumulator = truthy?(frame.accumulator)

        when Opcodes::LOGICAL_OR
          rhs = frame.registers[instr.operands[0]]
          frame.accumulator = truthy?(frame.accumulator) ? frame.accumulator : rhs
        when Opcodes::LOGICAL_AND
          rhs = frame.registers[instr.operands[0]]
          frame.accumulator = truthy?(frame.accumulator) ? rhs : frame.accumulator
        when Opcodes::LOGICAL_NOT
          frame.accumulator = !truthy?(frame.accumulator)
        when Opcodes::NULLISH_COALESCE
          rhs = frame.registers[instr.operands[0]]
          frame.accumulator = rhs if frame.accumulator.nil? || frame.accumulator.equal?(UNDEFINED)

        when Opcodes::PRINT
          @output << vm_to_s(frame.accumulator)

        when Opcodes::HALT then return frame.accumulator

        else
          raise VMError.new("Unknown opcode: #{Opcodes.name(instr.opcode)}")
        end
      end

      # -----------------------------------------------------------------------
      # Function Call Dispatch
      # -----------------------------------------------------------------------

      # Execute a call to `callee` with arguments taken from `caller_frame`.
      # Returns the callee's return value (which becomes the caller's accumulator).
      def dispatch_call(callee, caller_frame, arg_count, first_arg_reg)
        @call_depth += 1
        if @call_depth > @max_depth
          @call_depth -= 1
          raise VMError.new("Maximum call depth #{@max_depth} exceeded")
        end

        begin
          case callee
          when VMFunction
            callee_frame = new_frame(callee.code, caller_frame)
            # Copy arguments into the callee's registers 0..arg_count-1
            arg_count.times do |i|
              callee_frame.registers[i] = caller_frame.registers[first_arg_reg + i]
            end
            # Inherit the closure's captured context
            callee_frame.context = callee.context
            run_frame(callee_frame)

          when Proc, Method
            # Host function (registered built-in passed as a VMFunction alternative).
            args = (0...arg_count).map { |i| caller_frame.registers[first_arg_reg + i] }
            callee.call(*args)

          else
            raise VMError.new(
              "CALL target is not a function: #{Feedback.value_type(callee)}",
              instruction_index: caller_frame.ip - 1,
              opcode: Opcodes::CALL
            )
          end
        ensure
          @call_depth -= 1
        end
      end

      # -----------------------------------------------------------------------
      # Helper: fetch current instruction and advance IP
      # -----------------------------------------------------------------------
      def fetch(frame)
        instr = frame.code.instructions[frame.ip]
        frame.ip += 1
        instr
      end

      # -----------------------------------------------------------------------
      # Boundary check
      # -----------------------------------------------------------------------
      def check_ip!(frame)
        if frame.ip >= frame.code.instructions.length
          raise VMError.new(
            "IP #{frame.ip} out of bounds (#{frame.code.instructions.length} instructions)",
            instruction_index: frame.ip
          )
        end
      end

      # -----------------------------------------------------------------------
      # Value semantics helpers
      # -----------------------------------------------------------------------

      # JS-style truthiness:
      #   false, nil, UNDEFINED, 0, 0.0, "" are all falsy; everything else truthy.
      def truthy?(v)
        return false if v == false
        return false if v.nil?
        return false if v.equal?(UNDEFINED)
        return false if v == 0
        return false if v == ""
        true
      end

      # Addition: numeric sum for numbers, string concatenation otherwise.
      def do_add(a, b)
        if a.is_a?(Numeric) && b.is_a?(Numeric)
          a + b
        else
          vm_to_s(a) + vm_to_s(b)
        end
      end

      # Coerce a value to a Numeric.
      def coerce_num(v)
        case v
        when Numeric then v
        when String  then v.include?(".") ? v.to_f : v.to_i
        when TrueClass then 1
        when FalseClass, NilClass then 0
        else 0
        end
      end

      # Coerce a value to an Integer (for bitwise ops).
      def coerce_int(v)
        coerce_num(v).to_i
      end

      # Coerce for comparison (raise on incompatible types).
      def comparable(v)
        case v
        when Numeric, String then v
        when TrueClass  then 1
        when FalseClass then 0
        else coerce_num(v)
        end
      end

      # VM-level to_s (mirrors JS String() coercion).
      def vm_to_s(v)
        return "undefined" if v.equal?(UNDEFINED)
        return "null"      if v.nil?
        return v.to_s      if v.is_a?(VMObject)
        v.to_s
      end

      # typeof operator — matches JS semantics (including the famous null bug).
      #
      #   typeof null === "object"   // JS footgun since 1995
      #
      def typeof_value(v)
        case v
        when Integer, Float        then "number"
        when String                then "string"
        when TrueClass, FalseClass then "boolean"
        when NilClass              then "object"     # JS null quirk
        when VMObject, Array       then "object"
        when VMFunction            then "function"
        else
          v.equal?(UNDEFINED) ? "undefined" : "unknown"
        end
      end

      # -----------------------------------------------------------------------
      # Built-in Registration
      # -----------------------------------------------------------------------
      # Pre-seed @globals with a handful of host-provided functions.
      # Programs can call these via CALL_BUILTIN.

      def register_builtins
        # Math.abs
        @globals["Math.abs"] = ->(x) { coerce_num(x).abs }

        # Math.floor / ceil / round
        @globals["Math.floor"] = ->(x) { coerce_num(x).floor }
        @globals["Math.ceil"]  = ->(x) { coerce_num(x).ceil }
        @globals["Math.round"] = ->(x) { coerce_num(x).round }

        # Math.max / min
        @globals["Math.max"] = ->(*args) { args.map { |a| coerce_num(a) }.max }
        @globals["Math.min"] = ->(*args) { args.map { |a| coerce_num(a) }.min }

        # String length
        @globals["String.length"] = ->(s) { s.to_s.length }
      end
    end
  end
end
