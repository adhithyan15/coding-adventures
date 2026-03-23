# frozen_string_literal: true

# ==========================================================================
# Starlark Opcode Handlers -- The VM's Instruction Set Implementation
# ==========================================================================
#
# This module registers handlers for all 46+ Starlark opcodes with the
# GenericVM. Each handler is a lambda that receives three arguments:
#
#   (vm, instruction, code)
#
# Where:
#   - vm: the GenericVM instance (provides stack, variables, locals, pc, etc.)
#   - instruction: an Instruction with .opcode and .operand
#   - code: a CodeObject with .instructions, .constants, and .names
#
# The handler must:
#   1. Perform its operation (push/pop stack, modify variables, etc.)
#   2. Advance the program counter (vm.advance_pc) unless it's a jump
#   3. Return a String (for output) or nil
#
# == Stack Machine Basics
#
# All computation happens through the stack. For example, to compute 3 + 4:
#
#   LOAD_CONST 0  (push 3)     stack: [3]
#   LOAD_CONST 1  (push 4)     stack: [3, 4]
#   ADD                        stack: [7]
#
# Binary operators pop two values and push the result. The first value
# pushed (deeper in the stack) is the LEFT operand.
#
# == Handler Registration
#
# Handlers.register_all(vm) wires up every opcode. After registration,
# the GenericVM's execute() loop can dispatch any Starlark instruction.
# ==========================================================================

