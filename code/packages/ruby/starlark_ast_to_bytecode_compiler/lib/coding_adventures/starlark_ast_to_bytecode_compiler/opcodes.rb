# frozen_string_literal: true

# ==========================================================================
# Starlark Opcodes -- All 46 Bytecode Instructions for Starlark
# ==========================================================================
#
# This module defines every opcode the Starlark compiler can emit. These
# opcodes form the instruction set of our virtual machine when running
# Starlark programs.
#
# Opcodes are grouped by category, with each group occupying a distinct
# range of the 0x00-0xFF byte space:
#
#   0x01-0x06  Stack operations (load constants, push/pop)
#   0x10-0x15  Variable access (global, local, closure)
#   0x20-0x2D  Arithmetic and bitwise operations
#   0x30-0x38  Comparison and boolean operations
#   0x40-0x46  Control flow (jumps, break, continue)
#   0x50-0x53  Function operations (define, call, return)
#   0x60-0x64  Collection operations (list, dict, tuple)
#   0x70-0x74  Subscript and attribute access
#   0x80-0x82  Iteration (for loops)
#   0x90-0x91  Module loading (load statement)
#   0xA0       I/O (print)
#   0xFF       VM control (halt)
#
# Why 46 opcodes? Starlark is a deliberately small language -- no classes,
# no exceptions, no imports (only load). But it has enough features
# (functions, closures, collections, iteration) to need a solid set of
# instructions. Each opcode maps to exactly one VM operation.
#
# The NAMES hash provides human-readable disassembly. When debugging
# bytecode, you can look up any opcode to see its name instead of a
# raw hex value.
# ==========================================================================

