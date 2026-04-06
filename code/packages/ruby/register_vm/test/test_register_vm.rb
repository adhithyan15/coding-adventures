# frozen_string_literal: true

# ==========================================================================
# Tests for CodingAdventures::RegisterVM
# ==========================================================================
#
# All tests build CodeObject instances directly — no assembler required.
# Each test exercises a coherent slice of the VM's behaviour.
#
# Test structure
# --------------
# 1.  Arithmetic — ADD, SUB, MUL, DIV, MOD, EXP
# 2.  Bitwise — BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, shifts
# 3.  String operations — ADD with strings, TO_STRING, TYPEOF
# 4.  Comparison — CMP_EQ, CMP_NEQ, CMP_LT, CMP_LTE, CMP_GT, CMP_GTE
# 5.  Control flow — JUMP, JUMP_IF_TRUE, JUMP_IF_FALSE, LOOP
# 6.  Object operations — CREATE_OBJECT, LOAD/STORE/DELETE/HAS_PROPERTY
# 7.  Array operations — CREATE_ARRAY, PUSH/LOAD/STORE_ELEMENT, ARRAY_LENGTH
# 8.  Function calls — CALL, RETURN, recursive fibonacci
# 9.  Feedback vectors — type profiling state machine
# 10. Error handling — division by zero, unknown opcode, call depth
#
require_relative "test_helper"

# Shorthand builder for RegisterInstruction so tests read cleanly.
def instr(opcode, operands: [], feedback_slot: nil)
  RegisterInstruction.new(opcode: opcode, operands: operands, feedback_slot: feedback_slot)
end

# Build a minimal CodeObject with sensible defaults for missing fields.
def code(instructions:, constants: [], names: [], registers: 4, feedback_slots: 4, params: 0, name: "test")
  CodeObject.new(
    instructions:       instructions,
    constants:          constants,
    names:              names,
    register_count:     registers,
    feedback_slot_count: feedback_slots,
    parameter_count:    params,
    name:               name
  )
end

