# frozen_string_literal: true

# ==========================================================================
# Opcodes -- The Complete Register-VM Instruction Set (~70 opcodes)
# ==========================================================================
#
# This VM follows the V8 Ignition execution model. V8 Ignition is Google's
# JavaScript bytecode interpreter. Its key insight is the ACCUMULATOR:
# instead of an operand stack (like the JVM or Python's ceval loop), most
# arithmetic and comparison results are implicitly stored in a single
# "accumulator" register. This eliminates many push/pop instructions.
#
# Instruction categories:
#
#   0x00–0x0F  Accumulator loads         — LDA_* opcodes
#   0x10–0x1F  Register moves            — MOV, LDAR, STAR
#   0x20–0x2F  Arithmetic & bitwise      — ADD, SUB, MUL, DIV, …
#   0x30–0x3F  Comparison                — CMP_EQ, CMP_LT, …
#   0x40–0x4F  Control flow              — JUMP, JUMP_IF_TRUE, LOOP, …
#   0x50–0x5F  Function / call           — CALL, RETURN, CALL_BUILTIN
#   0x60–0x6F  Variables / scope         — LOAD_GLOBAL, STORE_GLOBAL, …
#   0x70–0x7F  Object / property         — CREATE_OBJECT, LOAD_PROPERTY, …
#   0x80–0x8F  Array                     — CREATE_ARRAY, LOAD_ELEMENT, …
#   0x90–0x9F  Type / coercion           — TYPEOF, TO_NUMBER, TO_STRING
#   0xA0–0xAF  Logical                   — LOGICAL_OR, LOGICAL_AND, …
#   0xB0–0xBF  I/O                       — PRINT
#   0xFF       VM control                — HALT
#
# The encoding mirrors V8's real Ignition ISA closely enough to be
# educational — see https://v8.dev/blog/ignition-interpreter for the
# original design notes.
#
module CodingAdventures
  module RegisterVM
    module Opcodes
      # -----------------------------------------------------------------------
      # 0x00–0x0F  Accumulator Loads
      # -----------------------------------------------------------------------
      # "LDA" stands for "LoaD to Accumulator" — the operand or a constant
      # is placed directly into acc. No stack involved.

      # Load a constant from the constants pool: acc = constants[operand[0]]
      LDA_CONSTANT   = 0x00

      # Load the integer 0: acc = 0  (common enough to deserve its own opcode)
      LDA_ZERO       = 0x01

      # Load true / false / null / undefined into the accumulator
      LDA_TRUE       = 0x02
      LDA_FALSE      = 0x03
      LDA_NULL       = 0x04
      LDA_UNDEFINED  = 0x05

      # Load the accumulator from a register: acc = registers[operand[0]]
      LDAR           = 0x06

      # -----------------------------------------------------------------------
      # 0x10–0x1F  Register Moves
      # -----------------------------------------------------------------------
      # "STAR" = "STore Accumulator to Register"

      # Store accumulator to register: registers[operand[0]] = acc
      STAR           = 0x10

      # Copy one register to another: registers[dst] = registers[src]
      MOV            = 0x11

      # -----------------------------------------------------------------------
      # 0x20–0x2F  Arithmetic & Bitwise
      # -----------------------------------------------------------------------
      # Binary operations: result is always written back to the accumulator.
      # operand[0] = register index (right-hand side); acc = left-hand side.
      # A feedback_slot index may be provided for type profiling.
      #
      # Example — ADD:
      #   left  = acc
      #   right = registers[operand[0]]
      #   acc   = left + right   (numeric addition or string concatenation)
      #
      # Truth table for ADD with types:
      #
      #   left type   right type   result
      #   ---------   ----------   ------
      #   number      number       number (arithmetic sum)
      #   string      anything     string (coerce right to string, concatenate)
      #   anything    string       string (coerce left to string, concatenate)

      ADD            = 0x20   # acc = acc + reg
      SUB            = 0x21   # acc = acc - reg
      MUL            = 0x22   # acc = acc * reg
      DIV            = 0x23   # acc = acc / reg  (float division; raises on div-by-zero)
      MOD            = 0x24   # acc = acc % reg
      EXP            = 0x25   # acc = acc ** reg

      # Bitwise integer operations (Ruby's Integer handles arbitrary precision,
      # but real JITs usually clamp to 32-bit for performance).
      BIT_AND        = 0x26   # acc = acc & reg
      BIT_OR         = 0x27   # acc = acc | reg
      BIT_XOR        = 0x28   # acc = acc ^ reg
      BIT_NOT        = 0x29   # acc = ~acc  (unary; no register operand)
      SHIFT_LEFT     = 0x2A   # acc = acc << reg
      SHIFT_RIGHT    = 0x2B   # acc = acc >> reg  (arithmetic / sign-preserving)
      SHIFT_RIGHT_U  = 0x2C   # acc = acc >>> reg (logical / zero-fill, Ruby: acc >> reg & 0xFFFF_FFFF)

      # Unary arithmetic
      NEG            = 0x2D   # acc = -acc
      INC            = 0x2E   # acc = acc + 1
      DEC            = 0x2F   # acc = acc - 1

      # -----------------------------------------------------------------------
      # 0x30–0x3F  Comparison
      # -----------------------------------------------------------------------
      # Each comparison leaves 0 (false) or 1 (true) in the accumulator.
      # The right-hand operand is in registers[operand[0]]; acc is the left.
      #
      #   CMP_EQ: acc = (acc == reg) ? true : false
      #
      # Note: we use Ruby's `==` semantics here, not strict triple-equals.

      CMP_EQ         = 0x30   # acc == reg
      CMP_NEQ        = 0x31   # acc != reg
      CMP_LT         = 0x32   # acc < reg
      CMP_LTE        = 0x33   # acc <= reg
      CMP_GT         = 0x34   # acc > reg
      CMP_GTE        = 0x35   # acc >= reg

      # Test / predicate instructions (accumulator only, no register operand)
      TEST_NULL      = 0x36   # acc = (acc == nil) ? true : false
      TEST_UNDEFINED = 0x37   # acc = acc.equal?(UNDEFINED) ? true : false
      TEST_BOOLEAN   = 0x38   # acc = (acc == true || acc == false) ? true : false
      TEST_NUMBER    = 0x39   # acc = acc.is_a?(Numeric) ? true : false
      TEST_STRING    = 0x3A   # acc = acc.is_a?(String)  ? true : false

      # -----------------------------------------------------------------------
      # 0x40–0x4F  Control Flow
      # -----------------------------------------------------------------------
      # All jump targets are absolute instruction indices (integers), NOT
      # relative byte offsets. This simplifies the assembler greatly.

      # Unconditional jump: ip = operand[0]
      JUMP           = 0x40

      # Jump if accumulator is truthy / falsy
      JUMP_IF_TRUE   = 0x41
      JUMP_IF_FALSE  = 0x42

      # Jump if accumulator is null or undefined (common JS null-check pattern)
      JUMP_IF_NULL   = 0x43
      JUMP_IF_NOT_NULL = 0x44

      # Back-edge loop jump (semantically identical to JUMP, but the name tells
      # the runtime "this is a loop back-edge" so it can trigger OSR / tier-up).
      LOOP           = 0x45

      # -----------------------------------------------------------------------
      # 0x50–0x5F  Function / Call
      # -----------------------------------------------------------------------
      # CALL expects:
      #   operand[0] = register that holds the VMFunction object
      #   operand[1] = number of argument registers
      #   operand[2] = index of first argument register
      #
      # The return value is placed in the accumulator of the calling frame.

      CALL           = 0x50

      # Return from the current frame; acc becomes the caller's accumulator.
      RETURN         = 0x51

      # Call a registered built-in function (host-provided):
      #   operand[0] = name index in the code object's names array
      #   operand[1] = number of argument registers
      #   operand[2] = index of first argument register
      CALL_BUILTIN   = 0x52

      # Create a closure (VMFunction) wrapping the current context:
      #   operand[0] = index of nested CodeObject in constants pool
      # acc = VMFunction.new(code: constants[operand[0]], context: frame.context)
      CREATE_CLOSURE = 0x53

      # -----------------------------------------------------------------------
      # 0x60–0x6F  Variables / Scope Chain
      # -----------------------------------------------------------------------
      # Global variables are stored in the interpreter's @globals hash.
      # Context slots model lexical scope (closures): each Context holds
      # an array of slots and a pointer to its parent (the enclosing scope).
      #
      #   LOAD_CONTEXT_SLOT depth, idx  ->  walks `depth` parent links, reads slots[idx]
      #   STORE_CONTEXT_SLOT depth, idx ->  walks `depth` parent links, writes slots[idx]

      LOAD_GLOBAL    = 0x60   # acc = @globals[names[operand[0]]]
      STORE_GLOBAL   = 0x61   # @globals[names[operand[0]]] = acc

      LOAD_CONTEXT_SLOT  = 0x62   # acc = scope_chain[depth][idx]
      STORE_CONTEXT_SLOT = 0x63   # scope_chain[depth][idx] = acc

      PUSH_CONTEXT   = 0x64   # Push a new scope frame (operand[0] = slot count)
      POP_CONTEXT    = 0x65   # Pop the innermost scope frame

      # -----------------------------------------------------------------------
      # 0x70–0x7F  Object / Property Operations
      # -----------------------------------------------------------------------
      # VMObject is a Hash-backed JS-style object with a hidden class ID.
      # Hidden classes allow the JIT to predict the layout of objects and
      # emit fast inline caches instead of hash lookups.
      #
      # LOAD_PROPERTY and STORE_PROPERTY record their hidden class ID in the
      # feedback vector so a future JIT can specialize for specific shapes.

      CREATE_OBJECT     = 0x70   # acc = VMObject.new(hidden_class_id: fresh_id, properties: {})
      LOAD_PROPERTY     = 0x71   # acc = acc.properties[names[operand[0]]]
      STORE_PROPERTY    = 0x72   # acc.properties[names[operand[0]]] = registers[operand[1]]
      DELETE_PROPERTY   = 0x73   # acc.properties.delete(names[operand[0]]);acc=true
      HAS_PROPERTY      = 0x74   # acc = acc.properties.key?(names[operand[0]])

      # -----------------------------------------------------------------------
      # 0x80–0x8F  Array Operations
      # -----------------------------------------------------------------------
      CREATE_ARRAY      = 0x80   # acc = []  (empty array)
      LOAD_ELEMENT      = 0x81   # acc = acc[registers[operand[0]]]  (integer index)
      STORE_ELEMENT     = 0x82   # acc[registers[operand[0]]] = registers[operand[1]]
      PUSH_ELEMENT      = 0x83   # acc << registers[operand[0]]
      ARRAY_LENGTH      = 0x84   # acc = acc.length

      # -----------------------------------------------------------------------
      # 0x90–0x9F  Type / Coercion
      # -----------------------------------------------------------------------
      TYPEOF            = 0x90   # acc = typeof_value(acc) — string name of type
      TO_NUMBER         = 0x91   # acc = coerce acc to Numeric (Integer or Float)
      TO_STRING         = 0x92   # acc = acc.to_s
      TO_BOOLEAN        = 0x93   # acc = truthy?(acc)

      # -----------------------------------------------------------------------
      # 0xA0–0xAF  Logical Operators
      # -----------------------------------------------------------------------
      # These are short-circuiting in real JS. Here we evaluate eagerly because
      # both sides are already computed into registers.
      #
      # LOGICAL_OR:  acc = truthy?(acc) ? acc : registers[operand[0]]
      # LOGICAL_AND: acc = truthy?(acc) ? registers[operand[0]] : acc
      # LOGICAL_NOT: acc = !truthy?(acc)

      LOGICAL_OR        = 0xA0
      LOGICAL_AND       = 0xA1
      LOGICAL_NOT       = 0xA2

      # Nullish coalescing: acc = (acc == nil || acc.equal?(UNDEFINED)) ? reg : acc
      NULLISH_COALESCE  = 0xA3

      # -----------------------------------------------------------------------
      # 0xB0–0xBF  I/O
      # -----------------------------------------------------------------------
      # PRINT sends the accumulator's string representation to @output.
      # This is analogous to Python's PRINT_ITEM opcode.

      PRINT             = 0xB0   # @output << acc.to_s; acc unchanged

      # -----------------------------------------------------------------------
      # 0xFF  VM Control
      # -----------------------------------------------------------------------

      HALT              = 0xFF   # Stop the current frame; return accumulator

      # -----------------------------------------------------------------------
      # Name lookup — for disassemblers, error messages, and trace output
      # -----------------------------------------------------------------------
      NAMES = {
        LDA_CONSTANT   => "LDA_CONSTANT",
        LDA_ZERO       => "LDA_ZERO",
        LDA_TRUE       => "LDA_TRUE",
        LDA_FALSE      => "LDA_FALSE",
        LDA_NULL       => "LDA_NULL",
        LDA_UNDEFINED  => "LDA_UNDEFINED",
        LDAR           => "LDAR",
        STAR           => "STAR",
        MOV            => "MOV",
        ADD            => "ADD",
        SUB            => "SUB",
        MUL            => "MUL",
        DIV            => "DIV",
        MOD            => "MOD",
        EXP            => "EXP",
        BIT_AND        => "BIT_AND",
        BIT_OR         => "BIT_OR",
        BIT_XOR        => "BIT_XOR",
        BIT_NOT        => "BIT_NOT",
        SHIFT_LEFT     => "SHIFT_LEFT",
        SHIFT_RIGHT    => "SHIFT_RIGHT",
        SHIFT_RIGHT_U  => "SHIFT_RIGHT_U",
        NEG            => "NEG",
        INC            => "INC",
        DEC            => "DEC",
        CMP_EQ         => "CMP_EQ",
        CMP_NEQ        => "CMP_NEQ",
        CMP_LT         => "CMP_LT",
        CMP_LTE        => "CMP_LTE",
        CMP_GT         => "CMP_GT",
        CMP_GTE        => "CMP_GTE",
        TEST_NULL      => "TEST_NULL",
        TEST_UNDEFINED => "TEST_UNDEFINED",
        TEST_BOOLEAN   => "TEST_BOOLEAN",
        TEST_NUMBER    => "TEST_NUMBER",
        TEST_STRING    => "TEST_STRING",
        JUMP           => "JUMP",
        JUMP_IF_TRUE   => "JUMP_IF_TRUE",
        JUMP_IF_FALSE  => "JUMP_IF_FALSE",
        JUMP_IF_NULL   => "JUMP_IF_NULL",
        JUMP_IF_NOT_NULL => "JUMP_IF_NOT_NULL",
        LOOP           => "LOOP",
        CALL           => "CALL",
        RETURN         => "RETURN",
        CALL_BUILTIN   => "CALL_BUILTIN",
        CREATE_CLOSURE => "CREATE_CLOSURE",
        LOAD_GLOBAL    => "LOAD_GLOBAL",
        STORE_GLOBAL   => "STORE_GLOBAL",
        LOAD_CONTEXT_SLOT  => "LOAD_CONTEXT_SLOT",
        STORE_CONTEXT_SLOT => "STORE_CONTEXT_SLOT",
        PUSH_CONTEXT   => "PUSH_CONTEXT",
        POP_CONTEXT    => "POP_CONTEXT",
        CREATE_OBJECT  => "CREATE_OBJECT",
        LOAD_PROPERTY  => "LOAD_PROPERTY",
        STORE_PROPERTY => "STORE_PROPERTY",
        DELETE_PROPERTY => "DELETE_PROPERTY",
        HAS_PROPERTY   => "HAS_PROPERTY",
        CREATE_ARRAY   => "CREATE_ARRAY",
        LOAD_ELEMENT   => "LOAD_ELEMENT",
        STORE_ELEMENT  => "STORE_ELEMENT",
        PUSH_ELEMENT   => "PUSH_ELEMENT",
        ARRAY_LENGTH   => "ARRAY_LENGTH",
        TYPEOF         => "TYPEOF",
        TO_NUMBER      => "TO_NUMBER",
        TO_STRING      => "TO_STRING",
        TO_BOOLEAN     => "TO_BOOLEAN",
        LOGICAL_OR     => "LOGICAL_OR",
        LOGICAL_AND    => "LOGICAL_AND",
        LOGICAL_NOT    => "LOGICAL_NOT",
        NULLISH_COALESCE => "NULLISH_COALESCE",
        PRINT          => "PRINT",
        HALT           => "HALT"
      }.freeze

      # Return the human-readable name of an opcode, or a hex placeholder if unknown.
      #
      # @param opcode [Integer] the opcode byte value
      # @return [String] e.g. "ADD", "HALT", or "0x??(99)"
      def self.name(opcode)
        NAMES.fetch(opcode) { "0x??(%d)" % opcode }
      end
    end
  end
end
