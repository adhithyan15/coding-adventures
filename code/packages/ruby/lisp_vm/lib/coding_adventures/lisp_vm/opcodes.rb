# frozen_string_literal: true

# ================================================================
# Lisp Opcodes — Instruction Set for McCarthy's 1960 Lisp VM
# ================================================================
#
# The GenericVM has no built-in opcodes. Languages register their own
# via register_opcode(number, handler). This module defines the opcode
# numbers and names for the Lisp VM.
#
# Lisp needs fewer opcodes than Starlark because Lisp's data model is
# minimal: atoms (numbers, symbols) and cons cells (pairs). No lists,
# dicts, tuples, or attribute access.
#
# However, Lisp adds Cons-cell operations that Starlark doesn't need:
#   CONS, CAR, CDR, IS_ATOM, IS_NIL, MAKE_SYMBOL
#
# Opcode organization (high nibble = category):
#
#   0x0_ Stack operations     — push constants, nil, true
#   0x1_ Variable operations  — store/load by name or index
#   0x2_ Arithmetic           — add, sub, mul, div
#   0x3_ Comparison           — eq, lt, gt
#   0x4_ Control flow         — jump, conditional branch
#   0x5_ Functions            — make closure, call, tail call, return
#   0x7_ Lisp-specific        — cons, car, cdr, predicates, symbols
#   0xA_ I/O                  — print
#   0xF_ VM control           — halt
# ================================================================

module CodingAdventures
  module LispVm
    module LispOp
      # Stack Operations (0x0_)
      LOAD_CONST = 0x01  # Push constants[operand] → value
      POP        = 0x02  # Discard top of stack value →
      LOAD_NIL   = 0x03  # Push NIL sentinel → NIL
      LOAD_TRUE  = 0x04  # Push true → true

      # Variable Operations (0x1_)
      STORE_NAME  = 0x10  # names[operand] = pop()
      LOAD_NAME   = 0x11  # push(names[operand])
      STORE_LOCAL = 0x12  # locals[operand] = pop()
      LOAD_LOCAL  = 0x13  # push(locals[operand])

      # Arithmetic (0x2_)
      ADD = 0x20  # a b → a+b
      SUB = 0x21  # a b → a-b
      MUL = 0x22  # a b → a*b
      DIV = 0x23  # a b → a/b

      # Comparison (0x3_)
      CMP_EQ = 0x30  # a b → (a == b)
      CMP_LT = 0x31  # a b → (a < b)
      CMP_GT = 0x32  # a b → (a > b)

      # Control Flow (0x4_)
      JUMP          = 0x40  # pc = operand
      JUMP_IF_FALSE = 0x41  # cond → (jump if cond is nil/false)
      JUMP_IF_TRUE  = 0x42  # cond → (jump if cond is not nil/false)

      # Functions (0x5_)
      MAKE_CLOSURE  = 0x50  # Create a closure from code object and env → closure_addr
      CALL_FUNCTION = 0x51  # args... func → result
      TAIL_CALL     = 0x52  # args... func → result (reuse frame)
      RETURN        = 0x53  # value →  (pop call frame, push to caller)

      # Lisp-Specific (0x7_)
      CONS        = 0x70  # cdr car → cons_addr
      CAR         = 0x71  # cons_addr → car value
      CDR         = 0x72  # cons_addr → cdr value
      IS_ATOM     = 0x73  # value → bool (true if not a cons cell)
      IS_NIL      = 0x74  # value → bool (true if NIL)
      MAKE_SYMBOL = 0x75  # Push interned symbol → symbol_addr
      PRINT       = 0xA0  # value → (print to output, push nil)

      # VM Control (0xF_)
      HALT = 0xFF  # Stop execution

      NIL_SENTINEL = :__lisp_nil__

      NAMES = constants.each_with_object({}) do |c, h|
        h[const_get(c)] = c.to_s if const_get(c).is_a?(Integer)
      end.freeze

      def self.name_of(opcode)
        NAMES[opcode] || format("0x%02X", opcode)
      end
    end
  end
end