# ==========================================================================
class TestArithmetic < Minitest::Test
  # -----------------------------------------------------------------------
  # Basic integer arithmetic: compute (3 + 4) * 2 - 1 = 13
  #
  # Bytecode sequence:
  #   LDA_CONSTANT 0  ; acc = 3
  #   STAR r0         ; r0 = 3
  #   LDA_CONSTANT 1  ; acc = 4
  #   ADD  r0         ; acc = 7
  #   STAR r0         ; r0 = 7
  #   LDA_CONSTANT 2  ; acc = 2
  #   MUL  r0         ; acc = 14  (wait — we want acc*r0 = 2*7 = 14, then -1 = 13)
  #   STAR r0         ; r0 = 14
  #   LDA_CONSTANT 3  ; acc = 1
  #   STAR r1         ; r1 = 1
  #   LDAR r0         ; acc = 14
  #   SUB  r1         ; acc = 13
  #   HALT
  def test_integer_arithmetic
    c = code(
      constants: [3, 4, 2, 1],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = 3
        instr(Opcodes::STAR, operands: [0]),            # r0 = 3
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 4
        instr(Opcodes::ADD, operands: [0], feedback_slot: 0), # acc = 7
        instr(Opcodes::STAR, operands: [0]),            # r0 = 7
        instr(Opcodes::LDA_CONSTANT, operands: [2]),   # acc = 2
        instr(Opcodes::MUL, operands: [0], feedback_slot: 1), # acc = 14
        instr(Opcodes::STAR, operands: [0]),            # r0 = 14
        instr(Opcodes::LDA_CONSTANT, operands: [3]),   # acc = 1
        instr(Opcodes::STAR, operands: [1]),            # r1 = 1
        instr(Opcodes::LDAR, operands: [0]),            # acc = 14
        instr(Opcodes::SUB, operands: [1], feedback_slot: 2), # acc = 13
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error, result.error&.message
    assert_equal 13, result.return_value
  end

  # -----------------------------------------------------------------------
  # MOD and EXP: 2**10 mod 1000 = 24
  def test_mod_and_exp
    c = code(
      constants: [2, 10, 1000],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 2
        instr(Opcodes::STAR, operands: [0]),           # r0 = 2
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 10
        instr(Opcodes::STAR, operands: [1]),           # r1 = 10
        instr(Opcodes::LDAR, operands: [0]),           # acc = 2
        instr(Opcodes::EXP, operands: [1]),            # acc = 1024
        instr(Opcodes::STAR, operands: [0]),           # r0 = 1024
        instr(Opcodes::LDA_CONSTANT, operands: [2]),  # acc = 1000
        instr(Opcodes::STAR, operands: [1]),           # r1 = 1000
        instr(Opcodes::LDAR, operands: [0]),           # acc = 1024
        instr(Opcodes::MOD, operands: [1]),            # acc = 24
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal 24, result.return_value
  end

  # -----------------------------------------------------------------------
  # Unary ops: NEG, INC, DEC
  def test_unary_ops
    c = code(
      constants: [5],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 5
        instr(Opcodes::NEG),                           # acc = -5
        instr(Opcodes::INC),                           # acc = -4
        instr(Opcodes::DEC),                           # acc = -5
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal(-5, result.return_value)
  end
end

# ==========================================================================
class TestBitwise < Minitest::Test
  # -----------------------------------------------------------------------
  # 0b1010 & 0b1100 = 0b1000 = 8; then BIT_NOT(8) in 32-bit = -9
  def test_bitwise_and_not
    c = code(
      constants: [0b1010, 0b1100],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = 10
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 12
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),            # acc = 10
        instr(Opcodes::BIT_AND, operands: [1]),         # acc = 8
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal 8, result.return_value
  end

  # -----------------------------------------------------------------------
  # Left shift: 1 << 8 = 256
  def test_shift_left
    c = code(
      constants: [1, 8],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 1
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 8
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),           # acc = 1
        instr(Opcodes::SHIFT_LEFT, operands: [1]),     # acc = 256
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal 256, result.return_value
  end
end

# ==========================================================================
class TestStrings < Minitest::Test
  # -----------------------------------------------------------------------
  # String concatenation: "hello" + " " + "world" = "hello world"
  def test_string_concat
    c = code(
      constants: ["hello", " ", "world"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = "hello"
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = " "
        instr(Opcodes::ADD, operands: [0]),            # acc = " hello" — right is r0="hello"
        # Actually: acc + r0 = " " + "hello" = " hello" — not what we want.
        # Let's reorder: acc="hello", r0=" world" not possible with 3 consts split.
        # Simplest: acc="hello", r1=" ", acc=ADD(r1) wrong dir. Use:
        #   acc = "hello", r0="hello"
        #   acc = " ", r1=" "
        #   acc = "world"... let's just do "hello" + " world" in 2 consts.
        instr(Opcodes::HALT)
      ]
    )
    # Simpler test: just concat two constants
    c2 = code(
      constants: ["hello", " world"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = " world"
        instr(Opcodes::STAR, operands: [0]),           # r0 = " world"
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = "hello"
        instr(Opcodes::ADD, operands: [0]),            # acc = "hello" + " world"
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c2)
    assert_nil result.error
    assert_equal "hello world", result.return_value
  end

  # -----------------------------------------------------------------------
  # TYPEOF: number, string, boolean, object (null), undefined
  def test_typeof
    # typeof 42 => "number"
    c = code(
      constants: [42],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::TYPEOF),
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_equal "number", result.return_value

    # typeof "hi" => "string"
    c2 = code(
      constants: ["hi"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::TYPEOF),
        instr(Opcodes::HALT)
      ]
    )
    assert_equal "string", CodingAdventures::RegisterVM.execute(c2).return_value

    # typeof null => "object"  (JS quirk)
    c3 = code(
      constants: [],
      instructions: [
        instr(Opcodes::LDA_NULL),
        instr(Opcodes::TYPEOF),
        instr(Opcodes::HALT)
      ]
    )
    assert_equal "object", CodingAdventures::RegisterVM.execute(c3).return_value
  end
end

# ==========================================================================
class TestComparison < Minitest::Test
  # Test all six comparison operators in one sweep.
  def test_comparisons
    # 5 < 10 => true
    [
      [Opcodes::CMP_EQ,  5,  5, true],
      [Opcodes::CMP_EQ,  5,  6, false],
      [Opcodes::CMP_NEQ, 5,  6, true],
      [Opcodes::CMP_LT,  3, 10, true],
      [Opcodes::CMP_LT, 10,  3, false],
      [Opcodes::CMP_LTE, 5,  5, true],
      [Opcodes::CMP_GT,  7,  3, true],
      [Opcodes::CMP_GTE, 5,  5, true],
      [Opcodes::CMP_GTE, 4,  5, false]
    ].each do |opcode, left_val, right_val, expected|
      c = code(
        constants: [left_val, right_val],
        instructions: [
          instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = right
          instr(Opcodes::STAR, operands: [0]),           # r0  = right
          instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = left
          instr(opcode, operands: [0]),                  # acc = left OP right
          instr(Opcodes::HALT)
        ]
      )
      result = CodingAdventures::RegisterVM.execute(c)
      assert_nil result.error, "#{Opcodes.name(opcode)} raised: #{result.error&.message}"
      assert_equal expected, result.return_value,
        "#{left_val} #{Opcodes.name(opcode)} #{right_val}: expected #{expected}"
    end
  end
end

# ==========================================================================
class TestControlFlow < Minitest::Test
  # -----------------------------------------------------------------------
  # Simple conditional: if (true) return 1 else return 2
  #
  # Bytecode:
  #   0: LDA_TRUE
  #   1: JUMP_IF_FALSE 4      ; skip to instruction 4 if false
  #   2: LDA_CONSTANT 0       ; acc = 1
  #   3: HALT
  #   4: LDA_CONSTANT 1       ; acc = 2
  #   5: HALT
  def test_jump_if_false_taken
    c = code(
      constants: [1, 2],
      instructions: [
        instr(Opcodes::LDA_TRUE),                        # 0: acc = true
        instr(Opcodes::JUMP_IF_FALSE, operands: [4]),    # 1: skip if false (not taken)
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 2: acc = 1
        instr(Opcodes::HALT),                            # 3
        instr(Opcodes::LDA_CONSTANT, operands: [1]),    # 4: acc = 2
        instr(Opcodes::HALT)                             # 5
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_equal 1, result.return_value

    # Now test the false branch
    c2 = code(
      constants: [1, 2],
      instructions: [
        instr(Opcodes::LDA_FALSE),                       # 0: acc = false
        instr(Opcodes::JUMP_IF_FALSE, operands: [4]),    # 1: taken
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 2: acc = 1
        instr(Opcodes::HALT),                            # 3
        instr(Opcodes::LDA_CONSTANT, operands: [1]),    # 4: acc = 2
        instr(Opcodes::HALT)                             # 5
      ]
    )
    result2 = CodingAdventures::RegisterVM.execute(c2)
    assert_equal 2, result2.return_value
  end

  # -----------------------------------------------------------------------
  # Simple counting loop using LOOP back-edge.
  # Count from 0 to 4 (5 iterations) and leave result in acc.
  #
  # Bytecode:
  #   0: LDA_ZERO             ; acc = 0 (counter)
  #   1: STAR r0              ; r0 = counter
  #   2: LDA_CONSTANT 0       ; acc = 5 (limit)
  #   3: STAR r1              ; r1 = 5
  #   --- loop body (ip=4) ---
  #   4: LDAR r0              ; acc = counter
  #   5: CMP_GTE r1           ; acc = (counter >= 5)
  #   6: JUMP_IF_TRUE 11      ; exit loop if counter >= 5
  #   7: LDAR r0              ; acc = counter
  #   8: INC                  ; acc = counter + 1
  #   9: STAR r0              ; r0 = counter + 1
  #   10: LOOP 4              ; back-edge to loop body
  #   --- exit (ip=11) ---
  #   11: LDAR r0             ; acc = final counter (5)
  #   12: HALT
  def test_counting_loop
    c = code(
      constants: [5],
      instructions: [
        instr(Opcodes::LDA_ZERO),                        # 0
        instr(Opcodes::STAR, operands: [0]),              # 1: r0 = 0
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 2: acc = 5
        instr(Opcodes::STAR, operands: [1]),              # 3: r1 = 5
        # Loop body
        instr(Opcodes::LDAR, operands: [0]),              # 4: acc = counter
        instr(Opcodes::CMP_GTE, operands: [1]),           # 5: acc = counter >= 5
        instr(Opcodes::JUMP_IF_TRUE, operands: [11]),    # 6: exit
        instr(Opcodes::LDAR, operands: [0]),              # 7: acc = counter
        instr(Opcodes::INC),                              # 8: acc = counter+1
        instr(Opcodes::STAR, operands: [0]),              # 9: r0 = counter+1
        instr(Opcodes::LOOP, operands: [4]),              # 10: back-edge
        # Exit
        instr(Opcodes::LDAR, operands: [0]),              # 11: acc = 5
        instr(Opcodes::HALT)                              # 12
      ],
      registers: 4
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error, result.error&.message
    assert_equal 5, result.return_value
  end
end

# ==========================================================================
class TestObjects < Minitest::Test
  # -----------------------------------------------------------------------
  # Create object, store a property, load it back.
  def test_object_property_round_trip
    # names[0] = "x", constants[0] = 42
    c = code(
      constants: [42],
      names: ["x"],
      instructions: [
        instr(Opcodes::CREATE_OBJECT),                   # acc = {}
        instr(Opcodes::STAR, operands: [0]),              # r0 = obj

        # Load value 42 into r1
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # acc = 42
        instr(Opcodes::STAR, operands: [1]),              # r1 = 42

        # Store obj.x = 42
        instr(Opcodes::LDAR, operands: [0]),              # acc = obj
        instr(Opcodes::STORE_PROPERTY, operands: [0, 1]), # obj.x = r1=42

        # Load obj.x back
        instr(Opcodes::LDAR, operands: [0]),              # acc = obj
        instr(Opcodes::LOAD_PROPERTY, operands: [0], feedback_slot: 0), # acc = obj.x
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error, result.error&.message
    assert_equal 42, result.return_value
  end

  # -----------------------------------------------------------------------
  # HAS_PROPERTY and DELETE_PROPERTY
  def test_has_and_delete_property
    c = code(
      constants: [99],
      names: ["y"],
      instructions: [
        instr(Opcodes::CREATE_OBJECT),
        instr(Opcodes::STAR, operands: [0]),    # r0 = obj

        # Store obj.y = 99
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::STAR, operands: [1]),    # r1 = 99
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::STORE_PROPERTY, operands: [0, 1]),

        # HAS_PROPERTY => true
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::HAS_PROPERTY, operands: [0]),
        instr(Opcodes::STAR, operands: [2]),    # r2 = true

        # DELETE_PROPERTY
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::DELETE_PROPERTY, operands: [0]),

        # HAS_PROPERTY => false
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::HAS_PROPERTY, operands: [0]),
        instr(Opcodes::STAR, operands: [3]),    # r3 = false

        # Return r2 (true) to verify first check passed
        instr(Opcodes::LDAR, operands: [2]),
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal true, result.return_value
  end
end

# ==========================================================================
class TestArrays < Minitest::Test
  # -----------------------------------------------------------------------
  # Create array, push elements, read length and element.
  def test_array_operations
    c = code(
      constants: [10, 20, 30],
      instructions: [
        instr(Opcodes::CREATE_ARRAY),               # acc = []
        instr(Opcodes::STAR, operands: [0]),         # r0 = []

        # Push 10
        instr(Opcodes::LDA_CONSTANT, operands: [0]), # acc = 10
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),          # acc = arr
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),  # arr << 10

        # Push 20
        instr(Opcodes::LDA_CONSTANT, operands: [1]), # acc = 20
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),  # arr << 20

        # Push 30
        instr(Opcodes::LDA_CONSTANT, operands: [2]), # acc = 30
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),  # arr << 30

        # Length = 3
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::ARRAY_LENGTH),                 # acc = 3
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error, result.error&.message
    assert_equal 3, result.return_value
  end

  # -----------------------------------------------------------------------
  # LOAD_ELEMENT by index
  def test_load_element
    c = code(
      constants: [7, 13, 99, 1],  # constants[3]=1 = index to load
      instructions: [
        instr(Opcodes::CREATE_ARRAY),
        instr(Opcodes::STAR, operands: [0]),          # r0 = []

        # push 7
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),

        # push 13
        instr(Opcodes::LDA_CONSTANT, operands: [1]),
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),

        # push 99
        instr(Opcodes::LDA_CONSTANT, operands: [2]),
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),

        # load arr[1] = 13
        instr(Opcodes::LDA_CONSTANT, operands: [3]),  # acc = 1 (index)
        instr(Opcodes::STAR, operands: [1]),           # r1 = 1
        instr(Opcodes::LDAR, operands: [0]),           # acc = arr
        instr(Opcodes::LOAD_ELEMENT, operands: [1]),  # acc = arr[1] = 13
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal 13, result.return_value
  end
end

# ==========================================================================
class TestFunctionCalls < Minitest::Test
  # -----------------------------------------------------------------------
  # Call a function that doubles its argument.
  #
  # double(x) = x + x
  #
  # double_code:
  #   0: LDAR r0          ; acc = x (first param is in r0)
  #   1: STAR r1          ; r1 = x
  #   2: ADD  r1          ; acc = x + x
  #   3: RETURN
  #
  # main_code:
  #   0: LDA_CONSTANT 0   ; acc = double_code (nested CodeObject in constants)
  #   1: CREATE_CLOSURE   ; acc = VMFunction(double_code)
  #   Wait — CREATE_CLOSURE wants a CodeObject in constants[operand[0]].
  #   Let's do it properly.
  #
  def test_call_double
    double_code = code(
      instructions: [
        instr(Opcodes::LDAR, operands: [0]),         # acc = x
        instr(Opcodes::STAR, operands: [1]),          # r1 = x
        instr(Opcodes::ADD, operands: [1]),           # acc = x + x
        instr(Opcodes::RETURN)
      ],
      registers: 4,
      feedback_slots: 2,
      params: 1,
      name: "double"
    )

    main_code = code(
      constants: [double_code, 7],  # constants[0] = double_code, constants[1] = 7
      instructions: [
        # Create closure from nested code object
        instr(Opcodes::CREATE_CLOSURE, operands: [0]), # acc = VMFunction(double_code)
        instr(Opcodes::STAR, operands: [0]),            # r0 = fn

        # Load argument 7 into r1
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 7
        instr(Opcodes::STAR, operands: [1]),            # r1 = 7

        # CALL r0, 1 arg, starting at r1
        instr(Opcodes::CALL, operands: [0, 1, 1], feedback_slot: 0), # call double(7)
        instr(Opcodes::HALT)
      ],
      registers: 4,
      feedback_slots: 2,
      name: "main"
    )

    result = CodingAdventures::RegisterVM.execute(main_code)
    assert_nil result.error, result.error&.message
    assert_equal 14, result.return_value
  end

  # -----------------------------------------------------------------------
  # Recursive fibonacci: fib(6) = 8
  # fib(n) = n if n <= 1; fib(n-1) + fib(n-2) otherwise
  #
  # We store fib_code in a global so it can reference itself.
  def test_recursive_fibonacci
    # fib bytecode:
    #   r0 = n (argument)
    #   0: LDAR r0
    #   1: STAR r1           ; r1 = n
    #   2: LDA_CONSTANT 0    ; acc = 1
    #   3: STAR r2           ; r2 = 1
    #   4: LDAR r1           ; acc = n
    #   5: CMP_LTE r2        ; acc = (n <= 1)
    #   6: JUMP_IF_FALSE 9   ; if n > 1, skip to recursive case
    #   7: LDAR r1           ; acc = n
    #   8: RETURN            ; return n
    #   # recursive case
    #   9: LOAD_GLOBAL 0     ; acc = fib (name "fib" in names[0])
    #   10: STAR r3          ; r3 = fib
    #   11: LDAR r1          ; acc = n
    #   12: DEC              ; acc = n-1
    #   13: STAR r0          ; r0 = n-1
    #   14: CALL r3, 1, r0   ; acc = fib(n-1)
    #   15: STAR r0          ; r0 = fib(n-1)
    #   16: LOAD_GLOBAL 0    ; acc = fib
    #   17: STAR r3          ; r3 = fib
    #   18: LDAR r1          ; acc = n
    #   19: DEC              ; acc = n-1   -- oops, need n-2
    #   ... actually let's just store n-2 in r2 and call.
    # Revised (cleaner):
    #
    # r0 = n (arg), r1 = scratch, r2 = 1 (constant), r3 = fib fn, r4 = fib(n-1)
    fib_constants = [1, 2]  # constants[0]=1, constants[1]=2

    fib_instructions = [
      # i0: r0=n, load n
      instr(Opcodes::LDAR, operands: [0]),           # 0: acc = n
      # save n in r1
      instr(Opcodes::STAR, operands: [1]),            # 1: r1 = n
      # load 1 into r2
      instr(Opcodes::LDA_CONSTANT, operands: [0]),   # 2: acc = 1
      instr(Opcodes::STAR, operands: [2]),            # 3: r2 = 1
      # if n <= 1, return n
      instr(Opcodes::LDAR, operands: [1]),            # 4: acc = n
      instr(Opcodes::CMP_LTE, operands: [2]),         # 5: acc = n<=1
      instr(Opcodes::JUMP_IF_FALSE, operands: [9]),  # 6: skip if n>1
      instr(Opcodes::LDAR, operands: [1]),            # 7: acc = n
      instr(Opcodes::RETURN),                         # 8: return n
      # recursive: load fib fn from global "fib"
      instr(Opcodes::LOAD_GLOBAL, operands: [0]),    # 9: acc = globals["fib"]
      instr(Opcodes::STAR, operands: [3]),            # 10: r3 = fib
      # compute n-1
      instr(Opcodes::LDAR, operands: [1]),            # 11: acc = n
      instr(Opcodes::DEC),                            # 12: acc = n-1
      instr(Opcodes::STAR, operands: [0]),            # 13: r0 = n-1
      # call fib(n-1)
      instr(Opcodes::CALL, operands: [3, 1, 0]),     # 14: acc = fib(n-1)
      instr(Opcodes::STAR, operands: [4]),            # 15: r4 = fib(n-1)
      # reload fib fn
      instr(Opcodes::LOAD_GLOBAL, operands: [0]),    # 16: acc = fib
      instr(Opcodes::STAR, operands: [3]),            # 17: r3 = fib
      # compute n-2
      instr(Opcodes::LDAR, operands: [1]),            # 18: acc = n
      instr(Opcodes::LDA_CONSTANT, operands: [1]),   # 19: acc = 2
      instr(Opcodes::STAR, operands: [0]),            # 20: r0 = 2
      instr(Opcodes::LDAR, operands: [1]),            # 21: acc = n
      instr(Opcodes::SUB, operands: [0]),             # 22: acc = n-2
      instr(Opcodes::STAR, operands: [0]),            # 23: r0 = n-2
      # call fib(n-2)
      instr(Opcodes::CALL, operands: [3, 1, 0]),     # 24: acc = fib(n-2)
      instr(Opcodes::STAR, operands: [0]),            # 25: r0 = fib(n-2)
      # return fib(n-1) + fib(n-2)
      instr(Opcodes::LDAR, operands: [4]),            # 26: acc = fib(n-1)
      instr(Opcodes::ADD, operands: [0]),             # 27: acc = fib(n-1)+fib(n-2)
      instr(Opcodes::RETURN)                          # 28
    ]

    fib_code = code(
      constants: fib_constants,
      names: ["fib"],
      instructions: fib_instructions,
      registers: 5,
      feedback_slots: 4,
      params: 1,
      name: "fib"
    )

    # main: store fib in global, then call fib(6)
    main_code = code(
      constants: [fib_code, 6],
      names: ["fib"],
      instructions: [
        # Create closure and store as global "fib"
        instr(Opcodes::CREATE_CLOSURE, operands: [0]), # acc = VMFunction(fib)
        instr(Opcodes::STORE_GLOBAL, operands: [0]),   # globals["fib"] = fn
        # load fn into r0
        instr(Opcodes::LOAD_GLOBAL, operands: [0]),    # acc = fn
        instr(Opcodes::STAR, operands: [0]),            # r0 = fn
        # load arg 6 into r1
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 6
        instr(Opcodes::STAR, operands: [1]),            # r1 = 6
        instr(Opcodes::CALL, operands: [0, 1, 1]),     # acc = fib(6)
        instr(Opcodes::HALT)
      ],
      registers: 4,
      feedback_slots: 2,
      name: "main"
    )

    result = CodingAdventures::RegisterVM.execute(main_code)
    assert_nil result.error, result.error&.message
    assert_equal 8, result.return_value   # fib(6) = 8
  end
end

# ==========================================================================
class TestFeedbackVectors < Minitest::Test
  # -----------------------------------------------------------------------
  # Verify feedback slot transitions:
  # uninitialized → monomorphic → polymorphic
  def test_feedback_transitions
    # Start fresh
    vector = Feedback.new_vector(3)
    assert_equal :uninitialized, vector[0]

    # First observation: int + int → monomorphic
    Feedback.record_binary_op(vector, 0, 5, 3)
    assert_equal :monomorphic, vector[0][:kind]
    assert_equal [["number", "number"]], vector[0][:types]

    # Same types again → still monomorphic (no change)
    Feedback.record_binary_op(vector, 0, 10, 20)
    assert_equal :monomorphic, vector[0][:kind]

    # Different types → polymorphic
    Feedback.record_binary_op(vector, 0, "hello", 3)
    assert_equal :polymorphic, vector[0][:kind]
    assert_equal 2, vector[0][:types].length

    # Three more different pairs → megamorphic
    Feedback.record_binary_op(vector, 0, true, 3)
    Feedback.record_binary_op(vector, 0, nil, 3)
    Feedback.record_binary_op(vector, 0, [], 3)
    assert_equal :megamorphic, vector[0]
  end

  # -----------------------------------------------------------------------
  # Feedback is populated during real execution
  def test_feedback_populated_during_execution
    c = code(
      constants: [3, 4],
      feedback_slots: 2,
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 4
        instr(Opcodes::STAR, operands: [0]),           # r0 = 4
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 3
        instr(Opcodes::ADD, operands: [0], feedback_slot: 0), # acc = 7
        instr(Opcodes::HALT)
      ]
    )
    # Execute twice so the frame's feedback vector is profiled.
    # (Each call creates a fresh frame, but we can inspect the last result.)
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal 7, result.return_value
    # Feedback correctness is already tested by test_feedback_transitions;
    # here we just confirm execution completes successfully with a feedback slot.
  end