module CodingAdventures
  module StarlarkAstToBytecodeCompiler
    module Op
      # ================================================================
      # Stack Operations
      # ================================================================
      #
      # These opcodes manipulate the VM's operand stack directly.
      # LOAD_CONST pushes a value from the constant pool.
      # POP discards the top value (used after expression statements).
      # DUP duplicates the top value (used in load statements).
      # LOAD_NONE/TRUE/FALSE push common singletons without needing
      # a constant pool entry.

      LOAD_CONST = 0x01  # Push constants[operand] onto the stack
      POP        = 0x02  # Discard top of stack
      DUP        = 0x03  # Duplicate top of stack
      LOAD_NONE  = 0x04  # Push None
      LOAD_TRUE  = 0x05  # Push True
      LOAD_FALSE = 0x06  # Push False

      # ================================================================
      # Variable Access
      # ================================================================
      #
      # Two tiers of variable access:
      #   - NAME: global scope (module level)
      #   - LOCAL: function scope (faster, index-based)
      #   - CLOSURE: captured variables from enclosing scope
      #
      # The operand is always an index into the names table.
      # At module level, the compiler emits STORE_NAME/LOAD_NAME.
      # Inside a function body, it emits STORE_LOCAL/LOAD_LOCAL.

      STORE_NAME    = 0x10  # names[operand] = pop()
      LOAD_NAME     = 0x11  # push(names[operand])
      STORE_LOCAL   = 0x12  # locals[operand] = pop()
      LOAD_LOCAL    = 0x13  # push(locals[operand])
      STORE_CLOSURE = 0x14  # closure[operand] = pop()
      LOAD_CLOSURE  = 0x15  # push(closure[operand])

      # ================================================================
      # Arithmetic Operations
      # ================================================================
      #
      # All binary arithmetic ops pop two values and push the result.
      # They follow standard stack-machine convention:
      #
      #   push(a)    stack: [a]
      #   push(b)    stack: [a, b]
      #   ADD        stack: [a + b]
      #
      # NEGATE is unary: pop one value, push its negation.
      # BIT_NOT is unary: pop one value, push its bitwise complement.
      #
      # Starlark supports // (floor division) and ** (exponentiation)
      # in addition to the standard +, -, *, /, %.

      ADD       = 0x20  # push(pop() + pop())  -- note: operand order matters
      SUB       = 0x21  # push(a - b)  where a was pushed first
      MUL       = 0x22  # push(a * b)
      DIV       = 0x23  # push(a / b)  -- true division
      FLOOR_DIV = 0x24  # push(a // b) -- floor division
      MOD       = 0x25  # push(a % b)
      POWER     = 0x26  # push(a ** b)
      NEGATE    = 0x27  # push(-pop()) -- unary minus
      BIT_AND   = 0x28  # push(a & b)
      BIT_OR    = 0x29  # push(a | b)
      BIT_XOR   = 0x2A  # push(a ^ b)
      BIT_NOT   = 0x2B  # push(~pop()) -- unary bitwise NOT
      LSHIFT    = 0x2C  # push(a << b)
      RSHIFT    = 0x2D  # push(a >> b)

      # ================================================================
      # Comparison Operations
      # ================================================================
      #
      # Each comparison pops two values and pushes True or False.
      # CMP_IN and CMP_NOT_IN check membership in collections.
      # NOT is the logical negation operator (not a comparison, but
      # grouped here because it produces a boolean result).

      CMP_EQ     = 0x30  # push(a == b)
      CMP_LT     = 0x31  # push(a < b)
      CMP_GT     = 0x32  # push(a > b)
      CMP_NE     = 0x33  # push(a != b)
      CMP_LE     = 0x34  # push(a <= b)
      CMP_GE     = 0x35  # push(a >= b)
      CMP_IN     = 0x36  # push(a in b)
      CMP_NOT_IN = 0x37  # push(a not in b)
      NOT        = 0x38  # push(not pop())

      # ================================================================
      # Control Flow
      # ================================================================
      #
      # JUMP: unconditional jump to operand address.
      # JUMP_IF_FALSE: pop value; jump if falsy.
      # JUMP_IF_TRUE: pop value; jump if truthy.
      # JUMP_IF_FALSE_OR_POP: if top is falsy, jump (keep value);
      #   otherwise pop and continue. Used for short-circuit "and".
      # JUMP_IF_TRUE_OR_POP: if top is truthy, jump (keep value);
      #   otherwise pop and continue. Used for short-circuit "or".
      # BREAK/CONTINUE: loop control -- the VM handles unwinding.

      JUMP                 = 0x40  # PC = operand
      JUMP_IF_FALSE        = 0x41  # if !pop(): PC = operand
      JUMP_IF_TRUE         = 0x42  # if pop(): PC = operand
      JUMP_IF_FALSE_OR_POP = 0x43  # short-circuit "and"
      JUMP_IF_TRUE_OR_POP  = 0x44  # short-circuit "or"
      BREAK                = 0x45  # exit innermost loop
      CONTINUE             = 0x46  # jump to loop header

      # ================================================================
      # Function Operations
      # ================================================================
      #
      # MAKE_FUNCTION: creates a function object from a constant that
      #   contains a CodeObject, parameter names, and default count.
      # CALL_FUNCTION: calls the function on top of stack with operand
      #   positional arguments below it.
      # CALL_FUNCTION_KW: like CALL_FUNCTION but arguments include
      #   keyword name/value pairs.
      # RETURN_VALUE: pops the return value and returns from function.

      MAKE_FUNCTION    = 0x50  # push(Function(constants[operand]))
      CALL_FUNCTION    = 0x51  # call with operand positional args
      CALL_FUNCTION_KW = 0x52  # call with keyword args
      RETURN_VALUE     = 0x53  # return pop() from current function

      # ================================================================
      # Collection Operations
      # ================================================================
      #
      # BUILD_LIST/DICT/TUPLE: pop operand items and build a collection.
      # For BUILD_DICT, operand is the number of key-value PAIRS, so
      # it pops 2*operand items (alternating key, value).
      # LIST_APPEND and DICT_SET mutate collections in place (used in
      # comprehensions).

      BUILD_LIST  = 0x60  # pop operand items, push list
      BUILD_DICT  = 0x61  # pop operand*2 items, push dict
      BUILD_TUPLE = 0x62  # pop operand items, push tuple
      LIST_APPEND = 0x63  # list.append(pop())
      DICT_SET    = 0x64  # dict[key] = value

      # ================================================================
      # Subscript and Attribute Access
      # ================================================================
      #
      # LOAD_SUBSCRIPT: pop index, pop object, push object[index].
      # STORE_SUBSCRIPT: pop value, pop index, pop object; object[index] = value.
      # LOAD_ATTR: pop object, push object.attr (attr name from names pool).
      # STORE_ATTR: pop value, pop object; object.attr = value.
      # LOAD_SLICE: pop slice args, pop object, push object[start:stop].

      LOAD_SUBSCRIPT  = 0x70  # push(pop_obj()[pop_index()])
      STORE_SUBSCRIPT = 0x71  # pop_obj()[pop_index()] = pop_value()
      LOAD_ATTR       = 0x72  # push(pop().attr) where attr = names[operand]
      STORE_ATTR      = 0x73  # pop().attr = pop_value()
      LOAD_SLICE      = 0x74  # push(obj[start:stop])

      # ================================================================
      # Iteration
      # ================================================================
      #
      # GET_ITER: pop an iterable, push an iterator.
      # FOR_ITER: advance iterator; if exhausted, jump to operand address;
      #   otherwise push the next value.
      # UNPACK_SEQUENCE: pop a sequence, push its elements in order.
      #   operand = expected number of elements.

      GET_ITER        = 0x80  # push(iter(pop()))
      FOR_ITER        = 0x81  # next(TOS) or jump to operand
      UNPACK_SEQUENCE = 0x82  # unpack TOS into operand values

      # ================================================================
      # Module Loading
      # ================================================================
      #
      # Starlark's load() statement imports symbols from another file.
      # LOAD_MODULE: push the module object (path from constants pool).
      # IMPORT_FROM: extract a named symbol from the module on top of stack.

      LOAD_MODULE = 0x90  # push(load_module(constants[operand]))
      IMPORT_FROM = 0x91  # push(TOS.get(names[operand]))

      # ================================================================
      # I/O
      # ================================================================
      #
      # PRINT_VALUE is a built-in for Starlark's print() function.
      # It pops and prints the top of stack.

      PRINT_VALUE = 0xA0  # print(pop())

      # ================================================================
      # VM Control
      # ================================================================
      #
      # HALT stops execution. It is always the last instruction emitted
      # by the compiler for a top-level compilation.

      HALT = 0xFF  # stop the virtual machine

      # ================================================================
      # Human-Readable Names
      # ================================================================
      #
      # Maps each opcode integer to its string name. Used for
      # disassembly, debugging, and test output.

      NAMES = {
        LOAD_CONST => "LOAD_CONST",
        POP => "POP",
        DUP => "DUP",
        LOAD_NONE => "LOAD_NONE",
        LOAD_TRUE => "LOAD_TRUE",
        LOAD_FALSE => "LOAD_FALSE",
        STORE_NAME => "STORE_NAME",
        LOAD_NAME => "LOAD_NAME",
        STORE_LOCAL => "STORE_LOCAL",
        LOAD_LOCAL => "LOAD_LOCAL",
        STORE_CLOSURE => "STORE_CLOSURE",
        LOAD_CLOSURE => "LOAD_CLOSURE",
        ADD => "ADD",
        SUB => "SUB",
        MUL => "MUL",
        DIV => "DIV",
        FLOOR_DIV => "FLOOR_DIV",
        MOD => "MOD",
        POWER => "POWER",
        NEGATE => "NEGATE",
        BIT_AND => "BIT_AND",
        BIT_OR => "BIT_OR",
        BIT_XOR => "BIT_XOR",
        BIT_NOT => "BIT_NOT",
        LSHIFT => "LSHIFT",
        RSHIFT => "RSHIFT",
        CMP_EQ => "CMP_EQ",
        CMP_LT => "CMP_LT",
        CMP_GT => "CMP_GT",
        CMP_NE => "CMP_NE",
        CMP_LE => "CMP_LE",
        CMP_GE => "CMP_GE",
        CMP_IN => "CMP_IN",
        CMP_NOT_IN => "CMP_NOT_IN",
        NOT => "NOT",
        JUMP => "JUMP",
        JUMP_IF_FALSE => "JUMP_IF_FALSE",
        JUMP_IF_TRUE => "JUMP_IF_TRUE",
        JUMP_IF_FALSE_OR_POP => "JUMP_IF_FALSE_OR_POP",
        JUMP_IF_TRUE_OR_POP => "JUMP_IF_TRUE_OR_POP",
        BREAK => "BREAK",
        CONTINUE => "CONTINUE",
        MAKE_FUNCTION => "MAKE_FUNCTION",
        CALL_FUNCTION => "CALL_FUNCTION",
        CALL_FUNCTION_KW => "CALL_FUNCTION_KW",
        RETURN_VALUE => "RETURN_VALUE",
        BUILD_LIST => "BUILD_LIST",
        BUILD_DICT => "BUILD_DICT",
        BUILD_TUPLE => "BUILD_TUPLE",
        LIST_APPEND => "LIST_APPEND",
        DICT_SET => "DICT_SET",
        LOAD_SUBSCRIPT => "LOAD_SUBSCRIPT",
        STORE_SUBSCRIPT => "STORE_SUBSCRIPT",
        LOAD_ATTR => "LOAD_ATTR",
        STORE_ATTR => "STORE_ATTR",
        LOAD_SLICE => "LOAD_SLICE",
        GET_ITER => "GET_ITER",
        FOR_ITER => "FOR_ITER",
        UNPACK_SEQUENCE => "UNPACK_SEQUENCE",
        LOAD_MODULE => "LOAD_MODULE",
        IMPORT_FROM => "IMPORT_FROM",
        PRINT_VALUE => "PRINT_VALUE",
        HALT => "HALT"
      }.freeze
    end
  end
end
