# frozen_string_literal: true

# ================================================================
# Lisp VM — GenericVM Configured for McCarthy's 1960 Lisp
# ================================================================
#
# This factory creates a GenericVM and registers all Lisp opcodes.
# It mirrors the Python create_lisp_vm() function precisely:
# each opcode handler is a lambda that captures GC and symbol_table
# references via closures.
#
# The VM's stack holds:
#   - Ruby integers and floats (for numbers)
#   - Heap addresses (integers >= 0x10000) for cons cells, symbols, closures
#   - LispOp::NIL_SENTINEL for Lisp's NIL value
#   - true / false for boolean results
#
# Why use a sentinel for NIL?
# Ruby's nil is used for "no value" in our framework. Using LispOp::NIL_SENTINEL
# (the symbol :__lisp_nil__) lets us distinguish "no value" from Lisp's NIL.
#
# IMPORTANT: Every handler must call vn.advance_pc at the end, EXCEPT
# unconditional JUMP which calls vn.jump_to directly. Conditional jumps
# must call either vn.advance_pc (fall-through) or vn.jump_to (taken).
# Forgetting advance_pc causes the VM to loop forever on the same instruction.
# ================================================================

require "coding_adventures_virtual_machine"
require "coding_adventures_garbage_collector"
require_relative "opcodes"