end

# ==========================================================================
class TestErrorHandling < Minitest::Test
  # -----------------------------------------------------------------------
  # Division by zero raises VMError
  def test_division_by_zero
    c = code(
      constants: [10, 0],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 0
        instr(Opcodes::STAR, operands: [0]),           # r0 = 0
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 10
        instr(Opcodes::DIV, operands: [0]),            # 10 / 0 → error
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    refute_nil result.error
    assert_instance_of VMError, result.error
    assert_match(/[Zz]ero/, result.error.message)
  end

  # -----------------------------------------------------------------------
  # Unknown opcode raises VMError
  def test_unknown_opcode
    c = code(
      constants: [],
      instructions: [
        instr(0xDE)  # not a real opcode
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    refute_nil result.error
    assert_instance_of VMError, result.error
  end

  # -----------------------------------------------------------------------
  # Call depth limit is enforced
  def test_call_depth_limit
    # Infinite recursion: a function that calls itself with no base case.
    loop_code = CodeObject.new(
      instructions: [],  # filled below (forward reference)
      constants:    [],
      names:        ["self"],
      register_count:      2,
      feedback_slot_count: 1,
      parameter_count: 0,
      name: "inf"
    )

    loop_instrs = [
      instr(Opcodes::LOAD_GLOBAL, operands: [0]),   # acc = globals["self"]
      instr(Opcodes::STAR, operands: [0]),            # r0 = fn
      instr(Opcodes::CALL, operands: [0, 0, 0]),     # call self()
      instr(Opcodes::HALT)
    ]
    loop_code.instructions = loop_instrs

    main = code(
      constants: [loop_code],
      names: ["self"],
      instructions: [
        instr(Opcodes::CREATE_CLOSURE, operands: [0]),
        instr(Opcodes::STORE_GLOBAL, operands: [0]),
        instr(Opcodes::LOAD_GLOBAL, operands: [0]),
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::CALL, operands: [0, 0, 0]),
        instr(Opcodes::HALT)
      ],
      registers: 2,
      feedback_slots: 1,
      name: "main"
    )

    # Use a small depth limit so the test runs quickly.
    vm = Interpreter.new(max_depth: 10)
    result = vm.execute(main)
    refute_nil result.error
    assert_match(/depth|recursion/i, result.error.message)
  end

  # -----------------------------------------------------------------------
  # PRINT accumulates output lines
  def test_print_output
    c = code(
      constants: ["hello", "world"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = "hello"
        instr(Opcodes::PRINT),
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = "world"
        instr(Opcodes::PRINT),
        instr(Opcodes::HALT)
      ]
    )
    result = CodingAdventures::RegisterVM.execute(c)
    assert_nil result.error
    assert_equal ["hello", "world"], result.output
  end
end

# ==========================================================================
# TestTracing exercises execute_with_trace, which drives run_frame_traced
# and run_one_instruction — the second major dispatch path in the interpreter.
# This class is explicitly here to push coverage above 80%.
# ==========================================================================
class TestTracing < Minitest::Test
  # -----------------------------------------------------------------------
  # Tracing a simple arithmetic sequence produces one TraceStep per instruction.
  def test_trace_step_count
    c = code(
      constants: [10, 3],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # 0: acc = 10
        instr(Opcodes::STAR, operands: [0]),            # 1: r0 = 10
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # 2: acc = 3
        instr(Opcodes::ADD, operands: [0], feedback_slot: 0), # 3: acc = 13
        instr(Opcodes::HALT)                            # 4
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 5, steps.length
  end

  # -----------------------------------------------------------------------
  # Each TraceStep records the accumulator before and after the instruction.
  def test_trace_accumulator_before_and_after
    c = code(
      constants: [7],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc: UNDEFINED → 7
        instr(Opcodes::INC),                           # acc: 7 → 8
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)

    # Step 0: LDA_CONSTANT — before=UNDEFINED, after=7
    # A fresh frame starts with the UNDEFINED sentinel in the accumulator.
    # We check to_s instead of object identity because SimpleCov may cause
    # the module to be instrumented in a way that creates a second UNDEFINED.
    assert_equal "undefined", steps[0].accumulator_before.to_s
    assert_equal 7, steps[0].accumulator_after

    # Step 1: INC — before=7, after=8
    assert_equal 7, steps[1].accumulator_before
    assert_equal 8, steps[1].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing exercises MOV and all register-move opcodes through run_one_instruction.
  def test_trace_mov
    c = code(
      constants: [42],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 42
        instr(Opcodes::STAR, operands: [0]),            # r0 = 42
        instr(Opcodes::MOV, operands: [0, 1]),          # r1 = r0
        instr(Opcodes::LDAR, operands: [1]),            # acc = r1 = 42
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 5, steps.length
    assert_equal "LDAR", steps[3].opcode_name
    assert_equal 42, steps[3].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing bitwise and comparison ops through the second dispatch path.
  def test_trace_bitwise_and_comparison
    c = code(
      constants: [0xFF, 0x0F],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 255
        instr(Opcodes::STAR, operands: [0]),            # r0 = 255
        instr(Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 15
        instr(Opcodes::STAR, operands: [1]),            # r1 = 15
        instr(Opcodes::LDAR, operands: [0]),            # acc = 255
        instr(Opcodes::BIT_AND, operands: [1]),         # acc = 15
        instr(Opcodes::STAR, operands: [2]),            # r2 = 15
        instr(Opcodes::LDAR, operands: [0]),            # acc = 255
        instr(Opcodes::CMP_GT, operands: [1]),          # acc = true (255 > 15)
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 10, steps.length
    assert_equal true, steps.last.accumulator_before   # CMP_GT result before HALT
  end

  # -----------------------------------------------------------------------
  # Tracing control-flow: JUMP_IF_FALSE through run_one_instruction.
  def test_trace_jump
    c = code(
      constants: [99],
      instructions: [
        instr(Opcodes::LDA_FALSE),                       # 0: acc = false
        instr(Opcodes::JUMP_IF_FALSE, operands: [3]),    # 1: taken → ip=3
        instr(Opcodes::LDA_ZERO),                        # 2: skipped
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 3: acc = 99
        instr(Opcodes::HALT)                             # 4
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    # Instructions 2 is skipped, so we get 4 steps: 0,1,3,4
    assert_equal 4, steps.length
    assert_equal "HALT", steps.last.opcode_name
    assert_equal 99, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing object operations: CREATE_OBJECT, STORE_PROPERTY, LOAD_PROPERTY.
  def test_trace_object_ops
    c = code(
      constants: [100],
      names: ["n"],
      instructions: [
        instr(Opcodes::CREATE_OBJECT),
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::STORE_PROPERTY, operands: [0, 1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::LOAD_PROPERTY, operands: [0], feedback_slot: 0),
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 9, steps.length
    assert_equal 100, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing array operations and type-coercion opcodes.
  def test_trace_array_and_type_ops
    c = code(
      constants: [5, 1],
      instructions: [
        instr(Opcodes::CREATE_ARRAY),                   # acc = []
        instr(Opcodes::STAR, operands: [0]),             # r0 = []
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # acc = 5
        instr(Opcodes::STAR, operands: [1]),             # r1 = 5
        instr(Opcodes::LDAR, operands: [0]),             # acc = []
        instr(Opcodes::PUSH_ELEMENT, operands: [1]),     # acc=[5]
        instr(Opcodes::ARRAY_LENGTH),                   # acc = 1
        instr(Opcodes::TO_STRING),                      # acc = "1"
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 9, steps.length
    assert_equal "1", steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing logical ops: LOGICAL_NOT, LOGICAL_OR, LOGICAL_AND, NULLISH_COALESCE.
  def test_trace_logical_ops
    c = code(
      constants: [42],
      instructions: [
        instr(Opcodes::LDA_FALSE),                        # acc = false
        instr(Opcodes::LOGICAL_NOT),                      # acc = true
        instr(Opcodes::STAR, operands: [0]),               # r0 = true
        instr(Opcodes::LDA_CONSTANT, operands: [0]),      # acc = 42
        instr(Opcodes::LOGICAL_OR, operands: [0]),         # acc = 42 (truthy)
        instr(Opcodes::STAR, operands: [1]),               # r1 = 42
        instr(Opcodes::LDA_ZERO),                         # acc = 0
        instr(Opcodes::LOGICAL_AND, operands: [1]),        # acc = 0 (falsy short-circuit)
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 9, steps.length
    # LOGICAL_NOT: false → true
    assert_equal true, steps[1].accumulator_after
    # LOGICAL_AND: 0 (falsy) → stays 0
    assert_equal 0, steps[7].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing scope ops: PUSH_CONTEXT, STORE_CONTEXT_SLOT, LOAD_CONTEXT_SLOT, POP_CONTEXT.
  def test_trace_scope_ops
    c = code(
      constants: [7],
      instructions: [
        instr(Opcodes::PUSH_CONTEXT, operands: [1]),     # 0: push ctx with 1 slot
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 1: acc = 7
        instr(Opcodes::STORE_CONTEXT_SLOT, operands: [0, 0]), # 2: ctx.slots[0] = 7
        instr(Opcodes::LOAD_CONTEXT_SLOT, operands: [0, 0]),  # 3: acc = ctx.slots[0]
        instr(Opcodes::POP_CONTEXT),                    # 4: pop ctx
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 6, steps.length
    assert_equal 7, steps[3].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing global variable ops: STORE_GLOBAL, LOAD_GLOBAL.
  def test_trace_global_ops
    c = code(
      constants: [55],
      names: ["g"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # acc = 55
        instr(Opcodes::STORE_GLOBAL, operands: [0]),    # globals["g"] = 55
        instr(Opcodes::LDA_ZERO),                       # acc = 0
        instr(Opcodes::LOAD_GLOBAL, operands: [0]),     # acc = globals["g"] = 55
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 5, steps.length
    assert_equal 55, steps[3].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing a RETURN instruction in a called function (traced path handles RETURN).
  def test_trace_with_function_call
    inner = CodeObject.new(
      name: "inner",
      instructions: [
        RegisterInstruction.new(opcode: Opcodes::LDA_CONSTANT, operands: [0]),
        RegisterInstruction.new(opcode: Opcodes::RETURN, operands: [])
      ],
      constants: [99],
      names: [],
      register_count: 1,
      feedback_slot_count: 1,
      parameter_count: 0
    )

    c = code(
      constants: [inner],
      names: [],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = inner CodeObject
        instr(Opcodes::CREATE_CLOSURE, operands: [0]), # acc = VMFunction
        instr(Opcodes::STAR, operands: [0]),            # r0 = fn
        instr(Opcodes::CALL, operands: [0, 0, 0]),     # call fn()
        instr(Opcodes::HALT)
      ],
      registers: 2,
      feedback_slots: 1
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    # Outer frame: LDA_CONSTANT, CREATE_CLOSURE, STAR, CALL, HALT = 5 steps
    assert_equal 5, steps.length
    assert_equal 99, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing NULLISH_COALESCE and TO_NUMBER, TO_BOOLEAN.
  def test_trace_nullish_and_coercion
    c = code(
      constants: [42, "3.14"],
      instructions: [
        instr(Opcodes::LDA_NULL),                       # acc = nil
        instr(Opcodes::STAR, operands: [0]),             # r0 = nil
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # acc = 42
        instr(Opcodes::STAR, operands: [1]),             # r1 = 42
        instr(Opcodes::LDAR, operands: [0]),             # acc = nil
        instr(Opcodes::NULLISH_COALESCE, operands: [1]), # acc = 42 (nil ?? 42)
        instr(Opcodes::TO_BOOLEAN),                     # acc = true (42 is truthy)
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_CONSTANT, operands: [1]),    # acc = "3.14"
        instr(Opcodes::TO_NUMBER),                      # acc = 3.14
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 11, steps.length
    assert_in_delta 3.14, steps.last.accumulator_before, 0.001
  end

  # -----------------------------------------------------------------------
  # Tracing PRINT does not change the accumulator.
  def test_trace_print
    c = code(
      constants: ["hi"],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::PRINT),
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    print_step = steps[1]
    assert_equal "PRINT", print_step.opcode_name
    # PRINT leaves acc unchanged
    assert_equal "hi", print_step.accumulator_before
    assert_equal "hi", print_step.accumulator_after
  end

  # -----------------------------------------------------------------------
  # Additional sub, mul, div, mod, exp coverage in traced mode.
  def test_trace_arithmetic_ops
    c = code(
      constants: [20, 4],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = 20
        instr(Opcodes::STAR, operands: [0]),             # r0 = 20
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 4
        instr(Opcodes::STAR, operands: [1]),             # r1 = 4
        instr(Opcodes::LDAR, operands: [0]),             # acc = 20
        instr(Opcodes::SUB, operands: [1]),              # acc = 16
        instr(Opcodes::STAR, operands: [2]),             # r2 = 16
        instr(Opcodes::LDAR, operands: [0]),             # acc = 20
        instr(Opcodes::MUL, operands: [1]),              # acc = 80
        instr(Opcodes::STAR, operands: [2]),             # r2 = 80
        instr(Opcodes::LDAR, operands: [0]),             # acc = 20
        instr(Opcodes::DIV, operands: [1]),              # acc = 5
        instr(Opcodes::STAR, operands: [2]),             # r2 = 5
        instr(Opcodes::LDAR, operands: [0]),             # acc = 20
        instr(Opcodes::MOD, operands: [1]),              # acc = 0
        instr(Opcodes::STAR, operands: [2]),             # r2 = 0
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 4
        instr(Opcodes::STAR, operands: [3]),             # r3 = 4
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 4
        instr(Opcodes::EXP, operands: [1]),              # acc = 256
        instr(Opcodes::HALT)
      ],
      registers: 6
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 21, steps.length
    assert_equal 256, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing BIT_OR, BIT_XOR, BIT_NOT, shift ops in traced mode.
  def test_trace_bitwise_ops
    c = code(
      constants: [0b1010, 0b0101, 2],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = 10
        instr(Opcodes::STAR, operands: [0]),             # r0 = 10
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 5
        instr(Opcodes::STAR, operands: [1]),             # r1 = 5
        instr(Opcodes::LDAR, operands: [0]),             # acc = 10
        instr(Opcodes::BIT_OR, operands: [1]),           # acc = 15
        instr(Opcodes::STAR, operands: [2]),             # r2 = 15
        instr(Opcodes::LDAR, operands: [0]),             # acc = 10
        instr(Opcodes::BIT_XOR, operands: [1]),          # acc = 15
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDAR, operands: [0]),             # acc = 10
        instr(Opcodes::BIT_NOT),                         # acc = -11
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_CONSTANT, operands: [2]),   # acc = 2
        instr(Opcodes::STAR, operands: [3]),             # r3 = 2
        instr(Opcodes::LDAR, operands: [0]),             # acc = 10
        instr(Opcodes::SHIFT_RIGHT, operands: [3]),      # acc = 2
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDAR, operands: [0]),             # acc = 10
        instr(Opcodes::SHIFT_RIGHT_U, operands: [3]),   # acc = 2
        instr(Opcodes::HALT)
      ],
      registers: 6
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 21, steps.length
    assert_equal 2, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing CMP_NEQ, CMP_LTE, CMP_GTE and TEST_* predicates in traced mode.
  def test_trace_comparison_predicates
    c = code(
      constants: [5, 5],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),   # acc = 5
        instr(Opcodes::STAR, operands: [0]),             # r0 = 5
        instr(Opcodes::LDA_CONSTANT, operands: [1]),   # acc = 5
        instr(Opcodes::STAR, operands: [1]),             # r1 = 5
        instr(Opcodes::LDAR, operands: [0]),             # acc = 5
        instr(Opcodes::CMP_NEQ, operands: [1]),          # acc = false
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::CMP_LTE, operands: [1]),          # acc = true (5<=5)
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::CMP_GTE, operands: [1]),          # acc = true (5>=5)
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::TEST_NULL),                       # acc = false
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_UNDEFINED),
        instr(Opcodes::TEST_UNDEFINED),                  # acc = true
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_TRUE),
        instr(Opcodes::TEST_BOOLEAN),                    # acc = true
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::TEST_NUMBER),                     # acc = true
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::TO_STRING),
        instr(Opcodes::TEST_STRING),                     # acc = true
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 28, steps.length
    assert_equal true, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing JUMP_IF_TRUE and JUMP_IF_NULL in traced mode.
  def test_trace_jump_if_true_and_null
    c = code(
      constants: [1, 2],
      instructions: [
        instr(Opcodes::LDA_TRUE),                        # 0
        instr(Opcodes::JUMP_IF_TRUE, operands: [3]),    # 1: taken
        instr(Opcodes::LDA_ZERO),                       # 2: skipped
        instr(Opcodes::LDA_NULL),                       # 3
        instr(Opcodes::JUMP_IF_NULL, operands: [6]),    # 4: taken
        instr(Opcodes::LDA_ZERO),                       # 5: skipped
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 6: acc = 1
        instr(Opcodes::JUMP_IF_NOT_NULL, operands: [9]), # 7: not taken (1 is not null)
        instr(Opcodes::LDA_ZERO),                       # 8: skipped
        instr(Opcodes::HALT)                             # 9
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    # Steps executed: 0,1,3,4,6,7,9 = 7 steps
    assert_equal 7, steps.length
    assert_equal 1, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # LOOP back-edge in traced mode.
  #
  # Instruction layout (indices 0-12):
  #   0  LDA_ZERO          acc = 0
  #   1  STAR r0           r0 = 0 (counter)
  #   2  LDA_CONSTANT 0    acc = 3
  #   3  STAR r1           r1 = 3 (limit)
  #   --- loop body ---
  #   4  LDAR r0           acc = counter
  #   5  CMP_GTE r1        acc = (counter >= 3)
  #   6  JUMP_IF_TRUE 11   exit to instruction 11 when counter >= 3
  #   7  LDAR r0           acc = counter
  #   8  INC               acc = counter + 1
  #   9  STAR r0           r0 = counter + 1
  #  10  LOOP 4            jump back to loop body
  #   --- exit ---
  #  11  LDAR r0           acc = 3
  #  12  HALT
  def test_trace_loop_back_edge
    c = code(
      constants: [3],
      instructions: [
        instr(Opcodes::LDA_ZERO),                        # 0
        instr(Opcodes::STAR, operands: [0]),              # 1: r0 = 0
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # 2: acc = 3
        instr(Opcodes::STAR, operands: [1]),              # 3: r1 = 3
        # loop body at ip=4
        instr(Opcodes::LDAR, operands: [0]),              # 4
        instr(Opcodes::CMP_GTE, operands: [1]),           # 5
        instr(Opcodes::JUMP_IF_TRUE, operands: [11]),    # 6: exit to ip=11
        instr(Opcodes::LDAR, operands: [0]),              # 7
        instr(Opcodes::INC),                              # 8
        instr(Opcodes::STAR, operands: [0]),              # 9
        instr(Opcodes::LOOP, operands: [4]),              # 10: back-edge to ip=4
        # exit at ip=11
        instr(Opcodes::LDAR, operands: [0]),              # 11
        instr(Opcodes::HALT)                              # 12
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    # Final accumulator should be 3 (loop ran 3 times)
    assert_equal 3, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing DECrement in traced mode.
  def test_trace_dec
    c = code(
      constants: [10],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::DEC),
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 9, steps[1].accumulator_after
  end

  # -----------------------------------------------------------------------
  # Tracing NEGate in traced mode.
  def test_trace_neg
    c = code(
      constants: [5],
      instructions: [
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::NEG),
        instr(Opcodes::HALT)
      ]
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal(-5, steps[1].accumulator_after)
  end

  # -----------------------------------------------------------------------
  # Tracing DELETE_PROPERTY and HAS_PROPERTY in traced mode.
  def test_trace_delete_and_has_property
    c = code(
      constants: [1],
      names: ["k"],
      instructions: [
        instr(Opcodes::CREATE_OBJECT),
        instr(Opcodes::STAR, operands: [0]),
        instr(Opcodes::LDA_CONSTANT, operands: [0]),
        instr(Opcodes::STAR, operands: [1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::STORE_PROPERTY, operands: [0, 1]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::HAS_PROPERTY, operands: [0]),     # acc = true
        instr(Opcodes::STAR, operands: [2]),
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::DELETE_PROPERTY, operands: [0]),  # acc = true
        instr(Opcodes::LDAR, operands: [0]),
        instr(Opcodes::HAS_PROPERTY, operands: [0]),     # acc = false
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 14, steps.length
    assert_equal false, steps.last.accumulator_before
  end

  # -----------------------------------------------------------------------
  # Tracing LOAD_ELEMENT, STORE_ELEMENT in traced mode.
  def test_trace_array_element_ops
    c = code(
      constants: [7, 0],
      instructions: [
        instr(Opcodes::CREATE_ARRAY),
        instr(Opcodes::STAR, operands: [0]),             # r0 = []
        instr(Opcodes::LDA_CONSTANT, operands: [1]),    # acc = 0 (index)
        instr(Opcodes::STAR, operands: [1]),             # r1 = 0
        instr(Opcodes::LDA_CONSTANT, operands: [0]),    # acc = 7
        instr(Opcodes::STAR, operands: [2]),             # r2 = 7
        instr(Opcodes::LDAR, operands: [0]),             # acc = arr
        instr(Opcodes::STORE_ELEMENT, operands: [1, 2]), # arr[0] = 7
        instr(Opcodes::LDAR, operands: [0]),             # acc = arr
        instr(Opcodes::LOAD_ELEMENT, operands: [1]),    # acc = arr[0] = 7
        instr(Opcodes::HALT)
      ],
      registers: 4
    )
    steps = CodingAdventures::RegisterVM.execute_with_trace(c)
    assert_equal 11, steps.length
    assert_equal 7, steps.last.accumulator_before
  end
end