module CodingAdventures
  module StarlarkVM
    module Handlers
      # Register all Starlark opcode handlers with the given GenericVM.
      #
      # This is the main entry point. Call it once after creating a GenericVM
      # to make it Starlark-capable.
      def self.register_all(vm)
        op = CodingAdventures::StarlarkAstToBytecodeCompiler::Op

        # ==============================================================
        # Stack Operations
        # ==============================================================

        # LOAD_CONST: Push a constant value onto the stack.
        #
        # The operand is an index into the code's constants pool.
        # Constants include numbers, strings, and compiled function objects.
        #
        #   constants = [42, "hello"]
        #   LOAD_CONST 0  =>  push(42)
        #   LOAD_CONST 1  =>  push("hello")
        vm.register_opcode(op::LOAD_CONST, ->(v, instr, c) {
          v.push(c.constants[instr.operand])
          v.advance_pc
          nil
        })

        # POP: Discard the top value on the stack.
        #
        # Used after expression statements where the result isn't assigned.
        # For example, a bare function call `foo()` leaves a return value
        # on the stack that needs to be discarded.
        vm.register_opcode(op::POP, ->(v, _instr, _c) {
          v.pop
          v.advance_pc
          nil
        })

        # DUP: Duplicate the top value on the stack.
        #
        # Used in load() statements to keep the module object on the stack
        # while extracting multiple symbols from it.
        #
        #   stack: [module]  =>  stack: [module, module]
        vm.register_opcode(op::DUP, ->(v, _instr, _c) {
          val = v.peek
          v.push(val)
          v.advance_pc
          nil
        })

        # LOAD_NONE: Push None (Ruby's nil) onto the stack.
        vm.register_opcode(op::LOAD_NONE, ->(v, _instr, _c) {
          v.push(nil)
          v.advance_pc
          nil
        })

        # LOAD_TRUE: Push boolean true onto the stack.
        vm.register_opcode(op::LOAD_TRUE, ->(v, _instr, _c) {
          v.push(true)
          v.advance_pc
          nil
        })

        # LOAD_FALSE: Push boolean false onto the stack.
        vm.register_opcode(op::LOAD_FALSE, ->(v, _instr, _c) {
          v.push(false)
          v.advance_pc
          nil
        })

        # ==============================================================
        # Variable Access
        # ==============================================================

        # STORE_NAME: Store the top-of-stack value in a global variable.
        #
        # The operand indexes into the code's names table to get the
        # variable name string. The value is popped from the stack.
        #
        #   LOAD_CONST 0   (push 42)
        #   STORE_NAME 0   (names[0] = "x", so x = 42)
        vm.register_opcode(op::STORE_NAME, ->(v, instr, c) {
          name = c.names[instr.operand]
          val = v.pop
          v.variables[name] = val
          v.advance_pc
          nil
        })

        # LOAD_NAME: Look up a variable by name and push its value.
        #
        # Search order:
        #   1. Local variables (v.locals)
        #   2. Global variables (v.variables)
        #   3. Builtin functions (v.get_builtin)
        #
        # This mirrors Python/Starlark's LEGB rule (minus Enclosing).
        vm.register_opcode(op::LOAD_NAME, ->(v, instr, c) {
          name = c.names[instr.operand]
          if v.locals.is_a?(Hash) && v.locals.key?(name)
            v.push(v.locals[name])
          elsif v.variables.key?(name)
            v.push(v.variables[name])
          elsif v.get_builtin(name)
            v.push(v.get_builtin(name))
          else
            raise "NameError: name '#{name}' is not defined"
          end
          v.advance_pc
          nil
        })

        # STORE_LOCAL: Store a value in the local variable at the given index.
        #
        # Inside function bodies, local variables are stored by name in
        # a hash (v.locals) for simplicity. The operand indexes the names
        # table to get the variable name.
        vm.register_opcode(op::STORE_LOCAL, ->(v, instr, c) {
          name = c.names[instr.operand]
          val = v.pop
          v.locals = {} unless v.locals.is_a?(Hash)
          v.locals[name] = val
          v.advance_pc
          nil
        })

        # LOAD_LOCAL: Load a local variable and push it onto the stack.
        #
        # Looks up by name in the locals hash. Falls back to globals
        # and builtins if not found locally (this handles cases where
        # functions reference module-level names).
        vm.register_opcode(op::LOAD_LOCAL, ->(v, instr, c) {
          name = c.names[instr.operand]
          if v.locals.is_a?(Hash) && v.locals.key?(name)
            v.push(v.locals[name])
          elsif v.variables.key?(name)
            v.push(v.variables[name])
          elsif v.get_builtin(name)
            v.push(v.get_builtin(name))
          else
            raise "NameError: local variable '#{name}' is not defined"
          end
          v.advance_pc
          nil
        })

        # STORE_CLOSURE: Store a value in the closure scope.
        # (Starlark doesn't have true closures, but this opcode exists
        # for completeness. We store in locals as a fallback.)
        vm.register_opcode(op::STORE_CLOSURE, ->(v, instr, c) {
          name = c.names[instr.operand]
          val = v.pop
          v.locals = {} unless v.locals.is_a?(Hash)
          v.locals[name] = val
          v.advance_pc
          nil
        })

        # LOAD_CLOSURE: Load a value from the closure scope.
        vm.register_opcode(op::LOAD_CLOSURE, ->(v, instr, c) {
          name = c.names[instr.operand]
          if v.locals.is_a?(Hash) && v.locals.key?(name)
            v.push(v.locals[name])
          elsif v.variables.key?(name)
            v.push(v.variables[name])
          else
            raise "NameError: closure variable '#{name}' is not defined"
          end
          v.advance_pc
          nil
        })

        # ==============================================================
        # Arithmetic Operations
        # ==============================================================
        #
        # All binary arithmetic ops follow the same pattern:
        #   1. Pop right operand (b) -- it was pushed second, so it's on top
        #   2. Pop left operand (a)  -- it was pushed first, so it's below
        #   3. Push result (a op b)
        #
        # For example, to compute 10 - 3:
        #   LOAD_CONST 0   push(10)   stack: [10]
        #   LOAD_CONST 1   push(3)    stack: [10, 3]
        #   SUB            pop 3, pop 10, push 7   stack: [7]

        # ADD: Addition (also string concatenation and list concatenation).
        vm.register_opcode(op::ADD, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a + b)
          v.advance_pc
          nil
        })

        # SUB: Subtraction.
        vm.register_opcode(op::SUB, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a - b)
          v.advance_pc
          nil
        })

        # MUL: Multiplication (also string repetition: "ab" * 3 = "ababab").
        vm.register_opcode(op::MUL, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a * b)
          v.advance_pc
          nil
        })

        # DIV: True division (always returns a float in Python/Starlark).
        #
        # 7 / 2 = 3.5 (not 3)
        # In Starlark, division of two integers still produces a float.
        vm.register_opcode(op::DIV, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          raise "ZeroDivisionError: division by zero" if b == 0
          v.push(a.to_f / b.to_f)
          v.advance_pc
          nil
        })

        # FLOOR_DIV: Floor division (rounds toward negative infinity).
        #
        # 7 // 2 = 3
        # -7 // 2 = -4 (floors, doesn't truncate)
        vm.register_opcode(op::FLOOR_DIV, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          raise "ZeroDivisionError: integer division by zero" if b == 0
          v.push((a.to_f / b.to_f).floor)
          v.advance_pc
          nil
        })

        # MOD: Modulo (remainder after floor division).
        vm.register_opcode(op::MOD, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          raise "ZeroDivisionError: modulo by zero" if b == 0
          v.push(a % b)
          v.advance_pc
          nil
        })

        # POWER: Exponentiation (a ** b).
        vm.register_opcode(op::POWER, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a**b)
          v.advance_pc
          nil
        })

        # NEGATE: Unary minus (-a).
        vm.register_opcode(op::NEGATE, ->(v, _instr, _c) {
          a = v.pop
          v.push(-a)
          v.advance_pc
          nil
        })

        # BIT_AND: Bitwise AND (a & b).
        vm.register_opcode(op::BIT_AND, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a & b)
          v.advance_pc
          nil
        })

        # BIT_OR: Bitwise OR (a | b).
        vm.register_opcode(op::BIT_OR, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a | b)
          v.advance_pc
          nil
        })

        # BIT_XOR: Bitwise XOR (a ^ b).
        vm.register_opcode(op::BIT_XOR, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a ^ b)
          v.advance_pc
          nil
        })

        # BIT_NOT: Bitwise NOT (~a). Unary operation.
        vm.register_opcode(op::BIT_NOT, ->(v, _instr, _c) {
          a = v.pop
          v.push(~a)
          v.advance_pc
          nil
        })

        # LSHIFT: Left shift (a << b).
        vm.register_opcode(op::LSHIFT, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a << b)
          v.advance_pc
          nil
        })

        # RSHIFT: Right shift (a >> b).
        vm.register_opcode(op::RSHIFT, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a >> b)
          v.advance_pc
          nil
        })

        # ==============================================================
        # Comparison Operations
        # ==============================================================
        #
        # Each comparison pops two values and pushes true or false.
        # Like arithmetic, the left operand (a) is deeper in the stack.

        # CMP_EQ: Equality (a == b).
        vm.register_opcode(op::CMP_EQ, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a == b)
          v.advance_pc
          nil
        })

        # CMP_NE: Inequality (a != b).
        vm.register_opcode(op::CMP_NE, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a != b)
          v.advance_pc
          nil
        })

        # CMP_LT: Less than (a < b).
        vm.register_opcode(op::CMP_LT, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a < b)
          v.advance_pc
          nil
        })

        # CMP_GT: Greater than (a > b).
        vm.register_opcode(op::CMP_GT, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a > b)
          v.advance_pc
          nil
        })

        # CMP_LE: Less than or equal (a <= b).
        vm.register_opcode(op::CMP_LE, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a <= b)
          v.advance_pc
          nil
        })

        # CMP_GE: Greater than or equal (a >= b).
        vm.register_opcode(op::CMP_GE, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          v.push(a >= b)
          v.advance_pc
          nil
        })

        # CMP_IN: Membership test (a in b).
        #
        # Works with arrays, hashes (checks keys), and strings.
        #   3 in [1, 2, 3]     => true
        #   "x" in {"x": 1}    => true
        #   "ab" in "abc"       => true
        vm.register_opcode(op::CMP_IN, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          result = if b.is_a?(Hash)
            b.key?(a)
          elsif b.is_a?(String)
            b.include?(a.to_s)
          else
            b.include?(a)
          end
          v.push(result)
          v.advance_pc
          nil
        })

        # CMP_NOT_IN: Negated membership test (a not in b).
        vm.register_opcode(op::CMP_NOT_IN, ->(v, _instr, _c) {
          b = v.pop
          a = v.pop
          result = if b.is_a?(Hash)
            !b.key?(a)
          elsif b.is_a?(String)
            !b.include?(a.to_s)
          else
            !b.include?(a)
          end
          v.push(result)
          v.advance_pc
          nil
        })

        # NOT: Logical negation (not a).
        #
        # Starlark truthiness rules:
        #   - None, False, 0, "", [], {}, () are falsy
        #   - Everything else is truthy
        vm.register_opcode(op::NOT, ->(v, _instr, _c) {
          a = v.pop
          v.push(!starlark_truthy?(a))
          v.advance_pc
          nil
        })

        # ==============================================================
        # Control Flow
        # ==============================================================

        # JUMP: Unconditional jump to the operand address.
        #
        # Sets the program counter directly -- does NOT advance_pc.
        vm.register_opcode(op::JUMP, ->(v, instr, _c) {
          v.jump_to(instr.operand)
          nil
        })

        # JUMP_IF_FALSE: Pop a value; if falsy, jump to operand.
        #
        # Used for `if` conditions and `while` loops:
        #   if x > 0:     =>  CMP_GT, JUMP_IF_FALSE <else_label>
        vm.register_opcode(op::JUMP_IF_FALSE, ->(v, instr, _c) {
          val = v.pop
          if starlark_truthy?(val)
            v.advance_pc
          else
            v.jump_to(instr.operand)
          end
          nil
        })

        # JUMP_IF_TRUE: Pop a value; if truthy, jump to operand.
        vm.register_opcode(op::JUMP_IF_TRUE, ->(v, instr, _c) {
          val = v.pop
          if starlark_truthy?(val)
            v.jump_to(instr.operand)
          else
            v.advance_pc
          end
          nil
        })

        # JUMP_IF_FALSE_OR_POP: Short-circuit "and".
        #
        # If top-of-stack is falsy: leave it on the stack and jump.
        # If truthy: pop it and continue (evaluate the right operand).
        #
        # This implements `a and b`:
        #   push(a)
        #   JUMP_IF_FALSE_OR_POP <end>   -- if a is falsy, result is a
        #   push(b)                       -- if a is truthy, result is b
        #   <end>:
        vm.register_opcode(op::JUMP_IF_FALSE_OR_POP, ->(v, instr, _c) {
          val = v.peek
          if starlark_truthy?(val)
            v.pop
            v.advance_pc
          else
            v.jump_to(instr.operand)
          end
          nil
        })

        # JUMP_IF_TRUE_OR_POP: Short-circuit "or".
        #
        # If top-of-stack is truthy: leave it on the stack and jump.
        # If falsy: pop it and continue (evaluate the right operand).
        vm.register_opcode(op::JUMP_IF_TRUE_OR_POP, ->(v, instr, _c) {
          val = v.peek
          if starlark_truthy?(val)
            v.jump_to(instr.operand)
          else
            v.pop
            v.advance_pc
          end
          nil
        })

        # BREAK: Exit the innermost loop.
        #
        # In our implementation, BREAK is compiled into a JUMP to the
        # loop's exit label. This handler exists as a fallback but
        # the compiler should emit JUMP instead.
        vm.register_opcode(op::BREAK, ->(v, _instr, _c) {
          v.advance_pc
          nil
        })

        # CONTINUE: Jump to the loop header.
        #
        # Like BREAK, the compiler typically emits JUMP instead. This
        # handler exists for completeness.
        vm.register_opcode(op::CONTINUE, ->(v, _instr, _c) {
          v.advance_pc
          nil
        })

        # ==============================================================
        # Function Operations
        # ==============================================================

        # MAKE_FUNCTION: Create a StarlarkFunction from a compiled constant.
        #
        # The compiler stores a hash in the constants pool:
        #   { "code" => CodeObject, "params" => ["a", "b"], "default_count" => 1 }
        #
        # MAKE_FUNCTION reads this hash and pushes a StarlarkFunction
        # object onto the stack. Default values were pushed onto the
        # stack by the caller, so we pop default_count values.
        vm.register_opcode(op::MAKE_FUNCTION, ->(v, instr, c) {
          func_info = c.constants[instr.operand]
          func_code = func_info["code"]
          param_names = func_info["params"] || []
          default_count = func_info["default_count"] || 0

          # Pop default values from the stack (they were pushed before
          # MAKE_FUNCTION by the compiler).
          defaults = []
          default_count.times { defaults.unshift(v.pop) }

          func = StarlarkFunction.new(
            code: func_code,
            defaults: defaults,
            name: param_names.empty? ? "<lambda>" : "<function>",
            param_count: param_names.length,
            param_names: param_names
          )
          v.push(func)
          v.advance_pc
          nil
        })

        # CALL_FUNCTION: Call a function with positional arguments.
        #
        # The operand is the number of arguments. The stack layout is:
        #
        #   [callable, arg0, arg1, ..., argN-1]
        #
        # For StarlarkFunction:
        #   1. Pop N args, pop the callable
        #   2. Save the current execution state (push_frame)
        #   3. Set up locals with parameter bindings
        #   4. Jump into the function's CodeObject
        #
        # For BuiltinFunction:
        #   1. Pop N args, pop the callable
        #   2. Call the implementation directly
        #   3. Push the return value
        vm.register_opcode(op::CALL_FUNCTION, ->(v, instr, c) {
          arg_count = instr.operand
          args = []
          arg_count.times { args.unshift(v.pop) }
          callable = v.pop

          if callable.is_a?(StarlarkFunction)
            # call_starlark_function handles PC advancement internally
            call_starlark_function(v, callable, args, c)
          elsif callable.is_a?(CodingAdventures::VirtualMachine::BuiltinFunction)
            result = callable.implementation.call(args)
            v.push(result)
            v.advance_pc
          else
            raise "TypeError: '#{callable.inspect}' is not callable"
          end
          nil
        })

        # CALL_FUNCTION_KW: Call a function with keyword arguments.
        #
        # The operand is the total number of arguments (positional + keyword).
        # Keyword arguments are interleaved on the stack as name-value pairs:
        #
        #   [callable, pos_arg0, ..., kw_name0, kw_val0, kw_name1, kw_val1, ...]
        #
        # We detect keyword args by checking if a value is a string that
        # matches a parameter name. The compiler pushes LOAD_CONST for the
        # keyword name followed by the expression value.
        vm.register_opcode(op::CALL_FUNCTION_KW, ->(v, instr, c) {
          arg_count = instr.operand
          # Pop all args as a flat list
          all_args = []
          arg_count.times { all_args.unshift(v.pop) }
          callable = v.pop

          if callable.is_a?(StarlarkFunction)
            # call_starlark_function_kw handles PC advancement internally
            call_starlark_function_kw(v, callable, all_args, c)
          elsif callable.is_a?(CodingAdventures::VirtualMachine::BuiltinFunction)
            result = callable.implementation.call(all_args)
            v.push(result)
            v.advance_pc
          else
            raise "TypeError: '#{callable.inspect}' is not callable"
          end
          nil
        })

        # RETURN_VALUE: Return from the current function.
        #
        # Sets the VM's halted flag to signal the nested execution loop
        # in call_starlark_function to stop. The return value is left on
        # the stack -- the caller will pop it after restoring state.
        #
        # This works because function bodies run in a nested while loop
        # (inside call_starlark_function), not in the main execute() loop.
        # Setting halted=true exits the nested loop, and the caller
        # handles state restoration.
        vm.register_opcode(op::RETURN_VALUE, ->(v, _instr, _c) {
          # Leave the return value on the stack for the caller to grab
          v.halted = true
          nil
        })

        # ==============================================================
        # Collection Operations
        # ==============================================================

        # BUILD_LIST: Pop N items from the stack and push a list.
        #
        # The items are pushed left-to-right, so the first element is
        # deepest in the stack. We pop in reverse and then reverse again.
        #
        #   LOAD_CONST 0  (push 1)    stack: [1]
        #   LOAD_CONST 1  (push 2)    stack: [1, 2]
        #   LOAD_CONST 2  (push 3)    stack: [1, 2, 3]
        #   BUILD_LIST 3              stack: [[1, 2, 3]]
        vm.register_opcode(op::BUILD_LIST, ->(v, instr, _c) {
          items = []
          instr.operand.times { items.unshift(v.pop) }
          v.push(items)
          v.advance_pc
          nil
        })

        # BUILD_DICT: Pop N key-value pairs and push a hash.
        #
        # The operand is the number of PAIRS. Each pair is pushed as
        # (key, value), so we pop 2*N items total.
        #
        #   LOAD_CONST "a"
        #   LOAD_CONST 1
        #   LOAD_CONST "b"
        #   LOAD_CONST 2
        #   BUILD_DICT 2   =>  {"a" => 1, "b" => 2}
        vm.register_opcode(op::BUILD_DICT, ->(v, instr, _c) {
          dict = {}
          pairs = []
          (instr.operand * 2).times { pairs.unshift(v.pop) }
          pairs.each_slice(2) { |k, val| dict[k] = val }
          v.push(dict)
          v.advance_pc
          nil
        })

        # BUILD_TUPLE: Pop N items and push an array (Ruby doesn't have tuples).
        #
        # In Starlark, tuples are immutable. We represent them as frozen arrays.
        vm.register_opcode(op::BUILD_TUPLE, ->(v, instr, _c) {
          items = []
          instr.operand.times { items.unshift(v.pop) }
          v.push(items)
          v.advance_pc
          nil
        })

        # LIST_APPEND: Append the top-of-stack value to the list below it.
        #
        # Used in list comprehensions:
        #   BUILD_LIST 0     push empty list
        #   <loop body>
        #   LIST_APPEND       list.append(value)
        vm.register_opcode(op::LIST_APPEND, ->(v, _instr, _c) {
          val = v.pop
          list = v.peek
          list.push(val)
          v.advance_pc
          nil
        })

        # DICT_SET: Set a key-value pair in the dict on the stack.
        #
        # Stack: [..., dict, key, value]
        # After: [..., dict]  (dict now has key=>value)
        vm.register_opcode(op::DICT_SET, ->(v, _instr, _c) {
          val = v.pop
          key = v.pop
          dict = v.peek
          dict[key] = val
          v.advance_pc
          nil
        })

        # ==============================================================
        # Subscript and Attribute Access
        # ==============================================================

        # LOAD_SUBSCRIPT: Pop index, pop object, push object[index].
        #
        # Works with arrays (integer index) and hashes (key lookup).
        #   [10, 20, 30][1]  => 20
        #   {"a": 1}["a"]    => 1
        vm.register_opcode(op::LOAD_SUBSCRIPT, ->(v, _instr, _c) {
          index = v.pop
          obj = v.pop
          v.push(obj[index])
          v.advance_pc
          nil
        })

        # STORE_SUBSCRIPT: Pop value, pop index, pop object; set object[index] = value.
        #
        # Stack: [..., object, index, value]
        # After: [...]  (object[index] is now value)
        vm.register_opcode(op::STORE_SUBSCRIPT, ->(v, _instr, _c) {
          val = v.pop
          index = v.pop
          obj = v.pop
          obj[index] = val
          v.advance_pc
          nil
        })

        # LOAD_ATTR: Pop object, push object.attr_name.
        #
        # The operand indexes into the names table for the attribute name.
        # Used for method calls like list.append() or dict.keys().
        #
        # We handle common Starlark methods here:
        #   - list: append, extend, insert, remove, pop, clear, index, count
        #   - dict: keys, values, items, get, pop, update, clear, setdefault
        #   - string: upper, lower, strip, split, join, startswith, endswith,
        #             replace, find, format, count, capitalize, title
        vm.register_opcode(op::LOAD_ATTR, ->(v, instr, c) {
          attr_name = c.names[instr.operand]
          obj = v.pop
          method_impl = resolve_attribute(obj, attr_name)
          v.push(method_impl)
          v.advance_pc
          nil
        })

        # STORE_ATTR: Pop value, pop object; set object.attr = value.
        #
        # Starlark doesn't have true attribute assignment, but this opcode
        # exists for completeness. Used in struct-like patterns.
        vm.register_opcode(op::STORE_ATTR, ->(v, instr, c) {
          val = v.pop
          obj = v.pop
          attr_name = c.names[instr.operand]
          if obj.is_a?(Hash)
            obj[attr_name] = val
          end
          v.advance_pc
          nil
        })

        # LOAD_SLICE: Pop slice arguments, pop object, push slice result.
        #
        # For now, handles simple two-argument slicing: obj[start:stop].
        vm.register_opcode(op::LOAD_SLICE, ->(v, instr, _c) {
          slice_count = instr.operand || 2
          slice_args = []
          slice_count.times { slice_args.unshift(v.pop) }
          obj = v.pop

          start_idx = slice_args[0] || 0
          stop_idx = slice_args[1]

          if obj.is_a?(String) || obj.is_a?(Array)
            if stop_idx
              v.push(obj[start_idx...stop_idx])
            else
              v.push(obj[start_idx..])
            end
          else
            v.push(nil)
          end
          v.advance_pc
          nil
        })

        # ==============================================================
        # Iteration
        # ==============================================================

        # GET_ITER: Pop an iterable, push a StarlarkIterator.
        #
        # Converts arrays, hashes (iterates keys), strings (iterates chars),
        # and ranges into an iterator object.
        vm.register_opcode(op::GET_ITER, ->(v, _instr, _c) {
          iterable = v.pop
          items = case iterable
          when Array then iterable
          when Hash then iterable.keys
          when String then iterable.chars
          when Range then iterable.to_a
          else
            raise "TypeError: '#{iterable.class}' is not iterable"
          end
          v.push(StarlarkIterator.new(items))
          v.advance_pc
          nil
        })

        # FOR_ITER: Advance the iterator on top of stack.
        #
        # If the iterator has more items: push the next value and advance PC.
        # If exhausted: pop the iterator and jump to the operand address.
        #
        # This is the core of `for x in items:` loops:
        #
        #   GET_ITER              push iterator
        #   FOR_ITER <end>        push next item or jump to <end>
        #   STORE_NAME "x"        bind loop variable
        #   <loop body>
        #   JUMP <FOR_ITER>       repeat
        #   <end>:
        vm.register_opcode(op::FOR_ITER, ->(v, instr, _c) {
          iterator = v.peek
          if iterator.done?
            v.pop  # Remove exhausted iterator
            v.jump_to(instr.operand)
          else
            val = iterator.next_value
            v.push(val)
            v.advance_pc
          end
          nil
        })

        # UNPACK_SEQUENCE: Pop a sequence and push its elements individually.
        #
        # Used for tuple unpacking: `a, b = [1, 2]`
        #
        # The operand is the expected number of elements. Elements are
        # pushed in reverse order so that the first element ends up on top.
        vm.register_opcode(op::UNPACK_SEQUENCE, ->(v, instr, _c) {
          seq = v.pop
          expected = instr.operand
          if seq.length != expected
            raise "ValueError: not enough values to unpack " \
                  "(expected #{expected}, got #{seq.length})"
          end
          # Push in reverse so first element is on top of the stack
          seq.reverse_each { |item| v.push(item) }
          v.advance_pc
          nil
        })

        # ==============================================================
        # Module Loading (stub)
        # ==============================================================

        # LOAD_MODULE: Stub handler for module loading.
        #
        # The real implementation is provided by the interpreter layer,
        # which knows how to resolve file paths and compile other modules.
        # This stub pushes an empty hash as a placeholder.
        vm.register_opcode(op::LOAD_MODULE, ->(v, instr, c) {
          # The module path is stored in constants
          _module_path = c.constants[instr.operand]
          v.push({})  # Stub: push empty module namespace
          v.advance_pc
          nil
        })

        # IMPORT_FROM: Extract a named symbol from the module on top of stack.
        #
        # The module (a hash of name => value) is on top of the stack (DUP'd).
        # The operand indexes into names to get the symbol name.
        # Push the value associated with that name.
        vm.register_opcode(op::IMPORT_FROM, ->(v, instr, c) {
          name = c.names[instr.operand]
          mod = v.peek
          if mod.is_a?(Hash) && mod.key?(name)
            v.push(mod[name])
          else
            raise "ImportError: cannot import name '#{name}'"
          end
          v.advance_pc
          nil
        })

        # ==============================================================
        # I/O
        # ==============================================================

        # PRINT_VALUE: Pop a value and append its string representation to output.
        #
        # Returns the printed string so the trace can record it.
        vm.register_opcode(op::PRINT_VALUE, ->(v, _instr, _c) {
          val = v.pop
          str = starlark_repr(val)
          v.output.push(str)
          v.advance_pc
          str
        })

        # ==============================================================
        # VM Control
        # ==============================================================

        # HALT: Stop the virtual machine.
        #
        # Always the last instruction in a top-level compilation unit.
        vm.register_opcode(op::HALT, ->(v, _instr, _c) {
          v.halted = true
          v.advance_pc
          nil
        })
      end

      # ================================================================
      # Helper Methods
      # ================================================================

      # Determine Starlark truthiness for a value.
      #
      # Starlark follows Python's truthiness rules:
      #   - nil (None), false, 0, 0.0, "", [], {}, () are FALSY
      #   - Everything else is TRUTHY
      #
      # @param value [Object] any Starlark value
      # @return [Boolean]
      def self.starlark_truthy?(value)
        case value
        when nil then false
        when false then false
        when true then true
        when Integer, Float then value != 0
        when String then !value.empty?
        when Array then !value.empty?
        when Hash then !value.empty?
        else true
        end
      end

      # Convert a Starlark value to its string representation.
      #
      # This follows Python/Starlark conventions:
      #   - nil => "None"
      #   - true/false => "True"/"False"
      #   - strings are displayed without quotes (in print context)
      #   - lists use [a, b, c] notation
      #   - dicts use {k: v} notation
      #
      # @param value [Object] any Starlark value
      # @return [String]
      def self.starlark_repr(value)
        case value
        when nil then "None"
        when true then "True"
        when false then "False"
        when String then value
        when Array
          "[" + value.map { |v| starlark_repr_quoted(v) }.join(", ") + "]"
        when Hash
          "{" + value.map { |k, v|
            "#{starlark_repr_quoted(k)}: #{starlark_repr_quoted(v)}"
          }.join(", ") + "}"
        when StarlarkFunction
          "<function #{value.name}>"
        when CodingAdventures::VirtualMachine::BuiltinFunction
          "<built-in function #{value.name}>"
        else
          value.to_s
        end
      end

      # Like starlark_repr but adds quotes around strings.
      #
      # Used inside collections where strings need to be distinguishable
      # from other types: [1, "hello", True] not [1, hello, True].
      def self.starlark_repr_quoted(value)
        case value
        when String then "\"#{value}\""
        else starlark_repr(value)
        end
      end

      # Call a StarlarkFunction with positional arguments.
      #
      # This handles the mechanics of function calling:
      #   1. Validate argument count (considering defaults)
      #   2. Save the caller's state as a CallFrame
      #   3. Set up local variables with parameter bindings
      #   4. Execute the function body
      #   5. The RETURN_VALUE handler restores the caller's state
      def self.call_starlark_function(vm, func, args, code)
        param_count = func.param_count
        default_count = func.defaults.length

        # Fill in missing args with defaults
        if args.length < param_count
          missing = param_count - args.length
          if missing > default_count
            raise "TypeError: #{func.name}() takes #{param_count} " \
                  "arguments (#{args.length} given)"
          end
          # Defaults fill from the right: def f(a, b=1, c=2)
          # If called as f(10), then a=10, b=1, c=2
          defaults_start = default_count - missing
          args += func.defaults[defaults_start..]
        end

        # Save the caller's entire execution state.
        # We don't use GenericVM's call_stack here -- instead we save
        # everything on the Ruby stack (this method's local variables).
        saved_pc = vm.pc
        saved_halted = vm.halted
        saved_variables = vm.variables.dup
        saved_locals = vm.locals
        saved_stack = vm.stack.dup

        # Track recursion depth via the VM's call stack
        frame = CodingAdventures::VirtualMachine::CallFrame.new(
          return_address: 0,
          saved_variables: {},
          saved_locals: nil
        )
        vm.push_frame(frame)

        # Set up function's local scope with parameter bindings.
        new_locals = {}
        func.param_names.each_with_index do |name, i|
          new_locals[name] = args[i] if i < args.length
        end
        vm.locals = new_locals
        vm.stack = []
        vm.pc = 0
        vm.halted = false

        # Execute the function body in a nested loop.
        # RETURN_VALUE sets vm.halted = true and leaves the return value
        # on the stack. HALT also stops execution.
        while !vm.halted && vm.pc < func.code.instructions.length
          vm.step(func.code)
        end

        # Grab the return value (if any) from the function's stack.
        return_val = vm.stack.empty? ? nil : vm.pop

        # Pop the recursion tracking frame
        vm.pop_frame

        # Restore the caller's state.
        vm.pc = saved_pc + 1  # advance past CALL_FUNCTION
        vm.halted = saved_halted
        vm.variables = saved_variables
        vm.locals = saved_locals
        vm.stack = saved_stack

        # But keep any variable changes the function made to globals.
        # Actually, Starlark functions don't mutate global state (by design).
        # However, if the function modified a mutable object (list, dict)
        # that was passed as an argument, those changes are visible because
        # Ruby objects are reference types.

        # Push the return value onto the caller's stack.
        vm.push(return_val)
      end

      # Call a StarlarkFunction with keyword arguments.
      #
      # Keyword arguments arrive as alternating name-value pairs mixed
      # with positional arguments. We match them to parameter names.
      def self.call_starlark_function_kw(vm, func, all_args, code)
        # The compiler pushes keyword name strings before their values.
        # We need to separate positional from keyword args.
        # Keyword args: if an arg is a string that matches a param name
        # and the next arg exists, treat as keyword.
        param_names = func.param_names
        positional = []
        kwargs = {}

        i = 0
        while i < all_args.length
          val = all_args[i]
          if val.is_a?(String) && param_names.include?(val) && i + 1 < all_args.length
            kwargs[val] = all_args[i + 1]
            i += 2
          else
            positional << val
            i += 1
          end
        end

        # Build final args array matching parameter order
        final_args = Array.new(func.param_count)
        positional.each_with_index { |val, idx| final_args[idx] = val }
        kwargs.each do |name, val|
          idx = param_names.index(name)
          final_args[idx] = val if idx
        end

        # Fill remaining with defaults
        func.param_names.each_with_index do |name, idx|
          next unless final_args[idx].nil?
          default_offset = idx - (func.param_count - func.defaults.length)
          if default_offset >= 0 && default_offset < func.defaults.length
            final_args[idx] = func.defaults[default_offset]
          end
        end

        call_starlark_function(vm, func, final_args, code)
      end

      # Resolve an attribute access on a Starlark object.
      #
      # Returns a BuiltinFunction wrapping the method implementation,
      # so that `obj.method(args)` works via LOAD_ATTR + CALL_FUNCTION.
      #
      # Supported attributes by type:
      #
      # Arrays (lists):
      #   - append(item)     -- add item to end
      #   - extend(items)    -- add all items from another list
      #   - insert(i, item)  -- insert item at index i
      #   - remove(item)     -- remove first occurrence
      #   - pop([i])         -- remove and return item at index (default: last)
      #   - clear()          -- remove all items
      #   - index(item)      -- return index of first occurrence
      #   - count(item)      -- count occurrences
      #   - reverse()        -- reverse in place
      #   - sort()           -- sort in place
      #
      # Hashes (dicts):
      #   - keys()           -- list of keys
      #   - values()         -- list of values
      #   - items()          -- list of [key, value] pairs
      #   - get(key[, default]) -- get value or default
      #   - pop(key[, default]) -- remove and return value
      #   - update(other)    -- merge another dict
      #   - clear()          -- remove all pairs
      #   - setdefault(k, v) -- set if absent, return value
      #
      # Strings:
      #   - upper()          -- uppercase copy
      #   - lower()          -- lowercase copy
      #   - strip()          -- remove leading/trailing whitespace
      #   - split([sep])     -- split into list
      #   - join(items)      -- join list with separator
      #   - startswith(s)    -- check prefix
      #   - endswith(s)      -- check suffix
      #   - replace(old, new)-- replace occurrences
      #   - find(sub)        -- find index of substring (-1 if not found)
      #   - count(sub)       -- count occurrences
      #   - capitalize()     -- capitalize first letter
      #   - title()          -- title case
      #   - format(*args)    -- string formatting
      def self.resolve_attribute(obj, attr_name)
        case obj
        when Array
          resolve_list_attr(obj, attr_name)
        when Hash
          resolve_dict_attr(obj, attr_name)
        when String
          resolve_string_attr(obj, attr_name)
        else
          raise "AttributeError: '#{obj.class}' has no attribute '#{attr_name}'"
        end
      end

      # Resolve a list method into a callable BuiltinFunction.
      def self.resolve_list_attr(list, attr_name)
        impl = case attr_name
        when "append"
          ->(args) { list.push(args[0]); list }
        when "extend"
          ->(args) { list.concat(args[0]); list }
        when "insert"
          ->(args) { list.insert(args[0], args[1]); list }
        when "remove"
          ->(args) { list.delete_at(list.index(args[0])); list }
        when "pop"
          ->(args) { args.empty? ? list.pop : list.delete_at(args[0]) }
        when "clear"
          ->(_args) { list.clear; list }
        when "index"
          ->(args) { list.index(args[0]) }
        when "count"
          ->(args) { list.count(args[0]) }
        when "reverse"
          ->(_args) { list.reverse!; list }
        when "sort"
          ->(_args) { list.sort!; list }
        else
          raise "AttributeError: 'list' has no attribute '#{attr_name}'"
        end
        CodingAdventures::VirtualMachine::BuiltinFunction.new(
          name: attr_name, implementation: impl
        )
      end

      # Resolve a dict method into a callable BuiltinFunction.
      def self.resolve_dict_attr(dict, attr_name)
        impl = case attr_name
        when "keys"
          ->(_args) { dict.keys }
        when "values"
          ->(_args) { dict.values }
        when "items"
          ->(_args) { dict.map { |k, v| [k, v] } }
        when "get"
          ->(args) { dict.key?(args[0]) ? dict[args[0]] : args[1] }
        when "pop"
          ->(args) {
            if dict.key?(args[0])
              dict.delete(args[0])
            elsif args.length > 1
              args[1]
            else
              raise "KeyError: #{args[0].inspect}"
            end
          }
        when "update"
          ->(args) { dict.merge!(args[0]); dict }
        when "clear"
          ->(_args) { dict.clear; dict }
        when "setdefault"
          ->(args) {
            dict[args[0]] = args[1] unless dict.key?(args[0])
            dict[args[0]]
          }
        else
          raise "AttributeError: 'dict' has no attribute '#{attr_name}'"
        end
        CodingAdventures::VirtualMachine::BuiltinFunction.new(
          name: attr_name, implementation: impl
        )
      end

      # Resolve a string method into a callable BuiltinFunction.
      def self.resolve_string_attr(str, attr_name)
        impl = case attr_name
        when "upper"
          ->(_args) { str.upcase }
        when "lower"
          ->(_args) { str.downcase }
        when "strip"
          ->(_args) { str.strip }
        when "lstrip"
          ->(_args) { str.lstrip }
        when "rstrip"
          ->(_args) { str.rstrip }
        when "split"
          ->(args) {
            if args.empty? || args[0].nil?
              str.split
            else
              str.split(args[0])
            end
          }
        when "join"
          ->(args) { args[0].join(str) }
        when "startswith"
          ->(args) { str.start_with?(args[0]) }
        when "endswith"
          ->(args) { str.end_with?(args[0]) }
        when "replace"
          ->(args) { str.gsub(args[0], args[1]) }
        when "find"
          ->(args) { str.index(args[0]) || -1 }
        when "count"
          ->(args) { str.scan(args[0]).length }
        when "capitalize"
          ->(_args) { str.capitalize }
        when "title"
          ->(_args) {
            str.split.map(&:capitalize).join(" ")
          }
        when "format"
          ->(args) {
            result = str.dup
            args.each_with_index do |arg, i|
              result.gsub!("{#{i}}", arg.to_s)
              result.gsub!("{}", arg.to_s) if i == 0
            end
            result
          }
        when "elems"
          ->(_args) { str.chars }
        else
          raise "AttributeError: 'string' has no attribute '#{attr_name}'"
        end
        CodingAdventures::VirtualMachine::BuiltinFunction.new(
          name: attr_name, implementation: impl
        )
      end
    end
  end
end