module CodingAdventures
  module LispVm
    NIL = LispOp::NIL_SENTINEL

    GC_MOD = CodingAdventures::GarbageCollector
    VM_MOD = CodingAdventures::VirtualMachine

    # Create a GenericVM configured for McCarthy's Lisp.
    #
    # @param gc [MarkAndSweepGC, nil] optional GC instance; default is fresh MarkAndSweepGC
    # @return [CodingAdventures::VirtualMachine::GenericVM]
    def self.create_lisp_vm(gc: nil)
      vm  = VM_MOD::GenericVM.new
      gc  = gc || GC_MOD::MarkAndSweepGC.new
      st  = GC_MOD::SymbolTable.new(gc)

      # Expose GC and symbol table on the VM for external access (tests, debuggers)
      vm.instance_variable_set(:@lisp_gc, gc)
      vm.instance_variable_set(:@lisp_symbol_table, st)
      vm.define_singleton_method(:lisp_gc)           { @lisp_gc }
      vm.define_singleton_method(:lisp_symbol_table) { @lisp_symbol_table }

      # ================================================================
      # Stack Operations
      # ================================================================

      vm.register_opcode(LispOp::LOAD_CONST, lambda do |vn, instr, code|
        vn.stack.push(code.constants[instr.operand])
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::POP, lambda do |vn, _instr, _code|
        vn.stack.pop
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::LOAD_NIL, lambda do |vn, _instr, _code|
        vn.stack.push(NIL)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::LOAD_TRUE, lambda do |vn, _instr, _code|
        vn.stack.push(true)
        vn.advance_pc
        nil
      end)

      # ================================================================
      # Variable Operations
      # ================================================================

      vm.register_opcode(LispOp::STORE_NAME, lambda do |vn, instr, code|
        vn.variables[code.names[instr.operand]] = vn.stack.pop
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::LOAD_NAME, lambda do |vn, instr, code|
        name = code.names[instr.operand]
        vn.stack.push(vn.variables[name])
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::STORE_LOCAL, lambda do |vn, instr, _code|
        vn.locals[instr.operand] = vn.stack.pop
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::LOAD_LOCAL, lambda do |vn, instr, _code|
        vn.stack.push(vn.locals[instr.operand])
        vn.advance_pc
        nil
      end)

      # ================================================================
      # Arithmetic Operations
      # ================================================================

      vm.register_opcode(LispOp::ADD, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop; vn.stack.push(a + b)
        vn.advance_pc; nil
      end)

      vm.register_opcode(LispOp::SUB, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop; vn.stack.push(a - b)
        vn.advance_pc; nil
      end)

      vm.register_opcode(LispOp::MUL, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop; vn.stack.push(a * b)
        vn.advance_pc; nil
      end)

      vm.register_opcode(LispOp::DIV, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop; vn.stack.push(a / b)
        vn.advance_pc; nil
      end)

      # ================================================================
      # Comparison Operations
      # ================================================================

      vm.register_opcode(LispOp::CMP_EQ, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop
        vn.stack.push(a == b ? true : NIL)
        vn.advance_pc; nil
      end)

      vm.register_opcode(LispOp::CMP_LT, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop
        vn.stack.push(a < b ? true : NIL)
        vn.advance_pc; nil
      end)

      vm.register_opcode(LispOp::CMP_GT, lambda do |vn, _i, _c|
        b = vn.stack.pop; a = vn.stack.pop
        vn.stack.push(a > b ? true : NIL)
        vn.advance_pc; nil
      end)

      # ================================================================
      # Control Flow
      # ================================================================

      # Unconditional jump — set PC to operand directly (no advance_pc)
      vm.register_opcode(LispOp::JUMP, lambda do |vn, instr, _c|
        vn.jump_to(instr.operand)
        nil
      end)

      # Jump if falsy (NIL or false) — otherwise fall through
      vm.register_opcode(LispOp::JUMP_IF_FALSE, lambda do |vn, instr, _c|
        cond = vn.stack.pop
        if cond == NIL || cond == false
          vn.jump_to(instr.operand)
        else
          vn.advance_pc
        end
        nil
      end)

      # Jump if truthy — otherwise fall through
      vm.register_opcode(LispOp::JUMP_IF_TRUE, lambda do |vn, instr, _c|
        cond = vn.stack.pop
        if cond != NIL && cond != false
          vn.jump_to(instr.operand)
        else
          vn.advance_pc
        end
        nil
      end)

      # ================================================================
      # Function Operations
      # ================================================================

      vm.register_opcode(LispOp::MAKE_CLOSURE, lambda do |vn, instr, code|
        fn_code  = code.constants[instr.operand]
        captured = vn.variables.dup
        closure  = GC_MOD::LispClosure.new(code: fn_code, env: captured, params: fn_code.names)
        addr     = gc.allocate(closure)
        vn.stack.push(addr)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::CALL_FUNCTION, lambda do |vn, instr, _code|
        arg_count = instr.operand
        # The compiler emits: push arg0, push arg1, ..., push closure, CALL_FUNCTION n
        # So on the stack: bottom → [arg0, arg1, ..., closure] ← top
        # Pop the closure first (top of stack), then pop args (next layer down).
        closure_addr = vn.stack.pop
        args = []
        arg_count.times { args.unshift(vn.stack.pop) }
        closure      = gc.deref(closure_addr)
        # Save current frame state
        saved = { pc: vn.pc, vars: vn.variables.dup, locals: vn.locals.dup,
                  halted: vn.halted }
        vn.call_stack.push(saved)
        # Set up new frame with closure's environment + arguments bound to params
        new_vars = closure.env.dup
        closure.params.each_with_index { |p, i| new_vars[p] = args[i] }
        vn.variables = new_vars
        vn.locals    = []
        vn.halted    = false
        vn.pc        = 0
        # Execute the closure's code object synchronously
        vn.execute(closure.code)
        result = vn.stack.pop
        # Restore the caller's frame
        saved_frame  = vn.call_stack.pop
        vn.pc        = saved_frame[:pc]
        vn.variables = saved_frame[:vars]
        vn.locals    = saved_frame[:locals]
        vn.halted    = saved_frame[:halted]
        # Push function's return value onto caller's stack, then advance PC
        vn.stack.push(result)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::RETURN, lambda do |vn, _i, _c|
        vn.halted = true
        nil
      end)

      # ================================================================
      # Lisp-Specific Operations
      # ================================================================

      vm.register_opcode(LispOp::CONS, lambda do |vn, _i, _c|
        cdr_val = vn.stack.pop
        car_val = vn.stack.pop
        cell    = GC_MOD::ConsCell.new(car: car_val, cdr: cdr_val)
        addr    = gc.allocate(cell)
        vn.stack.push(addr)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::CAR, lambda do |vn, _i, _c|
        addr = vn.stack.pop
        cell = gc.deref(addr)
        vn.stack.push(cell.car)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::CDR, lambda do |vn, _i, _c|
        addr = vn.stack.pop
        cell = gc.deref(addr)
        vn.stack.push(cell.cdr)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::IS_ATOM, lambda do |vn, _i, _c|
        val     = vn.stack.pop
        is_atom = !(val.is_a?(Integer) && gc.valid_address?(val) &&
                    gc.deref(val).is_a?(GC_MOD::ConsCell))
        vn.stack.push(is_atom ? true : NIL)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::IS_NIL, lambda do |vn, _i, _c|
        val = vn.stack.pop
        vn.stack.push(val == NIL ? true : NIL)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::MAKE_SYMBOL, lambda do |vn, instr, code|
        name = code.constants[instr.operand]
        addr = st.intern(name)
        vn.stack.push(addr)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::PRINT, lambda do |vn, _i, _c|
        val = vn.stack.pop
        str = lisp_value_to_s(val, gc)
        vn.output << str
        vn.stack.push(NIL)
        vn.advance_pc
        nil
      end)

      vm.register_opcode(LispOp::HALT, lambda do |vn, _i, _c|
        vn.halted = true
        nil
      end)

      vm
    end

    # Convert a Lisp VM value to its printable string representation.
    def self.lisp_value_to_s(val, gc)
      case val
      when LispOp::NIL_SENTINEL then "()"
      when Integer
        if gc.valid_address?(val)
          obj = gc.deref(val)
          case obj
          when GC_MOD::ConsCell then cons_to_s(obj, gc)
          when GC_MOD::LispSymbol then obj.name
          when GC_MOD::LispClosure then "#<closure>"
          else val.to_s
          end
        else
          val.to_s
        end
      when true then "t"
      when false then "()"
      else val.to_s
      end
    end

    def self.cons_to_s(cell, gc)
      parts = ["("]
      current = cell
      loop do
        parts << lisp_value_to_s(current.car, gc)
        cdr = current.cdr
        if cdr == LispOp::NIL_SENTINEL
          break
        elsif cdr.is_a?(Integer) && gc.valid_address?(cdr) && gc.deref(cdr).is_a?(GC_MOD::ConsCell)
          parts << " "
          current = gc.deref(cdr)
        else
          parts << " . "
          parts << lisp_value_to_s(cdr, gc)
          break
        end
      end
      parts << ")"
      parts.join
    end
  end
end
